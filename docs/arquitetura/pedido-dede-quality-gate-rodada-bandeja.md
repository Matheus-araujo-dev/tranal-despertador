# Quality gate — Tranal Despertador: ícone na bandeja do Windows + instalador

Repositório: `D:\tranal-despertador` (branch `feat/interface-nova`, base `main`)
Pedido original do owner (PT-BR): *"o projeto então já tem um instalador que eu rodo, ele cria as dependencias e salva no local definido, se o windows reiniciar ele não perde os despertadores programados e fica na bandeja do windows ne"*
Resposta do owner ao questionário: **"Instalador + ícone na bandeja (recomendado)"**.

## O que está sendo entregue nesta rodada

1. **Ícone na bandeja do Windows** com menu próprio ("Abrir o Tranal" / "Sair").
2. **Instalador** (`scripts\instalar.ps1`, instalação por usuário) que compila, copia para
   `%LOCALAPPDATA%\Programs\Tranal Despertador`, gera o ícone dos atalhos, cria atalho do
   Menu Iniciar e da Área de trabalho, grava a chave de início automático do Windows e abre o app.
3. **Início automático** apontando para o executável **instalado**, para o despertador sobreviver
   a reinício do Windows.

## Critérios de aceite a fiscalizar

| # | Critério |
|---|---|
| C1 | `cargo test` passa (72 testes) |
| C2 | `cargo clippy --all-targets -- -D warnings` sem aviso |
| C3 | `cargo build --release` compila |
| C4 | O ícone da bandeja é criado na thread principal e o `TrayIcon` fica guardado no app (senão o Windows o remove) |
| C5 | O ícone do Windows 11 aparece **na faixa principal** (não só em "ícones ocultos"), e isso se sustenta sem ação manual do dono |
| C6 | O menu da bandeja **ancora acima da barra de tarefas** (o item "Sair" não fica escondido atrás dela) |
| C7 | "Abrir o Tranal" restaura a janela; "Sair" encerra o app de verdade (roda o `on_exit`) |
| C8 | Fechar a janela **minimiza** (não encerra) — o app continua tocando o alarme |
| C9 | O instalador roda sem administrador, grava em caminho estável, cria atalhos com ícone e registra o início automático |
| C10 | O instalador **não apaga os dados do dono** (`%APPDATA%\Tranal Despertador`); só `-ApagarDados` apaga |
| C11 | Nada de código GPL/AGPL no mesmo processo (regra D012); `tray-icon` e `windows` são MIT/Apache |
| C12 | Branch, nunca commit direto na `main`; PR para revisão |

## Evidências já medidas nesta máquina (Windows 11 build 26200)

- Ícone do app: blob laranja 9x9 (~52 px) na faixa de notificação, **não** no flyout de ocultos.
- `HKCU\Control Panel\NotifyIconSettings\<hash>` com `ExecutablePath`, `InitialTooltip`, `UID`,
  `IconGuid`, `IsPromoted` (REG_DWORD). `IsPromoted = 1` = "sempre mostrar".
- O Windows **reescreve** a chave do ícone logo depois de criá-lo e apaga o `IsPromoted` que o app
  acabou de gravar (race medida). Por isso o app grava e **confere**, repetindo até ler 1 de volta.
  Log: `ok: icone promovido para a faixa de notificacao`.
- Com `IconGuid` fixo, o Windows 11 passa a guardar o ícone pela identidade e **os cliques deixam de
  chegar no app** (o shell chama a janela velha). Foi medido e a abordagem foi **abandonada**;
  a promoção é feita pelo caminho **exato** do executável.
