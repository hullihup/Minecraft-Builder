; Windows installer for Minecraft Builder (Inno Setup 6).
; Build:  ISCC.exe /DAppVersion=0.2.0 installer\setup.iss
; Needs target\release\minecraft-builder.exe (cargo build --release).

#define AppName "Minecraft Builder"
#ifndef AppVersion
  #define AppVersion "0.2.0"
#endif
#define AppExe "minecraft-builder.exe"

[Setup]
AppId={{8C1E5E1A-4C2B-4F7B-9B5E-2B9D3F6A7C11}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=Minecraft Builder
DefaultDirName={autopf}\Minecraft Builder
DefaultGroupName={#AppName}
DisableProgramGroupPage=yes
UninstallDisplayIcon={app}\{#AppExe}
UninstallDisplayName={#AppName}
OutputDir=..\target\installer
OutputBaseFilename=MinecraftBuilder-Setup-{#AppVersion}
SetupIconFile=..\assets\icon.ico
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
; Install for the current user without admin rights (can be changed in the first dialog).
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
ChangesAssociations=yes
ShowLanguageDialog=yes
CloseApplications=yes

[Languages]
Name: "de"; MessagesFile: "compiler:Languages\German.isl"; LicenseFile: "license_de.txt"
Name: "en"; MessagesFile: "compiler:Default.isl"; LicenseFile: "license_en.txt"

[CustomMessages]
de.SetupTools=Java und Gradle jetzt einrichten (empfohlen – lädt einmalig ca. 300 MB herunter)
en.SetupTools=Set up Java and Gradle now (recommended – downloads about 300 MB once)
de.SettingUpTools=Java und Gradle werden eingerichtet. Das kann einige Minuten dauern …
en.SettingUpTools=Setting up Java and Gradle. This can take a few minutes …
de.AssocMcb=.mcb-Projektdateien mit Minecraft Builder öffnen
en.AssocMcb=Open .mcb project files with Minecraft Builder
de.ProjectFile=Minecraft-Builder-Projekt
en.ProjectFile=Minecraft Builder project
de.Tools=Werkzeuge:
en.Tools=Tools:

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "assoc"; Description: "{cm:AssocMcb}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "setuptools"; Description: "{cm:SetupTools}"; GroupDescription: "{cm:Tools}"

[Files]
Source: "..\target\release\{#AppExe}"; DestDir: "{app}"; Flags: ignoreversion
Source: "license_de.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "license_en.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{group}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{group}\{cm:UninstallProgram,{#AppName}}"; Filename: "{uninstallexe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon

[Registry]
Root: HKA; Subkey: "Software\Classes\.mcb"; ValueType: string; ValueName: ""; ValueData: "MinecraftBuilder.Project"; Flags: uninsdeletevalue; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\MinecraftBuilder.Project"; ValueType: string; ValueName: ""; ValueData: "{cm:ProjectFile}"; Flags: uninsdeletekey; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\MinecraftBuilder.Project\DefaultIcon"; ValueType: string; ValueName: ""; ValueData: "{app}\{#AppExe},0"; Tasks: assoc
Root: HKA; Subkey: "Software\Classes\MinecraftBuilder.Project\shell\open\command"; ValueType: string; ValueName: ""; ValueData: """{app}\{#AppExe}"" ""%1"""; Tasks: assoc

[Run]
; Use the language picked in the installer inside the app too.
Filename: "{app}\{#AppExe}"; Parameters: "--lang {language}"; Flags: runhidden waituntilterminated
; Download Java (if missing) and prepare Gradle, so the first build is fast.
Filename: "{app}\{#AppExe}"; Parameters: "--setup-tools"; StatusMsg: "{cm:SettingUpTools}"; Flags: runhidden waituntilterminated; Tasks: setuptools
Filename: "{app}\{#AppExe}"; Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall skipifsilent
