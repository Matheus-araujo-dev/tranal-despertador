# Pedido ao Tutu (Hermes, perfil tutu) — revisão de código do Tranal Despertador

Use a skill `development.code-review` e as regras de `D:\project-ai\AGENTS.md` e do `AGENTS.md` deste repositório.

## O que foi feito

Aplicativo desktop Windows em **Rust** (egui/eframe 0.37) para criar e tocar despertadores.
Repositório: `D:\tranal-despertador`. Código em `src/`:

| Arquivo | Responsabilidade |
|---|---|
| `src/main.rs` | entrada do programa, janela native, opções do viewport |
| `src/model.rs` | modelo de dados (Alarm, RepeatRule, SoundChoice, DismissPolicy, Settings) + serde |
| `src/schedule.rs` | cálculo puro do próximo disparo (repetição, atraso, "uma vez") + testes unitários |
| `src/storage.rs` | persistência em JSON em `%APPDATA%`, gravação atômica |
| `src/tones.rs` | síntese dos 5 sons embutidos + leitor de WAV (PCM 8/16/24/32 e float) |
| `src/audio.rs` | reprodução com `cpal`: vários dispositivos ao mesmo tempo, ganho atômico, fade in |
| `src/device_volume.rs` | Core Audio (windows crate): força 100% no endpoint do Windows e restaura depois; iniciar com o Windows |
| `src/ui.rs` | tema dark, cartões, chips, interruptor, botões |
| `src/app.rs` | janela principal, lista, editor, painel de alarme tocando, agendador (thread) |

## Requisitos que o app precisa cumprir

1. Repetição: uma vez, diário, dias úteis, semanal com dias marcados.
2. Som: 5 presets sintetizados + WAV do dono; botão de teste.
3. Volume do app (0-100), fade in progressivo e opção de forçar 100% no endpoint do Windows.
4. Tocar em **vários dispositivos ao mesmo tempo**, em paralelo, **sem parar sozinho** até o dono desativar.
5. Desativar: Parar, Soneca (minutos e número máximo), desafio opcional (digitar/contar).
6. Consumo baixo: agendador em thread que dorme até o próximo evento (sem polling); UI repinta a cada 500 ms.
7. Persistência em JSON; iniciar junto com o Windows.

## Restrições que a máquina impõe (já medidas, não precisa provar)

- O PC **não tem MSVC nem Windows SDK**: o toolchain é `x86_64-pc-windows-gnu` (rustup, toolchain estável).
- O backend WASAPI falha ao abrir **vários streams em paralelo** neste PC (`WdmSyncIoctl ... 0x490`);
  a solução no código é abrir os streams **um por vez** e tocar em paralelo depois.
- `pycaw`/COM exige `CoInitialize` na thread; por isso todo o controle de volume do Windows fica
  na thread principal.

## Pedido

Responda **no chat** (não crie arquivos, não edite código) com:

1. **Defeitos concretos** por arquivo e linha (ou função), em ordem de gravidade: correção de
   crash, de lógica de agendamento, de concorrência, delossa de dados (JSON), de áudio.
2. **Riscos de segurança** ou de privacidade (arquivo de dados, inicialização automática, WAV do usuário).
3. **O que está faltando** para o app ser realmente confiável como despertador (por exemplo:
   o que acontece se o PC hibernar, se o relógio do Windows mudar, se dois alarmes caírem na mesma hora).
4. **Veredito**: pode ser entregue ao dono como está? O que é bloqueante.

Seja direto, cite o código (arquivo:função) e não repita o que está correto.
