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
; Light, to match the artwork: the small logo sits on the white page header.
WizardStyle=modern
; Branded artwork (make-artwork.ps1) at each display scaling; Inno Setup picks the closest.
WizardImageFile=wizard-100.bmp,wizard-125.bmp,wizard-150.bmp,wizard-175.bmp,wizard-200.bmp
WizardSmallImageFile=wizard-small-100.bmp,wizard-small-125.bmp,wizard-small-150.bmp,wizard-small-175.bmp,wizard-small-200.bmp
WizardImageStretch=no
DisableWelcomePage=no
OutputDir=..\..\build\windows
; No version in the name: the website links to releases/latest/download/NuktaBangla-Setup.exe.
OutputBaseFilename=NuktaBangla-Setup
Compression=lzma2
SolidCompression=yes
; Code signing, once a certificate is in place: define a sign tool named "signtool" in Inno
; Setup (or pass /S on the command line) and uncomment.
; SignTool=signtool
; SignedUninstaller=yes

[Files]
Source: "{#Release64}\nukta_bangla.dll"; DestDir: "{app}\x64"; Flags: ignoreversion regserver restartreplace uninsrestartdelete 64bit; Check: Is64BitInstallMode
Source: "{#Release32}\nukta_bangla.dll"; DestDir: "{app}\x86"; Flags: ignoreversion regserver restartreplace uninsrestartdelete 32bit
; The top bar and tray icon, and the settings program: 32-bit, so one copy runs everywhere.
Source: "{#Release32}\NuktaBangla.exe"; DestDir: "{app}"; Flags: ignoreversion restartreplace uninsrestartdelete
Source: "{#Release32}\NuktaBanglaSettings.exe"; DestDir: "{app}"; Flags: ignoreversion restartreplace uninsrestartdelete
Source: "nukta-bangla.ico"; DestDir: "{app}"; Flags: ignoreversion
; The brand's Bangla font, for নুকতা বাংলা's own windows and its suggestion list only: loaded from
; here through DirectWrite by the programs and the keyboard, never installed in Windows. Apps may
; hold it open, so an update in use is replaced on the next restart.
; (Not for this installer's own text: Inno Setup draws with GDI, which misplaces this font's ু —
; নুকতা came out as নকতু. Its pages keep Windows' fonts; its pictures are drawn in Noto.)
Source: "..\assets\fonts\NotoSerifBengali.ttf"; DestDir: "{app}\fonts"; Flags: ignoreversion restartreplace uninsrestartdelete
Source: "..\assets\fonts\OFL.txt"; DestDir: "{app}\Licenses"; DestName: "OFL-NotoSerifBengali.txt"; Flags: ignoreversion
Source: "..\..\LICENSE"; DestDir: "{app}\Licenses"; DestName: "LICENSE.txt"; Flags: ignoreversion
Source: "..\..\Licenses\MPL-2.0.txt"; DestDir: "{app}\Licenses"; Flags: ignoreversion
Source: "..\..\Licenses\THIRD-PARTY-NOTICES-WINDOWS.txt"; DestDir: "{app}\Licenses"; DestName: "THIRD-PARTY-NOTICES.txt"; Flags: ignoreversion
; riti is MPL-2.0: its source ships with it (made by build.ps1).
Source: "..\..\build\windows\MPL-source.zip"; DestDir: "{app}\Licenses"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\নুকতা বাংলা"; Filename: "{app}\NuktaBangla.exe"; IconFilename: "{app}\nukta-bangla.ico"
Name: "{autoprograms}\নুকতা বাংলা সেটিংস"; Filename: "{app}\NuktaBanglaSettings.exe"; IconFilename: "{app}\nukta-bangla.ico"

[Registry]
; The top bar and tray icon start with Windows, for every user, showing the splash screen first
; (--startup).
Root: HKLM; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "NuktaBangla"; ValueData: """{app}\NuktaBangla.exe"" --startup"; Flags: uninsdeletevalue

[Messages]
WelcomeLabel1=নুকতা বাংলা-য় স্বাগতম
WelcomeLabel2=এটি আপনার কম্পিউটারে নুকতা বাংলা {#AppVersion} ইনস্টল করবে: বিজয় লেআউট আর ফোনেটিক টাইপিং, ইউনিকোডে।%n%nইনস্টলের পর স্ক্রিনের উপরে নুকতা বাংলার বার আর টাস্কবারের ট্রেতে আইকন দেখবেন। F12 চাপলে বা আইকনে ক্লিক করলে বাংলা চালু বা বন্ধ হবে।%n%nThis installs Nukta Bangla {#AppVersion}. Press F12 or click the tray icon to switch between English and Bangla.
FinishedHeadingLabel=নুকতা বাংলা তৈরি
FinishedLabel=ইনস্টল শেষ। F12 চাপুন বা বারে ক্লিক করুন, বাংলা লেখা শুরু।%n%nকিবোর্ড নিয়ন্ত্রণ করতে টাস্কবারের ডান দিকে সিস্টেম ট্রেতে নুকতা বাংলার আইকন দেখুন: ক্লিক করলে বাংলা চালু বা বন্ধ হবে, রাইট-ক্লিক করলে মেনু খুলবে। আইকন না দেখলে ট্রের ^ চিহ্নে ক্লিক করুন।%n%nNukta Bangla is installed. Press F12 to start typing Bangla. Check the system tray (bottom right of the taskbar) for the Nukta Bangla icon: click it to switch Bangla on or off, right-click it to open the keyboard menu.

[Run]
; Adds the keyboard to the languages of the user who ran Setup (not the administrator account it
; may have been elevated to).
Filename: "{app}\NuktaBanglaSettings.exe"; Parameters: "--enable"; Flags: runasoriginaluser waituntilterminated; StatusMsg: "Adding নুকতা বাংলা to your keyboards…"
; Every install ends with the top bar on screen, not in the tray, even if it was left there before
; (the settings outlive an uninstall, and an update keeps them).
Filename: "{app}\NuktaBanglaSettings.exe"; Parameters: "--show-bar"; Flags: runasoriginaluser waituntilterminated
; A first install starts in English, with ফোনেটিক as the layout F12 turns on and ফোনেটিক আগে
; as its typing mode.
Filename: "{app}\NuktaBanglaSettings.exe"; Parameters: "--first-install"; Flags: runasoriginaluser waituntilterminated; Check: IsFreshInstall
; With the splash screen, as when Windows starts: no restart is needed on a first install.
; An update run from the update dialog (/SILENT) skips the finish page: start the new top bar here.
Filename: "{app}\NuktaBangla.exe"; Flags: nowait runasoriginaluser; Check: WizardSilent
Filename: "{app}\NuktaBangla.exe"; Parameters: "--startup"; Description: "নুকতা বাংলা চালু করুন (Launch Nukta Bangla)"; Flags: postinstall nowait skipifsilent runasoriginaluser
Filename: "{app}\NuktaBanglaSettings.exe"; Description: "সেটিংস খুলুন (Open settings)"; Flags: postinstall nowait skipifsilent runasoriginaluser unchecked

[UninstallRun]
Filename: "{app}\NuktaBangla.exe"; Parameters: "--quit"; RunOnceId: "CloseBar"; Flags: waituntilterminated
Filename: "{app}\NuktaBanglaSettings.exe"; Parameters: "--disable"; RunOnceId: "RemoveFromKeyboards"; Flags: waituntilterminated

[Code]
var
  FreshInstall: Boolean;

// An update closes the running top bar first, so its program can be replaced; Setup starts the
// new one at the end. No program there yet means a first install (or one after an uninstall).
function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  Code: Integer;
begin
  FreshInstall := not FileExists(ExpandConstant('{app}\NuktaBangla.exe'));
  if not FreshInstall then
    Exec(ExpandConstant('{app}\NuktaBangla.exe'), '--quit', '', SW_HIDE, ewWaitUntilTerminated, Code);
  Result := '';
end;

function IsFreshInstall: Boolean;
begin
  Result := FreshInstall;
end;
