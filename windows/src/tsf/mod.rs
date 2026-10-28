//! The TSF text service: the Windows counterpart of `Sources/NuktaInputMethod` (InputMethodKit)
//! and `linux/nukta_bangla/ibus_*.py` (IBus).
//!
//! Windows loads this DLL into every app that takes text, on each thread that has a text-services
//! thread manager, and creates one `TextService` there. It types with both layouts: which one is
//! the shared `layout` setting, switched from the top bar and the tray icon (NuktaBangla.exe, the
//! counterpart of the macOS নু menu) and applied to every app. F12 is caught here.
//!
//! - `service`: activation, keys, focus, and the two layouts.
//! - `edit`: edit sessions, the only way TSF lets a text service change an app's text.
//! - `candidates`: the phonetic suggestion list window.
//! - `display_attribute`: the underline under the text being composed.
//! - `keys`: physical key → US-layout character, as on macOS and Linux.
//! - `register`: what `regsvr32` (the installer) writes and removes.

mod candidates;
mod display_attribute;
mod edit;
mod keys;
mod register;
mod service;

pub(crate) use register::guid_string;

use std::ffi::c_void;
use std::sync::atomic::{AtomicIsize, AtomicPtr, Ordering};

use windows::core::{implement, Interface, BOOL, GUID, HRESULT, IUnknown, Ref, Result};
use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, CLASS_E_NOAGGREGATION, E_POINTER, HINSTANCE, S_FALSE, S_OK,
};
use windows::Win32::System::Com::{IClassFactory, IClassFactory_Impl};
use windows::Win32::System::LibraryLoader::DisableThreadLibraryCalls;
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;

/// The text service's COM class.
pub const CLSID_TEXT_SERVICE: GUID = GUID::from_u128(0xCF0D8A77_8C0C_492E_A866_6818BD4A8999);
/// Its one language profile: নুকতা বাংলা, both layouts.
pub const GUID_PROFILE: GUID = GUID::from_u128(0xC648CDA6_E335_491C_9AA9_CF777AEB20AE);
/// The look of the text being composed (underlined).
pub const GUID_DISPLAY_ATTRIBUTE: GUID = GUID::from_u128(0x6F0482DF_89E8_43FA_9381_9FBB933A886B);
/// English (United States): the keyboard sits beside the user's English keyboard and replaces it,
/// as Avro does. Its English mode passes keys through, so they type with the US layout under this
/// language; Bangla is switched on from the floating bar or the tray, not with Win+Space.
pub const LANGID_KEYBOARD: u16 = 0x0409;
/// Bangla (Bangladesh), where versions before the floating bar registered the keyboard; removed on
/// registration and when the keyboard is enabled, so an upgrade leaves no second entry behind.
pub const LANGID_OLD: u16 = 0x0845;
/// The name Windows shows in its language and keyboard lists.
pub const DISPLAY_NAME: &str = "নুকতা বাংলা";

/// This DLL, for its own path (registration, finding the settings program).
static MODULE: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());
/// Live COM objects plus `LockServer` locks: the DLL may unload only at zero.
static OBJECTS: AtomicIsize = AtomicIsize::new(0);

pub(crate) fn module() -> HINSTANCE {
    HINSTANCE(MODULE.load(Ordering::Relaxed))
}

/// Counts a COM object of this DLL for as long as it lives.
pub(crate) struct DllRef;

impl DllRef {
    pub(crate) fn new() -> Self {
        OBJECTS.fetch_add(1, Ordering::SeqCst);
        DllRef
    }
}

impl Drop for DllRef {
    fn drop(&mut self) {
        OBJECTS.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Runs `f`, turning a panic into an error. Every call from Windows into this DLL goes through
/// here: the DLL runs inside other people's apps, and a panic unwinding into Windows would end the
/// app. riti panics when it can't write a file, so this is not hypothetical.
pub(crate) fn guard<T>(f: impl FnOnce() -> Result<T>) -> Result<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(result) => result,
        Err(_) => Err(windows::Win32::Foundation::E_UNEXPECTED.into()),
    }
}

#[no_mangle]
extern "system" fn DllMain(instance: HINSTANCE, reason: u32, _reserved: *mut c_void) -> BOOL {
    if reason == DLL_PROCESS_ATTACH {
        MODULE.store(instance.0, Ordering::Relaxed);
        unsafe {
            let _ = DisableThreadLibraryCalls(instance.into());
        }
    }
    true.into()
}

#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(clsid: *const GUID, iid: *const GUID, out: *mut *mut c_void) -> HRESULT {
    if out.is_null() || clsid.is_null() || iid.is_null() {
        return E_POINTER;
    }
    *out = std::ptr::null_mut();
    if *clsid != CLSID_TEXT_SERVICE {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    let factory: IClassFactory = ClassFactory { _dll: DllRef::new() }.into();
    factory.query(iid, out)
}

#[no_mangle]
pub extern "system" fn DllCanUnloadNow() -> HRESULT {
    if OBJECTS.load(Ordering::SeqCst) == 0 { S_OK } else { S_FALSE }
}

#[no_mangle]
pub extern "system" fn DllRegisterServer() -> HRESULT {
    match guard(register::register) {
        Ok(()) => S_OK,
        Err(error) => {
            let _ = guard(register::unregister);
            error.code()
        }
    }
}

#[no_mangle]
pub extern "system" fn DllUnregisterServer() -> HRESULT {
    match guard(register::unregister) {
        Ok(()) => S_OK,
        Err(error) => error.code(),
    }
}

#[implement(IClassFactory)]
struct ClassFactory {
    _dll: DllRef,
}

impl IClassFactory_Impl for ClassFactory_Impl {
    fn CreateInstance(&self, outer: Ref<IUnknown>, iid: *const GUID, out: *mut *mut c_void) -> Result<()> {
        guard(|| unsafe {
            if out.is_null() || iid.is_null() {
                return Err(E_POINTER.into());
            }
            *out = std::ptr::null_mut();
            if !outer.is_null() {
                return Err(CLASS_E_NOAGGREGATION.into());
            }
            let service: IUnknown = service::TextService::new().into();
            service.query(iid, out).ok()
        })
    }

    fn LockServer(&self, lock: BOOL) -> Result<()> {
        if lock.as_bool() {
            OBJECTS.fetch_add(1, Ordering::SeqCst);
        } else {
            OBJECTS.fetch_sub(1, Ordering::SeqCst);
        }
        Ok(())
    }
}

/// UTF-16, nul-terminated, for Windows APIs.
pub(crate) fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
