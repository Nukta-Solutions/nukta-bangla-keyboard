//! The text service: the counterpart of `Sources/NuktaInputMethod/NuktaInputController.swift`.
//!
//! One per app thread. It reads keys (TSF asks first whether a key is wanted, `OnTestKeyDown`, then
//! hands it over, `OnKeyDown`), feeds them to the Bijoy engine or the phonetic composer, and writes
//! the result as a composition plus finished text (see `edit`).
//!
//! A key the layout doesn't use goes to the app, as on macOS and Linux. While a syllable or word is
//! in progress, it first finishes it, then leaves the key unclaimed so the app gets it after the
//! finished text. All of this happens the first time TSF asks about the key (`OnTestKeyDown`), as
//! Weasel (Rime) does: the key itself then goes on to the app in its place in the queue. Sending a
//! copy of it instead (`SendInput`) put it behind keys typed after it: a space landing after the
//! next word's first letters.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use windows::core::{implement, w, IUnknownImpl, Interface, Ref, Result, BOOL, GUID, PCWSTR};
use windows::Win32::Foundation::{E_INVALIDARG, HWND, LPARAM, POINT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, IEnumTfDisplayAttributeInfo, ITfCategoryMgr, ITfComposition, ITfCompositionSink,
    ITfCompositionSink_Impl, ITfContext, ITfDisplayAttributeInfo, ITfDisplayAttributeProvider,
    ITfDisplayAttributeProvider_Impl, ITfDocumentMgr, ITfKeyEventSink, ITfKeyEventSink_Impl, ITfKeystrokeMgr,
    ITfSource, ITfTextInputProcessor_Impl, ITfTextInputProcessorEx, ITfTextInputProcessorEx_Impl,
    ITfThreadMgr, ITfThreadMgrEventSink, ITfThreadMgrEventSink_Impl,
};
use windows::Win32::UI::WindowsAndMessaging::{
    GetCursorPos, GetGUIThreadInfo, GUITHREADINFO, SW_SHOWNORMAL,
};

