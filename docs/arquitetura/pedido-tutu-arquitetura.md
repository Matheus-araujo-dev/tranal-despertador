# Pedido ao Tutu (Hermes, perfil tutu) — arquitetura do Tranal Despertador

Use a skill `development.architecture` (procedure: analisar requisitos e restrições; identificar componentes e
responsabilidades; definir interfaces e contratos; selecionar padrões arquiteturais; documentar trade-offs;
gerar diagramas). Respeite `AGENTS.md` deste repositório e as regras de `D:\project-ai\AGENTS.md`.

## Contexto

Aplicativo desktop **Windows 10/11**, escrito em **Rust**, para o dono criar e tocar despertadores.
Interface **dark**, bonita, de uso simples em um clique, com **consumo mínimo de memória e processador**.
O dono pediu explicitamente: linguagem Rust, interface bonita em dark mode, fácil de usar, mínimo de consumo,
e documentado no segundo cérebro Obsidian (`D:\project-ai\ai-context`).

## Requisitos funcionais

1. Criar, editar e desativar despertadores: rótulo, hora (HH:MM), interruptor liga/desliga.
2. Repetição: uma vez, diário, dias úteis (seg-sex), semanal com escolha dos dias da semana.
3. Som: presets embutidos (gerados por síntese, sem material de terceiros) + arquivos WAV do usuário
   (pasta `sounds/` e importação por diálogo nativo); botão "testar".
4. Volume: controle 0-100% no app; opção "forçar 100% no dispositivo de saída" (endpoint de áudio do Windows);
   fade-in progressivo até o volume alvo.
5. Desativação: botão Parar; Snooze com minutos configuráveis e número máximo de repetições; desafio opcional
   (digitar texto ou resolver conta) para "despertar de verdade".
6. Reprodução: tocar em **vários dispositivos de saída ao mesmo tempo** (no PC do dono existem 3 saídas
   "PG27QRT1B" da AMD), em paralelo, **sem parar sozinho** até o usuário desativar.
7. Persistência em JSON no perfil do usuário; iniciar junto com o Windows; continuar rodando em segundo plano.
8. Mostrar o próximo alarme, contagem regressiva e o estado de cada despertador.

## Restrições (importantes)

- Sem servidor, sem navegador embarcado, binário único.
- RAM alvo: menos de 80 MB em uso. CPU ociosa: ~0%. Sem polling agressivo: o agendamento pode dormir até o
  próximo evento (thread dedicada com `Condvar`/timer).
- Dependências mínimas e todas com licença MIT/Apache (nada GPL/AGPL no mesmo processo, regra D012).
- Rust estável; o PC do dono **não tem MSVC nem Windows SDK**, então o toolchain é
  `x86_64-pc-windows-gnu` (linker GNU embutido). Crates com código C nativo não são aceitáveis.

## Contexto técnico já medido na máquina do dono (use, não repita a pesquisa)

- O backend WASAPI do PortAudio falha ao abrir **vários streams em paralelo** neste PC
  (`WdmSyncIoctl DeviceIoControl GLE = 0x00000490`); abrir **um por vez** funciona sempre.
- Para forçar 100% no endpoint de saída, a via confiável é a interface Core Audio
  `IAudioEndpointVolume.SetMasterVolumeLevelScalar` (o que o mixer do Windows mostra), em modo compartilhado.
- Em Rust, `cpal` (WASAPI compartilhado) cobre a reprodução; para o volume de endpoint, a via é a crate
  `windows` com `Win32_Media_Audio` (COM) — avalie o custo de build e a dependência.

## Entrega

Responda **no chat** (não crie arquivos) no formato de output da skill `development.architecture`:

- `design`: componentes com responsabilidade clara, em módulos Rust, com o caminho do arquivo de cada um.
- `interfaces`: contratos entre módulos (structs, funções públicas, quem chama quem).
- `diagrams`: pelo menos o diagrama de componentes e o de sequência do disparo do alarme (texto/ASCII).
- `trade_offs`: framework de GUI, crates de áudio e volume, persistência, modelo de thread do agendador.
- `recommendations`: o que fazer agora e o que fica para depois.
- `risks`: o que pode quebrar e como reduzir.

Seja direto e específico. Nada de generalidade: cada componente com arquivo e responsabilidade.
