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

use crate::tsf::{guid_string, CLSID_TEXT_SERVICE, GUID_PROFILE, LANGID_BANGLA};

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

/// `InstallLayoutOrTip`'s name for our profile: language:{text service}{profile}.
fn profile_string() -> Vec<u16> {
    format!("{LANGID_BANGLA:04x}:{}{}", guid_string(&CLSID_TEXT_SERVICE), guid_string(&GUID_PROFILE))
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

const ILOT_UNINSTALL: u32 = 0x0000_0001;

/// Adds নুকতা বাংলা to the current user's keyboards (adding Bangla to their languages if needed),
/// or removes it. The DLLs must be registered first.
pub fn enable_for_user(enable: bool) -> Result<()> {
    type InstallLayoutOrTip = unsafe extern "system" fn(PCWSTR, u32) -> BOOL;
    unsafe {
        let library = LoadLibraryW(w!("input.dll"))?;
        let Some(function) = GetProcAddress(library, s!("InstallLayoutOrTip")) else {
            return Err(windows::core::Error::from_thread());
        };
        let install: InstallLayoutOrTip = std::mem::transmute(function);
        let profile = profile_string();
        install(PCWSTR(profile.as_ptr()), if enable { 0 } else { ILOT_UNINSTALL }).ok()
    }
}

/// The settings program, beside the installed DLLs' folders (`x64\`, `x86\`).
pub fn settings_program(dll: &Path) -> PathBuf {
    dll.parent()
        .and_then(Path::parent)
        .unwrap_or(Path::new(""))
        .join("NuktaBanglaSettings.exe")
}
