; ---------------------------------------------------------------------------
; Tranal Despertador - instalador (Inno Setup 6)
;
; Padrao de todos os apps Tranal: DEC-APPS-DISTRIBUICAO-001
; (D:\project-ai\docs\decisions\DEC-APPS-DISTRIBUICAO-001.md).
;
; Compilacao (o CI faz isto em .github\workflows\publicar.yml):
;
;   target\release\tranal-despertador.exe --exportar-icone target\release\tranal-despertador.ico
;   iscc /DMyAppVersion=0.4.1 installer\tranal-despertador.iss
;
; Defines (todos tem padrao, menos a versao):
;   /DMyAppVersion=X.Y.Z   versao da release (obrigatoria)
;   /DSourceExe=...        o executavel compilado
;   /DIconFile=...         o .ico gerado pelo proprio app (--exportar-icone)
;   /DOutputDir=...        onde gravar o Tranal-Despertador-Setup.exe
;
; Decisoes:
;  - Instalacao POR USUARIO (PrivilegesRequired=lowest), como o instalar.ps1
;    antigo: o despertador e um app do dono, roda no perfil dele, grava o
;    inicio automatico em HKCU e se atualiza sozinho de hora em hora. Com
;    "lowest", {autopf} e a pasta de programas do usuario
;    (%LOCALAPPDATA%\Programs) e a atualizacao silenciosa nao abre o UAC.
;    A pasta continua sendo {autopf}\Tranal Group\Tranal Despertador.
;  - Os dados do dono (despertadores, sons) ficam em %APPDATA%\Tranal
;    Despertador. Nem a instalacao nem a desinstalacao tocam neles.
;  - Migracao do instalar.ps1: a pasta antiga
;    (%LOCALAPPDATA%\Programs\Tranal Despertador) e apagada; os atalhos tem o
;    mesmo caminho e sao sobrescritos; o valor "Tranal Despertador" da chave
;    Run e reescrito para o exe novo. O app, ao abrir, converte esse valor no
;    ajuste "iniciar com o Windows" (src\device_volume.rs) e passa a gravar
;    o dele ("TranalDespertador"). Assim nada fica orfao.
;  - Inicio automatico: o instalar.ps1 sempre ligava. Numa instalacao nova a
;    tarefa "Iniciar com o Windows" vem marcada e grava o mesmo valor legado
;    (o pedido que o app converte). Numa atualizacao ela nao e regravada: o
;    que vale e o ajuste que o dono escolheu na tela.
;  - O app reaberto no fim da instalacao silenciosa: o [Run] nao tem
;    skipifsilent. /MINIMIZADO=1 (o atualizador passa quando a janela estava
;    escondida) reabre o app na bandeja.
;  - Compressao zip, e nao lzma2: no Tranal Uninstaller o McAfee bloqueava o
;    setup com carga LZMA (falso positivo). O exe e pequeno; a diferenca de
;    tamanho e irrelevante.
;
; Este arquivo e gravado em UTF-8 COM BOM: o Inno Setup 6 le script sem BOM
; como ANSI e os acentos sairiam errados.
; ---------------------------------------------------------------------------

#ifndef MyAppVersion
  #error "MyAppVersion nao definido: iscc /DMyAppVersion=X.Y.Z"
#endif
#ifndef SourceExe
  #define SourceExe AddBackslash(SourcePath) + "..\target\release\tranal-despertador.exe"
#endif
#ifndef IconFile
  #define IconFile AddBackslash(SourcePath) + "..\target\release\tranal-despertador.ico"
#endif
#ifndef OutputDir
  #define OutputDir AddBackslash(SourcePath) + "..\dist"
#endif

#define AppName "Tranal Despertador"
#define AppPublisher "Tranal"
#define AppGroup "Tranal Group"
#define AppExeName "tranal-despertador.exe"
#define AppIconName "tranal-despertador.ico"
#define AppHomeUrl "https://github.com/Matheus-araujo-dev/tranal-despertador"
; AppId fixo: nunca muda entre versoes (e o que faz a nova substituir a velha).
#define AppIdGuid "{259DBB0D-2F4A-4D3F-9B7A-CFEF530376E6}"

