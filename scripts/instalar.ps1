<#
.SYNOPSIS
    Instala (ou desinstala) o Tranal Despertador no PC do dono.

.DESCRIPTION
    Instalação por usuário: nada precisa de administrador e nada invade
    "Program Files". O app fica em

        %LOCALAPPDATA%\Programs\Tranal Despertador

    que é o lugar padrão dos programas de usuário no Windows (o mesmo que o
    Windows usa quando alguém instala pelo Microsoft Store ou pelo winget).

    O que o instalador faz:

    1. compila a versão release (ou reaproveita a que já está compilada);
    2. fecha o app que estiver rodando (o arquivo não pode ser copiado
       enquanto ele está aberto);
    3. copia o executável para o caminho final e grava o ícone dos atalhos;
    4. cria o atalho do Menu Iniciar (obrigatório) e o da Área de trabalho
       (opcional);
    5. grava a chave de "iniciar junto com o Windows" apontando para o
       executável instalado — é ela que faz o despertador sobreviver a um
       reinício do PC;
    6. abre o app instalado.

    Os dados do dono (os despertadores, os sons da biblioteca) NÃO são tocados:
    eles ficam em %APPDATA%\Tranal Despertador e continuam onde estavam. A
    desinstalação também preserva esses dados, a menos que o dono peça para
    apagá-los com -ApagarDados.

.PARAMETER Desinstalar
    Remove o app: atalhos, chave de início automático e a pasta do programa.
    Os dados do dono só são apagados com -ApagarDados.

.PARAMETER ApagarDados
    Só faz sentido junto com -Desinstalar: apaga também os despertadores e os
    sons do dono (%APPDATA%\Tranal Despertador).

.PARAMETER SemAtalhoDesktop
    Não cria o atalho na Área de trabalho.

.PARAMETER NaoAbrir
    Não abre o app depois de instalar.

.PARAMETER NaoCompilar
    Não recompila: usa o executável release que já existe na pasta target.

.EXAMPLE
    .\scripts\instalar.ps1
    Instala, cria os dois atalhos e abre o app.

.EXAMPLE
    .\scripts\instalar.ps1 -Desinstalar
    Remove o app, preservando os despertadores do dono.

.EXAMPLE
    .\scripts\instalar.ps1 -Desinstalar -ApagarDados
    Remove tudo, inclusive os despertadores e os sons.
#>
param(
    [switch]$Desinstalar,
    [switch]$ApagarDados,
    [switch]$SemAtalhoDesktop,
    [switch]$NaoAbrir,
    [switch]$NaoCompilar
)

$ErrorActionPreference = 'Stop'

$NomeApp = 'Tranal Despertador'
$NomeExe = 'tranal-despertador.exe'
$Raiz = Split-Path -Parent $PSScriptRoot
$Destino = Join-Path $env:LOCALAPPDATA "Programs\$NomeApp"
$DestinoExe = Join-Path $Destino $NomeExe
$DestinoIcone = Join-Path $Destino 'tranal-despertador.ico'
$AtalhoInicio = Join-Path $env:APPDATA "Microsoft\Windows\Start Menu\Programs\$NomeApp.lnk"
$AtalhoDesktop = Join-Path ([Environment]::GetFolderPath('Desktop')) "$NomeApp.lnk"
$ChaveRun = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$DadosDono = Join-Path $env:APPDATA $NomeApp

function Escrever($texto, $cor = 'Gray') {
    Write-Host $texto -ForegroundColor $cor
}

function PararApp {
    $processos = Get-Process -Name 'tranal-despertador' -ErrorAction SilentlyContinue
    if (-not $processos) { return }
    Escrever "Fechando o Tranal que esta rodando..."
    foreach ($p in $processos) {
        try {
            $p.CloseMainWindow() | Out-Null
        } catch {
            # A janela pode estar minimizada; o Stop-Process resolve.
        }
    }
    Start-Sleep -Milliseconds 900
    $processos = Get-Process -Name 'tranal-despertador' -ErrorAction SilentlyContinue
    foreach ($p in $processos) {
        try {
            Stop-Process -Id $p.Id -Force -ErrorAction Stop
            Escrever "Processo encerrado (PID $($p.Id))."
        } catch {
            Escrever "Nao consegui encerrar o PID $($p.Id): $($_.Exception.Message)" 'Yellow'
        }
    }
    Start-Sleep -Milliseconds 400
}

function Compilar {
    if ($NaoCompilar) {
        Escrever "Usando o executavel que ja estava compilado (-NaoCompilar)."
        return
    }
    Escrever "Compilando a versao release..."
    $script = Join-Path $PSScriptRoot 'build.ps1'
    & $script | Out-Host
    if ($LASTEXITCODE -ne 0) {
        throw "A compilacao falhou (codigo $LASTEXITCODE)."
    }
}

