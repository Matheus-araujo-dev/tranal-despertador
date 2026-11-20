<#
.SYNOPSIS
    Captura a janela do Tranal Despertador em PNG (evidencia visual).

.DESCRIPTION
    Usa PrintWindow com PW_RENDERFULLCONTENT para desenhar a janela do
    processo direto no bitmap, sem precisar trazer a janela para o primeiro
    plano (funciona mesmo com ela atras de outra janela). O conteudo da tela
    em volta da janela nunca entra na imagem.

.EXAMPLE
    .\scripts\capturar.ps1 -Nome "interface"
    .\scripts\capturar.ps1 -Nome "editor" -AtalhoDespertador
#>
param(
    [Parameter(Mandatory = $true)][string]$Nome,
    [string]$Processo = 'tranal-despertador',
    [switch]$AtalhoDespertador,
    # Clique dentro da janela (coordenadas relativas ao canto da janela)
    # antes de capturar: e o jeito de abrir o editor e o seletor de arquivos.
    [int]$ClicarX = -1,
    [int]$ClicarY = -1
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

Add-Type @"
using System;
using System.Runtime.InteropServices;
public class CapturaJanela {
  [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
  [DllImport("user32.dll")] public static extern bool PrintWindow(IntPtr h, IntPtr hdc, uint flags);
  [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
  [DllImport("user32.dll")] public static extern bool ShowWindow(IntPtr h, int n);
  [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
  [DllImport("user32.dll")] public static extern void mouse_event(uint f, uint dx, uint dy, uint d, IntPtr e);
  [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
  [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
  public static readonly IntPtr TOPMOST = new IntPtr(-1);
  public static readonly IntPtr NOTOPMOST = new IntPtr(-2);
  public const uint SWP_NOMOVE = 0x0002, SWP_NOSIZE = 0x0001, SWP_NOACTIVATE = 0x0010;
  public static void AoTopo(IntPtr h) { SetWindowPos(h, TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE); }
  public static void DoTopo(IntPtr h) { SetWindowPos(h, NOTOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE); }
  public const uint LEFTDOWN = 0x0002, LEFTUP = 0x0004;
  public static void Clicar(int x, int y) {
    SetCursorPos(x, y);
    System.Threading.Thread.Sleep(120);
    mouse_event(LEFTDOWN, 0, 0, 0, IntPtr.Zero);
    System.Threading.Thread.Sleep(60);
    mouse_event(LEFTUP, 0, 0, 0, IntPtr.Zero);
  }
}
"@

$proc = Get-Process -Name $Processo -ErrorAction Stop |
    Where-Object { $_.MainWindowHandle -ne 0 } |
    Sort-Object StartTime -Descending | Select-Object -First 1
if (-not $proc) { throw "janela do processo '$Processo' nao encontrada" }

[CapturaJanela]::ShowWindow($proc.MainWindowHandle, 9) | Out-Null   # SW_RESTORE
Start-Sleep -Milliseconds 400

if ($AtalhoDespertador) {
    $r0 = New-Object CapturaJanela+RECT
    [CapturaJanela]::GetWindowRect($proc.MainWindowHandle, [ref]$r0) | Out-Null
    # A janela pode estar atras de outra; sem ir ao topo o clique sintetico
    # cairia na janela da frente.
    [CapturaJanela]::AoTopo($proc.MainWindowHandle)
    [CapturaJanela]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
    Start-Sleep -Milliseconds 700
    [CapturaJanela]::Clicar(($r0.Left + 88), ($r0.Top + 478))
    Start-Sleep -Milliseconds 1000
    [CapturaJanela]::DoTopo($proc.MainWindowHandle)
    Start-Sleep -Milliseconds 300
}

$r = New-Object CapturaJanela+RECT
[CapturaJanela]::GetWindowRect($proc.MainWindowHandle, [ref]$r) | Out-Null

if ($ClicarX -ge 0 -and $ClicarY -ge 0) {
    # A janela precisa estar na frente: sem isso o clique sintetico cairia
    # na janela de cima.
    [CapturaJanela]::AoTopo($proc.MainWindowHandle)
    [CapturaJanela]::SetForegroundWindow($proc.MainWindowHandle) | Out-Null
    Start-Sleep -Milliseconds 500
    [CapturaJanela]::Clicar(($r.Left + $ClicarX), ($r.Top + $ClicarY))
    Start-Sleep -Milliseconds 900
    [CapturaJanela]::DoTopo($proc.MainWindowHandle)
    Start-Sleep -Milliseconds 300
    [CapturaJanela]::GetWindowRect($proc.MainWindowHandle, [ref]$r) | Out-Null
}

$largura = $r.Right - $r.Left
$altura = $r.Bottom - $r.Top
if ($largura -le 0 -or $altura -le 0) { throw "janela sem tamanho valido" }

$bmp = New-Object System.Drawing.Bitmap $largura, $altura
$gfx = [System.Drawing.Graphics]::FromImage($bmp)
$hdc = $gfx.GetHdc()
$ok = [CapturaJanela]::PrintWindow($proc.MainWindowHandle, $hdc, 2)   # PW_RENDERFULLCONTENT
$gfx.ReleaseHdc($hdc)
$gfx.Dispose()

if (-not $ok) {
    $bmp.Dispose()
    throw "PrintWindow falhou"
}

$destino = Join-Path (Split-Path -Parent $PSScriptRoot) "docs\captura-$Nome.png"
$bmp.Save($destino, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Host "Captura salva: $destino" -ForegroundColor Green
