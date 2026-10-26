//! What the text service and the settings program share on Windows: where the files live, letting
//! Store apps read them, and adding the keyboard to (or removing it from) a user's languages.

use std::path::{Path, PathBuf};

use windows::core::{s, w, Result, BOOL, PCWSTR, PWSTR};
use windows::Win32::Foundation::{LocalFree, ERROR_SUCCESS, HLOCAL};
use windows::Win32::Security::Authorization::{
    ConvertStringSidToSidW, GetNamedSecurityInfoW, SetEntriesInAclW, SetNamedSecurityInfoW, EXPLICIT_ACCESS_W,
    GRANT_ACCESS, SE_FILE_OBJECT, TRUSTEE_IS_SID, TRUSTEE_IS_WELL_KNOWN_GROUP, TRUSTEE_W,
};
use windows::Win32::Security::{
    ACL, DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, SUB_CONTAINERS_AND_OBJECTS_INHERIT,
};
use windows::Win32::Storage::FileSystem::{DELETE, FILE_GENERIC_EXECUTE, FILE_GENERIC_READ, FILE_GENERIC_WRITE};
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows::Win32::UI::Shell::{FOLDERID_RoamingAppData, SHGetKnownFolderPath, KF_FLAG_NO_PACKAGE_REDIRECTION};

use crate::tsf::{guid_string, CLSID_TEXT_SERVICE, GUID_PROFILE, LANGID_KEYBOARD, LANGID_OLD};

/// `%APPDATA%\Nukta Bangla`: the settings file, and what riti and the pick memory learn.
///
/// Asked for without package redirection, so a Store app (which has an `%APPDATA%` of its own)
/// gets the same directory as every other app.
pub fn data_directory() -> PathBuf {
    let base = unsafe {
        SHGetKnownFolderPath(&FOLDERID_RoamingAppData, KF_FLAG_NO_PACKAGE_REDIRECTION, None)
            .ok()
            .map(|path| {
                let text = path.to_string().unwrap_or_default();
                CoTaskMemFree(Some(path.0 as *const _));
                PathBuf::from(text)
            })
    };
    base.or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))
        .unwrap_or_else(std::env::temp_dir)
        .join("Nukta Bangla")
}

/// Creates the data directory if needed, readable and writable by Store apps too.
///
/// Store apps run in an app container, which can't open a user's files unless they say so. Without
/// this, phonetic typing in them couldn't learn, and the shared layout setting couldn't reach them.
pub fn prepare_data_directory() -> PathBuf {
    let directory = data_directory();
    let existed = directory.exists();
    let _ = std::fs::create_dir_all(&directory);
    if !existed || !app_containers_allowed(&directory) {
        let _ = allow_app_containers(&directory);
    }
    directory
}

/// ALL APPLICATION PACKAGES and ALL RESTRICTED APPLICATION PACKAGES.
const APP_CONTAINER_SIDS: [PCWSTR; 2] = [w!("S-1-15-2-1"), w!("S-1-15-2-2")];

/// A marker written after the permissions are set, so they are set once, not on every start.
const PERMISSIONS_MARKER: &str = ".app-containers";

fn app_containers_allowed(directory: &Path) -> bool {
    directory.join(PERMISSIONS_MARKER).exists()
}

fn allow_app_containers(directory: &Path) -> Result<()> {
    let path: Vec<u16> = directory.as_os_str().to_string_lossy().encode_utf16().chain(Some(0)).collect();
    unsafe {
        let mut old_acl: *mut ACL = std::ptr::null_mut();
        let mut descriptor = PSECURITY_DESCRIPTOR::default();
        let status = GetNamedSecurityInfoW(
            PCWSTR(path.as_ptr()),
            SE_FILE_OBJECT,
            DACL_SECURITY_INFORMATION,
            None,
            None,
            Some(&mut old_acl),
            None,
            &mut descriptor,
        );
        if status != ERROR_SUCCESS {
            return Err(status.to_hresult().into());
        }

        let mut sids = Vec::new();
        for text in APP_CONTAINER_SIDS {
            let mut sid = PSID::default();
            if ConvertStringSidToSidW(text, &mut sid).is_ok() {
                sids.push(sid);
            }
        }
        let entries: Vec<EXPLICIT_ACCESS_W> = sids
            .iter()
            .map(|sid| EXPLICIT_ACCESS_W {
                grfAccessPermissions: (FILE_GENERIC_READ | FILE_GENERIC_WRITE | FILE_GENERIC_EXECUTE | DELETE).0,
                grfAccessMode: GRANT_ACCESS,
                grfInheritance: SUB_CONTAINERS_AND_OBJECTS_INHERIT,
                Trustee: TRUSTEE_W {
                    TrusteeForm: TRUSTEE_IS_SID,
                    TrusteeType: TRUSTEE_IS_WELL_KNOWN_GROUP,
                    ptstrName: PWSTR(sid.0 as *mut u16),
                    ..Default::default()
                },
            })
            .collect();

        let mut new_acl: *mut ACL = std::ptr::null_mut();
        let status = SetEntriesInAclW(Some(&entries), Some(old_acl), &mut new_acl);
        let result = if status == ERROR_SUCCESS {
            let status = SetNamedSecurityInfoW(
                PCWSTR(path.as_ptr()),
                SE_FILE_OBJECT,
                DACL_SECURITY_INFORMATION,
                None,
                None,
                Some(new_acl),
                None,
            );
            if status == ERROR_SUCCESS { Ok(()) } else { Err(status.to_hresult().into()) }
        } else {
            Err(status.to_hresult().into())
        };

        if !new_acl.is_null() {
            LocalFree(Some(HLOCAL(new_acl as *mut _)));
        }
        for sid in sids {
            LocalFree(Some(HLOCAL(sid.0)));
        }
        LocalFree(Some(HLOCAL(descriptor.0)));
        if result.is_ok() {
            let _ = std::fs::write(directory.join(PERMISSIONS_MARKER), b"");
        }
        result
    }
}