function CriarAtalho {
    param([string]$CaminhoAtalho, [string]$Icone)
    $shell = New-Object -ComObject WScript.Shell
    $atalho = $shell.CreateShortcut($CaminhoAtalho)
    $atalho.TargetPath = $DestinoExe
    $atalho.WorkingDirectory = $Destino
    $atalho.Description = "Despertador com audio, para acordar no horario certo."
    if ($Icone -and (Test-Path $Icone)) {
        $atalho.IconLocation = $Icone
    }
    $atalho.Save()
    Escrever "Atalho criado: $CaminhoAtalho" 'Green'
}

function Desinstalar {
    Escrever "Desinstalando o $NomeApp..." 'Yellow'
    PararApp

    foreach ($atalho in @($AtalhoInicio, $AtalhoDesktop)) {
        if (Test-Path $atalho) {
            try {
                Remove-Item -LiteralPath $atalho -Force
                Escrever "Atalho removido: $atalho"
            } catch {
                Escrever "Nao consegui remover o atalho $atalho : $($_.Exception.Message)" 'Yellow'
            }
        }
    }

    if (Test-Path $ChaveRun) {
        $valorAtual = (Get-ItemProperty -Path $ChaveRun -Name $NomeApp -ErrorAction SilentlyContinue).$NomeApp
        if ($valorAtual) {
            try {
                Remove-ItemProperty -Path $ChaveRun -Name $NomeApp -ErrorAction Stop
                Escrever "Chave de inicio automatico removida."
            } catch {
                Escrever "Nao consegui remover a chave de inicio automatico: $($_.Exception.Message)" 'Yellow'
            }
        }
    }

    if (Test-Path $Destino) {
        try {
            Remove-Item -LiteralPath $Destino -Recurse -Force
            Escrever "Pasta do programa removida: $Destino"
        } catch {
            Escrever "Nao consegui remover a pasta $Destino : $($_.Exception.Message)" 'Yellow'
        }
    }

    if ($ApagarDados) {
        if (Test-Path $DadosDono) {
            try {
                Remove-Item -LiteralPath $DadosDono -Recurse -Force
                Escrever "Dados do dono apagados: $DadosDono"
            } catch {
                Escrever "Nao consegui apagar os dados: $($_.Exception.Message)" 'Yellow'
            }
        }
    } else {
        Escrever "Despertadores e sons preservados em: $DadosDono" 'Green'
    }

    Escrever ""
    Escrever "Pronto: o $NomeApp foi removido." 'Green'
}

function Instalar {
    Compilar

    $exeOrigem = Join-Path $Raiz "target\release\$NomeExe"
    if (-not (Test-Path $exeOrigem)) {
        throw "Nao achei o executavel compilado em $exeOrigem. Rode sem -NaoCompilar."
    }

    PararApp

    if (-not (Test-Path $Destino)) {
        New-Item -ItemType Directory -Path $Destino -Force | Out-Null
        Escrever "Pasta criada: $Destino"
    }

    Copy-Item -LiteralPath $exeOrigem -Destination $DestinoExe -Force
    Escrever "Executavel instalado: $DestinoExe" 'Green'

    # O icone dos atalhos: o app nao tem recurso de icone embutido, entao o
    # proprio app escreve o arquivo .ico a partir do mesmo desenho da bandeja.
    & $DestinoExe --exportar-icone $DestinoIcone | Out-Null
    if (-not (Test-Path $DestinoIcone)) {
        Escrever "Aviso: nao consegui gerar o icone dos atalhos." 'Yellow'
    }

    CriarAtalho -CaminhoAtalho $AtalhoInicio -Icone $DestinoIcone
    if (-not $SemAtalhoDesktop) {
        CriarAtalho -CaminhoAtalho $AtalhoDesktop -Icone $DestinoIcone
    }

    # A chave de inicio automatico aponta para o executavel INSTALADO (e nao
    # para a pasta de trabalho): e o que faz o despertador sobreviver a um
    # reinicio do PC mesmo depois de uma reinstalacao.
    if (-not (Test-Path $ChaveRun)) {
        New-Item -Path $ChaveRun -Force | Out-Null
    }
    New-ItemProperty -Path $ChaveRun -Name $NomeApp -Value "`"$DestinoExe`" --minimizado" -PropertyType String -Force | Out-Null
    Escrever "Inicio automatico gravado: $DestinoExe --minimizado" 'Green'

    if (-not $NaoAbrir) {
        Escrever "Abrindo o app instalado..."
        Start-Process -FilePath $DestinoExe | Out-Null
    }

    Escrever ""
    Escrever "Instalado: $NomeApp" 'Green'
    Escrever "  programa : $DestinoExe"
    Escrever "  menu     : $AtalhoInicio"
    if (-not $SemAtalhoDesktop) { Escrever "  area trab: $AtalhoDesktop" }
    Escrever "  dados    : $DadosDono (preservados)"
}

if ($Desinstalar) {
    Desinstalar
} else {
    Instalar
}
