//! The text service: the counterpart of `Sources/NuktaInputMethod/NuktaInputController.swift`.
//!
//! One per app thread. It reads keys (TSF asks first whether a key is wanted, `OnTestKeyDown`, then
//! hands it over, `OnKeyDown`), feeds them to the Bijoy engine or the phonetic composer, and writes
//! the result as a composition plus finished text (see `edit`).
//!
//! A key the layout doesn't use goes to the app, as on macOS and Linux. While a syllable or word is
//! in progress, it first finishes it: the key is taken, the text committed, and the key sent again,
//! so the app gets it after the finished text (TSF has no "finish, then pass on" of its own).

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

use windows::core::{implement, w, ComObject, IUnknownImpl, Interface, Ref, Result, BOOL, GUID, PCWSTR};
use windows::Win32::Foundation::{E_INVALIDARG, HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::SystemInformation::GetTickCount64;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_EXTENDEDKEY, VIRTUAL_KEY,
};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, IEnumTfDisplayAttributeInfo, ITfCategoryMgr, ITfComposition, ITfCompositionSink,
    ITfCompositionSink_Impl, ITfContext, ITfDisplayAttributeInfo, ITfDisplayAttributeProvider,
    ITfDisplayAttributeProvider_Impl, ITfDocumentMgr, ITfKeyEventSink, ITfKeyEventSink_Impl, ITfKeystrokeMgr,
    ITfLangBarItemMgr, ITfSource, ITfTextInputProcessor_Impl, ITfTextInputProcessorEx, ITfTextInputProcessorEx_Impl,
    ITfThreadMgr, ITfThreadMgrEventSink, ITfThreadMgrEventSink_Impl,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, GetGUIThreadInfo, GetMessageExtraInfo, TrackPopupMenuEx,
    GUITHREADINFO, MF_CHECKED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, SW_SHOWNORMAL, TPM_NONOTIFY, TPM_RETURNCMD,
    TPM_RIGHTBUTTON,
};

use super::candidates::{CandidateWindow, Event};
use super::display_attribute::{DisplayAttributeEnum, DisplayAttributeInfo};
use super::keys::KeyEvent;
use super::langbar::{self, Command, LangBarButton};
use super::{edit, guard, register, DllRef, GUID_DISPLAY_ATTRIBUTE};
use crate::bijoy::{self, Engine};
use crate::phonetic::{self, PhoneticComposer};
use crate::settings::{Layout, PopupDirection, Settings};
use crate::win;

/// Marks the keys this service sends again, so it lets them through to the app.
const RESENT_MARK: usize = 0x4E55_4B54; // "NUKT"

#[implement(ITfTextInputProcessorEx, ITfThreadMgrEventSink, ITfKeyEventSink, ITfCompositionSink, ITfDisplayAttributeProvider)]
pub(super) struct TextService {
    state: RefCell<State>,
    _dll: DllRef,
}

#[derive(Default)]
struct State {
    thread_mgr: Option<ITfThreadMgr>,
    client: u32,
    thread_mgr_cookie: Option<u32>,
    key_sink_advised: bool,
    /// The composition on screen, and the document it is in.
    composition: Option<ITfComposition>,
    composition_context: Option<ITfContext>,
    /// Bijoy: this thread's syllable engine.
    bijoy: Engine,
    /// The layout this service is typing with; follows the `layout` setting.
    layout: Layout,
    settings: Option<Settings>,
    button: Option<ComObject<LangBarButton>>,
    candidates: Option<CandidateWindow>,
    /// TSF's number for our display attribute (the underline).
    attribute: u32,
    /// Keys sent again (`resend`) and not yet seen coming back: (virtual key, when).
    resent: VecDeque<(u16, u64)>,
}

thread_local! {
    /// Phonetic typing for this thread: one composer (riti is large, and keeps one file of learned
    /// picks), made on first use so the Bijoy layout never loads riti.
    static PHONETIC: RefCell<Option<PhoneticComposer>> = const { RefCell::new(None) };
    /// Making the composer failed (riti panicked): phonetic typing is off until settings change.
    static PHONETIC_FAILED: Cell<bool> = const { Cell::new(false) };
}

