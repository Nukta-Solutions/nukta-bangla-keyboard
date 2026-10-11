; নুকতা বাংলা for Windows: one installer for 64-bit and 32-bit Windows (Inno Setup 6.3 or later).
;
; Built by windows/build.ps1, which passes the version and expects the release builds in
; windows/target/<target>/release. Saved as UTF-8 with a byte order mark, which Inno Setup needs for
; the Bangla text.
;
; On 64-bit Windows both DLLs are installed and registered, because 32-bit apps load the text
; service too: x64\nukta_bangla.dll for 64-bit apps, x86\nukta_bangla.dll for 32-bit ones. On
; 32-bit Windows only the x86 one is. Windows on Arm isn't supported yet: its native apps would
; need an Arm64 build of the DLL.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif

#define Release64 "..\target\x86_64-pc-windows-msvc\release"
#define Release32 "..\target\i686-pc-windows-msvc\release"

[Setup]
AppId={{6FB78ED3-FD8B-4DEF-9E2B-6315D8FAE825}
AppName=নুকতা বাংলা
AppVersion={#AppVersion}
AppVerName=নুকতা বাংলা {#AppVersion}
AppPublisher=Nukta Solutions
AppPublisherURL=https://github.com/Nukta-Solutions/nukta-bangla-keyboard
DefaultDirName={autopf}\Nukta Bangla
DisableProgramGroupPage=yes
DisableDirPage=auto
PrivilegesRequired=admin
MinVersion=10.0
ArchitecturesAllowed=x64os or x86os
ArchitecturesInstallIn64BitMode=x64os
; Every app that types has the DLL loaded: never try to close them all. An update in use is
; replaced on the next restart instead (restartreplace below), and Setup asks for one.
CloseApplications=no
RestartIfNeededByRun=no
SetupIconFile=nukta-bangla.ico
UninstallDisplayIcon={app}\nukta-bangla.ico
UninstallDisplayName=নুকতা বাংলা
WizardStyle=modern
OutputDir=..\..\build\windows
OutputBaseFilename=NuktaBangla-Setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
; Code signing, once a certificate is in place: define a sign tool named "signtool" in Inno
; Setup (or pass /S on the command line) and uncomment.
; SignTool=signtool
; SignedUninstaller=yes

[Files]
Source: "{#Release64}\nukta_bangla.dll"; DestDir: "{app}\x64"; Flags: ignoreversion regserver restartreplace uninsrestartdelete 64bit; Check: Is64BitInstallMode
Source: "{#Release32}\nukta_bangla.dll"; DestDir: "{app}\x86"; Flags: ignoreversion regserver restartreplace uninsrestartdelete 32bit
; The settings program is 32-bit, so one copy runs everywhere.
Source: "{#Release32}\NuktaBanglaSettings.exe"; DestDir: "{app}"; Flags: ignoreversion restartreplace uninsrestartdelete
Source: "nukta-bangla.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}\Licenses"; DestName: "LICENSE.txt"; Flags: ignoreversion
Source: "..\..\Licenses\MPL-2.0.txt"; DestDir: "{app}\Licenses"; Flags: ignoreversion
Source: "..\..\Licenses\THIRD-PARTY-NOTICES-WINDOWS.txt"; DestDir: "{app}\Licenses"; DestName: "THIRD-PARTY-NOTICES.txt"; Flags: ignoreversion
; riti is MPL-2.0: its source ships with it (made by build.ps1).
Source: "..\..\build\windows\MPL-source.zip"; DestDir: "{app}\Licenses"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\নুকতা বাংলা সেটিংস"; Filename: "{app}\NuktaBanglaSettings.exe"; IconFilename: "{app}\nukta-bangla.ico"

[Run]
; Adds the keyboard to the languages of the user who ran Setup (not the administrator account it
; may have been elevated to).
Filename: "{app}\NuktaBanglaSettings.exe"; Parameters: "--enable"; Flags: runasoriginaluser waituntilterminated; StatusMsg: "Adding নুকতা বাংলা to your keyboards…"
Filename: "{app}\NuktaBanglaSettings.exe"; Description: "Open নুকতা বাংলা settings"; Flags: postinstall nowait skipifsilent runasoriginaluser unchecked

[UninstallRun]
Filename: "{app}\NuktaBanglaSettings.exe"; Parameters: "--disable"; RunOnceId: "RemoveFromKeyboards"; Flags: waituntilterminated
