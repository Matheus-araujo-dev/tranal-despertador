<#
.SYNOPSIS
    Compila (e opcionalmente testa) o Tranal Despertador.

.DESCRIPTION
    Ajusta o PATH com o toolchain Rust e com o MinGW-w64 (WinLibs) antes de
    compilar. Sem o MinGW no PATH o build falha em "dlltool could not create
    import library", porque o rustc precisa de `as`, `ar` e `dlltool` para
    gerar as bibliotecas de importação do Windows.

.EXAMPLE
    .\scripts\build.ps1
    .\scripts\build.ps1 -Testes
    .\scripts\build.ps1 -Debug
#>
param(
    [switch]$Testes,
    [switch]$Debug
)

$ErrorActionPreference = 'Stop'

# 1) cargo do rustup
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
if (Test-Path $cargoBin) {
    $env:PATH = "$cargoBin;$env:PATH"
}

# 2) ferramentas do próprio toolchain (dlltool)
$toolchain = Join-Path $env:USERPROFILE '.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin'
if (Test-Path $toolchain) {
    $env:PATH = "$toolchain;$toolchain\self-contained;$env:PATH"
}

# 3) MinGW-w64 (WinLibs): as, ar, gcc
$wingetPkgs = Join-Path $env:LOCALAPPDATA 'Microsoft\WinGet\Packages'
if (Test-Path $wingetPkgs) {
    $mingw = Get-ChildItem $wingetPkgs -Directory -Filter 'BrechtSanders.WinLibs*' -ErrorAction SilentlyContinue |
        ForEach-Object { Join-Path $_.FullName 'mingw64\bin' } |
        Where-Object { Test-Path (Join-Path $_ 'gcc.exe') } |
        Select-Object -First 1
    if ($mingw) {
        $env:PATH = "$mingw;$env:PATH"
    }
}

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "cargo não encontrado. Instale o Rust: https://rustup.rs" -ForegroundColor Red
    exit 1
}

Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    # O cargo escreve o progresso em stderr. Com ErrorActionPreference = 'Stop'
    # o PowerShell transforma isso em NativeCommandError e aborta o script,
    # então aqui o stderr é apenas repassado e o que vale e o codigo de saida.
    $ErrorActionPreference = 'Continue'

    if ($Testes) {
        cargo test
        if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
    }

    $argumentos = @('build', '--release')
    if ($Debug) { $argumentos = @('build') }
    & cargo @argumentos
    if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }

    $perfil = if ($Debug) { 'debug' } else { 'release' }
    $exe = Join-Path (Get-Location) "target\$perfil\tranal-despertador.exe"
    Write-Host ""
    Write-Host "Pronto: $exe" -ForegroundColor Green
}
finally {
    Pop-Location
}