/// Runs `f` on this thread's composer, making it first if needed. None if phonetic typing can't
/// run, or riti panicked (the composer is then dropped, and made again next time).
fn with_composer<R>(settings: Option<&Settings>, f: impl FnOnce(&mut PhoneticComposer) -> R) -> Option<R> {
    if PHONETIC_FAILED.with(Cell::get) {
        return None;
    }
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        PHONETIC.with(|cell| {
            let mut slot = cell.borrow_mut();
            if slot.is_none() {
                let options = settings.map(Settings::phonetic_options).unwrap_or_default();
                let mut composer = PhoneticComposer::new(options, &win::data_directory());
                composer.horizontal_navigation =
                    settings.is_some_and(|s| s.popup_direction() == PopupDirection::Horizontal);
                *slot = Some(composer);
            }
            slot.as_mut().map(f)
        })
    }));
    match result {
        Ok(value) => value,
        Err(_) => {
            PHONETIC.with(|cell| {
                if let Ok(mut slot) = cell.try_borrow_mut() {
                    *slot = None;
                }
            });
            PHONETIC_FAILED.with(|failed| failed.set(true));
            None
        }
    }
}

/// The composer only if it already exists: Bijoy must not create it.
fn with_existing_composer<R>(f: impl FnOnce(&mut PhoneticComposer) -> R) -> Option<R> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        PHONETIC.with(|cell| cell.try_borrow_mut().ok().and_then(|mut slot| slot.as_mut().map(f)))
    }));
    result.unwrap_or_else(|_| {
        PHONETIC.with(|cell| {
            if let Ok(mut slot) = cell.try_borrow_mut() {
                *slot = None;
            }
        });
        None
    })
}

fn phonetic_composing() -> bool {
    with_existing_composer(|c| c.is_composing()).unwrap_or(false)
}

impl TextService {
    pub(super) fn new() -> Self {
        TextService { state: RefCell::new(State::default()), _dll: DllRef::new() }
    }
}

impl TextService_Impl {
    // MARK: Deciding about a key

    /// Whether this key is ours. No side effects on the text: TSF asks before every key.
    fn wants(&self, event: &KeyEvent) -> bool {
        if event.is_modifier() || self.is_resent(event) {
            return false;
        }
        self.refresh_settings();
        let (current, wanted, bijoy_composing, has_composition) = {
            let state = self.state.borrow();
            let wanted = state.settings.as_ref().map_or(state.layout, Settings::layout);
            (state.layout, wanted, state.bijoy.is_composing(), state.composition.is_some())
        };
        let composing = bijoy_composing || phonetic_composing() || has_composition;
        if wanted != current && composing {
            // The layout was switched elsewhere: the word typed with the old one is finished first.
            return true;
        }
        let layout = wanted;
        let bijoy_composing = bijoy_composing && layout == current;

        if event.is_shortcut() {
            // Ctrl+Backspace deletes the word being typed; any other shortcut finishes it first.
            return composing;
        }
        match layout {
            Layout::Bijoy => {
                if event.is_backspace() {
                    bijoy_composing
                } else if event.character().and_then(bijoy::keymap::key).is_some() {
                    true
                } else {
                    composing
                }
            }
            Layout::Phonetic => {
                if PHONETIC_FAILED.with(Cell::get) {
                    return composing;
                }
                match event.phonetic_key() {
                    Some(phonetic::Key::Character(..)) => true,
                    _ => composing,
                }
            }
        }
    }

    /// A key this service sent again is on its way back: it goes to the app.
    fn is_resent(&self, event: &KeyEvent) -> bool {
        if unsafe { GetMessageExtraInfo() }.0 as usize == RESENT_MARK {
            return true;
        }
        let now = unsafe { GetTickCount64() };
        let mut state = self.state.borrow_mut();
        state.resent.retain(|&(_, when)| now.saturating_sub(when) < 1000);
        match state.resent.iter().position(|&(vk, _)| vk == event.vk) {
            Some(index) => {
                state.resent.remove(index);
                true
            }
            None => false,
        }
    }

    // MARK: Handling a key

    fn handle_key(&self, context: &ITfContext, event: &KeyEvent) -> Result<bool> {
        if !self.wants(event) {
            return Ok(false);
        }
        if self.follow_layout_setting(Some(context)) && !self.wants(event) {
            // Taken only to finish the old layout's word; the new layout leaves it to the app.
            self.resend(event);
            return Ok(true);
        }
        let layout = self.state.borrow().layout;

        let word_delete = layout == Layout::Phonetic && event.ctrl && !event.alt && event.is_backspace();
        if event.is_shortcut() && !word_delete {
            self.commit_all(Some(context), true);
            self.resend(event);
            return Ok(true);
        }

        match layout {
            Layout::Bijoy => self.handle_bijoy(context, event),
            Layout::Phonetic => self.handle_phonetic(context, event),
        }
        Ok(true)
    }

