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

## Instalação, publicação e atualização (DEC-APPS-DISTRIBUICAO-001)

Padrão de todo app Tranal (`D:\project-ai\docs\decisions\DEC-APPS-DISTRIBUICAO-001.md`):

- **Instalador:** `installer\tranal-despertador.iss` (Inno Setup 6, UTF-8 com BOM) gera um `Tranal-Despertador-Setup.exe` único em `{autopf}\Tranal Group\Tranal Despertador`. Instalação por usuário (`PrivilegesRequired=lowest`: `{autopf}` = `%LOCALAPPDATA%\Programs`, sem UAC na atualização silenciosa). `AppId` fixo `{259DBB0D-2F4A-4D3F-9B7A-CFEF530376E6}` — nunca mude. Migra a instalação do `instalar.ps1` antigo (pasta, atalhos, valor `Tranal Despertador` da chave Run) e não toca em `%APPDATA%\Tranal Despertador`.
- **Publicação:** `.github\workflows\publicar.yml`. Todo push na `main` = release nova (maior tag `vX.Y.Z` + 1 no patch; botão `workflow_dispatch` para minor/major; `Cargo.toml` é piso). O número é gravado no `Cargo.toml` do runner antes do build; o CI confere com `tranal-despertador.exe --versao` e o `VersionInfo` do Setup. Não suba a versão do `Cargo.toml` à mão a cada PR.
- **Atualizador:** `src\atualizador.rs` (escolha da versão é pura e testada: `escolher_release`). Ao abrir e de hora em hora; baixa o Setup, confere o tamanho, roda `/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /CLOSEAPPLICATIONS` e fecha; o Setup reabre o app. Só roda de uma pasta de instalação (nunca de `target\`).
- **Quem mescla o PR é o agente** (`gh pr merge --merge`), e acompanha o Actions até a release sair.
