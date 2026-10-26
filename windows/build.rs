//! - Gives the programs their icon (the logo) and version details: `installer/nukta-bangla.rc`.
//! - Gives the settings program a manifest asking for the current look of Windows controls (common
//!   controls 6); without it, its buttons and lists would be drawn as in Windows 95.

fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if !target.contains("windows-msvc") {
        return;
    }
    println!("cargo:rerun-if-changed=installer/nukta-bangla.rc");
    println!("cargo:rerun-if-changed=installer/nukta-bangla.ico");

    let version = std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.0.0".into());
    let mut parts: Vec<&str> = version.split('.').collect();
    parts.resize(4, "0");
    let comma = format!("NUKTA_VERSION_COMMA={}", parts.join(","));
    let text = format!("NUKTA_VERSION_TEXT=\"{version}\"");
    embed_resource::compile("installer/nukta-bangla.rc", [comma.as_str(), text.as_str()])
        .manifest_optional()
        .expect("compiling installer/nukta-bangla.rc");

    println!("cargo:rustc-link-arg-bin=NuktaBanglaSettings=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-bin=NuktaBanglaSettings=/MANIFESTDEPENDENCY:type='win32' \
         name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' \
         publicKeyToken='6595b64144ccf1df' language='*'"
    );
}