    fn handle_bijoy(&self, context: &ITfContext, event: &KeyEvent) {
        if event.is_backspace() {
            let out = self.state.borrow_mut().bijoy.backspace();
            if let Some(out) = out {
                self.show_bijoy(context, out);
            }
            return;
        }
        let out = event.character().and_then(|c| self.state.borrow_mut().bijoy.process_character(c));
        match out {
            Some(out) => self.show_bijoy(context, out),
            None => {
                // Space, Enter, Tab, arrows, punctuation…: finish the syllable, let the app have the key.
                self.commit_all(Some(context), true);
                self.resend(event);
            }
        }
    }

    fn show_bijoy(&self, context: &ITfContext, out: bijoy::Output) {
        // `visible`, not `display`: a kar waiting for its consonant stays off screen.
        if self.update(context, out.committed, out.visible, true).is_err() {
            self.state.borrow_mut().bijoy.reset();
        }
    }

    fn handle_phonetic(&self, context: &ITfContext, event: &KeyEvent) {
        let Some(key) = event.phonetic_key() else {
            self.commit_all(Some(context), true);
            self.resend(event);
            return;
        };
        let out = {
            let state = self.state.borrow();
            with_composer(state.settings.as_ref(), |c| c.handle(key))
        };
        let Some(out) = out else {
            // riti failed: whatever was on screen stays, as typed so far.
            self.forget_composition();
            self.resend(event);
            return;
        };
        let handled = out.handled;
        self.apply_phonetic(context, out, true);
        if !handled {
            self.resend(event);
        }
    }

    /// Puts a phonetic result into the app and updates the suggestion list.
    fn apply_phonetic(&self, context: &ITfContext, out: phonetic::Output, sync: bool) {
        let rect = match self.update(context, out.insert, out.marked, sync) {
            Ok(rect) => rect,
            Err(_) => {
                with_existing_composer(|c| c.discard());
                None
            }
        };
        self.show_candidates(rect);
    }

