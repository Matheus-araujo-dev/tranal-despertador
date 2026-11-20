<#
.SYNOPSIS
    Gera o instalador do Tranal Despertador nesta maquina e o executa.

.DESCRIPTION
    A instalacao oficial e o Tranal-Despertador-Setup.exe (Inno Setup) da
    release mais nova no GitHub: o CI publica um a cada PR mesclado e o app
    instalado se atualiza sozinho (DEC-APPS-DISTRIBUICAO-001).

    Este script e so para testar o instalador a partir do codigo local:

    1. compila a versao release (scripts\build.ps1);
    2. grava o icone dos atalhos (o app o desenha: --exportar-icone);
    3. compila installer\tranal-despertador.iss com a versao do Cargo.toml;
    4. roda o Setup gerado (em dist\).

    O Setup instala em {autopf}\Tranal Group\Tranal Despertador (instalacao
    por usuario: %LOCALAPPDATA%\Programs\Tranal Group\Tranal Despertador),
    migra a instalacao antiga deste script e preserva os despertadores em
    %APPDATA%\Tranal Despertador.

    Cuidado: instalar uma versao local IGUAL ou MAIOR que a ultima release
    faz o app ignorar as atualizacoes ate sair uma versao maior.

.PARAMETER Desinstalar
    Roda o desinstalador do Inno (preserva os dados do dono).

.PARAMETER Silencioso
    Roda o Setup com /VERYSILENT, como o atualizador faz.

.PARAMETER NaoCompilar
    Usa o executavel release que ja existe em target\.
#>
param(
    [switch]$Desinstalar,
    [switch]$Silencioso,
    [switch]$NaoCompilar
)

$ErrorActionPreference = 'Stop'
$Raiz = Split-Path -Parent $PSScriptRoot

function Achar-Iscc {
    foreach ($c in @(
            (Join-Path $env:LOCALAPPDATA 'Programs\Inno Setup 6\ISCC.exe'),
            (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'),
            (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe'))) {
        if (Test-Path $c) { return $c }
    }
    throw "Inno Setup 6 nao encontrado. Instale: winget install JRSoftware.InnoSetup"
}

if ($Desinstalar) {
    $chave = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{259DBB0D-2F4A-4D3F-9B7A-CFEF530376E6}_is1'
    $item = Get-ItemProperty "HKCU:\$chave" -ErrorAction SilentlyContinue
    if (-not $item) { $item = Get-ItemProperty "HKLM:\$chave" -ErrorAction SilentlyContinue }
    if (-not $item) { throw "O Tranal Despertador nao esta instalado pelo Setup." }
    $desinstalador = $item.UninstallString.Trim('"')
    Start-Process $desinstalador -Wait
    return
}

if (-not $NaoCompilar) {
    & (Join-Path $PSScriptRoot 'build.ps1')
    if ($LASTEXITCODE -ne 0) { throw "A compilacao falhou (codigo $LASTEXITCODE)." }
}

$exe = Join-Path $Raiz 'target\release\tranal-despertador.exe'
$ico = Join-Path $Raiz 'target\release\tranal-despertador.ico'
if (-not (Test-Path $exe)) { throw "Nao achei $exe. Rode sem -NaoCompilar." }
Start-Process $exe -ArgumentList '--exportar-icone', "`"$ico`"" -Wait -NoNewWindow

$linha = Select-String -Path (Join-Path $Raiz 'Cargo.toml') -Pattern '^version\s*=\s*"([^"]+)"' | Select-Object -First 1
$versao = $linha.Matches[0].Groups[1].Value

& (Achar-Iscc) "/DMyAppVersion=$versao" (Join-Path $Raiz 'installer\tranal-despertador.iss')
if ($LASTEXITCODE -ne 0) { throw "iscc falhou ($LASTEXITCODE)." }

$setup = Join-Path $Raiz 'dist\Tranal-Despertador-Setup.exe'
$argumentos = if ($Silencioso) { @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/CLOSEAPPLICATIONS') } else { @() }
Write-Host "Rodando $setup (versao $versao)..." -ForegroundColor Green
Start-Process $setup -ArgumentList $argumentos -Wait