- Menu do `tray-icon` 0.26.1: `show_tray_menu` ancora em `TPM_BOTTOMALIGN|TPM_LEFTALIGN` na
  **posição do cursor** (dentro da barra de tarefas) → popup de 59 px com base em y=1416 e a barra
  começando em 1393 → o último item ficava atrás da barra. Solução: menu nativo próprio
  (`CreatePopupMenu` + `TrackPopupMenuEx`), ancorado em `rcWork.bottom` do monitor, exibido **na
  thread principal** via `PostMessage` para uma janela dona invisível (`WM_APP+42`).
  Medido depois: popup `(2193,1333)-(2362,1392)` → base 1392, **acima** da barra.
- Instalador executado de verdade no PC do owner: app rodando de
  `%LOCALAPPDATA%\Programs\Tranal Despertador\tranal-despertador.exe` (pid 26848, ~157 MB),
  atalhos apontando para o exe instalado, chave Run =
  `"C:\Users\Matheus\AppData\Local\Programs\Tranal Despertador\tranal-despertador.exe" --minimizado`,
  `IsPromoted=1` na chave do caminho novo, e **dados do dono intactos**
  (alarme `al8a262d2c` 07:00, `last_rung` null, `iniciar_com_o_windows=true`).

## Comandos de verificação

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
$toolchain = "$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin"
$env:PATH = "$toolchain;$toolchain\self-contained;" + $env:PATH
$mingw = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Directory -Filter 'BrechtSanders.WinLibs*' | ForEach-Object { Join-Path $_.FullName 'mingw64\bin' } | Where-Object { Test-Path (Join-Path $_ 'gcc.exe') } | Select-Object -First 1
$env:PATH = "$mingw;$env:PATH"
Set-Location "D:\tranal-despertador"
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

## Arquivos da rodada

- `src/tray.rs` (novo) — ícone, promoção no registro, eventos de clique.
- `src/tray_menu.rs` (novo) — menu nativo Win32 + janela dona invisível.
- `src/icone.rs` (novo) — desenho do despertador (32/64 px) + exportação de `.ico`.
- `src/app.rs` — cria a bandeja no 1º quadro; guarda o `TrayIcon`; "Sair" via `ViewportCommand::Close`.
- `src/main.rs` — `mod icone; mod tray_menu;` + `with_icon(...)`.
- `src/cli.rs` — `--exportar-icone <arquivo.ico>`.
- `scripts/instalar.ps1` (novo) — instalador/desinstalador por usuário.
- `scripts/e2e.py` — refactor: `janelas_do_pid(pid)` + busca por título "Tranal".
- `Cargo.toml` — `tray-icon = "0.26.1"`; `windows` 0.62.2 com `Win32_Graphics_Gdi` e `Win32_System_LibraryLoader`.
- `README.md`, `docs/SPEC.md` (§12 nova) — documentação.

## Riscos que a rodada mexeu

- **R5** (alarme só existe se o app estiver aberto) → **Resolvido**: início automático minimizado + ícone na bandeja.
- **R6** (ícone novo nasce oculto no Windows 11) → **Mitigado**: auto-promoção conferida pelo próprio app.

## Limite conhecido a fiscalizar

A sessão de desktop do owner ficou **não interativa** no fim (o `SetCursorPos` parou de mover o mouse e
a resolução alternou entre 5120x1441 e 4160x1441). Por isso a re-verificação *por clique* do menu
**no app instalado** não pôde ser refeita ao fim. O comportamento foi provado com o **mesmo código**
rodando de `D:\tranal-despertador\target\release` (mesmo binário, outro caminho). No app instalado
foram verificados: ícone visível na faixa, `IsPromoted=1` na chave do caminho novo, chave Run e
atalhos corretos, dados intactos.

## Como responder

Siga o `structured_output` do seu contrato:
`status` (PASS | PASS_WITH_WARNINGS | REWORK_REQUIRED | BLOCKED), `criteria` (lista de
{description, met}), `issues` (lista de {severity, description, recommendation}),
`required_actions` (lista), `warnings` (lista).
Fique no mesmo formato da rodada anterior (que saiu em PASS_WITH_WARNING, 8/8 critérios, 3 issues
corrigidas depois).