    /// Sends the key again, for the app: after the text just committed.
    fn resend(&self, event: &KeyEvent) {
        let flags = if event.extended { KEYEVENTF_EXTENDEDKEY } else { KEYBD_EVENT_FLAGS(0) };
        let input = INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT { wVk: VIRTUAL_KEY(event.vk), wScan: event.scan as u16, dwFlags: flags, time: 0, dwExtraInfo: RESENT_MARK },
            },
        };
        let now = unsafe { GetTickCount64() };
        self.state.borrow_mut().resent.push_back((event.vk, now));
        unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    }

    // MARK: Writing

    /// Commits `committed`, then shows `composing` as the composition (or ends it when empty).
    /// Returns where the composition is on screen, when the app knows.
    fn update(&self, context: &ITfContext, committed: String, composing: String, sync: bool) -> Result<Option<RECT>> {
        let (composition, target, client, attribute) = {
            let state = self.state.borrow();
            let target = if state.composition.is_some() { state.composition_context.clone() } else { None };
            (state.composition.clone(), target.unwrap_or_else(|| context.clone()), state.client, state.attribute)
        };
        if composition.is_none() && committed.is_empty() && composing.is_empty() {
            return Ok(None);
        }
        let sink: ITfCompositionSink = self.to_interface();
        let result: Rc<RefCell<(Option<ITfComposition>, Option<RECT>)>> = Rc::default();
        let out = result.clone();
        let session_context = target.clone();
        edit::run(&target, client, sync, move |ec| {
            let context = &session_context;
            let mut composition = composition;
            if !committed.is_empty() {
                edit::commit(ec, context, composition.take(), &committed)?;
            }
            let mut rect = None;
            if !composing.is_empty() {
                if composition.is_none() {
                    composition = Some(edit::start_composition(ec, context, &sink)?);
                }
                if let Some(composition) = &composition {
                    edit::set_composition_text(ec, context, composition, &composing, attribute)?;
                    rect = edit::composition_rect(ec, context, composition);
                }
            } else if let Some(composition) = composition.take() {
                edit::cancel_composition(ec, context, &composition)?;
            }
            *out.borrow_mut() = (composition, rect);
            Ok(())
        })?;
        if !sync {
            // Runs when the app allows. Only finishing is ever done this way (a click in the list,
            // a layout switch, focus moving away), so the composition is over either way.
            let mut state = self.state.borrow_mut();
            state.composition = None;
            state.composition_context = None;
            return Ok(None);
        }
        let (composition, rect) = result.take();
        let mut state = self.state.borrow_mut();
        state.composition_context = composition.as_ref().map(|_| target);
        state.composition = composition;
        Ok(rect)
    }

    /// Finishes whatever is being composed, in either layout (a click, a shortcut, a layout switch).
    fn commit_all(&self, context: Option<&ITfContext>, sync: bool) {
        let mut text = self.state.borrow_mut().bijoy.commit().committed;
        if let Some(out) = with_existing_composer(|c| c.commit()) {
            text += &out.insert;
        }
        self.hide_candidates();
        let target = {
            let state = self.state.borrow();
            state.composition_context.clone().or_else(|| context.cloned())
        };
        let Some(target) = target else { return };
        if self.update(&target, text, String::new(), sync).is_err() || !sync {
            self.forget_composition();
        }
    }

    /// The composition is no longer ours to change.
    fn forget_composition(&self) {
        let mut state = self.state.borrow_mut();
        state.composition = None;
        state.composition_context = None;
        state.bijoy.reset();
        drop(state);
        with_existing_composer(|c| c.discard());
        self.hide_candidates();
    }

    // MARK: Suggestion list

    fn show_candidates(&self, rect: Option<RECT>) {
        let shown = with_existing_composer(|c| {
            (c.is_composing() && !c.candidates().is_empty())
                .then(|| (c.candidates().to_vec(), c.auxiliary().to_owned(), c.selected_index()))
        })
        .flatten();
        let Some((candidates, auxiliary, selected)) = shown else {
            self.hide_candidates();
            return;
        };
        if self.ensure_candidate_window().is_none() {
            return;
        }
        let (exact, cursor) = match rect {
            Some(rect) => (true, rect),
            None => (false, caret_rect()),
        };
        let state = self.state.borrow();
        let (position, direction) = state
            .settings
            .as_ref()
            .map(|s| (s.popup_position(), s.popup_direction()))
            .unwrap_or_default();
        if let Some(window) = &state.candidates {
            window.show(&candidates, &auxiliary, selected, cursor, exact, position, direction);
        }
    }

    fn hide_candidates(&self) {
        if let Ok(state) = self.state.try_borrow() {
            if let Some(window) = &state.candidates {
                window.hide();
            }
        }
    }

    /// The list window, made on this thread on first use. Also the owner of the taskbar menu.
    fn ensure_candidate_window(&self) -> Option<HWND> {
        if let Some(window) = &self.state.borrow().candidates {
            return Some(window.hwnd());
        }
        let this = self.to_object();
        let window = CandidateWindow::new(move |event| {
            let _ = guard(|| {
                this.candidate_event(event);
                Ok(())
            });
        })
        .ok()?;
        let hwnd = window.hwnd();
        self.state.borrow_mut().candidates = Some(window);
        Some(hwnd)
    }

    fn candidate_event(&self, event: Event) {
        let Some(context) = self.state.borrow().composition_context.clone() else {
            self.hide_candidates();
            return;
        };
        match event {
            Event::Pick(index) => {
                if let Some(out) = with_existing_composer(|c| c.commit_at(index)) {
                    self.apply_phonetic(&context, out, false);
                }
            }
            Event::Reposition => {
                let (composition, client) = {
                    let state = self.state.borrow();
                    (state.composition.clone(), state.client)
                };
                let Some(composition) = composition else { return };
                let this = self.to_object();
                let session_context = context.clone();
                let _ = edit::run(&context, client, false, move |ec| {
                    if let Some(rect) = edit::composition_rect(ec, &session_context, &composition) {
                        let state = this.state.borrow();
                        let position = state.settings.as_ref().map(|s| s.popup_position()).unwrap_or_default();
                        if let Some(window) = &state.candidates {
                            window.move_to(rect, position);
                        }
                    }
                    Ok(())
                });
            }
        }
    }

    // MARK: Settings and layout

    /// Re-reads the settings file when another process (the settings program, another app's
    /// taskbar menu) has written it.
    fn refresh_settings(&self) {
        let changed = self.state.borrow_mut().settings.as_mut().is_some_and(Settings::reload_if_changed);
        if !changed {
            return;
        }
        PHONETIC_FAILED.with(|failed| failed.set(false));
        let (options, horizontal) = {
            let state = self.state.borrow();
            let Some(settings) = state.settings.as_ref() else { return };
            (settings.phonetic_options(), settings.popup_direction() == PopupDirection::Horizontal)
        };
        let needs_update = with_existing_composer(|c| {
            c.horizontal_navigation = horizontal;
            c.options() != &options
        })
        .unwrap_or(false);
        if needs_update {
            // A rebuilt riti drops the word in progress: finish it first.
            if phonetic_composing() {
                self.commit_all(None, false);
            }
            with_existing_composer(|c| c.update(options));
        }
    }

    /// Takes up the layout picked elsewhere, finishing the word in progress with the old one first.
    /// True if the layout changed.
    fn follow_layout_setting(&self, context: Option<&ITfContext>) -> bool {
        let wanted = self.state.borrow().settings.as_ref().map(Settings::layout);
        let Some(wanted) = wanted else { return false };
        if wanted == self.state.borrow().layout {
            return false;
        }
        self.commit_all(context, context.is_some());
        let state = &mut *self.state.borrow_mut();
        state.layout = wanted;
        if let Some(button) = &state.button {
            button.set_layout(wanted);
        }
        true
    }

    fn switch_layout(&self, layout: Layout) {
        // Finish the word in progress with the layout it was typed in.
        self.commit_all(None, false);
        let state = &mut *self.state.borrow_mut();
        state.layout = layout;
        if let Some(settings) = state.settings.as_mut() {
            settings.set_layout(layout);
        }
        if let Some(button) = &state.button {
            button.set_layout(layout);
        }
    }

    fn command(&self, command: Command) {
        match command {
            Command::Toggle => {
                let layout = self.state.borrow().layout.toggled();
                self.switch_layout(layout);
            }
            Command::Menu(point) => self.show_menu(point),
        }
    }

    /// The নু menu: the two layouts, settings, about. Owned by the focused document's window, as a
    /// menu must belong to a window of the thread showing it.
    fn show_menu(&self, point: POINT) {
        let Some(owner) = self.focused_window().or_else(|| self.ensure_candidate_window()) else { return };
        let current = self.state.borrow().layout;
        const BIJOY: usize = 1;
        const PHONETIC: usize = 2;
        const SETTINGS: usize = 3;
        const ABOUT: usize = 4;
        let chosen = unsafe {
            let Ok(menu) = CreatePopupMenu() else { return };
            let check = |layout| if current == layout { MF_CHECKED } else { MF_UNCHECKED };
            let _ = AppendMenuW(menu, MF_STRING | check(Layout::Bijoy), BIJOY, w!("বিজয় লেআউট"));
            let _ = AppendMenuW(menu, MF_STRING | check(Layout::Phonetic), PHONETIC, w!("ফোনেটিক"));
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
            let _ = AppendMenuW(menu, MF_STRING, SETTINGS, w!("সেটিংস…"));
            let _ = AppendMenuW(menu, MF_STRING, ABOUT, w!("নুকতা বাংলা সম্পর্কে…"));
            let chosen = TrackPopupMenuEx(menu, (TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON).0, point.x, point.y, owner, None);
            let _ = DestroyMenu(menu);
            chosen.0 as usize
        };
        match chosen {
            BIJOY => self.switch_layout(Layout::Bijoy),
            PHONETIC => self.switch_layout(Layout::Phonetic),
            SETTINGS => open_settings_program(None),
            ABOUT => open_settings_program(Some(w!("--about"))),
            _ => {}
        }
    }

    /// The window of the document that has the keyboard on this thread.
    fn focused_window(&self) -> Option<HWND> {
        let thread_mgr = self.state.borrow().thread_mgr.clone()?;
        unsafe {
            let window = thread_mgr.GetFocus().ok()?.GetTop().ok()?.GetActiveView().ok()?.GetWnd().ok()?;
            (!window.is_invalid()).then_some(window)
        }
    }

    // MARK: Activation

    fn activate(&self, thread_mgr: &ITfThreadMgr, client: u32) -> Result<()> {
        let settings = Settings::new(&win::prepare_data_directory());
        let layout = settings.layout();
        {
            let mut state = self.state.borrow_mut();
            state.thread_mgr = Some(thread_mgr.clone());
            state.client = client;
            state.layout = layout;
            state.settings = Some(settings);
            state.bijoy.reset();
        }
        unsafe {
            let source: ITfSource = thread_mgr.cast()?;
            let sink: ITfThreadMgrEventSink = self.to_interface();
            let cookie = source.AdviseSink(&ITfThreadMgrEventSink::IID, &sink)?;
            self.state.borrow_mut().thread_mgr_cookie = Some(cookie);

            let keystrokes: ITfKeystrokeMgr = thread_mgr.cast()?;
            let key_sink: ITfKeyEventSink = self.to_interface();
            keystrokes.AdviseKeyEventSink(client, &key_sink, true)?;
            self.state.borrow_mut().key_sink_advised = true;

            if let Ok(categories) = CoCreateInstance::<_, ITfCategoryMgr>(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER) {
                if let Ok(atom) = categories.RegisterGUID(&GUID_DISPLAY_ATTRIBUTE) {
                    self.state.borrow_mut().attribute = atom;
                }
            }

            let this = self.to_object();
            let button = ComObject::new(LangBarButton::new(layout, move |command| {
                let _ = guard(|| {
                    this.command(command);
                    Ok(())
                });
            }));
            if let Ok(items) = thread_mgr.cast::<ITfLangBarItemMgr>() {
                let _ = items.AddItem(&langbar::as_button(&button));
            }
            self.state.borrow_mut().button = Some(button);
        }
        Ok(())
    }

    fn deactivate(&self) {
        self.hide_candidates();
        let (thread_mgr, client, cookie, key_sink, button, candidates) = {
            let mut state = self.state.borrow_mut();
            state.bijoy.reset();
            state.composition = None;
            state.composition_context = None;
            (
                state.thread_mgr.take(),
                state.client,
                state.thread_mgr_cookie.take(),
                std::mem::take(&mut state.key_sink_advised),
                state.button.take(),
                state.candidates.take(),
            )
        };
        with_existing_composer(|c| c.discard());
        drop(candidates);
        if let Some(button) = &button {
            button.disconnect();
        }
        let Some(thread_mgr) = thread_mgr else { return };
        unsafe {
            if let (Some(button), Ok(items)) = (&button, thread_mgr.cast::<ITfLangBarItemMgr>()) {
                let _ = items.RemoveItem(&langbar::as_button(button));
            }
            if key_sink {
                if let Ok(keystrokes) = thread_mgr.cast::<ITfKeystrokeMgr>() {
                    let _ = keystrokes.UnadviseKeyEventSink(client);
                }
            }
            if let (Some(cookie), Ok(source)) = (cookie, thread_mgr.cast::<ITfSource>()) {
                let _ = source.UnadviseSink(cookie);
            }
        }
    }

    /// This thread got the keyboard, or focus moved to another document in it.
    fn focus_changed(&self) {
        // A riti failure is retried in each new place, not given up on for good.
        PHONETIC_FAILED.with(|failed| failed.set(false));
        self.refresh_settings();
        self.follow_layout_setting(None);
        with_existing_composer(|c| c.reload_if_learned_elsewhere());
    }
}

