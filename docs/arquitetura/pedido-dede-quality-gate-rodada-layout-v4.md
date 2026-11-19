# Quality gate — Tranal Despertador: refactor do layout ("100% do layout", base Tranal Sentinel)

Repositório: `D:\tranal-despertador` (branch `feat/interface-nova`, base `main`)

Pedido original do owner (PT-BR): *"veja, os botões de repetir, editar e testar
sobrepõe as informações do despetador, o botão novo despertador completamente
fora de posição com o texto sem centralizar, acho que precisamos usar as skill
de UX e UI disponível neste computador para refatorar 100% do laytout desse
sistema, baseando-se no layout do tranal sentinel"*.

## O que foi entregue nesta rodada

Um **refactor do layout inteiro**, com o **Tranal Sentinel**
(`E:\projetos\tranal-sentinel`, WPF/.NET) como referência de estrutura e as
skills de UX/UI da máquina (**UI Designer**, **UX Architect** e o roteiro
`skills/ui-ux/interface-review.yaml` do Projeto IA) como método.

### 1. As três causas-raiz (lidas na fonte do egui 0.37, não deduzidas)

- **Sobreposição dos botões sobre as etiquetas.** Em `cartao_alarme()`, as
  etiquetas (`horizontal_wrapped`) e os botões (`with_layout(right_to_left)`)
  dividiam a **mesma linha**. O `with_layout` cria um escopo que ocupa **todo**
  o espaço do pai; como as etiquetas já tomavam a largura inteira, o escopo dos
  botões ficou com ~0 px e eles foram desenhados **por cima** das etiquetas.
- **"+ Novo despertador" fora de posição e com o texto torto.** O mesmo
  `with_layout`: em `Layout::right_to_left` o `main_align` é `Center` (o
  alinhamento principal é ignorado no eixo da linha) e o argumento do
  construtor controla só o eixo transversal — o botão ficava centralizado no
  espaço restante, não na borda. E no egui 0.37 o texto do `Button` é
  posicionado pelo sistema de **atoms**, que não garante centralização.
- **Lista encostando na borda direita.** A coluna da lista usava a largura
  toda do pai, sem respiro próprio; com a margem de página de 24 px, o cartão
  chegava a ~22 px da borda da janela.

### 2. O design system refeito (`src/ui.rs`)

- **Um componente só de botão**, desenhado à mão (como o chip e o interruptor):
  `EstiloBotao::{Primario, Neutro, Fantasma, Perigo, Icone}`, com as medidas do
  Sentinel (`Padding 28,11` e canto 10 no principal; `Padding 20` e canto 16 no
  cartão; etiqueta com `Padding 13,5`, canto 14, fundo transparente e borda de
  1 px). O texto é **sempre** centralizado no retângulo.
- **A largura do botão é medida antes de pintá-lo** (`largura_botao`), com a
  mesma fonte — é o que permite alinhar um grupo na borda direita.
- **`acoes_a_direita`**: mede cada botão, reserva o espaço que sobra com
  `add_space` e desenha o grupo encostado na borda direita. **`with_layout` não
  é mais usado** para alinhar nada à direita.
- **`empurra_para_direita`**: para quando o item da direita tem largura
  conhecida (o interruptor do cartão).
- Margem de página **32 px** (era 24) e **24 px** na vertical (era 20); o
  Sentinel usa 36/30.
- Relógio em 48 px (era 56), para competir com a hora do alarme (34 px).
- **Seletor segmentado e múltiplo corrigidos**: descontam o espaço entre os
  segmentos da largura. Antes eles estouravam a borda e esticavam o `ScrollArea`
  do editor em 35 px (achado **medindo** as capturas).

### 3. O cartão do despertador

Três linhas, sem nada por cima de nada: (1) hora, nome e interruptor; (2) as
etiquetas, **sozinhas** na linha; (3) as ações ("Repetir amanhã", "Editar",
"Testar", "×") encostadas na borda direita do cartão.

