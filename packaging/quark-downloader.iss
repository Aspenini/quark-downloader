; Quark Downloader - Inno Setup 7 script

;

; Requires Inno Setup 7.0 beta or later.

;

; Build the application first:

;   just build

;

; Open this file in Inno Setup Compiler (paths are relative to packaging/).





#define MyAppName       "Quark Downloader"

#define MyAppVersion    "1.0.0"

#define MyAppPublisher  "Quark Downloader"

#define MyAppExeName    "quark-downloader.exe"

#define MyAppGuiExeName "quark-downloader-gui.exe"



#define BuildDir        "..\target\package\quark-downloader-" + MyAppVersion + "-windows-portable"

#define BuildSource     BuildDir + "\" + MyAppExeName

#define BuildGuiSource  BuildDir + "\" + MyAppGuiExeName

#define AppIcon         "..\icons\icon.ico"

#define CliAppIcon      "..\icons\icon-cli.ico"

#define GuiIconName     "icon.ico"

#define CliIconName     "icon-cli.ico"




#ifexist BuildSource

#else

  #pragma error "Run `just build` first - Windows package staging is missing"

#endif



#ifexist BuildGuiSource

#else

  #pragma error "Run `just build` first - Windows GUI package staging is missing"

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

LicenseFile=..\LICENSE



SetupIconFile={#AppIcon}



; Per-user install location.

; Writable by the current user, so the app can download/update tools in {app}\tools.

DefaultDirName={localappdata}\Programs\{#MyAppName}



DefaultGroupName={#MyAppName}

AllowNoIcons=yes



OutputDir=..\dist

OutputBaseFilename=quark-downloader-{#MyAppVersion}-setup



Compression=lzma2/max

SolidCompression=yes

; Inno Setup 7 dynamic theme.

; Automatically follows Windows light/dark mode.

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

; Tool directory used by bundled and downloaded subprocess tools.

Name: "{app}\tools"



[Files]

; CLI executable

Source: "{#BuildSource}"; DestDir: "{app}"; Flags: ignoreversion restartreplace uninsrestartdelete



; GUI launcher (Win32 dialog)

Source: "{#BuildGuiSource}"; DestDir: "{app}"; Flags: ignoreversion restartreplace uninsrestartdelete



; Shortcut / uninstall icons (distinct GUI vs CLI)

Source: "{#AppIcon}"; DestDir: "{app}"; DestName: "{#GuiIconName}"; Flags: ignoreversion

Source: "{#CliAppIcon}"; DestDir: "{app}"; DestName: "{#CliIconName}"; Flags: ignoreversion

Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion

Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion



; Native Win32 build — no extra runtime DLLs.



; Optional bundled tools.

; If present, they get included.

; If missing, your app can download them later into {app}\tools.

#ifexist BuildDir + "\tools\ffmpeg.exe"

Source: "{#BuildDir}\tools\ffmpeg.exe"; DestDir: "{app}\tools"; Flags: ignoreversion restartreplace uninsrestartdelete

#endif



#ifexist BuildDir + "\tools\ffprobe.exe"

Source: "{#BuildDir}\tools\ffprobe.exe"; DestDir: "{app}\tools"; Flags: ignoreversion restartreplace uninsrestartdelete

#endif



#ifexist BuildDir + "\tools\yt-dlp.exe"

Source: "{#BuildDir}\tools\yt-dlp.exe"; DestDir: "{app}\tools"; Flags: ignoreversion restartreplace uninsrestartdelete

#endif



[Icons]

Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppGuiExeName}"; IconFilename: "{app}\{#GuiIconName}"; WorkingDir: "{app}"

Name: "{group}\{#MyAppName} (CLI)"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\{#CliIconName}"; WorkingDir: "{app}"

Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppGuiExeName}"; IconFilename: "{app}\{#GuiIconName}"; WorkingDir: "{app}"; Tasks: gui

Name: "{autodesktop}\{#MyAppName} (CLI)"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\{#CliIconName}"; WorkingDir: "{app}"; Tasks: cli
