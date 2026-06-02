; Quark Downloader — Inno Setup 7 script
;
; Requires Inno Setup 7.0 beta or later.
;
; Build the application first:
;   just build
;
; Compile:
;   "C:\Program Files\Inno Setup 7\ISCC.exe" installer\quark-downloader.iss
;
; Output:
;   installer\output\quark-downloader-0.1.0-setup.exe

#define MyAppName       "Quark Downloader"
#define MyAppVersion    "0.1.0"
#define MyAppPublisher  "Quark Downloader"
#define MyAppExeName    "quark-downloader.exe"

#define BuildSource     "..\build\" + MyAppExeName

#ifexist BuildSource
#else
  #pragma error "Run `just build` first — expected ..\build\quark-downloader.exe"
#endif

[Setup]
AppId={{8F3C2A1B-4D5E-6F70-8A9B-0C1D2E3F4A5B}}

AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}

; Per-user install location.
; Writable by the current user, so the app can download/update tools in {app}\tools.
DefaultDirName={localappdata}\Programs\{#MyAppName}

DefaultGroupName={#MyAppName}
AllowNoIcons=yes

OutputDir=output
OutputBaseFilename=quark-downloader-{#MyAppVersion}-setup

Compression=lzma2/max
SolidCompression=yes
LZMAUseSeparateProcess=yes

; Inno Setup 7 dynamic theme.
; Automatically follows Windows light/dark mode.
WizardStyle=modern dynamic windows11

SetupArchitecture=x64
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

; No admin prompt.
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog

DisableProgramGroupPage=no

UninstallDisplayIcon={app}\{#MyAppExeName}
UninstallDisplayName={#MyAppName}

VersionInfoVersion={#MyAppVersion}.0
VersionInfoProductVersion={#MyAppVersion}
VersionInfoCompany={#MyAppPublisher}
VersionInfoDescription={#MyAppName} setup
VersionInfoProductName={#MyAppName}

MinVersion=10.0
ShowLanguageDialog=auto

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
; Main app executable
Source: "{#BuildSource}"; DestDir: "{app}"; Flags: ignoreversion restartreplace uninsrestartdelete

; Optional bundled tools.
; If present, they get included.
; If missing, your app can download them later into {app}\tools.
#ifexist "..\build\tools\ffmpeg.exe"
Source: "..\build\tools\ffmpeg.exe"; DestDir: "{app}\tools"; Flags: ignoreversion restartreplace uninsrestartdelete
#endif

#ifexist "..\build\tools\ffprobe.exe"
Source: "..\build\tools\ffprobe.exe"; DestDir: "{app}\tools"; Flags: ignoreversion restartreplace uninsrestartdelete
#endif

#ifexist "..\build\tools\yt-dlp.exe"
Source: "..\build\tools\yt-dlp.exe"; DestDir: "{app}\tools"; Flags: ignoreversion restartreplace uninsrestartdelete
#endif

[Dirs]
; Tool directory used by bundled and downloaded subprocess tools.
Name: "{app}\tools"

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"; Tasks: desktopicon