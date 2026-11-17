# Pedido ao Dedé (Hermes, perfil dede) — quality gate do Tranal Despertador

Use as skills `audit.quality-gate`, `audit.development-review` e `core.source-validation`,
além das regras de `D:\project-ai\AGENTS.md` e do `AGENTS.md` do repositório.

Você **não executa** nada: só valida e recomenda (contract do agente `dede.quality-gate`).

## O que foi entregue

Aplicativo desktop Windows em **Rust** (egui/eframe 0.37) para criar e tocar despertadores.
Repositório: `D:\tranal-despertador`. Código em `src/`:

| Arquivo | Responsabilidade |
|---|---|
| `src/main.rs` | entrada do programa, janela nativa, renderizador `glow` |
| `src/model.rs` | modelo de dados (Alarm, RepeatRule, SoundChoice, DismissPolicy, Settings) + serde |
| `src/schedule.rs` | cálculo puro do próximo disparo (repetição, atraso, "uma vez") + testes unitários |
| `src/storage.rs` | persistência em JSON em `%APPDATA%`, gravação atômica |
| `src/tones.rs` | síntese dos 5 sons embutidos + leitor de WAV (PCM 8/16/24/32 e float) |
| `src/audio.rs` | reprodução com `cpal`: vários dispositivos ao mesmo tempo, ganho atômico, fade in |
| `src/device_volume.rs` | Core Audio (windows crate): força 100% no endpoint do Windows e restaura depois; iniciar com o Windows |
| `src/ui.rs` | tema dark, cartões, chips, interruptor, botões |
| `src/app.rs` | janela principal, lista, editor, painel de alarme tocando, agendador (thread) |
| `src/cli.rs` | modos `--testar-som`, `--forcar-volume`, `--demo` |

## Critérios de aceite do pedido do owner

1. Repetição: uma vez, todo dia, dias úteis, semanal com dias marcados.
2. Som: 5 presets sintetizados + WAV do próprio dono; botão "Ouvir".
3. Volume do app (0-100%), fade-in e opção de **forçar 100% no aparelho** (saída do Windows).
4. Tocar em **vários dispositivos ao mesmo tempo**, **sem parar sozinho** até o dono desativar.
5. Desativar: Parar, Soneca (minutos e limite) e desafio (digitar palavra / conta).
6. Consumo mínimo de memória e processador.
7. Interface bonita, dark mode, fácil de usar.
8. Documentado no segundo cérebro Obsidian (`D:\project-ai\ai-context\92-Projetos\Tranal-Despertador`).

## Evidências já coletadas (para você auditar, não refazer)

| Item | Evidência |
|---|---|
| Testes unitários | `cargo test` → **20 passed; 0 failed** |
| Build release | `scripts\build.ps1` → binário único de **18,7 MB** |
| Interface | `docs\captura-interface-glow.png` e `docs\captura-editor-glow.png` (capturas reais da janela) |
| Áudio | `--testar-som` nas 3 saídas `PG27QRT1B`: **pico do endpoint 0,85 durante o toque, 0,00 depois** (medição por `IAudioMeterInformation.GetPeakValue()`) |
| Volume do aparelho | `--forcar-volume`: **40% → 100% → restaurado a 40%** (leitura direta de `GetMasterVolumeLevelScalar`) |
| Consumo | antes (wgpu/DX12): 364 MB e **99% de um core**; depois (glow/OpenGL): **~160 MB e 0,0% a 1,3% de CPU** |
| Segundo cérebro | `D:\project-ai\ai-context\92-Projetos\Tranal-Despertador\{README,00-Overview,01-Geral/README,01-Geral/Decisoes/README,01-Geral/Decisoes/DECISION-TECNOLOGIA}.md` |

## Restrições que a máquina impõe (já medidas, não precisa provar)

- O PC **não tem MSVC nem Windows SDK**: o toolchain é `x86_64-pc-windows-gnu` (rustup, estável).
- O backend WASAPI falha ao abrir **vários streams em paralelo** neste PC (`WdmSyncIoctl ... 0x490`);
  o código abre os streams **um por vez** e só então toca em paralelo.
- `pycaw`/COM exige `CoInitialize` na thread; todo o controle de volume do Windows roda na thread principal.
- O driver OpenGL da AMD (`atio6axx.dll`) ocupa 80,5 MB do espaço do processo; o app sem janela
  consome 14 MB de pico.

## Riscos que o próprio código assume

1. Sem ícone na bandeja: para interromper o alarme, o app precisa estar aberto.
2. Tolerância de 15 min para alarme perdido durante suspensão do PC.
3. O `alarms` do agendador é uma cópia: se o app for fechado com a janela, o agendador morre junto
   (por isso o "iniciar com o Windows" existe).

## O que quero do quality gate

1. **Veredito**: aprovado, aprovado com ressalvas ou reprovado — e por quê.
2. **Mapeamento critério a critério**: cada um dos 8 critérios acima está atendido? Qual não está?
3. **Riscos novos** que a revisão não encontrou, com probabilidade e impacto.
4. **Defeitos concretos** no código (arquivo e linha), ordenados por gravidade.
5. **O que falta** para a v1 poder ser considerada pronta para o dono usar todo dia.
6. Se algo do que afirmo acima **não for verificável pelo que está no repositório**, diga isso
   claramente em vez de aceitar a afirmação.

Não precisa escrever código nem alterar arquivos: o repositório é de leitura para você.