// MARK: TSF interfaces

impl ITfTextInputProcessor_Impl for TextService_Impl {
    fn Activate(&self, thread_mgr: Ref<ITfThreadMgr>, client: u32) -> Result<()> {
        self.ActivateEx(thread_mgr, client, 0)
    }

    fn Deactivate(&self) -> Result<()> {
        guard(|| {
            self.deactivate();
            Ok(())
        })
    }
}

impl ITfTextInputProcessorEx_Impl for TextService_Impl {
    fn ActivateEx(&self, thread_mgr: Ref<ITfThreadMgr>, client: u32, _flags: u32) -> Result<()> {
        guard(|| {
            let thread_mgr = thread_mgr.ok()?.clone();
            let result = self.activate(&thread_mgr, client);
            if result.is_err() {
                self.deactivate();
            }
            result
        })
    }
}

impl ITfThreadMgrEventSink_Impl for TextService_Impl {
    fn OnInitDocumentMgr(&self, _document: Ref<ITfDocumentMgr>) -> Result<()> {
        Ok(())
    }

    fn OnUninitDocumentMgr(&self, _document: Ref<ITfDocumentMgr>) -> Result<()> {
        Ok(())
    }

    /// Focus moved to another document (another field, another window of the app): the word in
    /// progress is finished where it was typed.
    fn OnSetFocus(&self, _focus: Ref<ITfDocumentMgr>, _previous: Ref<ITfDocumentMgr>) -> Result<()> {
        guard(|| {
            self.commit_all(None, false);
            self.focus_changed();
            Ok(())
        })
    }

