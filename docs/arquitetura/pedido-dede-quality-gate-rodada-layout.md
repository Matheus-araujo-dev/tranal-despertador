# Quality gate — Tranal Despertador: redesenho do layout (referência Tranal Sentinel)

Repositório: `D:\tranal-despertador` (branch `feat/interface-nova`, base `main`)
Pedido original do owner (PT-BR): *"o layuout do app ainda é muito feio, esse botão novo despertador e as informações estão grudadas na borda do app sem nenhum espaçamento, acho que podemos usar o layout do app tranal sentinel como inspiração."*

## O que foi entregue nesta rodada

Um **redesenho estrutural da tela principal**, com o layout do **Tranal Sentinel** (`E:\projetos\tranal-sentinel`, WPF/.NET) como referência:

1. **Nada encosta mais na borda da janela.** O `CentralPanel` usava `inner_margin = 0`; agora a página tem
   24 px de margem lateral e 20 px vertical (`ui::MARGEM_PAGINA` / `MARGEM_PAGINA_VERTICAL`).
2. **Cartões com respiro de 20 px** (o `Padding 20` do Sentinel), em vez de 18×16.
3. **Topo virou barra fina** (como a `Row 44` do Sentinel): nome do app à esquerda, "Configurações" à
   direita, e uma linha divisória separando a barra do conteúdo.
4. **Cabeçalho de página** com hierarquia do Sentinel: `H1` de 26 px ("Despertadores"), `Sub` de 13 px
   ("1 ativo · próximo às 07:00") e o **botão primário na linha do título** (o "+ Novo despertador",
   que antes ficava solto e grudado embaixo do cartão do relógio).
5. **A dica ("Fechar a janela minimiza...") foi para dentro do cartão do relógio**, com separador.
6. **Rótulos e tipografia**: `titulo_secao` passou a 10,5 px semi-bold (o `Label` do Sentinel) com 8 px
   de respiro; novos estilos `ESTILO_TITULO` (26), `ESTILO_HERO` (34), `ESTILO_ROTULO` (10,5),
   `ESTILO_SUB` (13); relógio e hora do próximo em monoespaçada.
7. **Altura das colunas calculada antes do `horizontal`** (o egui devolve 0 de `available_height()`
   dentro de um layout horizontal no primeiro quadro — a coluna inteira sumia da tela).
8. Correção de detalhe: o slider mostrava "100 %" e "12 s"; agora mostra "100%" e "12s".

**A paleta não foi tocada**: o laranja do app continua sendo o acento, porque ele já passou na WCAG AA
(7,1:1 e 5,2:1) e é a identidade que o dono aprovou. O Sentinel é monocromático; aqui a *estrutura* do
layout foi adotada, não a paleta.

## Critérios de aceite a fiscalizar

| # | Critério |
|---|---|
| C1 | Nenhum conteúdo encosta na borda da janela (margem de página real) |
| C2 | O botão "Novo despertador" está na linha do título da página, com respiro, e não solto na borda |
| C3 | As informações do próximo despertador e a dica estão dentro de cartões com respiro de 20 px |
| C4 | A hierarquia do Sentinel está aplicada (H1 26, Label 10,5, Sub 13, Hero 34) |
| C5 | O topo é uma barra fina com linha divisória, não um cartão |
| C6 | As colunas mantêm a altura disponível (sem faixa vazia de 520 px) |
| C7 | `cargo test` passa (72 testes) |
| C8 | `cargo clippy --all-targets -- -D warnings` sem aviso |
| C9 | `cargo build --release` compila |
| C10 | A paleta continua validada na WCAG AA (nenhuma cor nova) |
| C11 | Nada de GPL/AGPL no mesmo processo (regra D012) |
| C12 | Branch, nunca commit direto na `main`; PR para revisão |

## Evidências (capturas do app real, nesta máquina)

- `docs/captura-interface-nova.png` — o **antes** (versão que estava no repositório): o cartão do
  relógio encostado na borda esquerda, o "MEUS DESPERTADORES (1)" colado na borda direita e o botão
  "+ Novo despertador" solto, grudado embaixo do cartão.
