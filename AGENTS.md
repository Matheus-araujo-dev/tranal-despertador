# Tranal Despertador — instruções para agentes de código

Este repositório faz parte do Projeto IA. **As regras do projeto valem aqui** e estão em `D:\project-ai\AGENTS.md`: leia antes de agir.

Resumo do que se aplica a este repositório:
1. **Despacho:** desenvolvimento = **Tutu** (`development.architecture`, `development.implementation`, `development.testing`); testes e cenários = **Adan**; fiscalização das entregas = **Dedé** (`hermes -p dede`), modelo de família diferente do executor. Informar ao entregar qual agente e quais skills foram usados.
2. **Decisões do owner** vão para o log oficial do project-ai (`python -m runtime.decisions.tools` rodado em `D:\project-ai`), com nota em `ai-context/94-Decisoes/` e índice em `ai-context/92-Projetos/`. Este repositório só referencia as decisões (`docs/decisoes/README.md`).
3. **Branch sempre:** nunca commitar direto na `main`; PR para revisão.
4. **Orquestrar, não embutir (D012):** ferramentas de terceiros rodam como processo ou serviço separado atrás de um adaptador; nada de código de terceiros colado aqui; bibliotecas GPL/AGPL nunca no mesmo processo.
5. **Testes acompanham toda mudança** (`cargo test`); a suíte precisa passar antes do PR.

## O que este projeto é

Aplicativo desktop **Windows**, escrito em **Rust**, para criar e tocar despertadores. Interface **dark**, feita para ser usada em um clique, com consumo mínimo de memória e processador.

Decisões estruturais e o porquê de cada escolha: `docs/SPEC.md` e `docs/decisoes/`.

## Como rodar

```powershell
cargo run                 # debug
cargo build --release     # release (o binário fica em target\release\tranal-despertador.exe)
cargo test                # suíte
cargo clippy -- -D warnings
```

Documentos: `docs/SPEC.md` (especificação), `docs/arquitetura/` (desenhos por marco), `docs/decisoes/` (índice de decisões).
