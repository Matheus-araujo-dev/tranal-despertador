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
- **Seleção automática no campo da hora**: ao ganhar o foco, o texto inteiro é
  selecionado, para o dono **digitar por cima** do horário (o primeiro dígito
  substitui o valor antigo).
- **Configurações só gravam quando o valor muda**: antes, o `dados.json` era
  reescrito a cada quadro (60 vezes por segundo só pelo dono olhar a tela).

## Riscos em aberto

| ID | Risco | Situação |
|---|---|---|
| D2 | Saídas de áudio pareadas pelo **nome**, não pelo id do endpoint do Core Audio | **mitigado**: a *escolha* do dono continua pelo nome (é o que o mixer do Windows mostra), mas o volume é salvo e devolvido pelo **id do endpoint** — duas saídas com o mesmo nome não se atrapalham mais, e uma delas não fica presa em 100%. Se o nome se repete, o resumo que o app mostra avisa que todas as homônimas foram ajustadas |
| R5 | Sem ícone na bandeja: se o Windows reiniciar à noite, o alarme só existe se o app estiver aberto | pendente (documentado) |

## Licenças das dependências (auditoria)

`cargo tree` → 251 dependências transitivas. O inventário completo está no
[`README.md` do repositório](../README.md#licenças). Resumo: MIT/Apache/BSD/ISC/
Zlib/0BSD/Unlicense, **MPL-2.0** (symphonia, copyleft de arquivo, sem
modificação do código-fonte), OFL-1.1 (fontes do egui), BSL-1.0, Unicode-3.0 e
uma dupla **Apache-2.0 OU GPL-2.0-only** (`self_cell`, consumida sob
Apache-2.0). **Nenhuma GPL/AGPL no processo** (regra D012).
