# Tranal Despertador

Despertador desktop para **Windows 10/11**, escrito em **Rust**. Interface dark,
feita para ser usada em um clique, e leve: o binário é nativo (sem navegador
embarcado) e o agendamento **dorme** até o próximo evento.

![Interface principal](docs/captura-interface-nova.png)

## O que ele faz

- **Criar despertadores**: uma vez, todo dia, dias úteis (seg–sex) ou nos dias
  marcados da semana.
- **Digitar o horário** por cima do relógio (`0730`, `07:30` e `730` valem
  07:30) — os botões de ±1 h / ±5 min continuam lá para quem preferir.
- **Som**: cinco presets sintetizados pelo próprio app **ou um arquivo do
  dono** — WAV, MP3, FLAC e OGG. O arquivo é **copiado** para a pasta do app,
  então o despertador continua tocando mesmo que o original seja movido.
- **Volume** de 0 a 100 com fade-in progressivo e a opção de forçar 100% na
  saída do Windows enquanto toca.
- **Onde tocar**: escolher uma ou várias saídas de áudio. Sem escolha, toca na
  **saída padrão do Windows**.
- **Como desligar**: Parar, Soneca (com minutos e limite) ou desafio (digitar
  uma palavra / resolver uma conta).
- **Despertador padrão**: nas Configurações, monte um modelo que já vem
  preenchido quando você clica em “Novo despertador”.
- **Continua tocando com a janela fechada** (a janela minimiza; o app segue
  rodando).
- **Fica na bandeja do Windows**: o ícone do despertador laranja fica na área
  de notificação (o app se promove sozinho — o Windows 11 esconde ícone
  novo). Clique esquerdo abre a janela; clique direito abre o menu
  (“Abrir o Tranal” / “Sair”).
- **Instalador**: um comando instala o app no PC, cria os atalhos e liga o
  início automático.

## Instalar

```powershell
.\scripts\instalar.ps1
```

O instalador é **por usuário** (não precisa de administrador e não invade
“Program Files”):

| | |
|---|---|
| Programa | `%LOCALAPPDATA%\Programs\Tranal Despertador` |
| Atalho | Menu Iniciar e Área de trabalho |
| Início automático | ligado, com o app já minimizado |
| Dados do dono | `%APPDATA%\Tranal Despertador` — **preservados** na reinstalação |

Para desinstalar (os despertadores e os sons continuam no lugar):

```powershell
.\scripts\instalar.ps1 -Desinstalar
```

Com `-ApagarDados`, os dados do dono também são apagados.

## Como rodar (desenvolvimento)

```powershell
cargo run                 # debug
cargo build --release     # release (target\release\tranal-despertador.exe)
cargo test                # suíte de testes
cargo clippy -- -D warnings
```

O binário é único: não tem servidor, nem banco, nem serviço. Os dados ficam em
`%APPDATA%\Tranal Despertador\dados.json` e os sons em
`%APPDATA%\Tranal Despertador\sons\`.

> O toolchain usado nesta máquina é `x86_64-pc-windows-gnu` (sem MSVC): com
> `rustup default stable-x86_64-pc-windows-gnu` e o MinGW-w64 no `PATH`, o
> `cargo build` funciona sem instalar o Visual Studio.

## Como está organizado

| Módulo | O que faz |
|---|---|
| `src/app.rs` | janela, lista, editor, painel do alarme tocando, configurações |
| `src/ui.rs` | sistema de design: cores, espaçamento, componentes (botões, chip, interruptor, seletor segmentado, slider inteiro) |
| `src/estado.rs` | a **única** cópia do estado, compartilhada entre a interface e o agendador (`Mutex` + `Condvar`) |
| `src/schedule.rs` | cálculo da próxima hora (puro, testável) |
| `src/audio.rs` | reprodução via `cpal` (WASAPI), com fade-in |
| `src/tones.rs` | presets sintetizados e leitura de arquivo (via `symphonia`) |
| `src/arquivos.rs` | seletor de arquivos **dentro do app** (não usa o explorer do Windows) |
| `src/device_volume.rs` | volume do aparelho (Core Audio) e início automático com o Windows |
| `src/tray.rs` | ícone na bandeja do Windows, promoção automática e o clique que reabre a janela |
| `src/tray_menu.rs` | o menu de contexto da bandeja (nativo do Windows, ancorado acima da barra de tarefas) |
| `src/icone.rs` | o desenho do despertador laranja (bandeja, barra de título e `.ico` dos atalhos) |
| `src/model.rs` | modelo de dados (JSON legível) |
| `src/hora.rs` | interpretação do horário digitado |

Decisões estruturais, com o porquê de cada escolha: [`docs/SPEC.md`](docs/SPEC.md).
Índice das decisões do dono: [`docs/decisoes/README.md`](docs/decisoes/README.md).

## Licenças

O app é do dono e o repositório é **privado**. As dependências foram auditadas
com `cargo tree` (251 dependências transitivas):

| Licença | Quem usa |
|---|---|
| MIT, Apache-2.0, BSD-2/3, ISC, Zlib, 0BSD, Unlicense | o grosso: `egui`, `eframe`, `winit`, `cpal`, `windows`, `serde`, `chrono`, `dirs`, `winreg`… |
| **MPL-2.0** (copyleft de arquivo) | **symphonia** e suas crates (`symphonia-core`, `symphonia-common`, `symphonia-metadata`, `symphonia-bundle-mp3/flac`, `symphonia-codec-pcm`, `symphonia-format-ogg/riff`) — usada só para **ler** WAV/MP3/FLAC/OGG |
| MIT / Apache-2.0 / OFL-1.1 / Ubuntu-font-1.0 | `epaint_default_fonts` (as fontes padrão do egui) |
| BSL-1.0 | `clipboard-win`, `error-code` |
| Unicode-3.0 | `unicode-ident`, `unicode-properties` |
| **Apache-2.0 OU GPL-2.0-only** (dupla) | `self_cell` — o app usa a opção **Apache-2.0**, a permissiva |

**Nada GPL/AGPL no processo** (regra D012 do `AGENTS.md`): nenhuma crate é
GPL-only nem AGPL, e a única dupla (`self_cell`) é consumida sob a opção
Apache-2.0.

O que a MPL-2.0 do symphonia exige, na prática: se o **arquivo-fonte do
symphonia** for modificado, essas modificações precisam ser publicadas. O
app não modifica o symphonia — só o usa como biblioteca. O código-fonte do
symphonia vem junto no crate (e no `Cargo.lock` deste repositório).

## Regras do repositório

Leia o [`AGENTS.md`](AGENTS.md) antes de mexer: branch sempre (nunca commitar
direto na `main`), PR para revisão, `cargo test` obrigatório e
`cargo clippy -- -D warnings` limpo.
