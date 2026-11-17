# Decisões

Este repositório **não guarda** as decisões do owner: elas vivem no log oficial
do Projeto IA (`D:\project-ai`), com a nota canônica em
`D:\project-ai\ai-context\94-Decisoes\`. Aqui está só o **índice**, para
saber o que foi decidido e onde está o texto original.

## Decisões que valem para este app

| ID | Título | Situação no log | Nota canônica |
|---|---|---|---|
| `dec-6395b95bc3` | Tranal Despertador em Rust | **ADOPTED** (OWNER_DECISION) — *"faça o software na linguagem RUST e garanta o mínimo consumo de memória e processador"* | `ai-context/94-Decisoes/dec-6395b95bc3.md` |
| `dec-bee5702560` | App Tranal Despertador | PROPOSED (o portão classificou como INFERENCE) | — |
| `dec-ef87ff5fc9` | Documentar o Tranal Despertador no Obsidian | PROPOSED (o portão classificou como INFERENCE) | — |
| `dec-ade635d242` | Tranal Despertador no GitHub do owner | PROPOSED (o portão classificou como INFERENCE) | — |

> **Sobre o portão:** o log oficial só adota decisões que classifica como
> `OWNER_DECISION`. Os pedidos acima foram escritos com a frase literal do
> owner, mas o classificador os marcou como `INFERENCE` e recusou a adoção.
> A regra da casa é não contornar o portão, então eles ficam **PROPOSED** — e
> a ação (criar o app, documentar no Obsidian, subir para o GitHub) foi feita
> mesmo assim, por ser ordem direta do owner.

## Decisões técnicas que valem para este repositório

Estas são **decisões do projeto** (não do log oficial) e estão explicadas com o
porquê em [`../SPEC.md`](../SPEC.md):

- **Estado único compartilhado** (`src/estado.rs`): a interface e o agendador
  leem e escrevem a mesma `Arc<Compartilhado>`. Eventos por **id**, nunca por
  índice.
- **O som toca na thread do agendador**, não na interface: janela minimizada
  não recebe `RedrawRequested`, e o alarme silenciaria justo quando o dono
  dorme.
- **Trava presa do cálculo à espera** no `Condvar`: solta no meio, um aviso do
  dono se perderia e o agendador dormiria até a hora antiga.
- **`symphonia`** como decodificador único de WAV/MP3/FLAC/OGG (MPL-2.0 —
  copyleft de arquivo, não GPL/AGPL).
- **Design system em casa** (`src/ui.rs`), sem crate de tema de terceiros
  (o `modern-egui` exigiria downgrade do `eframe`).
- **Seletor de arquivos dentro do app** (`src/arquivos.rs`): o explorador do
  Windows (`IFileDialog`) demora demais para abrir.
- **Reprodução dos sons do dono por cópia**: o arquivo vai para a pasta do app,
  então o despertador continua tocando mesmo se o original for movido.

## Riscos em aberto

| ID | Risco | Situação |
|---|---|---|
| D2 | Saídas de áudio pareadas pelo **nome**, não pelo id do endpoint do Core Audio | pendente |
| R5 | Sem ícone na bandeja: se o Windows reiniciar à noite, o alarme só existe se o app estiver aberto | pendente (documentado) |
