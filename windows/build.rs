//! Gives the settings program a manifest asking for the current look of Windows controls (common
//! controls 6); without it, its buttons and lists would be drawn as in Windows 95.

fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("windows-msvc") {
        println!("cargo:rustc-link-arg-bin=NuktaBanglaSettings=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bin=NuktaBanglaSettings=/MANIFESTDEPENDENCY:type='win32' \
             name='Microsoft.Windows.Common-Controls' version='6.0.0.0' processorArchitecture='*' \
             publicKeyToken='6595b64144ccf1df' language='*'"
        );
    }
}
