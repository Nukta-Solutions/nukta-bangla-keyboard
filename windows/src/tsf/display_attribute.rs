//! The look of the text being composed: a plain underline in the text's own colour, like the macOS
//! marked text and the Linux preedit. TSF asks the text service for it by GUID.

use std::cell::Cell;

use windows::core::{implement, BSTR, GUID, Result};
use windows::Win32::Foundation::{E_INVALIDARG, S_FALSE};
use windows::Win32::UI::TextServices::{
    IEnumTfDisplayAttributeInfo, IEnumTfDisplayAttributeInfo_Impl, ITfDisplayAttributeInfo,
    ITfDisplayAttributeInfo_Impl, TF_ATTR_INPUT, TF_CT_NONE, TF_DA_COLOR, TF_DISPLAYATTRIBUTE, TF_LS_SOLID,
};

use super::{DllRef, GUID_DISPLAY_ATTRIBUTE};

fn attribute() -> TF_DISPLAYATTRIBUTE {
    let none = TF_DA_COLOR { r#type: TF_CT_NONE, ..Default::default() };
    TF_DISPLAYATTRIBUTE {
        crText: none,
        crBk: none,
        lsStyle: TF_LS_SOLID,
        fBoldLine: false.into(),
        crLine: none,
        bAttr: TF_ATTR_INPUT,
    }
}

#[implement(ITfDisplayAttributeInfo)]
pub(super) struct DisplayAttributeInfo {
    _dll: DllRef,
}

impl DisplayAttributeInfo {
    pub(super) fn new() -> Self {
        DisplayAttributeInfo { _dll: DllRef::new() }
    }
}

impl ITfDisplayAttributeInfo_Impl for DisplayAttributeInfo_Impl {
    fn GetGUID(&self) -> Result<GUID> {
        Ok(GUID_DISPLAY_ATTRIBUTE)
    }

    fn GetDescription(&self) -> Result<BSTR> {
        Ok(BSTR::from("নুকতা বাংলা: লেখা চলছে"))
    }

    fn GetAttributeInfo(&self, out: *mut TF_DISPLAYATTRIBUTE) -> Result<()> {
        if out.is_null() {
            return Err(E_INVALIDARG.into());
        }
        unsafe { *out = attribute() };
        Ok(())
    }

    /// The look is fixed.
    fn SetAttributeInfo(&self, _attribute: *const TF_DISPLAYATTRIBUTE) -> Result<()> {
        Ok(())
    }

    fn Reset(&self) -> Result<()> {
        Ok(())
    }
}

/// Lists the one display attribute.
#[implement(IEnumTfDisplayAttributeInfo)]
pub(super) struct DisplayAttributeEnum {
    position: Cell<u32>,
    _dll: DllRef,
}

impl DisplayAttributeEnum {
    pub(super) fn new() -> Self {
        DisplayAttributeEnum { position: Cell::new(0), _dll: DllRef::new() }
    }
}

impl IEnumTfDisplayAttributeInfo_Impl for DisplayAttributeEnum_Impl {
    fn Clone(&self) -> Result<IEnumTfDisplayAttributeInfo> {
        let copy = DisplayAttributeEnum::new();
        copy.position.set(self.position.get());
        Ok(copy.into())
    }

    fn Next(&self, count: u32, out: *mut Option<ITfDisplayAttributeInfo>, fetched: *mut u32) -> Result<()> {
        let mut given = 0;
        if count > 0 && self.position.get() == 0 && !out.is_null() {
            unsafe { out.write(Some(DisplayAttributeInfo::new().into())) };
            self.position.set(1);
            given = 1;
        }
        if !fetched.is_null() {
            unsafe { *fetched = given };
        }
        if given == count { Ok(()) } else { Err(S_FALSE.into()) }
    }

    fn Reset(&self) -> Result<()> {
        self.position.set(0);
        Ok(())
    }

    fn Skip(&self, count: u32) -> Result<()> {
        let position = self.position.get();
        self.position.set(position.saturating_add(count).min(1));
        if position == 0 && count <= 1 { Ok(()) } else { Err(S_FALSE.into()) }
    }
}
