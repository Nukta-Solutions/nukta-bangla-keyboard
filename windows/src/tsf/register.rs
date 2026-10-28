//! What `regsvr32` writes and removes: the COM class, the language profile and the TSF categories.
//! The installer runs it for the 64-bit and the 32-bit DLL; each writes its own registry view.
//!
//! Adding the keyboard to a user's language list is separate, per user, and done by the settings
//! program (`win::enable_for_user`), which the installer runs as the user who installed.

use windows::core::{Result, GUID, PCWSTR};
use windows::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Registry::{
    RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_CLASSES_ROOT, KEY_WRITE, REG_OPTION_NON_VOLATILE,
    REG_SZ,
};
use windows::Win32::UI::Input::KeyboardAndMouse::HKL;
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, CLSID_TF_InputProcessorProfiles, ITfCategoryMgr, ITfInputProcessorProfileMgr,
    GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER, GUID_TFCAT_TIPCAP_COMLESS, GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT, GUID_TFCAT_TIP_KEYBOARD,
};

use super::{module, wide, CLSID_TEXT_SERVICE, DISPLAY_NAME, GUID_PROFILE, LANGID_KEYBOARD, LANGID_OLD};

/// What the text service is, for TSF:
/// - a keyboard text service, with its own display attribute (the underline);
/// - fit for Store apps (immersive), which also need it loadable without COM (comless);
///
/// Not listed, on purpose: secure mode (the sign-in screen), UI-less mode (games drawing the
/// suggestion list themselves), and system-tray support: the keyboard puts nothing beside Windows'
/// language indicator. English and Bangla are switched with F12 and নুকতা বাংলা's own top bar and
/// tray icon.
const CATEGORIES: [GUID; 4] = [
    GUID_TFCAT_TIP_KEYBOARD,
    GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER,
    GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    GUID_TFCAT_TIPCAP_COMLESS,
];

/// Categories earlier versions registered, removed again on (re-)registration.
const OLD_CATEGORIES: [GUID; 1] = [GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT];

pub(super) fn register() -> Result<()> {
    let _com = Com::init();
    let dll = dll_path();
    register_class(&dll)?;

    unsafe {
        let profiles: ITfInputProcessorProfileMgr =
            CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
        let _ = profiles.UnregisterProfile(&CLSID_TEXT_SERVICE, LANGID_OLD, &GUID_PROFILE, 0);
        let name: Vec<u16> = DISPLAY_NAME.encode_utf16().collect();
        let icon: Vec<u16> = icon_path(&dll).encode_utf16().collect();
        profiles.RegisterProfile(
            &CLSID_TEXT_SERVICE,
            LANGID_KEYBOARD,
            &GUID_PROFILE,
            &name,
            &icon,
            0,
            HKL::default(),
            0,
            true,
            0,
        )?;

        let categories: ITfCategoryMgr = CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER)?;
        for category in &OLD_CATEGORIES {
            let _ = categories.UnregisterCategory(&CLSID_TEXT_SERVICE, category, &CLSID_TEXT_SERVICE);
        }
        for category in &CATEGORIES {
            categories.RegisterCategory(&CLSID_TEXT_SERVICE, category, &CLSID_TEXT_SERVICE)?;
        }
    }
    Ok(())
}

/// Removes everything `register` wrote. Each step runs even if an earlier one fails (half a
/// registration is what this cleans up), and the first error is returned.
pub(super) fn unregister() -> Result<()> {
    let _com = Com::init();
    let mut result = Ok(());
    unsafe {
        match CoCreateInstance::<_, ITfCategoryMgr>(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER) {
            Ok(categories) => {
                for category in CATEGORIES.iter().chain(&OLD_CATEGORIES) {
                    let _ = categories.UnregisterCategory(&CLSID_TEXT_SERVICE, category, &CLSID_TEXT_SERVICE);
                }
            }
            Err(error) => result = Err(error),
        }
        match CoCreateInstance::<_, ITfInputProcessorProfileMgr>(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER) {
            Ok(profiles) => {
                for language in [LANGID_KEYBOARD, LANGID_OLD] {
                    let _ = profiles.UnregisterProfile(&CLSID_TEXT_SERVICE, language, &GUID_PROFILE, 0);
                }
            }
            Err(error) => result = result.and(Err(error)),
        }
        let key = wide(&format!("CLSID\\{}", guid_string(&CLSID_TEXT_SERVICE)));
        let status = RegDeleteTreeW(HKEY_CLASSES_ROOT, PCWSTR(key.as_ptr()));
        if status != ERROR_SUCCESS && status != ERROR_FILE_NOT_FOUND {
            result = result.and(Err(status.to_hresult().into()));
        }
    }
    result
}

/// `HKCR\CLSID\{…}\InprocServer32` = this DLL, apartment-threaded, as every text service is.
fn register_class(dll: &str) -> Result<()> {
    let class = format!("CLSID\\{}", guid_string(&CLSID_TEXT_SERVICE));
    let key = create_key(&class)?;
    set_string(&key, None, DISPLAY_NAME)?;
    let server = create_key(&format!("{class}\\InprocServer32"))?;
    set_string(&server, None, dll)?;
    set_string(&server, Some("ThreadingModel"), "Apartment")?;
    Ok(())
}

/// An open registry key, closed when dropped: after all of its values are written, not after the
/// first one.
struct Key(HKEY);

impl Drop for Key {
    fn drop(&mut self) {
        let _ = unsafe { windows::Win32::System::Registry::RegCloseKey(self.0) };
    }
}

fn create_key(path: &str) -> Result<Key> {
    let path = wide(path);
    let mut key = HKEY::default();
    unsafe {
        RegCreateKeyExW(
            HKEY_CLASSES_ROOT,
            PCWSTR(path.as_ptr()),
            None,
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            None,
            &mut key,
            None,
        )
        .ok()?;
    }
    Ok(Key(key))
}

fn set_string(key: &Key, name: Option<&str>, value: &str) -> Result<()> {
    let name = name.map(wide);
    let value = wide(value);
    let bytes = unsafe { std::slice::from_raw_parts(value.as_ptr() as *const u8, value.len() * 2) };
    unsafe {
        RegSetValueExW(
            key.0,
            name.as_ref().map_or(PCWSTR::null(), |n| PCWSTR(n.as_ptr())),
            None,
            REG_SZ,
            Some(bytes),
        )
        .ok()
    }
}

/// This DLL's full path.
pub(crate) fn dll_path() -> String {
    let mut buffer = [0u16; 1024];
    let length = unsafe { GetModuleFileNameW(Some(module().into()), &mut buffer) } as usize;
    String::from_utf16_lossy(&buffer[..length])
}

/// The installer puts the icon beside the settings program, one level above the DLLs
/// (`x64\nukta_bangla.dll`, `x86\nukta_bangla.dll`).
fn icon_path(dll: &str) -> String {
    let dll = std::path::Path::new(dll);
    let root = dll.parent().and_then(|p| p.parent()).unwrap_or(std::path::Path::new(""));
    root.join("nukta-bangla.ico").to_string_lossy().into_owned()
}

pub(crate) fn guid_string(guid: &GUID) -> String {
    format!("{{{guid:?}}}")
}

/// COM for the length of a registration call. `regsvr32` has usually started it already.
struct Com(bool);

impl Com {
    fn init() -> Com {
        Com(unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }.is_ok())
    }
}

impl Drop for Com {
    fn drop(&mut self) {
        if self.0 {
            unsafe { CoUninitialize() };
        }
    }
}