    fn OnPushContext(&self, _context: Ref<ITfContext>) -> Result<()> {
        Ok(())
    }

    fn OnPopContext(&self, _context: Ref<ITfContext>) -> Result<()> {
        Ok(())
    }
}

impl ITfKeyEventSink_Impl for TextService_Impl {
    fn OnSetFocus(&self, foreground: BOOL) -> Result<()> {
        guard(|| {
            if foreground.as_bool() {
                self.focus_changed();
            } else {
                self.commit_all(None, false);
            }
            Ok(())
        })
    }

    fn OnTestKeyDown(&self, context: Ref<ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        if context.is_null() {
            return Ok(false.into());
        }
        let event = KeyEvent::new(wparam, lparam);
        Ok(guard(|| Ok(self.wants(&event))).unwrap_or(false).into())
    }

    fn OnKeyDown(&self, context: Ref<ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        let Some(context) = context.as_ref() else { return Ok(false.into()) };
        let event = KeyEvent::new(wparam, lparam);
        Ok(guard(|| self.handle_key(context, &event)).unwrap_or(false).into())
    }

    fn OnTestKeyUp(&self, _context: Ref<ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        Ok(false.into())
    }

    fn OnKeyUp(&self, _context: Ref<ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        Ok(false.into())
    }

    fn OnPreservedKey(&self, _context: Ref<ITfContext>, _guid: *const GUID) -> Result<BOOL> {
        Ok(false.into())
    }
}