use super::candidates::{CandidateWindow, Event};
use super::display_attribute::{DisplayAttributeEnum, DisplayAttributeInfo};
use super::keys::KeyEvent;
use super::{edit, guard, register, DllRef, GUID_DISPLAY_ATTRIBUTE};
use crate::bijoy::{self, Engine};
use crate::phonetic::{self, PhoneticComposer};
use crate::settings::{Layout, PopupDirection, Settings};
use crate::win;

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
    /// English mode: every key goes to the app. Follows the `english` setting.
    english: bool,
    settings: Option<Settings>,
    candidates: Option<CandidateWindow>,
    /// TSF's number for our display attribute (the underline).
    attribute: u32,
    /// The key (virtual key code) `OnTestKeyDown` handled and claimed: the `OnKeyDown` that follows
    /// only confirms it. Cleared when the key comes up, so an app that tests a key and never sends
    /// it can't make the next key vanish.
    key_handled: Option<u16>,
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
        if event.is_modifier() {
            return false;
        }
        if event.is_toggle() {
            return true;
        }
        self.refresh_settings();
        let (current, wanted, english, bijoy_composing, has_composition) = {
            let state = self.state.borrow();
            let wanted = state.settings.as_ref().map_or((state.layout, state.english), |s| (s.layout(), s.english()));
            ((state.layout, state.english), wanted.0, wanted.1, state.bijoy.is_composing(), state.composition.is_some())
        };
        let composing = bijoy_composing || phonetic_composing() || has_composition;
        if (wanted, english) != current && composing {
            // The mode was switched elsewhere: the word typed with the old one is finished first.
            return true;
        }
        if english {
            // English, as with Avro: the key is the app's, once a word in progress is finished.
            return composing;
        }
        let current = current.0;
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

    // MARK: Handling a key

    fn handle_key(&self, context: &ITfContext, event: &KeyEvent) -> Result<bool> {
        if !self.wants(event) {
            return Ok(false);
        }
        if event.is_toggle() {
            // The word in progress is finished where it was typed, then F12 switches.
            self.follow_layout_setting(Some(context));
            self.commit_all(Some(context), true);
            self.toggle_mode();
            return Ok(true);
        }
        if self.follow_layout_setting(Some(context)) && !self.wants(event) {
            // Only the old layout's word to finish; the new layout leaves the key to the app.
            return Ok(false);
        }
        let (layout, english) = {
            let state = self.state.borrow();
            (state.layout, state.english)
        };
        if english {
            self.commit_all(Some(context), true);
            return Ok(false);
        }

        let word_delete = layout == Layout::Phonetic && event.ctrl && !event.alt && event.is_backspace();
        if event.is_shortcut() && !word_delete {
            self.commit_all(Some(context), true);
            return Ok(false);
        }

        Ok(match layout {
            Layout::Bijoy => self.handle_bijoy(context, event),
            Layout::Phonetic => self.handle_phonetic(context, event),
        })
    }

    /// True if the key was used up; false if the app should have it (after the finished syllable).
    fn handle_bijoy(&self, context: &ITfContext, event: &KeyEvent) -> bool {
        if event.is_backspace() {
            let out = self.state.borrow_mut().bijoy.backspace();
            return match out {
                Some(out) => {
                    self.show_bijoy(context, out);
                    true
                }
                None => false,
            };
        }
        let out = event.character().and_then(|c| self.state.borrow_mut().bijoy.process_character(c));
        match out {
            Some(out) => {
                self.show_bijoy(context, out);
                true
            }
            None => {
                // Space, Enter, Tab, arrows, punctuation…: finish the syllable, let the app have the key.
                self.commit_all(Some(context), true);
                false
            }
        }
    }

    fn show_bijoy(&self, context: &ITfContext, out: bijoy::Output) {
        // `visible`, not `display`: a kar waiting for its consonant stays off screen.
        if self.update(context, out.committed, out.visible, true).is_err() {
            self.state.borrow_mut().bijoy.reset();
        }
    }

    /// True if the key was used up; false if the app should have it (after the finished word).
    fn handle_phonetic(&self, context: &ITfContext, event: &KeyEvent) -> bool {
        let Some(key) = event.phonetic_key() else {
            self.commit_all(Some(context), true);
            return false;
        };
        let out = {
            let state = self.state.borrow();
            with_composer(state.settings.as_ref(), |c| c.handle(key))
        };
        let Some(out) = out else {
            // riti failed: whatever was on screen stays, as typed so far.
            self.forget_composition();
            return false;
        };
        let handled = out.handled;
        self.apply_phonetic(context, out, true);
        handled
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

    /// Takes up the layout or English/Bangla picked elsewhere (the floating bar, another app),
    /// finishing the word in progress with the old one first. True if either changed.
    fn follow_layout_setting(&self, context: Option<&ITfContext>) -> bool {
        let wanted = self.state.borrow().settings.as_ref().map(|s| (s.layout(), s.english()));
        let Some((layout, english)) = wanted else { return false };
        let current = {
            let state = self.state.borrow();
            (state.layout, state.english)
        };
        if (layout, english) == current {
            return false;
        }
        self.commit_all(context, context.is_some());
        let state = &mut *self.state.borrow_mut();
        state.layout = layout;
        state.english = english;
        true
    }

    /// Picks a mode: English (None) or a Bangla layout. Saved to the settings file, so the top bar
    /// and every other app follow.
    fn switch_mode(&self, mode: Option<Layout>) {
        // Finish the word in progress with the layout it was typed in.
        self.commit_all(None, false);
        let state = &mut *self.state.borrow_mut();
        state.english = mode.is_none();
        if let Some(layout) = mode {
            state.layout = layout;
        }
        if let Some(settings) = state.settings.as_mut() {
            if let Some(layout) = mode {
                settings.set_layout(layout);
            }
            settings.set_english(mode.is_none());
        }
    }

    /// English ⇄ Bangla (F12), with the beep. The top bar follows through the settings file; if it
    /// was closed with Exit, it comes back.
    fn toggle_mode(&self) {
        let (english, layout) = {
            let state = self.state.borrow();
            (state.english, state.layout)
        };
        self.switch_mode(if english { Some(layout) } else { None });
        crate::sound::play_toggle(english);
        start_bar();
    }

    // MARK: Activation

    fn activate(&self, thread_mgr: &ITfThreadMgr, client: u32) -> Result<()> {
        let settings = Settings::new(&win::prepare_data_directory());
        let layout = settings.layout();
        let english = settings.english();
        {
            let mut state = self.state.borrow_mut();
            state.thread_mgr = Some(thread_mgr.clone());
            state.client = client;
            state.layout = layout;
            state.english = english;
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
        }
        Ok(())
    }

    fn deactivate(&self) {
        self.hide_candidates();
        let (thread_mgr, client, cookie, key_sink, candidates) = {
            let mut state = self.state.borrow_mut();
            state.bijoy.reset();
            state.composition = None;
            state.composition_context = None;
            (
                state.thread_mgr.take(),
                state.client,
                state.thread_mgr_cookie.take(),
                std::mem::take(&mut state.key_sink_advised),
                state.candidates.take(),
            )
        };
        with_existing_composer(|c| c.discard());
        drop(candidates);
        let Some(thread_mgr) = thread_mgr else { return };
        unsafe {
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

    /// The key is handled here, the first time TSF asks: a word finished for a key the app should
    /// have is committed before the app sees the key. Claiming the key makes TSF call `OnKeyDown`,
    /// which then only confirms.
    fn OnTestKeyDown(&self, context: Ref<ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        let event = KeyEvent::new(wparam, lparam);
        if self.state.borrow().key_handled == Some(event.vk) {
            return Ok(true.into()); // asked again about the key already handled
        }
        let Some(context) = context.as_ref() else { return Ok(false.into()) };
        let claimed = guard(|| self.handle_key(context, &event)).unwrap_or(false);
        self.state.borrow_mut().key_handled = claimed.then_some(event.vk);
        Ok(claimed.into())
    }

    /// After `OnTestKeyDown` claimed the key: nothing left to do. Apps that skip the test call
    /// this alone, and it handles the key itself.
    fn OnKeyDown(&self, context: Ref<ITfContext>, wparam: WPARAM, lparam: LPARAM) -> Result<BOOL> {
        let event = KeyEvent::new(wparam, lparam);
        if self.state.borrow_mut().key_handled.take() == Some(event.vk) {
            return Ok(true.into());
        }
        let Some(context) = context.as_ref() else { return Ok(false.into()) };
        Ok(guard(|| self.handle_key(context, &event)).unwrap_or(false).into())
    }

    fn OnTestKeyUp(&self, _context: Ref<ITfContext>, _wparam: WPARAM, _lparam: LPARAM) -> Result<BOOL> {
        if let Ok(mut state) = self.state.try_borrow_mut() {
            state.key_handled = None;
        }
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

/// The floating bar and tray icon (NuktaBangla.exe), unless it is already running.
fn start_bar() {
    if unsafe { windows::Win32::UI::WindowsAndMessaging::FindWindowW(w!("NuktaBanglaBar"), None) }.is_ok() {
        return;
    }
    let program = win::bar_program(std::path::Path::new(&register::dll_path()));
    if !program.exists() {
        return;
    }
    let program: Vec<u16> = program.to_string_lossy().encode_utf16().chain(Some(0)).collect();
    unsafe {
        ShellExecuteW(None, w!("open"), PCWSTR(program.as_ptr()), PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL);
    }
}
