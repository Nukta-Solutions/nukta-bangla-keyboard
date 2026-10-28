<#
Builds নুকতা বাংলা for Windows and its installer:

    build\windows\NuktaBangla-Setup.exe

Needs Rust (https://rustup.rs) with the MSVC build tools, and Inno Setup 6.3 or later
(https://jrsoftware.org/isinfo.php) for the installer. Run from any folder:

    powershell -ExecutionPolicy Bypass -File windows\build.ps1
    powershell -ExecutionPolicy Bypass -File windows\build.ps1 -SkipInstaller   # DLLs and tests only
#>
param([switch]$SkipInstaller)

$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
$Root = Split-Path $PSScriptRoot -Parent
$Targets = @('x86_64-pc-windows-msvc', 'i686-pc-windows-msvc')

function Invoke-Checked([string]$What, [scriptblock]$Command) {
    & $Command
    if ($LASTEXITCODE -ne 0) { throw "$What failed (exit code $LASTEXITCODE)" }
}

Invoke-Checked 'rustup' { rustup target add @Targets }
Invoke-Checked 'tests' { cargo test --release --locked }
foreach ($target in $Targets) {
    Invoke-Checked "build for $target" { cargo build --release --locked --target $target }
}

# The DLL must export the four COM entry points under their plain names, the 32-bit one included.
$dumpbin = Get-Command dumpbin -ErrorAction SilentlyContinue
if ($dumpbin) {
    foreach ($target in $Targets) {
        $exports = & $dumpbin.Source /exports "target\$target\release\nukta_bangla.dll" | Out-String
        foreach ($name in 'DllGetClassObject', 'DllCanUnloadNow', 'DllRegisterServer', 'DllUnregisterServer') {
            # 32-bit dumpbin lists the decorated symbol after the name: "DllGetClassObject = _DllGetClassObject@12".
            if ($exports -notmatch "(?m)\s$name(\s*=.*)?\r?$") { throw "$target DLL does not export $name" }
        }
    }
    Write-Host 'DLL exports checked.'
}

if ($SkipInstaller) { return }

# riti is MPL-2.0: its source ships with the app, as on macOS (MPL-source.zip) and Linux.
$Out = Join-Path $Root 'build\windows'
New-Item -ItemType Directory -Force $Out | Out-Null
$Source = Join-Path $Out 'MPL-source.zip'
Remove-Item $Source -ErrorAction SilentlyContinue
Push-Location $Root
Compress-Archive -Path 'riti-bridge\riti', 'riti-bridge\riti.patch' -DestinationPath $Source
Pop-Location

$Version = (Select-String -Path 'Cargo.toml' -Pattern '^version = "(.+)"').Matches[0].Groups[1].Value
$iscc = Get-Command iscc -ErrorAction SilentlyContinue
$isccPath = if ($iscc) { $iscc.Source } else { Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe' }
if (-not (Test-Path $isccPath)) { throw 'Inno Setup 6 not found: install it from https://jrsoftware.org/isinfo.php' }
Invoke-Checked 'Inno Setup' { & $isccPath "/DAppVersion=$Version" 'installer\nukta-bangla.iss' }
Write-Host "Built $(Join-Path $Out "NuktaBangla-Setup.exe") (v$Version)"