[Setup]
AppId={{#AppIdGuid}
AppName={#AppName}
AppVersion={#MyAppVersion}
AppVerName={#AppName} {#MyAppVersion}
AppPublisher={#AppPublisher}
AppPublisherURL={#AppHomeUrl}
AppSupportURL={#AppHomeUrl}/issues
AppUpdatesURL={#AppHomeUrl}/releases/latest
DefaultDirName={autopf}\{#AppGroup}\{#AppName}
DisableProgramGroupPage=yes
DisableDirPage=auto
OutputBaseFilename=Tranal-Despertador-Setup
OutputDir={#OutputDir}
Compression=zip
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
UninstallDisplayName={#AppName}
UninstallDisplayIcon={app}\{#AppIconName}
SetupIconFile={#IconFile}
CloseApplications=force
RestartApplications=no
SetupLogging=yes
VersionInfoVersion={#MyAppVersion}
VersionInfoProductVersion={#MyAppVersion}
VersionInfoCompany={#AppPublisher}
VersionInfoDescription={#AppName} - instalador

[Languages]
Name: "brazilianportuguese"; MessagesFile: "compiler:Languages\BrazilianPortuguese.isl"
Name: "english"; MessagesFile: "compiler:Default.isl"

[CustomMessages]
brazilianportuguese.IniciarComWindows=Iniciar com o Windows (minimizado na bandeja)
english.IniciarComWindows=Start with Windows (minimized to the tray)
brazilianportuguese.GrupoSistema=Sistema:
english.GrupoSistema=System:

[Tasks]
; O instalar.ps1 criava os dois atalhos e ligava o inicio automatico.
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "iniciarcomwindows"; Description: "{cm:IniciarComWindows}"; GroupDescription: "{cm:GrupoSistema}"

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "{#AppExeName}"; Flags: ignoreversion
Source: "{#IconFile}"; DestDir: "{app}"; DestName: "{#AppIconName}"; Flags: ignoreversion

[Icons]
; Mesmo caminho dos atalhos do instalar.ps1: a migracao os sobrescreve.
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\{#AppIconName}"; Comment: "Despertador com audio, para acordar no horario certo."
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExeName}"; WorkingDir: "{app}"; IconFilename: "{app}\{#AppIconName}"; Comment: "Despertador com audio, para acordar no horario certo."; Tasks: desktopicon

[Run]
; Sem skipifsilent: na atualizacao silenciosa o app e reaberto sozinho.
Filename: "{app}\{#AppExeName}"; Parameters: "{code:ParametrosAbrir}"; WorkingDir: "{app}"; \
    Description: "{cm:LaunchProgram,{#AppName}}"; Flags: nowait postinstall

[Code]
const
  ChaveRun = 'Software\Microsoft\Windows\CurrentVersion\Run';
  ValorApp = 'TranalDespertador';
  ValorLegado = 'Tranal Despertador';
  ChaveDesinstalar = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{#AppIdGuid}_is1';
  SufixoPasta = '\{#AppGroup}\{#AppName}';

var
  EraAtualizacao: Boolean;

{ O app nao fecha ao pedido da janela (minimiza para a bandeja), entao ele e
  encerrado a forca, como o instalar.ps1 fazia. Os dados ja estao gravados:
  o app salva a cada mudanca. }
procedure EncerrarApp;
var
  Codigo: Integer;
begin
  { Sem /T: no auto-update o proprio Setup e filho do app e morreria junto. }
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM {#AppExeName}', '',
    SW_HIDE, ewWaitUntilTerminated, Codigo);
  Sleep(500);
end;

function ParametrosAbrir(Param: String): String;
begin
  if ExpandConstant('{param:MINIMIZADO|0}') = '1' then
    Result := '--minimizado'
  else
    Result := '';
end;

function InitializeSetup(): Boolean;
begin
  EraAtualizacao := RegKeyExists(HKCU, ChaveDesinstalar) or RegKeyExists(HKLM, ChaveDesinstalar);
  Result := True;
end;

{ Escolher outra raiz instala em <raiz>\Tranal Group\Tranal Despertador. }
function NextButtonClick(CurPageID: Integer): Boolean;
var
  Pasta: String;
begin
  Result := True;
  if CurPageID <> wpSelectDir then
    Exit;
  Pasta := RemoveBackslashUnlessRoot(WizardForm.DirEdit.Text);
  if CompareText(Copy(Pasta, Length(Pasta) - Length(SufixoPasta) + 1, Length(SufixoPasta)), SufixoPasta) = 0 then
    Exit;
  { O "Procurar" do Inno acrescenta o nome do app no fim: tira antes. }
  if CompareText(ExtractFileName(Pasta), '{#AppName}') = 0 then
    Pasta := ExtractFileDir(Pasta);
  WizardForm.DirEdit.Text := AddBackslash(Pasta) + '{#AppGroup}\{#AppName}';
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  // O exe da instalacao antiga nao esta na pasta nova: o CloseApplications nao o ve.
  EncerrarApp;
  Result := '';
end;

procedure MigrarInstalacaoAntiga;
var
  Legado: String;
begin
  Legado := ExpandConstant('{localappdata}\Programs\{#AppName}');
  if DirExists(Legado) and (CompareText(Legado, ExpandConstant('{app}')) <> 0) then
  begin
    if DelTree(Legado, True, True, True) then
      Log('Instalacao antiga removida: ' + Legado)
    else
      Log('Nao consegui remover a instalacao antiga: ' + Legado);
  end;
end;

procedure GravarInicioAutomatico;
var
  Comando: String;
begin
  Comando := '"' + ExpandConstant('{app}\{#AppExeName}') + '" --minimizado';

  { O valor do proprio app, se ja existe, passa a apontar para o exe novo:
    o PC reiniciado antes de o app abrir de novo continua acordando. }
  if RegValueExists(HKCU, ChaveRun, ValorApp) then
    RegWriteStringValue(HKCU, ChaveRun, ValorApp, Comando);

  { O valor legado e o pedido de "iniciar com o Windows" que o app converte
    no ajuste dele. Vem do instalar.ps1 (migracao) ou da tarefa marcada numa
    instalacao nova. Numa atualizacao, vale o ajuste do dono. }
  if RegValueExists(HKCU, ChaveRun, ValorLegado) or
     ((not EraAtualizacao) and WizardIsTaskSelected('iniciarcomwindows')) then
    RegWriteStringValue(HKCU, ChaveRun, ValorLegado, Comando);
end;

procedure CurStepChanged(CurStep: TSetupStep);
begin
  if CurStep = ssPostInstall then
  begin
    MigrarInstalacaoAntiga;
    GravarInicioAutomatico;
  end;
end;

function InitializeUninstall(): Boolean;
begin
  EncerrarApp;
  Result := True;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if CurUninstallStep = usPostUninstall then
  begin
    { Nada de "iniciar com o Windows" apontando para um exe apagado. }
    RegDeleteValue(HKCU, ChaveRun, ValorApp);
    RegDeleteValue(HKCU, ChaveRun, ValorLegado);
    { O Setup baixado pelo atualizador (nao e dado do dono). }
    DelTree(ExpandConstant('{localappdata}\{#AppName}\update'), True, True, True);
    RemoveDir(ExpandConstant('{localappdata}\{#AppName}'));
    { A pasta "Tranal Group" so sai se ficou vazia (outros apps Tranal). }
    RemoveDir(ExtractFileDir(ExpandConstant('{app}')));
    { %APPDATA%\Tranal Despertador (despertadores e sons) fica onde esta. }
  end;
end;
