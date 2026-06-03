; Quark Downloader — Inno Setup 7 script

;

; Requires Inno Setup 7.0 beta or later.

;

; Build the application first:

;   just build

;

; Open this file in Inno Setup Compiler (paths are relative to packaging/).





#define MyAppName       "Quark Downloader"

#define MyAppVersion    "0.1.0"

#define MyAppPublisher  "Quark Downloader"

#define MyAppExeName    "quark-downloader.exe"

#define MyAppGuiExeName "quark-downloader-gui.exe"



#define BuildDir        "..\build"

#define BuildSource     BuildDir + "\" + MyAppExeName

#define BuildGuiSource  BuildDir + "\" + MyAppGuiExeName

#define AppIcon         "..\icons\icon.ico"



#define GcDll           BuildDir + "\gc.dll"

#define IconvDll        BuildDir + "\iconv-2.dll"

#define CryptoDll       BuildDir + "\libcrypto-3-x64.dll"

#define SslDll          BuildDir + "\libssl-3-x64.dll"

#define PcreDll         BuildDir + "\pcre2-8.dll"

#define ZlibDll         BuildDir + "\zlib1.dll"



#ifexist BuildSource

#else

  #pragma error "Run `just build` first — expected ..\build\quark-downloader.exe"

#endif



#ifexist BuildGuiSource

#else

  #pragma error "Run `just build` first — expected ..\build\quark-downloader-gui.exe"

#endif



#ifexist AppIcon

#else

  #pragma error "Expected icons\icon.ico"

#endif



#ifexist GcDll

#else

  #pragma error "Missing required runtime DLL: gc.dll"

#endif



#ifexist IconvDll

#else

  #pragma error "Missing required runtime DLL: iconv-2.dll"

#endif



#ifexist CryptoDll

#else

  #pragma error "Missing required runtime DLL: libcrypto-3-x64.dll"

#endif



#ifexist SslDll

#else

  #pragma error "Missing required runtime DLL: libssl-3-x64.dll"

#endif



#ifexist PcreDll

#else

  #pragma error "Missing required runtime DLL: pcre2-8.dll"

#endif



#ifexist ZlibDll

#else

  #pragma error "Missing required runtime DLL: zlib1.dll"

#endif



[Setup]

AppId={{8F3C2A1B-4D5E-6F70-8A9B-0C1D2E3F4A5B}}



AppName={#MyAppName}

AppVersion={#MyAppVersion}

AppPublisher={#MyAppPublisher}



SetupIconFile={#AppIcon}



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



ArchitecturesAllowed=x64compatible

ArchitecturesInstallIn64BitMode=x64compatible



; Per-user install only. No admin prompt.

PrivilegesRequired=lowest



DisableProgramGroupPage=no



UninstallDisplayIcon={app}\{#MyAppGuiExeName}

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



[Dirs]

; Tool directory used by bundled and downloaded subprocess tools.

Name: "{app}\tools"



[Files]

; CLI executable

Source: "{#BuildSource}"; DestDir: "{app}"; Flags: ignoreversion restartreplace uninsrestartdelete



; GUI launcher (Win32 dialog)

Source: "{#BuildGuiSource}"; DestDir: "{app}"; Flags: ignoreversion restartreplace uninsrestartdelete



; Required runtime DLLs

Source: "{#GcDll}"; DestDir: "{app}"; Flags: ignoreversion restartreplace uninsrestartdelete

Source: "{#IconvDll}"; DestDir: "{app}"; Flags: ignoreversion restartreplace uninsrestartdelete

Source: "{#CryptoDll}"; DestDir: "{app}"; Flags: ignoreversion restartreplace uninsrestartdelete

Source: "{#SslDll}"; DestDir: "{app}"; Flags: ignoreversion restartreplace uninsrestartdelete

Source: "{#PcreDll}"; DestDir: "{app}"; Flags: ignoreversion restartreplace uninsrestartdelete

Source: "{#ZlibDll}"; DestDir: "{app}"; Flags: ignoreversion restartreplace uninsrestartdelete



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



[Icons]

Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppGuiExeName}"; IconFilename: "{app}\{#MyAppGuiExeName}"; WorkingDir: "{app}"

Name: "{group}\{#MyAppName} (CLI)"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\{#MyAppExeName}"; WorkingDir: "{app}"

Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppGuiExeName}"; IconFilename: "{app}\{#MyAppGuiExeName}"; WorkingDir: "{app}"; Tasks: desktopicon

