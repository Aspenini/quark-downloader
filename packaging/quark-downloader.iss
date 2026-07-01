; Quark Downloader - Inno Setup 7 script
;
; Build the release binaries first:
;   cargo build --release --workspace   (or: just build)
;
; Open this file in the Inno Setup Compiler (paths are relative to packaging/).

#define MyAppName       "Quark Downloader"
#define MyAppVersion    "0.6.0"
#define MyAppPublisher  "Quark Downloader"
#define MyAppExeName    "quark-downloader.exe"
#define MyAppGuiExeName "quark-downloader-gui.exe"

#define BuildDir        "..\target\release"
#define BuildSource     BuildDir + "\" + MyAppExeName
#define BuildGuiSource  BuildDir + "\" + MyAppGuiExeName
#define AppIcon         "..\icons\icon.ico"
#define CliAppIcon      "..\icons\icon-cli.ico"
#define GuiIconName     "icon.ico"
#define CliIconName     "icon-cli.ico"

#ifexist BuildSource
#else
  #pragma error "Run `cargo build --release --workspace` first - expected ..\target\release\quark-downloader.exe"
#endif

#ifexist BuildGuiSource
#else
  #pragma error "Run `cargo build --release --workspace` first - expected ..\target\release\quark-downloader-gui.exe"
#endif

#ifexist AppIcon
#else
  #pragma error "Expected icons\icon.ico"
#endif

#ifexist CliAppIcon
#else
  #pragma error "Expected icons\icon-cli.ico"
#endif

[Setup]
AppId={{8F3C2A1B-4D5E-6F70-8A9B-0C1D2E3F4A5B}}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
SetupIconFile={#AppIcon}

; Per-user install location, writable so the app can download tools into {app}\tools.
DefaultDirName={localappdata}\Programs\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes

OutputDir=output
OutputBaseFilename=quark-downloader-{#MyAppVersion}-setup

Compression=lzma2/max
SolidCompression=yes
LZMAUseSeparateProcess=yes

; Inno Setup 7 dynamic theme; follows Windows light/dark mode.
WizardStyle=modern dynamic windows11

ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

; Per-user install only. No admin prompt.
PrivilegesRequired=lowest
DisableProgramGroupPage=no

UninstallDisplayIcon={app}\{#GuiIconName}
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
Name: "gui"; Description: "Create a &desktop shortcut for the GUI"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked
Name: "cli"; Description: "Create a &desktop shortcut for the CLI"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Dirs]
; Tool directory for bundled/downloaded subprocess tools (yt-dlp, ffmpeg).
Name: "{app}\tools"

[Files]
; Statically-linked Rust binaries — no extra runtime DLLs required.
Source: "{#BuildSource}"; DestDir: "{app}"; Flags: ignoreversion restartreplace uninsrestartdelete
Source: "{#BuildGuiSource}"; DestDir: "{app}"; Flags: ignoreversion restartreplace uninsrestartdelete

; Shortcut / uninstall icons (distinct GUI vs CLI)
Source: "{#AppIcon}"; DestDir: "{app}"; DestName: "{#GuiIconName}"; Flags: ignoreversion
Source: "{#CliAppIcon}"; DestDir: "{app}"; DestName: "{#CliIconName}"; Flags: ignoreversion

; Optional bundled tools. If present under packaging\tools they are included;
; otherwise the app downloads them into {app}\tools on first use.
#ifexist "tools\ffmpeg.exe"
Source: "tools\ffmpeg.exe"; DestDir: "{app}\tools"; Flags: ignoreversion restartreplace uninsrestartdelete
#endif
#ifexist "tools\ffprobe.exe"
Source: "tools\ffprobe.exe"; DestDir: "{app}\tools"; Flags: ignoreversion restartreplace uninsrestartdelete
#endif
#ifexist "tools\yt-dlp.exe"
Source: "tools\yt-dlp.exe"; DestDir: "{app}\tools"; Flags: ignoreversion restartreplace uninsrestartdelete
#endif

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppGuiExeName}"; IconFilename: "{app}\{#GuiIconName}"; WorkingDir: "{app}"
Name: "{group}\{#MyAppName} (CLI)"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\{#CliIconName}"; WorkingDir: "{app}"
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppGuiExeName}"; IconFilename: "{app}\{#GuiIconName}"; WorkingDir: "{app}"; Tasks: gui
Name: "{autodesktop}\{#MyAppName} (CLI)"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\{#CliIconName}"; WorkingDir: "{app}"; Tasks: cli