- `docs/captura-layout-v3.png` — o **depois**, tirada do app **instalado**
  (`%LOCALAPPDATA%\Programs\Tranal Despertador\tranal-despertador.exe`, pid 42304).
- `docs/captura-editor-v3.png` — o editor (modal), que já estava no padrão certo.
- `docs/captura-config-v3.png` — as Configurações (modal).

Verificações feitas por clique real no app rodando (com `PrintWindow` + `AttachThreadInput`):
"Configurações" abre, "Novo despertador" abre o editor, "Fechar" fecha o modal, "Fechar
configurações" fecha, "Editar" abre o editor do alarme existente.

## Três defeitos encontrados durante a execução (e corrigidos)

1. **`available_height()` dentro de `horizontal` devolve 0** no primeiro quadro: as duas colunas
   sumiram da tela. A altura agora é calculada **antes** do `horizontal_top` e passada por
   parâmetro.
2. **`Align` do egui alinha no eixo transversal**: num layout horizontal, `Align::Center` centraliza
   **verticalmente** — o botão do cabeçalho descia para o meio da janela. Trocado por `Align::Min`.
3. **`PrintWindow` (PW_RENDERFULLCONTENT) sai 1:1 com o `GetWindowRect`**: somar a borda da janela e
   a barra de título no clique deslocava o clique e o botão não recebia. Isso era do **script de
   teste**, não do app — o mesmo tipo de erro já tinha sido cometido antes com o `mouseData >> 16`.

## Comandos de verificação

```powershell
$env:PATH = "$env:USERPROFILE\.cargo\bin;" + $env:PATH
$toolchain = "$env:USERPROFILE\.rustup\toolchains\stable-x86_64-pc-windows-gnu\lib\rustlib\x86_64-pc-windows-gnu\bin"
$env:PATH = "$toolchain;$toolchain\self-contained;" + $env:PATH
$mingw = Get-ChildItem "$env:LOCALAPPDATA\Microsoft\WinGet\Packages" -Directory -Filter 'BrechtSanders.WinLibs*' | ForEach-Object { Join-Path $_.FullName 'mingw64\bin' } | Where-Object { Test-Path (Join-Path $_ 'gcc.exe') } | Select-Object -First 1
$env:PATH = "$mingw;$env:PATH"
Set-Location "D:\tranal-despertador"
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
```

## Arquivos da rodada

- `src/ui.rs` — escala de espaçamento (`XL`), `MARGEM_PAGINA`, `PADDING_CARTAO`, estilos de
  tipografia do Sentinel, `titulo_pagina`, `sub`, `botao_primario_grande`, `cartao()` com padding 20,
  `titulo_secao` com respiro.
- `src/app.rs` — `desenhar()` com a margem de página, `topo()` como barra fina, `cabecalho_pagina()`
  (novo), `coluna_relogio()` e `coluna_alarmes()` com altura calculada antes do `horizontal`, dica
  dentro do cartão, sufixos dos sliders corrigidos.
- `docs/captura-layout-v3.png`, `docs/captura-editor-v3.png`, `docs/captura-config-v3.png` — as
  capturas novas.

## O que NÃO foi feito (e por quê)

- **Não foi adotada a barra lateral de navegação** do Sentinel (256 px com pílulas). O Sentinel tem
  seis abas; o Tranal Despertador tem uma tela principal e duas Configurações — uma barra lateral
  com dois itens seria mais ruído que util. A *estrutura* do layout foi adotada, a navegação não.
- **A paleta não virou monocromática**: o laranja é a identidade do app e já passou na WCAG AA.
- **Nenhum dado do dono foi mexido**: o `last_rung = 2026-11-18` é o alarme das 07:00 tocando de
  verdade hoje (o agendador gravou), e o card passou a oferecer "Repetir amanhã" — a decisão de
  rearmar é do owner.

## Como responder

Siga o `structured_output` do seu contrato:
`status` (PASS | PASS_WITH_WARNINGS | REWORK_REQUIRED | BLOCKED), `criteria` (lista de
{description, met}), `issues` (lista de {severity, description, recommendation}),
`required_actions` (lista), `warnings` (lista).
