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
- **Layout com a referência do Tranal Sentinel** (`docs/SPEC.md` §13 e §14): o
  owner pediu "inspiração" no app da casa. O que foi trazido: a **margem de
  página** (32 px), o **respiro do cartão** (20 px, o `Padding 20` do
  Sentinel), a **barra de topo fina** com divisória, a **hierarquia
  tipográfica** (H1 26, Hero 34, Label 10,5, Sub 13) e a **ação principal na
  linha do título** da página.
- **Um componente só de botão, desenhado à mão** (`src/ui.rs`,
  `docs/SPEC.md` §14): o `egui::Button` do 0.37 posiciona o texto pelo sistema
  de atoms e não garante a centralização; o `with_layout` — a forma "certa" de
  alinhar algo à direita — ocupa o espaço todo do pai e **desenha por cima** do
  conteúdo anterior. Por isso: botão desenhado à mão, com a largura **medida**
  antes de ser pintado, e o grupo de ações alinhado à direita por
  `add_space`.
- **Etiquetas e botões do cartão em linhas separadas** (§14): dividiam a mesma
  linha, e era a causa da sobreposição que o owner viu.
- **A paleta do Sentinel NÃO foi adotada**: ele é monocromático; o laranja do
  Despertador é a identidade do app e já passa na WCAG AA (7,1:1 no texto,
  5,2:1 no botão). A reclamação era de **espaçamento**, e trocar a cor não a
  resolveria.
- **A barra lateral de navegação do Sentinel NÃO foi adotada**: o Sentinel tem
  seis abas; o Despertador tem uma tela principal e duas Configurações — uma
  barra lateral com dois itens seria ruído.
- **Altura das colunas calculada antes do `horizontal`**: dentro de um layout
  horizontal, o egui devolve `available_height() = 0` no primeiro quadro, e
  as colunas sumiam da tela.
- **O contraste da paleta é medido por teste, não por olho** (§14.7): o gate de
  qualidade apontou que a validação da WCAG AA era só "visual". O teste
  `a_paleta_passa_na_wcag_aa` calcula a razão de contraste de **todos** os
  pares de cor que o app usa. Ele achou uma violação real — o *hover* do botão
  primário (`rgb(214, 82, 16)` com texto branco) dava **4,14:1** — e a cor foi
  corrigida para `rgb(203, 72, 9)` (**4,70:1**), o laranja mais claro que ainda
  passa no AA. A borda da etiqueta também ficou mais presente (`gamma 0,55` →
  `0,70`), para chegar a 3:1 contra o cartão (WCAG 1.4.11).
- **As cores são as do app, com os números medidos**: o laranja continua sendo
  a identidade do Tranal; a correção foi no **valor**, não na paleta.

## Riscos em aberto

| ID | Risco | Situação |
|---|---|---|
| D2 | Saídas de áudio pareadas pelo **nome**, não pelo id do endpoint do Core Audio | **mitigado**: a *escolha* do dono continua pelo nome (é o que o mixer do Windows mostra), mas o volume é salvo e devolvido pelo **id do endpoint** — duas saídas com o mesmo nome não se atrapelham mais, e uma delas não fica presa em 100%. Se o nome se repete, o resumo que o app mostra avisa que todas as homônimas foram ajustadas |
| R5 | Sem ícone na bandeja: se o Windows reiniciar à noite, o alarme só existe se o app estiver aberto | **resolvido**: início automático minimizado (com instância única) **e** ícone na bandeja do Windows. Medido: ícone visível na faixa, clique esquerdo reabre a janela, menu com "Abrir o Tranal" e "Sair" funcionando |
| R6 | O ícone da bandeja nascer escondido (o Windows 11 esconde ícone novo) | **mitigado**: o app grava `IsPromoted = 1` na chave do ícone e confere se o Windows aceitou. O `IconGuid` fixo, que resolveria a chave, foi descartado porque quebra os cliques no Windows 11 |

## Licenças das dependências (auditoria)

Auditoria refeita nesta rodada com `cargo metadata` (o `cargo tree` sozinho não
mostra o campo de licença): **357** pacotes no grafo completo e **140** no
binário do Windows. O inventário completo está no
[`README.md` do repositório](../README.md#licenças). Resumo: MIT/Apache/BSD/ISC/
Zlib/0BSD/Unlicense, **MPL-2.0** (symphonia, copyleft de arquivo, sem
modificação do código-fonte), OFL-1.1 (fontes do egui), BSL-1.0, Unicode-3.0 e
uma dupla **Apache-2.0 OU GPL-2.0-only** (`self_cell`, consumida sob
Apache-2.0). **Nenhuma GPL/AGPL no processo** (regra D012).

O único outro pacote que menciona GPL no campo de licença é o `r-efi`
(MIT OU Apache-2.0 OU LGPL-2.1-or-later): ele entra só como
*build-dependency* do `cc` (que compila o `android-activity` e o `chrono`),
ou seja, **fora do processo** do app, e sob a opção permissiva.
