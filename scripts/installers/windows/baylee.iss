; Inno Setup script for Baylee-Setup-<version>-<arch>.exe
; (scripts/package-installers.sh passes the defines below).
;
;   Source   the unpacked package tree (target/package/baylee-client-<version>-<target>)
;   Version  the workspace version, e.g. 0.1.0-beta.5
;   Numeric  its numeric part as four numbers, e.g. 0.1.0.0 (the file version)
;   Arch     x64 or arm64
;   Icon     baylee.ico (beside this file)
;
; A per-user install: %LOCALAPPDATA%\Programs\Baylee, no administrator
; rights and no UAC prompt. The folder stays writable for its player, which
; is what lets the launcher install updates by itself (docs/releasing.md
; §"Desktop launcher and recovery"); a Program Files install would only link
; to the release page. The uninstall entry goes to HKCU and shows in
; Settings > Apps.

#ifndef Source
  #error Source is required
#endif
#ifndef Version
  #error Version is required
#endif
#ifndef Numeric
  #error Numeric is required
#endif
#ifndef Arch
  #error Arch is required
#endif
#ifndef Icon
  #error Icon is required
#endif

[Setup]
; Never change: the uninstall key and upgrade-in-place are keyed on it.
AppId={{01A111A7-93E6-7434-8D14-F893B3405EC8}
AppName=Baylee
AppVersion={#Version}
AppVerName=Baylee {#Version}
AppPublisher=Baylee (unofficial fan project)
AppPublisherURL=https://github.com/AceVik/baylee
AppSupportURL=https://github.com/AceVik/baylee/issues
AppUpdatesURL=https://github.com/AceVik/baylee/releases
VersionInfoVersion={#Numeric}
VersionInfoProductTextVersion={#Version}
DefaultDirName={localappdata}\Programs\Baylee
DefaultGroupName=Baylee
DisableProgramGroupPage=yes
DisableDirPage=auto
PrivilegesRequired=lowest
#if Arch == "arm64"
ArchitecturesAllowed=arm64
ArchitecturesInstallIn64BitMode=arm64
#else
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
#endif
MinVersion=10.0
SetupIconFile={#Icon}
UninstallDisplayIcon={app}\baylee.ico
UninstallDisplayName=Baylee
WizardStyle=modern
Compression=lzma2/max
SolidCompression=yes
; Baylee refuses a second instance anyway; this asks the player to quit it
; before files are replaced.
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "de"; MessagesFile: "compiler:Languages\German.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[InstallDelete]
; An older version's assets go first, so a font or file the new one dropped
; does not linger. The launcher's update state lives elsewhere (%LOCALAPPDATA%\baylee).
Type: filesandordirs; Name: "{app}\assets"

[Files]
Source: "{#Source}\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "{#Icon}"; DestDir: "{app}"; DestName: "baylee.ico"; Flags: ignoreversion

[Icons]
Name: "{userprograms}\Baylee"; Filename: "{app}\baylee-client.exe"; WorkingDir: "{app}"; IconFilename: "{app}\baylee.ico"
Name: "{userdesktop}\Baylee"; Filename: "{app}\baylee-client.exe"; WorkingDir: "{app}"; IconFilename: "{app}\baylee.ico"; Tasks: desktopicon

[Run]
Filename: "{app}\baylee-client.exe"; Description: "{cm:LaunchProgram,Baylee}"; Flags: nowait postinstall skipifsilent