### 4. Cabeçalho, barra do topo e modais

- H1 e `Sub` à esquerda, ação principal à direita (o arranjo do Sentinel); o
  botão fica **centrado na altura do H1** (o egui alinha caixas, e a caixa da
  linha tem o ascendente da fonte acima da letra maiúscula).
- Barra do topo no formato do Sentinel: `TRANAL` forte + `DESPERTADOR` fraco +
  o nome da página, ação à direita.
- Modais (editor e Configurações) com "Fechar" encostado na borda direita —
  antes ele ficava 55 px longe da borda do modal.

## Critérios de aceite a fiscalizar

| # | Critério |
|---|---|
| C1 | Os botões do cartão **não** se sobrepõem às etiquetas (o defeito apontado pelo owner) |
| C2 | O "+ Novo despertador" está na borda direita, com o texto **centralizado** no botão |
| C3 | A lista de despertadores não encosta na borda da janela (respiro de página real) |
| C4 | Um **único** componente de botão em todo o app (mesma forma, respiro e texto centralizado) |
| C5 | O texto dentro do botão tem o mesmo centro do retângulo do botão (medição, 1 px) |
| C6 | O grupo de botões à direita encosta na borda da linha e não invade o conteúdo anterior |
| C7 | A paleta continua validada na WCAG AA (nenhuma cor nova; o chip ficou com fundo transparente) |
| C8 | A hierarquia do Sentinel está aplicada (H1 26, Hero 34, Label 10,5, Sub 13) |
| C9 | `cargo test` passa (74 testes, 2 novos que medem o defeito) |
| C10 | `cargo clippy --all-targets -- -D warnings` sem aviso |
| C11 | `cargo build --release` compila e o app **instalado** foi reinstalado |
| C12 | Branch, nunca commit direto na `main`; PR para revisão; nada de GPL/AGPL no processo (D012) |
| C13 | Nenhum dado do owner foi alterado (o `last_rung = 2026-11-18` continua; rearmar é decisão do owner) |

## Evidências (capturas do app real, nesta máquina)

- `docs/captura-instalado-v4.png` — o **depois**, tirada do app **instalado**
  (`%LOCALAPPDATA%\Programs\Tranal Despertador\tranal-despertador.exe`) com os
  **dados reais do owner**: o card das 07:00 com as etiquetas na segunda linha
  e "Repetir amanhã / Editar / Testar / ×" na terceira, **sem sobreposição**.
- `docs/captura-layout-v4.png` — a tela principal com dois despertadores.
- `docs/captura-editor-v4.png` — o editor (modal).
- `docs/captura-config-v4.png` — as Configurações (modal).
- O **antes** está em `docs/captura-interface-nova.png` (interface original) e
  na captura do owner.

## Como o defeito foi medido (não "parece certo")

- `src/ui.rs::testes::o_texto_do_botao_fica_centralizado` — o retângulo do
  texto desenhado (tirado das `Shape::Text` do quadro) tem que ter o mesmo
  centro do retângulo do botão, com tolerância de 1 px.
- `src/ui.rs::testes::o_grupo_de_botoes_para_na_direita_sem_invadir_o_conteudo`
  — o grupo de botões encosta na borda direita da linha (1 px) e começa **depois**
  da etiqueta que vem antes.
- A causa do "modal mais largo que o conteúdo" foi achada **medindo** as
  capturas (o `seletor_segmentado` estourava 35 px), não lendo o código.

## O que o assessor não pode verificar sozinho

- O comportamento real da janela do Windows (a instalação foi conferida aqui, com
  `cargo build --release`, `scripts\instalar.ps1` e o app rodando).
- A validação do Som (o `IAudioMeterInformation` do Core Audio) não faz parte
  desta rodada: nada do áudio foi tocado.

Responda no formato estruturado do seu `agent.yaml` (status, criteria, issues,
required_actions, warnings).
