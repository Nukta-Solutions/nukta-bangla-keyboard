//! Edit sessions: the only way TSF lets a text service read or change an app's text. Everything
//! here takes the edit cookie (`ec`) TSF hands to `DoEditSession`.
//!
//! Both layouts write the same way, the way the Linux build does with IBus's preedit: the part
//! still being typed is a TSF composition (underlined, and rewritten freely as keys reorder it),
//! and only finished text is left in the document. None of the macOS `ClientWriter`'s
//! read-back-and-rewrite work is needed.

use std::cell::RefCell;
use std::mem::ManuallyDrop;

use windows::core::{implement, Interface, Result};
use windows::Win32::Foundation::RECT;
use windows::Win32::System::Variant::{VARIANT, VT_I4};
use windows::Win32::UI::TextServices::{
    ITfComposition, ITfCompositionSink, ITfContext, ITfContextComposition, ITfEditSession, ITfEditSession_Impl,
    ITfInsertAtSelection, ITfRange, GUID_PROP_ATTRIBUTE, INSERT_TEXT_AT_SELECTION_FLAGS, TF_AE_NONE, TF_ANCHOR_END,
    TF_ES_ASYNCDONTCARE, TF_ES_READWRITE, TF_ES_SYNC, TF_IAS_QUERYONLY, TF_SELECTION, TF_SELECTIONSTYLE,
};

use super::guard;

type Body = Box<dyn FnOnce(u32) -> Result<()>>;

#[implement(ITfEditSession)]
struct EditSession {
    body: RefCell<Option<Body>>,
}

impl ITfEditSession_Impl for EditSession_Impl {
    fn DoEditSession(&self, ec: u32) -> Result<()> {
        let body = self.body.borrow_mut().take();
        match body {
            Some(body) => guard(|| body(ec)),
            None => Ok(()),
        }
    }
}

/// Runs `body` with write access to `context`'s text. `sync`: now, before returning (allowed while
/// handling a key); otherwise whenever the app allows it (a click in the suggestion list, a layout
/// switch from the taskbar).
pub(super) fn run(context: &ITfContext, client: u32, sync: bool, body: impl FnOnce(u32) -> Result<()> + 'static) -> Result<()> {
    let session: ITfEditSession = EditSession { body: RefCell::new(Some(Box::new(body))) }.into();
    let flags = TF_ES_READWRITE | if sync { TF_ES_SYNC } else { TF_ES_ASYNCDONTCARE };
    unsafe { context.RequestEditSession(client, &session, flags)? }.ok()
}

/// Puts `text` into the document as final text: in place of the composition when there is one
/// (ending it), else at the cursor. The cursor ends up after it.
pub(super) fn commit(ec: u32, context: &ITfContext, composition: Option<ITfComposition>, text: &str) -> Result<()> {
    let text: Vec<u16> = text.encode_utf16().collect();
    unsafe {
        if let Some(composition) = composition {
            let range = composition.GetRange()?;
            range.SetText(ec, 0, &text)?;
            clear_attribute(ec, context, &range);
            move_cursor_to_end(ec, context, &range)?;
            composition.EndComposition(ec)?;
        } else if !text.is_empty() {
            let insert = context.cast::<ITfInsertAtSelection>()?;
            let range = insert.InsertTextAtSelection(ec, INSERT_TEXT_AT_SELECTION_FLAGS(0), &text)?;
            move_cursor_to_end(ec, context, &range)?;
        }
    }
    Ok(())
}

/// Starts a composition at the cursor (replacing any selection).
pub(super) fn start_composition(ec: u32, context: &ITfContext, sink: &ITfCompositionSink) -> Result<ITfComposition> {
    unsafe {
        let insert = context.cast::<ITfInsertAtSelection>()?;
        let range = insert.InsertTextAtSelection(ec, TF_IAS_QUERYONLY, &[])?;
        context.cast::<ITfContextComposition>()?.StartComposition(ec, &range, sink)
    }
}

/// Shows `text` as the composition, underlined, with the cursor after it.
pub(super) fn set_composition_text(ec: u32, context: &ITfContext, composition: &ITfComposition, text: &str, attribute: u32) -> Result<()> {
    let text: Vec<u16> = text.encode_utf16().collect();
    unsafe {
        let range = composition.GetRange()?;
        range.SetText(ec, 0, &text)?;
        if attribute != 0 {
            set_attribute(ec, context, &range, attribute);
        }
        move_cursor_to_end(ec, context, &range)
    }
}

/// Removes the composition's text and ends it.
pub(super) fn cancel_composition(ec: u32, context: &ITfContext, composition: &ITfComposition) -> Result<()> {
    unsafe {
        let range = composition.GetRange()?;
        range.SetText(ec, 0, &[])?;
        move_cursor_to_end(ec, context, &range)?;
        composition.EndComposition(ec)
    }
}

/// Where the composition is on screen, in screen coordinates; None while the app hasn't laid it
/// out yet.
pub(super) fn composition_rect(ec: u32, context: &ITfContext, composition: &ITfComposition) -> Option<RECT> {
    unsafe {
        let range = composition.GetRange().ok()?;
        let view = context.GetActiveView().ok()?;
        let mut rect = RECT::default();
        let mut clipped = Default::default();
        view.GetTextExt(ec, &range, &mut rect, &mut clipped).ok()?;
        (rect.bottom > rect.top || rect.right > rect.left || rect.left != 0 || rect.top != 0).then_some(rect)
    }
}

fn move_cursor_to_end(ec: u32, context: &ITfContext, range: &ITfRange) -> Result<()> {
    unsafe {
        let cursor = range.Clone()?;
        cursor.Collapse(ec, TF_ANCHOR_END)?;
        let selection = TF_SELECTION {
            range: ManuallyDrop::new(Some(cursor)),
            style: TF_SELECTIONSTYLE { ase: TF_AE_NONE, fInterimChar: false.into() },
        };
        let result = context.SetSelection(ec, std::slice::from_ref(&selection));
        drop(ManuallyDrop::into_inner(selection.range));
        result
    }
}

/// The underline: TSF's attribute property, set to our registered display attribute.
fn set_attribute(ec: u32, context: &ITfContext, range: &ITfRange, attribute: u32) {
    unsafe {
        let Ok(property) = context.GetProperty(&GUID_PROP_ATTRIBUTE) else { return };
        let mut value = VARIANT::default();
        (*value.Anonymous.Anonymous).vt = VT_I4;
        (*value.Anonymous.Anonymous).Anonymous.lVal = attribute as i32;
        let _ = property.SetValue(ec, range, &value);
    }
}

fn clear_attribute(ec: u32, context: &ITfContext, range: &ITfRange) {
    unsafe {
        if let Ok(property) = context.GetProperty(&GUID_PROP_ATTRIBUTE) {
            let _ = property.Clear(ec, range);
        }
    }
}