/// `InstallLayoutOrTip`'s name for our profile in `language`: language:{text service}{profile}.
fn profile_string(language: u16) -> Vec<u16> {
    format!("{language:04x}:{}{}", guid_string(&CLSID_TEXT_SERVICE), guid_string(&GUID_PROFILE))
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

const ILOT_UNINSTALL: u32 = 0x0000_0001;

/// The plain US keyboard, as `InstallLayoutOrTip` names it.
const US_KEYBOARD: PCWSTR = w!("0409:00000409");

/// Adds নুকতা বাংলা to the current user's keyboards, or removes it. The DLLs must be registered
/// first. An entry left under Bangla by an older version is removed either way.
///
/// Added, it is the default and the only English keyboard: its English mode types exactly as the
/// US keyboard does, and with one keyboard Windows shows no language indicator (EN) in the taskbar.
/// English and Bangla are switched with F12 and নুকতা বাংলা's own top bar and tray icon instead.
/// Keyboards of other languages the user added are left alone. Removed, the US keyboard comes back
/// first, so the user is never left without one.
pub fn enable_for_user(enable: bool) -> Result<()> {
    type LayoutOrTip = unsafe extern "system" fn(PCWSTR, u32) -> BOOL;
    unsafe {
        let library = LoadLibraryW(w!("input.dll"))?;
        let Some(install) = GetProcAddress(library, s!("InstallLayoutOrTip")) else {
            return Err(windows::core::Error::from_thread());
        };
        let install: LayoutOrTip = std::mem::transmute(install);
        let set_default: Option<LayoutOrTip> =
            GetProcAddress(library, s!("SetDefaultLayoutOrTip")).map(|f| std::mem::transmute(f));
        let old = profile_string(LANGID_OLD);
        let _ = install(PCWSTR(old.as_ptr()), ILOT_UNINSTALL);
        let profile = profile_string(LANGID_KEYBOARD);
        if enable {
            install(PCWSTR(profile.as_ptr()), 0).ok()?;
            if let Some(set_default) = set_default {
                let _ = set_default(PCWSTR(profile.as_ptr()), 0);
            }
            let _ = install(US_KEYBOARD, ILOT_UNINSTALL);
        } else {
            let _ = install(US_KEYBOARD, 0);
            if let Some(set_default) = set_default {
                let _ = set_default(US_KEYBOARD, 0);
            }
            install(PCWSTR(profile.as_ptr()), ILOT_UNINSTALL).ok()?;
        }
        Ok(())
    }
}

/// Makes নুকতা বাংলা the active keyboard in every app of this sign-in session, as when Bangla is
/// picked from the floating bar while another keyboard (the plain US one) is active.
pub fn activate_keyboard() -> Result<()> {
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
    use windows::Win32::UI::Input::KeyboardAndMouse::HKL;
    use windows::Win32::UI::TextServices::{
        CLSID_TF_InputProcessorProfiles, ITfInputProcessorProfileMgr, TF_IPPMF_DONTCARECURRENTINPUTLANGUAGE,
        TF_IPPMF_FORSESSION, TF_PROFILETYPE_INPUTPROCESSOR,
    };
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let profiles: ITfInputProcessorProfileMgr =
            CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER)?;
        profiles.ActivateProfile(
            TF_PROFILETYPE_INPUTPROCESSOR,
            LANGID_KEYBOARD,
            &CLSID_TEXT_SERVICE,
            &GUID_PROFILE,
            HKL::default(),
            TF_IPPMF_FORSESSION | TF_IPPMF_DONTCARECURRENTINPUTLANGUAGE,
        )
    }
}

/// The bundled Bangla font, Noto Serif Bengali, in `fonts\` of the install folder (`dir`: the
/// folder of the programs), if it is there.
pub fn bangla_font_in(dir: &Path) -> Option<PathBuf> {
    let font = dir.join("fonts").join("NotoSerifBengali.ttf");
    font.exists().then_some(font)
}

/// The bundled Bangla font for a program in the install folder (the top bar, the settings).
pub fn bangla_font_for_program() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    bangla_font_in(exe.parent()?)
}

/// Makes the bundled Bangla font usable by GDI in this process only (nothing is installed in
/// Windows), once. False if it isn't there.
pub fn load_bangla_font_for_gdi(font: &Path) -> bool {
    use std::sync::OnceLock;
    use windows::Win32::Graphics::Gdi::{AddFontResourceExW, FR_PRIVATE};
    static LOADED: OnceLock<bool> = OnceLock::new();
    *LOADED.get_or_init(|| {
        let path: Vec<u16> = font.as_os_str().to_string_lossy().encode_utf16().chain(Some(0)).collect();
        unsafe { AddFontResourceExW(PCWSTR(path.as_ptr()), FR_PRIVATE, None) > 0 }
    })
}

/// The floating bar and tray icon, beside the settings program.
pub fn bar_program(dll: &Path) -> PathBuf {
    settings_program(dll).with_file_name("NuktaBangla.exe")
}

/// The settings program, beside the installed DLLs' folders (`x64\`, `x86\`).
pub fn settings_program(dll: &Path) -> PathBuf {
    dll.parent()
        .and_then(Path::parent)
        .unwrap_or(Path::new(""))
        .join("NuktaBanglaSettings.exe")
}
