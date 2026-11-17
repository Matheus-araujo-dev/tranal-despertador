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

## Como rodar

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
| `src/model.rs` | modelo de dados (JSON legível) |
| `src/hora.rs` | interpretação do horário digitado |

Decisões estruturais, com o porquê de cada escolha: [`docs/SPEC.md`](docs/SPEC.md).
Índice das decisões do dono: [`docs/decisoes/README.md`](docs/decisoes/README.md).

## Licenças

O app é do dono e o repositório é **privado**. As dependências são MIT/Apache,
com uma exceção: [`symphonia`](https://github.com/pdeljanov/Symphonia) é
**MPL-2.0** (copyleft de arquivo, não GPL/AGPL) e é usada só para **ler**
arquivos de áudio.

## Regras do repositório

Leia o [`AGENTS.md`](AGENTS.md) antes de mexer: branch sempre (nunca commitar
direto na `main`), PR para revisão, `cargo test` obrigatório e
`cargo clippy -- -D warnings` limpo.
