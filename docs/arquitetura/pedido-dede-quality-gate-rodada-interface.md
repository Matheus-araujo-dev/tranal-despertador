# Pedido ao Dedé (Hermes, perfil dede) — quality gate da rodada do redesenho

Use as skills `audit.quality-gate`, `audit.development-review` e `core.source-validation`,
além das regras de `D:\project-ai\AGENTS.md` e do `AGENTS.md` do repositório.

Você **não executa** nada: só valida e recomenda (contract do agente `dede.quality-gate`).

Repositório: `D:\tranal-despertador`. Branch da rodada: `feat/interface-nova`
(PR: https://github.com/Matheus-araujo-dev/tranal-despertador/pull/1).
A `main` tem só o esqueleto (README, AGENTS.md, CLAUDE.md, .gitignore) — o app inteiro
está no PR.

## O que mudou nesta rodada

O owner reclamou (*"achei feio e grosseiro"*) e pediu, no mesmo lote:

1. **Refator visual** da interface usando as skills de UI/UX da casa.
2. **Subir áudios** do dono (não só WAV: MP3 também), na tela do editor **e** nas Configurações.
3. **Digitar o horário** por cima do display, em vez de só botões.
4. **Corrigir o padrão numérico feio** ("3.0 vezes" → "3").
5. **Corrigir o file picker lento** (o explorer do Windows demorava demais).
6. **Corrigir o editor que demorava a carregar**.
7. **Despertador padrão** configurável nas Configurações.
8. **Explicar** a diferença entre "dias úteis" e "semanal", e o que acontece quando
   "onde tocar" fica vazio.

## Como está o código agora

| Arquivo | Responsabilidade | Mudança |
|---|---|---|
| `src/ui.rs` | sistema de design (tokens, paleta com contraste conferido na WCAG AA, componentes) | **reescrito** |
| `src/app.rs` | janela, lista, editor (modal), alarme tocando, Configurações | **reescrito** |
| `src/hora.rs` | interpretação do horário digitado (`0730`, `07:30`, `730`) | **novo** |
| `src/arquivos.rs` | seletor de arquivos **dentro do app** (modal próprio) | **novo** |
| `src/tones.rs` | presets + leitura de WAV/MP3/FLAC/OGG por `symphonia`; `importar_audio` copia para a pasta do app | **reescrito** |
| `src/model.rs` | `Settings.alarme_padrao` (o despertador padrão); `som_padrao`/`volume_padrao` removidos | alterado |
| `src/device_volume.rs` | `saidas_com_volume()`: **uma** enumeração do Core Audio | alterado |
| `src/estado.rs`, `src/storage.rs`, `src/schedule.rs`, `src/audio.rs` | estado compartilhado, JSON atômico, próxima hora, reprodução | da rodada anterior (sem mudança) |

Dependências: entrou `symphonia` 0.6.1 (feature `pcm` obrigatória) e **saiu** o `rfd`.

## Critérios de aceite desta rodada

1. A interface está **bonita** e **dark**, no padrão das skills `ui-ux/interface-review` e
   `design-ui-designer` da casa (tokens, tipografia, espaçamento, contraste AA).
2. O dono **digita a hora** e ela é aceita (`0730`, `07:30`, `730`).
3. Nenhum número com casa decimal ("3.0 vezes").
4. O **editor abre rápido** (o gargalo era a enumeração do Core Audio).
5. O dono **sobe um MP3** e ele toca no alarme; o arquivo é copiado para a pasta do app.
6. O **seletor de arquivo abre na hora** (sem o explorer do Windows).
7. Existe um **despertador padrão** que preenche todo novo despertador.
8. A tela explica **dias úteis × semanal** e **onde tocar vazio = saída padrão**.

## Evidências já coletadas (audite, não refaça)

| Item | Evidência |
|---|---|
| Testes | `cargo test` → **54 passed; 0 failed** |
| Lint | `cargo clippy --all-targets -- -D warnings` → limpo (o clippy foi instalado nesta rodada; não estava no toolchain) |
| Digitação da hora | captura `docs\captura-hora-digitada.png` (o campo mostra `0630` enquanto digita) e `docs\captura-hora-canonica.png` (após perder o foco, `06:30`) |
| Sliders | `docs\captura-editor-soneca.png` → "10 min", "**3 vezes**" (sem casa decimal) |
| Seletor de arquivos | `docs\captura-seletor-arquivos.png`, `docs\captura-seletor-sons.png`, `docs\captura-seletor-marcado.png` |
| Som do dono | `docs\captura-editor-biblioteca.png` → "meu-teste.mp3" em *meus áudios*, "Som escolhido: meu-teste.mp3 (1.0s)" |
| Editor e Configurações | `docs\captura-editor-novo.png`, `docs\captura-configuracoes.png` |
| E2E fase 1 (app minimizado) | disparou sozinho, **pico 1,0000**, `last_rung` gravado |
| E2E fase 2 (clique real em "Soneca") | o som **parou** (pico 0,0032) e a soneca **tocou de novo** (pico 1,0000, 53 s depois) |

> Nota: o detector de botão do `scripts/e2e.py` procurava a cor antiga do botão vermelho
> (`232,76,76`); com o redesenho (`198,40,40`) ele achava um pixel e o clique caía no
> espaço entre os botões. O **teste** foi corrigido, não o app.

## Riscos que o próprio código assume

1. **D2** — as saídas de áudio são pareadas pelo **nome**, não pelo id do endpoint do Core Audio.
2. **R5** — sem ícone na bandeja: se o Windows reiniciar à noite, o alarme só existe se o app
   estiver aberto.
3. O `alarme_padrao` guarda um `Alarm` completo (com `id` e histórico) que são ignorados
   quando ele vira alarme de verdade (`Alarm::a_partir_do_padrao`).
4. `symphonia` é **MPL-2.0** (copyleft de arquivo, não GPL/AGPL) — a regra da casa proíbe
   GPL/AGPL no mesmo processo.

## O que quero do quality gate

1. **Veredito**: aprovado, aprovado com ressalvas ou reprovado — e por quê.
2. **Mapeamento critério a critério** (os 8 da rodada): cada um está atendido? Qual não está?
3. **Revisão adversarial do código novo**: `ui.rs`, `app.rs` (editor, Configurações, importador),
   `hora.rs`, `arquivos.rs`, `model.rs`, `device_volume.rs`, `tones.rs`. Quero defeito concreto
   com arquivo e linha, não opinião.
4. **Riscos novos** que a revisão não encontrou, com probabilidade e impacto.
5. **O que ainda impede o owner de usar isso todo dia**.
6. Se alguma afirmação minha **não for verificável pelo repositório**, diga claramente.

Não precisa escrever código nem alterar arquivos: o repositório é de leitura para você.
