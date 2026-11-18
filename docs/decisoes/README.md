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
| `dec-e91fd361d6` | Instalador + ícone na bandeja (escolha do owner no questionário da rodada) | PROPOSED (o portão classificou como INFERENCE) | — |

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
- **Ícone na bandeja** (`src/tray.rs`): criado **no primeiro quadro** da
  interface, porque o Windows exige um event loop na thread que cria o ícone.
  O `TrayIcon` fica guardado no app — se for solto, o Windows remove o ícone
  da área de notificação.
- **Menu da bandeja próprio** (`src/tray_menu.rs`): o menu pronto da biblioteca
  ancora **na posição do mouse**, que está dentro da barra de tarefas, e o
  último item ("Sair") fica escondido atrás dela. O menu próprio é nativo do
  Windows (`CreatePopupMenu` + `TrackPopupMenuEx`), ancorado na borda inferior
  da área de trabalho, e aberto **na thread principal** (aberto de outra
  thread, ele pisca e some).
- **O app se promove sozinho** (`IsPromoted = 1` no registro): o Windows 11
  esconde ícone novo na lista de ocultos, e o caminho para promovê-lo à mão
  muda a cada instalação. A gravação é **conferida** — o Windows reescreve a
  chave e apaga a preferência, então o app repete até ler o valor de volta.
- **Sem `IconGuid` fixo**: com GUID, o Windows 11 guarda o ícone pela
  identidade e **os cliques deixam de chegar no app**. A identidade do ícone
  é o caminho exato do executável; a preferência é reaplicada a cada arranque.
- **Instalação por usuário** (`scripts/instalar.ps1`):
  `%LOCALAPPDATA%\Programs\Tranal Despertador`, sem administrador e sem
  "Program Files". A chave de início automático aponta para o executável
  **instalado** — é o que faz o despertador sobreviver a um reinício do PC
  mesmo depois de uma reinstalação.
- **Desinstalar preserva os dados do dono** (`%APPDATA%\Tranal Despertador`);
  só `-ApagarDados` remove.
- **Ícone desenhado em código** (`src/icone.rs`), sem PNG/ICO versionado: o
  `.ico` dos atalhos é exportado pelo próprio app (`--exportar-icone`) a
  partir do mesmo desenho da bandeja — uma só fonte para o ícone.

## Riscos em aberto

| ID | Risco | Situação |
|---|---|---|
| D2 | Saídas de áudio pareadas pelo **nome**, não pelo id do endpoint do Core Audio | **mitigado**: a *escolha* do dono continua pelo nome (é o que o mixer do Windows mostra), mas o volume é salvo e devolvido pelo **id do endpoint** — duas saídas com o mesmo nome não se atrapelham mais, e uma delas não fica presa em 100%. Se o nome se repete, o resumo que o app mostra avisa que todas as homônimas foram ajustadas |
| R5 | Sem ícone na bandeja: se o Windows reiniciar à noite, o alarme só existe se o app estiver aberto | **resolvido**: início automático minimizado (com instância única) **e** ícone na bandeja do Windows. Medido: ícone visível na faixa, clique esquerdo reabre a janela, menu com "Abrir o Tranal" e "Sair" funcionando |
| R6 | O ícone da bandeja nascer escondido (o Windows 11 esconde ícone novo) | **mitigado**: o app grava `IsPromoted = 1` na chave do ícone e confere se o Windows aceitou. O `IconGuid` fixo, que resolveria a chave, foi descartado porque quebra os cliques no Windows 11 |

## Licenças das dependências (auditoria)

`cargo tree` → 251 dependências transitivas. O inventário completo está no
[`README.md` do repositório](../README.md#licenças). Resumo: MIT/Apache/BSD/ISC/
Zlib/0BSD/Unlicense, **MPL-2.0** (symphonia, copyleft de arquivo, sem
modificação do código-fonte), OFL-1.1 (fontes do egui), BSL-1.0, Unicode-3.0 e
uma dupla **Apache-2.0 OU GPL-2.0-only** (`self_cell`, consumida sob
Apache-2.0). **Nenhuma GPL/AGPL no processo** (regra D012).