impl ITfCompositionSink_Impl for TextService_Impl {
    /// The app ended the composition (a click elsewhere, its own commit): what is on screen stays,
    /// and the engines start afresh.
    fn OnCompositionTerminated(&self, _ec: u32, _composition: Ref<ITfComposition>) -> Result<()> {
        guard(|| {
            self.forget_composition();
            Ok(())
        })
    }
}

impl ITfDisplayAttributeProvider_Impl for TextService_Impl {
    fn EnumDisplayAttributeInfo(&self) -> Result<IEnumTfDisplayAttributeInfo> {
        Ok(DisplayAttributeEnum::new().into())
    }

    fn GetDisplayAttributeInfo(&self, guid: *const GUID) -> Result<ITfDisplayAttributeInfo> {
        if guid.is_null() || unsafe { *guid } != GUID_DISPLAY_ATTRIBUTE {
            return Err(E_INVALIDARG.into());
        }
        Ok(DisplayAttributeInfo::new().into())
    }
}

// MARK: Helpers

/// Where the cursor is, when the app can't say where the composition is yet: the caret, or failing
/// that the mouse.
fn caret_rect() -> RECT {
    unsafe {
        let mut info = GUITHREADINFO { cbSize: std::mem::size_of::<GUITHREADINFO>() as u32, ..Default::default() };
        if GetGUIThreadInfo(0, &mut info).is_ok() && !info.hwndCaret.is_invalid() {
            let mut top_left = POINT { x: info.rcCaret.left, y: info.rcCaret.top };
            let mut bottom_right = POINT { x: info.rcCaret.right, y: info.rcCaret.bottom };
            let _ = ClientToScreen(info.hwndCaret, &mut top_left);
            let _ = ClientToScreen(info.hwndCaret, &mut bottom_right);
            return RECT { left: top_left.x, top: top_left.y, right: bottom_right.x, bottom: bottom_right.y };
        }
        let mut mouse = POINT::default();
        let _ = GetCursorPos(&mut mouse);
        RECT { left: mouse.x, top: mouse.y, right: mouse.x, bottom: mouse.y + 20 }
    }
}

/// The settings program (or its about box), installed beside the DLLs. Store apps may not start
/// programs; the menu then does nothing there.
fn open_settings_program(arguments: Option<PCWSTR>) {
    let program = win::settings_program(std::path::Path::new(&register::dll_path()));
    let program: Vec<u16> = program.to_string_lossy().encode_utf16().chain(Some(0)).collect();
    unsafe {
        ShellExecuteW(
            None,
            w!("open"),
            PCWSTR(program.as_ptr()),
            arguments.unwrap_or(PCWSTR::null()),
            PCWSTR::null(),
            SW_SHOWNORMAL,
        );
    }
}
