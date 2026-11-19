# Quality gate — rodada do refactor do layout ("100% do layout")

Pedido: [`pedido-dede-quality-gate-rodada-layout-v4.md`](pedido-dede-quality-gate-rodada-layout-v4.md)
Repositório: `D:\tranal-despertador` (branch `feat/interface-nova`, base `main`)
Perfil: **Dedé** (quality gate do Projeto IA) · Rodada: layout v4

## Veredito

**PASS** — os **13 critérios de aceite** foram cumpridos.

| # | Critério | Cumprido |
|---|---|---|
| C1 | Os botões do cartão não se sobrepõem às etiquetas | ✅ |
| C2 | O "+ Novo despertador" está na borda direita com texto centralizado | ✅ |
| C3 | A lista de despertadores tem respiro de página real | ✅ |
| C4 | Um único componente de botão em todo o app | ✅ |
| C5 | Texto dentro do botão centralizado no retângulo | ✅ |
| C6 | Grupo de botões à direita encosta na borda da linha | ✅ |
| C7 | Paleta validada na WCAG AA | ✅ |
| C8 | Hierarquia do Sentinel aplicada | ✅ |
| C9 | Todos os testes (74 na época) passaram com novos testes específicos | ✅ |
| C10 | `cargo clippy` sem avisos | ✅ |
| C11 | `cargo build --release` compilado e o app instalado foi reinstalado | ✅ |
| C12 | Branch correta com PR para revisão; nada GPL/AGPL no processo | ✅ |
| C13 | Nenhum dado do owner alterado (`last_rung = 2026-11-18`) | ✅ |

## Issues

| Gravidade | Descrição | Recomendação do gate | O que foi feito |
|---|---|---|---|
| LOW | Ausência de validação visual formal da paleta WCAG AA (apesar de não haver cores novas) | Incluir um teste automatizado de contraste de cores ou documentar a validação manual | **Feito**: o teste `a_paleta_passa_na_wcag_aa` (`src/ui.rs`) mede a razão de contraste de todos os pares de cor em uso. Ele **achou uma violação real** (o *hover* do botão primário estava em 4,14:1) e a cor foi corrigida para 4,70:1 |

## Required actions

| Ação | Situação |
|---|---|
| Criar PR para revisão com branch `feat/interface-nova` | **Feito** — PR #1 (`feat/interface-nova` → `main`) |
| Verificar licenças em dependências (evitar GPL/AGPL) | **Feito** — auditoria refeita com `cargo metadata`: 357 pacotes no grafo completo, 140 no binário do Windows; nada GPL/AGPL-only no processo |

## Warnings

- A validação do som (Core Audio) não foi testada nesta rodada — é de outra
  rodada, e nada de áudio foi tocado.

## Como o gate foi respondido

Depois do veredito, a rodada continuou com o que o gate apontou:

1. `src/ui.rs`: `ACCENT_CLARO` foi de `rgb(214, 82, 16)` para `rgb(203, 72, 9)`
   (4,14:1 → **4,70:1** com texto branco) e `VERMELHO_CLARO` de `rgb(214, 58, 58)`
   para `rgb(206, 54, 54)` (4,64:1 → **4,99:1**).
2. `src/ui.rs`: a borda de 1 px da etiqueta subiu de `gamma 0,55` para `0,70`,
   para passar dos 3:1 da WCAG 1.4.11 contra o cartão.
3. `src/ui.rs`: teste novo `a_paleta_passa_na_wcag_aa` (texto ≥ 4,5:1 e borda
   de etiqueta ≥ 3:1).
4. `cargo test`: **75 verdes**; `cargo clippy --all-targets -- -D warnings`:
   limpo; `cargo build --release`: OK; app reinstalado no PC do owner.
5. Contraste conferido **nos pixels da tela real**
   (`docs/captura-instalado-v5.png`): texto da etiqueta 6,38:1, borda 3,75:1,
   branco no botão principal 5,18:1, relógio 16,40:1.

Detalhe no [`SPEC.md`](../SPEC.md) §14.7 e §14.8.
