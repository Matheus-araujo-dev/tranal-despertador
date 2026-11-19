# Tranal Despertador — especificação e arquitetura

> Documento no padrão da skill `development.architecture` do Projeto IA
> (design, interfaces, diagramas, trade-offs, recomendações, riscos).

## 1. Resumo

Aplicativo desktop **Windows 10/11**, escrito em **Rust**, que o dono usa para criar
despertadores e ouvi-los tocar na hora. Interface **dark**, feita para ser usada em um
clique, com consumo mínimo de memória e processador: o binário é nativo (sem navegador
embarcado) e o agendamento dorme até o próximo evento.

| Item | Decisão |
|---|---|
| Linguagem | Rust (estável 1.100), toolchain `x86_64-pc-windows-gnu` |
| Interface | `egui`/`eframe` 0.37 (janela nativa, GPU acceleration via glow) |
| Áudio | `cpal` 0.18 (WASAPI compartilhado) |
| Leitura de arquivo de áudio | `symphonia` 0.6 (WAV, MP3, FLAC, OGG) — MPL-2.0 |
| Volume do Windows | crate `windows` (Core Audio, `IAudioEndpointVolume`) |
| Dados | JSON em `%APPDATA%\Tranal Despertador\dados.json` |
| Persistência do agendamento | thread dedicada com canal, sem polling |

## 2. Objetivo e escopo

**Objetivo:** o dono cria despertadores (uma vez, diário, dias úteis, semanal), escolhe o
som, o volume e onde tocar, e **ouve o alarme com 100% de volume até ele desativar**.

**Dentro do escopo:**

- criar, editar, ativar, desativar e excluir despertadores;
- repetir (uma vez, todo dia, dias úteis, dias da semana marcados);
- som: 5 presets sintetizados no app + qualquer arquivo de áudio do dono
  (WAV, MP3, FLAC, OGG — o arquivo é **copiado** para a pasta do app);
- **digitar o horário** por cima do relógio, além dos botões de ±;
- **despertador padrão**: um modelo montado nas Configurações que já vem
  preenchido em todo novo despertador;
- volume do app (0–100%), fade-in progressivo e "forçar 100% no aparelho";
- escolher em quais saídas de áudio tocar (várias ao mesmo tempo);
- desativar de três formas: Parar, Soneca (com limite) e desafio (digitar ou contar);
- mostrar o próximo alarme e a contagem;
- iniciar junto com o Windows e continuar rodando em segundo plano.

**Fora do escopo (por enquanto):**

- app de celular, sincronização na nuvem, conta do usuário;
- tocar música do acervo (só arquivo local, até 120 s);
- atalhos globais de teclado e ícone na bandeja do sistema.

## 3. Design — componentes

```
┌──────────────────────────────────────────────────────────────────────┐
│                         app.rs (eframe::App)                         │
│  janela principal · lista de alarmes · editor (modal) · painel de     │
│  alarme tocando · configurações · seletor de arquivos                 │
└───────────┬───────────────┬───────────────┬──────────────┬──────────┘
            │               │               │              │
            ▼               ▼               ▼              ▼
      schedule.rs      storage.rs        audio.rs    device_volume.rs
      (puro, testável)  (JSON atômico)   (cpal)       (Core Audio COM)
            │               │               │              │
            ▼               ▼               ▼              ▼
       model.rs  ◄───────────┘               │              │
    (dados + serde)                          ▼              ▼
                                          tones.rs      winreg
                                     (síntese + WAV)  (iniciar com o Windows)

  ┌──────────────┐  ┌──────────────┐  ┌────────────────┐  ┌───────────┐
  │    ui.rs     │  │   hora.rs    │  │  arquivos.rs   │  │  owner    │
  │ design system│  │ digitação da │  │ seletor de    │  │ (decisões │
  │ (tokens,     │  │ hora (0730)  │  │ arquivos do   │  │  no log   │
  │  componentes)│  │              │  │ próprio app   │  │ oficial)  │
  └──────────────┘  └──────────────┘  └────────────────┘  └───────────┘

  ┌────────────────────────────────────────────────────────────────┐
  │            estado.rs — a ÚNICA cópia do estado                 │
  │  Estado { alarms, sonecas, settings } sob Mutex + Condvar     │
  │  Compartilhado = Arc<Mutex<Estado>> + Condvar + paradas        │
  └───────┬──────────────────────────────────────┬──────────────────┘
          │ lê e escreve                       │ lê e escreve
          ▼                                    ▼
   app.rs (interface)              thread do agendador (app.rs)
   lista · editor · painel          dorme até a hora · toca o som
                                   grava last_rung · re-tenta a cada 5 s
```

| Módulo | Responsabilidade única | Não faz |
|---|---|---|
| `model.rs` | Tipos de domínio e sua forma em JSON. | Regras de tempo; I/O. |
| `schedule.rs` | Quanto tempo falta para cada alarme disparar. Funções puras. | Não sabe o que é um alarme em tela; não dorme. |
| `estado.rs` | O estado compartilhado: despertadores, sonecas e ajustes, com trava e condvar. | Não decide a hora (isso é `schedule.rs`); não grava (isso é `storage.rs`). |
| `storage.rs` | Ler e gravar o arquivo de dados (gravação atômica, escrita serializada). | Não valida regra de negócio. |
| `tones.rs` | Gerar o áudio dos presets e ler WAV. | Não abre dispositivo. |
| `audio.rs` | Tocar em vários dispositivos ao mesmo tempo, com ganho e fade-in. | Não conhece a tela; não decide o que tocar. |
| `device_volume.rs` | Volume **do Windows** (o do mixer) e iniciar com o Windows. | Não controla o volume do app. |
| `ui.rs` | Tema dark e componentes desenhados à mão. | Não guarda estado do app. |
| `app.rs` | Telas, editor e a thread do agendador. | Não guarda cópia própria do estado; não fala com o hardware diretamente. |

## 4. Interfaces — contratos entre módulos

```rust
// schedule.rs — puro, sem relógio interno
pub fn proximo_disparo(alarme: &Alarm, depois: DateTime<Local>) -> Option<DateTime<Local>>;
pub fn proximo_disparo_global(alarms: &[Alarm], agora: DateTime<Local>) -> Option<(usize, DateTime<Local>)>;
pub fn atrasado_para_disparar(alarme: &Alarm, agora: DateTime<Local>, janela_min: i64) -> bool;
pub fn texto_ate(quando: DateTime<Local>, agora: DateTime<Local>) -> String;

// audio.rs
pub fn listar_saidas() -> Vec<Saida>;
pub fn tocar(escolha: &SoundChoice, nomes_alvo: &[String], ganho_final: u8, fade_in_s: u32)
    -> Result<Tocador, String>;
pub fn preparar_faixa(escolha: &SoundChoice, taxa: u32) -> Result<Faixa, String>;

// device_volume.rs
pub fn forcar_100(nomes: &[String]) -> Result<String, String>;  // devolve o resumo
pub fn restaurar(nomes: &[String]);
pub fn volume_atual(nome: &str) -> Option<f32>;
pub fn configurar_inicio_com_windows(ativar: bool) -> Result<(), String>;

// storage.rs
pub fn carregar() -> (Vec<Alarm>, Settings);
pub fn salvar(alarms: &[Alarm], settings: &Settings) -> Result<(), String>;
pub fn salvar_em(caminho: &Path, alarms: &[Alarm], settings: &Settings) -> Result<(), String>;

// estado.rs — a única cópia do estado, compartilhada por interface e agendador
impl Estado {
    pub fn todos(&self) -> Vec<Alarm>;                              // despertadores + sonecas
    pub fn proximo_disparo(&self, agora: DateTime<Local>) -> Option<(Alarm, DateTime<Local>)>;
    pub fn atrasado(&self, agora: DateTime<Local>, janela_min: i64) -> Option<Alarm>;
    pub fn por_id(&self, id: &str) -> Option<&Alarm>;
    pub fn marcar_tocado(&mut self, id: &str, hoje: &str);
}
impl Compartilhado {
    pub fn travar(&self) -> MutexGuard<'_, Estado>;                 // trava sem morrer
    pub fn notificar(&self);                                        // acorda o agendador
    pub fn pedir_parada(&self);                                     // dono dispensou
    pub fn salvar(&self) -> Result<(), String>;                     // só os despertadores do dono
    pub fn dispensar(&self, id: &str);                              // atendeu: não toca hoje
    pub fn adicionar_soneca(&self, soneca: Alarm, anterior_id: &str);
}
```

Regras que valem para todos:

- `schedule.rs` **nunca** chama `Local::now()` por conta própria — quem chama passa o "agora".
  Isso torna a lógica testável e determinística.
- `audio.rs` **nunca** decide o que tocar: recebe a escolha pronta (`SoundChoice`).
- `device_volume.rs` **nunca** mexe no áudio: só no volume do sistema e no registro do Windows.
- Quem chama `device_volume::forcar_100` guarda o valor anterior; `restaurar` devolve.
- `estado.rs` é a **única** cópia do estado. Interface e agendador leem e escrevem o mesmo
  `Arc<Compartilhado>`; ninguém guarda lista própria. A soneca só existe em memória
  (`Estado::sonecas`) e **não** entra no arquivo.
- Só o agendador grava `last_rung`, e **só depois** que alguma saída começou a tocar: quem sabe
  se o som saiu é quem disparou.

## 5. Diagramas

### 5.1 Sequência do disparo

O som **não** depende da interface: a thread do agendador é quem toca. Com a janela minimizada o
Windows nem desenha a tela, e o `App::update()` do `eframe` nem roda — se o som dependesse da
interface, o alarme silenciaria justo quando o dono dorme.

```
Thread do agendador        estado.rs        app.rs (UI)     audio.rs    device_volume.rs
      │                        │                   │              │                 │
      │ dorme no condvar       │                   │              │                 │
      ├───────────────────────>│                   │              │                 │
      │ (o estado mudou)       │                   │              │                 │
      │                        │                   │              │                 │
      │ acorda na hora         │                   │              │                 │
      ├───────────────────────>│                   │              │                 │
      │◄── alarme + horário ───┤                   │              │                 │
      │                        │                   │              │                 │
      │ forcar_100(saídas) ─────────────────────────────────────────────────────>│
      │◄────────────── "40% → 100% em 3 saídas" ───────────────────────────────┤
      │                        │                   │              │                 │
      │ tocar(som, saídas, 100%, fade) ───────────────────────>│                 │
      │                        │                   │  abre 1 stream por vez    │
      │                        │                   │  (ordem!) e toca em        │
      │                        │                   │  paralelo                 │
      │◄── started? ───────────────────────────────────────┤                 │
      │                        │                   │              │                 │
      │ se tocou: marcar_tocado + gravar last_rung        │              │                 │
      ├───────────────────────>│                   │              │                 │
      │ (evento no canal) ────────────────────────────>│              │                 │
      │                        │                   │ painel "DESPERTADOR"       │
      │                        │                   │ (tela cheia)               │
      │                        │                   │              │                 │
      │        (o dono clica "Parar" ou resolve o desafio)                    │
      │                        │◄── dispensar(id) ─┤              │                 │
      │◄── paradas mudou: para de insistir       │              │                 │
      │                        │                   │── stop ─────>│                 │
      │                        │                   │ restaurar(saídas) ─────────>│
```

Se o som **não** saiu, o agendador tenta de novo a cada 5 segundos por até uma hora, e o
`last_rung` **não** é gravado: o alarme continua pendente. Silenciar é pior do que repetir.

### 5.2 Estado do alarme tocando

```
        (nada)  ──disparo──>  TOCANDO  ──"Parar"──>  (nada)
                                │
                                └──"Soneca"──>  (nada) + alarme de soneca no estado
```

A soneca **não** entra no arquivo de dados: ela só vale para esta sessão e some quando o
app fecha. Mas ela **entra no estado compartilhado**, e por isso o agendador a vê e a toca.

### 5.3 Por que o estado é compartilhado (e não duas listas)

Antes do refactor, a interface tinha `alarms`/`sonecas` e a thread do agendador tinha as
**suas próprias** listas, trocadas por um canal. Isso desalinhou em três ways:

1. A soneca criada na interface **não existia** no agendador → ela nunca tocava.
2. O evento de disparo carregava um **índice**, e a interface indexava a lista combinada
   (despertadores + sonecas) → o índice apontava para o despertador errado.
3. O agendador gravava a **sua cópia** no arquivo → podia sobrescrever uma edição feita na
   interface (perda de dados).

A correção é estrutural, não um remendo: existe **um** `Arc<Compartilhado>` (mutex + condvar),
e os dois lados só consomem. O evento passou a carregar o **id** do despertador, não o índice.


## 6. Trade-offs (e por quê)

| Decisão | Alternativa descartada | Motivo |
|---|---|---|
| `egui` (nativo) | Tauri (HTML/CSS) | Sem navegador embarcado: menos RAM (~40 MB contra ~120 MB), menos dependências, build só com Rust. Em troca, a interface é desenhada à mão — mais trabalho, menos\emph{flexibilidade}. |
| Renderizador **glow** (OpenGL) | wgpu/DX12, que e o padrao do `eframe` | Medido nesta maquina: o wgpu consumia **364 MB e 99% de um core** com a janela aberta. O glow usa a mesma egui, com o mesmo visual, e consumiu **160 MB e ~0% de CPU**. Ver secao 7.4. |
| `cpal` | Reproduzir com `winsound`/PowerShell | `cpal` abre o stream por dispositivo e permiteganho atômico (fade-in). `winsound` só tocaria na saída padrão. |
| Volume do Windows via COM (`windows` crate) | Ajustar o volume "por dentro" do app | Só o volume do endpoint garante que o alarme não seja atenuado pelo mixer. Custo: uma dependência grande e código `unsafe`. |
| Agendador em thread com canal | `request_repaint` a cada segundo | CPU ociosa praticamente zero: a thread dorme até o próximo evento. A UI só repinta a cada 500 ms para o relógio. |
| JSON em `%APPDATA%` | Banco (SQLite) | Um arquivo, legível, que o dono pode conferir. SQLite é robustíssimo, mas pesado para meia dúzia de alarmes. |
| Sons sintetizados no app | Arquivos de áudio embutidos | Nada de material de terceiros dentro do app (licença) e zero bytes no repositório. |
| Sem ícone na bandeja | Ícone na bandeja | Menos código e menos dependências. Pending: o app continua rodando com a janela fechada, mas sem atalho rápido para reabrir. |

## 7. Decisões que a máquina do dono impôs

Estas não são preferências: são medições.

1. **Toolchain GNU.** O PC tem Visual Studio Community 2026, mas **sem o workload de C++**
   e **sem Windows SDK** — não há `link.exe` nem as bibliotecas do sistema. O toolchain
   `x86_64-pc-windows-gnu` do rustup resolve, mas exige `dlltool`, `as` e `ar` do MinGW-w64
   (WinLibs) no `PATH` para gerar as bibliotecas de importação.
2. **Streams de áudio um por vez.** O backend WASAPI recusa abrir três dispositivos ao
   mesmo tempo neste PC (`WdmSyncIoctl: DeviceIoControl GLE = 0x00000490`). O código abre
   os streams **sequencialmente** e só então toca em paralelo.
3. **COM na thread principal.** Toda chamada de Core Audio roda na thread da interface, com
   `CoInitializeEx` feito uma vez por thread. Nunca se passa objeto COM entre threads.
4. **Renderizador glow, nao wgpu.** O `eframe` usa wgpu/DX12 por padrao. Medido aqui, com a
   janela aberta: **364 MB de RAM e 99% de um core** de CPU, continuamente. Trocando para
   `glow` (OpenGL), o mesmo visual passou a **~160 MB e 0,0% a 1,3% de CPU**. A causa nao era
   o codigo do app, e o backend grafico. O `Cargo.toml` usa `default-features = false` com as
   features `glow`, `clipboard` e `default_fonts`.
   - O binario tambem encolheu de **33 MB para 18,7 MB** ao sair do wgpu.
   - A RAM que sobra e majoritariamente do driver: `atio6axx.dll` (driver OpenGL da AMD)
     ocupa **80,5 MB** do enderecamento do processo. O app sozinho, sem janela, consome
     **14 MB** de pico (medido com `--forcar-volume`, que nao abre a interface).
   - Remover `system_fonts` (que carrega as fontes do Windows inteiro) e `accesskit` ahorrou
     cerca de 8 MB. Os glifos fora do Latin-1 (`✕`, `−`, `▶`) foram trocados por
     equivalentes Latin-1 (`×`, `-`, `»`) para a fonte embutida da egui cobrir tudo.

## 7.4 Consumo medido

| Versao | RAM (janela aberta) | CPU ociosa | Binario |
|---|---|---|---|
| wgpu/DX12 (primeira versao) | 364 MB | 99% de um core | 33 MB |
| glow/OpenGL + sem system_fonts | ~160 MB | 0,0% a 1,3% | 18,7 MB |
| glow/OpenGL, app sem janela | 14 MB (pico) | - | - |

Medido com `Get-Process` em amostragens de 5 s, nesta maquina, com a janela visivel.
A CPU ociosa ficou medida, nao estimada: uma unica thread consumia 99% do core na versao com
wgpu, e o `cargo build --release` do `eframe` com `glow` eliminou essa thread.

## 8. Recomendações

**Agora (v1):** o que já está implementado — lista, editor, repetição, som, volume, fade-in,
soneca, desafio, múltiplas saídas, 100% no aparelho, iniciar com o Windows, próximo alarme.

**Depois (v2), em ordem de valor:**

1. **Ícone na bandeja do sistema** + menu com "Próximo alarme", "Testar som", "Sair" —
   é o que falta para o app virar ferramenta de mesa.
2. **Janela de padrão do Windows** (`--inicio` / `--fora`) para abrir minimized e para o app
   ser chamado por outra linha de comando.
3. **Execução como tarefa agendada do Windows** para o alarme tocar mesmo se o app não
   estiver aberto (hoje ele precisa estar rodando).
4. **Botão "adiar 5 minutos"** direto na notificação do Windows.
5. **Tema claro** além do dark.

**Não fazer:** sincronização em nuvem, contas, plugin de áudio. O app é de mesa e local.

## 9. Riscos

| Risco | Probabilidade | Impacto | Mitigação já feita / planejada |
|---|---|---|---|
| O app não estiver rodando na hora | média | alto | "Iniciar com o Windows" ligado por padrão + tarefa do Windows planejada (v2) |
| O PC estava dormendo na hora | alta | médio | Janela de tolerância de 15 min (`atrasado_para_disparar`) |
| O relógio do Windows mudar (fuso, NTP) | baixa | médio | O agendador recalcula a cada ciclo; `last_rung` impede disparo duplicado no mesmo dia |
| Loss de JSON na gravação | baixa | alto | Gravação atômica (`.tmp` + `rename`), **trava de escrita** (a interface e o agendador gravam o mesmo arquivo) e cópia do arquivo corrompido com outro nome |
| Som não sai em alguma saída | média | médio | O app mostra quais saídas estão tocando e o erro de cada uma, logo após o disparo |
| Volume do Windows não volta | baixa | médio | O valor anterior é salvo antes de forçar 100% e restaurado ao parar |
| `dlltool`/`as` sumirem do PATH ao compilar | média | baixo | O `scripts\build.ps1` do repositório configura o PATH do toolchain antes de compilar |
| UI travada com o alarme tocando | baixa | alto | O áudio roda em thread própria; a interface continua responsiva |
| Interface e agendador verem estados diferentes | **baixa** | **alto** | **Corrigido no refactor 9.6**: uma cópia só (`estado.rs`), evento por **id** e não por índice. Regressão coberta por teste (`a_soneca_entra_no_proximo_disparo`) **e** por E2E com clique real |
| A soneca tocar alguns segundos antes do pedido | alta | baixo | O modelo de dados guarda **hora e minuto** (sem segundos): a soneca de "9 min" cai no minuto cheio mais próximo. Limitação conhecida, igual em despertadores de mesa |

## 9.5 Quality gate (Dedé) — o que a revisão achou e o que foi corrigido

O **Dedé** (Hermes, perfil `dede`, skill `audit.development-review`) revisou o
repositório sem executar nada, como o contrato dele exige. Ele listou **15
defeitos (D1–D15)**, 5 riscos novos (R1–R5) e apontou o que não era
verificável pelo repositório. A parte que ele mesmo marcou como não
verificável foi medida de fora (pico de áudio, RAM/CPU) e está registrada na
seção 7.4.

O veredito dele foi: *a arquitetura está certa e o áudio foi pensado com
cuidado, mas o que está no repositório não sustenta "o alarme vai tocar quando
precisar"*. A distância entre as duas frases era o D1.

### Corrigidos nesta rodada

| ID | Defeito | Correção |
|---|---|---|
| D1 | Alarme marcado como tocado **antes** de se saber se tocou; sem nova tentativa | `last_rung` só é gravado depois que alguma saída **realmente** começou a tocar — e quem grava é a **thread do agendador** (`estado.rs::marcar_tocado`), que é quem sabe se o som saiu. Se o som falhar, o app fica na tela de alarme, mostra um aviso vermelho e **tenta de novo a cada 5 s** até o dono desativar |
| D4 | Sem fallback quando nenhuma saída escolhida existe | `app.rs::tentar_tocar` cai para a **saída padrão do Windows** antes de desistir: para um despertador, degradar é melhor que silenciar |
| D3 | Erro de stream invisível | O erro entra no aviso persistente da tela de alarme, junto com o erro de cada saída |
| D7 | "Sair ao fechar" era uma opção morta, e a interface prometia que o app continuava rodando | Fechar a janela agora **minimiza** o app (`ViewportCommand::Minimized(true)`) e o agendador continua. Para reabrir, o app tem **instância única** (`src/instancia.rs`): executar o app de novo traz a janela existente para o primeiro plano. A opção "encerrar o app" continua existindo e agora funciona de verdade. O padrão passou a ser **não encerrar** |
| D8 | Limite de sonecas nunca era alcançado | O alarme carrega `sonecas_usadas` (`model.rs::Alarm`) e a soneca herda o contador do disparo anterior (`app.rs::sonecar`) |
| D9 | "Uma vez" não podia ser rearmado pela interface | Botão **"Repetir amanhã"** no cartão do alarme que já tocou |
| D10 | Restauração de volume usava sempre o valor do primeiro alarme | `device_volume::restaurar` remove a entrada depois de restaurar, e o `on_exit` também restaura (R2) |
| D11 | Lista de saídas vazia forçava 100% em **todas** as saídas do PC | Lista vazia = **a saída padrão do Windows** |
| D12 | Chave de início automático desatualizada e sem modo minimizado | A chave `Run` passa a gravar `--minimizado`, e o app sobe escondido com o agendador rodando |
| D13 | `atrasado_para_disparar` ignorava a guarda de "uma vez" | A mesma guarda de `proximo_disparo` foi aplicada (com teste de regressão) |
| D14 | "Ouvir o som" ignorava o volume do editor | A prévia usa o volume do editor (`app.rs::testar_som(som, volume)`) |
| D15 | `BufferSize::Fixed(min)` sem plano B | `audio.rs::abrir_com_reserva` repete a abertura com `BufferSize::Default` se o driver recusar |
| D5 | WAV lido inteiro na memória, sem limite | Limite de **120 s** com aviso claro (`tones::DURACAO_MAXIMA_S`) |
| D6 | NaN do WAV float passava direto para o driver | Amostra não finita vira 0.0 |
| — | **WAV mono** deixava o 2º canal vazio e o callback indexava fora da faixa — pânico na thread de áudio | `tones.rs` monta um vetor por canal do WAV e o mono vira stereo de verdade; o callback usa `.get()` e nunca estoura. **Achado durante o gate, com teste de regressão** |

### Pendentes (conhecidos, documentados)

| ID | Pendência | Por que ficou |
|---|---|---|
| D2 | Saídas de áudio são pareadas **pelo nome**, não pelo id do endpoint | Trocar por id exigiria o `IMMDeviceId` do Core Audio bridado com o `cpal`, que não expõe o mesmo id. Mitigação: o app mostra o nome exato de cada saída e o dono confere. As três saídas `PG27QRT1B` do dono têm nomes distintos (`1 -`, `2 -`, `3 -`) e foram testadas separadamente |
| R5 | Se o Windows reiniciar durante a noite, o alarme só existe se o app estiver aberto | **Resolvido**: iniciar junto com o Windows (minimizado, com instância única) **e** ícone na bandeja do Windows. O app se promove sozinho para a faixa de notificação, então o dono não precisa fazer nada. Medido: ícone visível na faixa, clique esquerdo reabre a janela, menu com “Abrir o Tranal” e “Sair” funcionando |
| R6 | O ícone da bandeja sumir da faixa (o Windows 11 esconde ícone novo) | **Mitigado**: o app grava `IsPromoted = 1` na chave do ícone no registro e **confere** se o Windows aceitou (o Windows reescreve a chave e apagava a preferência). O `IconGuid` fixo, que resolveria a chave, foi **descartado** porque o Windows passa a guardar o ícone pela identidade e os cliques deixam de chegar no app |
| — | Teste de integração que abra um stream real e prove que saiu som | Hoje a prova é a medição externa de pico (`IAudioMeterInformation`), registrada em 7.4, e não um teste automatizado |

### O que o Dedé disse que não é verificável pelo repositório

- "20 testes passaram": não há artefato de saída versionado — o resultado vem de execução local.
- "binário de 18,7 MB": depende de execução, não de arquivo versionado.
- "pico 0,85 durante e 0,00 depois": o código não mede pico; a medição foi externa, por script.
- "~160 MB e 0,0–1,3% de CPU": sem artefato no repositório.

Todas essas afirmações foram **medidas de verdade** nesta máquina e estão
registradas na seção 7.4 e no segundo cérebro, mas o Dedé está certo: o
repositório, sozinho, não as prova.

## 9.6 Refactor do estado compartilhado (depois do gate)

O gate apontou que o `last_rung` era gravado antes de se saber se o som saiu. Corrigir isso
exigiu mexer na estrutura do app — e a mexida expôs um problema maior, que **ninguém** tinha
percebido porque a suíte de testes não cobria a interface:

**A interface e a thread do agendador tinham listas separadas.** Três consequências, todas
silenciosas:

1. **A soneca nunca tocava.** A soneca era criada na interface; o agendador calculava o
   próximo disparo só sobre `alarms`. O donut estava na tela, o aviso aparecia, e o som não
   saía.
2. **O índice do disparo apontava para o alarme errado.** O evento carregava o índice do
   agendador; a interface indexava a lista *combinada* (despertadores + sonecas).
3. **O agendador gravava a própria cópia no arquivo**, podendo sobrescrever uma edição feita
   na interface — perda de dados.

### O que mudou

| Antes | Depois |
|---|---|
| Duas listas (`app.rs` da interface e `app.rs` do agendador), trocadas por canal | **Um** `Arc<estado::Compartilhado>` = `Mutex<Estado>` + `Condvar` + contador de paradas. Interface e agendador **só consomem** |
| Canal de comandos `Reconfigurar(Vec<Alarm>, Vec<Alarm>, Settings)` | O agendador **dorme no condvar** e acorda quando o estado muda. Não existe mais "reconfigurar": o estado já é o mesmo |
| Evento `Disparo { indice }` | Evento `Disparo { id }` — o **id** do despertador, que não muda quando a lista muda |
| `last_rung` gravado pela interface | `last_rung` gravado pelo **agendador**, que é quem sabe se o som saiu; gravado **só quando tocou** |
| `sonecas` guardada na interface | `sonecas` no `Estado` — visível para o agendador, **sem** ir para o arquivo |
| `storage::salvar` sem proteção | `storage::salvar_em` com **trava de escrita** (mutex estático): a interface e o agendador gravam o mesmo arquivo e não podem se atropelar no `.tmp` |

### Módulos tocados

- **novo** `src/estado.rs` — `Estado` (alarms, sonecas, settings) e `Compartilhado`
  (mutex + condvar + contador de paradas). 6 testes.
- `src/storage.rs` reescrito — trava de escrita, gravação atômica, `salvar_em` para teste.
- `src/app.rs` reescrito — a interface lê e escreve o estado compartilhado; o agendador
  deixou de ter lista própria.

### Verificação

| Verificação | Resultado |
|---|---|
| `cargo test` | **36 passed; 0 failed** (era 28) |
| Alarme com o app **minimizado** | Disparou sozinho na hora marcada, **pico 1,0000**, e o `last_rung` foi gravado no arquivo |
| Soneca (clique real de mouse na tela de alarme) | O som **parou** (pico 0,0000), a soneca entrou na fila e **tocou de novo** (pico 1,0000, 53 s depois) |

### Revisão adversarial: o aviso perdido

Um achado da revisão do próprio refactor: o agendador soltaba a trava entre "qual é a próxima
hora" e "vou dormir". Se o dono mudasse o estado nesse intervalo, o aviso se perderia e o
agendador dormiria **até a hora antiga** — um alarme novo, mais cedo, só tocaria horas depois.

Correção: a trava fica presa do cálculo até a espera (`wait_timeout` já devolve a trava). O
padrão está travado por teste (`o_agendador_acorda_quando_o_estado_muda`).

### Bug de layout achado no mesmo round

A captura da tela de alarme mostrou a linha de botões colada na **esquerda** da janela. Causa:
em layout horizontal, o egui **força** o alinhamento à esquerda (`next_frame_ignore_wrap`
ignora o `main_align` e põe `Align::LEFT`), então `horizontal_centered` não centraliza o
grupo — centraliza os itens *dentro* da linha. Os botões foram **empilhados** e centralizados
(que é o que o layout vertical faz de verdade, como se vê nos rótulos).

## 10. Como compilar e rodar

```powershell
.\scripts\build.ps1          # release (o script ajusta o PATH do toolchain)
.\scripts\build.ps1 -Testes  # roda a suíte de testes
cargo run                    # debug
```

O binário fica em `target\release\tranal-despertador.exe` e é um executável único:
copiar para outro PC Windows 10/11 e funciona (depende só do Windows).

### Prova fim a fim (E2E)

`scripts\_e2e.py` não é um teste de unidade: ele **roda o binário de verdade** nesta máquina e
mede o **pico de áudio** pelo Core Audio (`IAudioMeterInformation`).

| Fase | O que faz | O que prova |
|---|---|---|
| `fase1` | Grava um alarme para o minuto cheio seguinte, sobe o app com `--minimizado` e mede o pico | O alarme dispara **sozinho, com a janela minimizada**, e o `last_rung` é gravado no arquivo |
| `fase2` | Mesmo alarme, app visível; **clica com o mouse de verdade** no botão "Soneca" (o botão é localizado pela cor na captura da tela) e mede o pico de novo | A soneca **para o som**, entra na fila e **toca de novo** |

O limiar de detecção é **pico > 0,30 em três amostras seguidas** — abaixo disso é ruído de
outro programa (notificação, chat), não o despertador. O que prova que o som saiu é o pico
do endpoint, não o "tocando" do aplicativo.

## 11. Redesenho da interface (rodada do owner)

O owner pediu: *"achei feio e grosseiro"*. A rodada trocou a interface por uma
versão com **sistema de design em casa**, editor como **diálogo central**, **horário
digitável**, números **inteiros** e a biblioteca de sons do dono.

### 11.1 O que mudou

| Antes | Agora | Por quê |
|---|---|---|
| Cores soltas em `app.rs` | `src/ui.rs` com **tokens** (espaçamento, raios, tipografia, cores semânticas) e componentes prontos | Uma só fonte de verdade visual; as cores foram conferidas contra o fundo pela **WCAG AA** |
| Editor em painel lateral apertado (520 px de largura) | Editor como **modal central** de 600 px, com o conteúdo rolando e o rodapé fixo | O dono achou a tela "grosseira"; o painel lateral espremia os campos |
| Só botões `+1h / +5m` para a hora | Campo de hora **digitável** (grande, monoespaçado) **além** dos botões | "Permita que eu possa digitar por cima do horário" |
| Slider de `f32` mostrando `3.0 vezes` | Slider de `i32`: mostra **`3 vezes`** | O egui calcula as casas decimais pelo passo do slider; com `f32` ele mostrava "3.0" |
| Botões de opção desenhados à mão em cada tela | **Seletor segmentado** (`ui::seletor_segmentado`) e **seletor múltiplo** (dias da semana) | Consistência e menos código repetido |
| Som: só presets + "Usar um WAV meu" | Presets **+ biblioteca de sons do dono** (WAV, MP3, FLAC, OGG) + importador | "Permita que eu suba áudios para a ferramenta" |
| Explorador do Windows para escolher o arquivo (`rfd`) | **Seletor dentro do app** (`src/arquivos.rs`) | O `IFileDialog` do Windows demorava demais para abrir; o app lista os arquivos ele mesmo |
| `Settings.som_padrao` e `Settings.volume_padrao` (que **ninguém consumia**) | `Settings.alarme_padrao`: um `Alarm` modelado nas Configurações, que é a **fonte única** do que é um despertador | Uma só interpretação canônica; o dono pediu "um despertador padrão" |
| `device_volume::volume_atual()` chamado **uma vez por saída** | `device_volume::saidas_com_volume()`: **uma** enumeração do Core Audio | Com 12 saídas eram 12 enumerações COM: o editor demorava uma eternidade para abrir |

### 11.2 Módulos novos

- **`src/ui.rs`** — sistema de design: paleta com contraste conferido, escala de
  espaçamento (4/8/12/16), raios, tipografia e os componentes (cartão, botão
  primário/neutro/fantasma/perigo, chip, interruptor, campo de texto, campo de
  hora, seletor segmentado, seletor múltiplo, slider inteiro).
  Duas variações de laranja e vermelho: uma **clara** para texto e bordas
  (8:1 no fundo) e outra **escura** para preenchimento de botão, para o texto
  branco em cima dela chegar a 4.5:1 (AA).
- **`src/hora.rs`** — interpretação do que foi digitado: `0730`, `07:30` e `730`
  valem 07:30; `129` vira 01:29. 6 testes.
- **`src/arquivos.rs`** — seletor de arquivos interno (modal com unidades,
  atalhos, filtro por nome e lista). 3 testes.

### 11.3 O que o dono ficou sabendo na tela

- **Dias úteis × Semanal**: "Dias úteis = de segunda a sexta, todo dia. Não
  precisa marcar nada." e "Semanal = só nos dias que você marcar abaixo."
- **Onde tocar vazio**: "Nada marcado: o alarme toca na **saída padrão do
  Windows** (a que o Windows usa). Marque mais saídas para tocar em várias ao
  mesmo tempo."
- **Despertador padrão**: nas Configurações, com o texto "É o que já vem
  preenchido quando você clica em 'Novo despertador'".

### 11.4 Decisões de meio de caminho

- **Symphonia no lugar do parser de WAV caseiro**: uma via só para WAV, MP3,
  FLAC e OGG. A feature `pcm` é obrigatória — sem ela o WAV PCM nem é registrado
  e o arquivo falha com `unsupported audio codec`.
- **Crate de tema de terceiros descartado**: o `modern-egui` (tema para egui
  0.36) exigiria downgrade do `eframe` 0.37. O design system foi construído em
  casa.
- **`rfd` removido** das dependências: o seletor é do app agora.
- **Áudio do dono é copiado**, não referenciado: o despertador continua
  tocando mesmo que o arquivo original seja movido ou apagado. Se o arquivo já
  está na pasta do app, ele **não** é copiado de novo.

### 11.5 Verificação

| Verificação | Resultado |
|---|---|
| `cargo test` | **61 passed; 0 failed** (era 36) |
| `cargo clippy --all-targets -- -D warnings` | **limpo** (o clippy nem estava instalado no toolchain: instalado nesta rodada) |
| Digitação da hora (teclado de verdade) | `0630` → campo vira `06:30` ao perder o foco |
| Digitação **por cima** (teste headless do egui) | Com o campo focado e o texto todo selecionado, digitar `0` em `07:00` deixa **`0`** (substituiu); sem a seleção seria `07:000` |
| Importação de MP3 | Arquivo escolhido no seletor → aparece em "meus áudios", com a duração lida (1,0 s) |
| Alarme com o app **minimizado** | Disparou sozinho, **pico 1,0000**, `last_rung` gravado |
| Soneca (clique real de mouse) | O som **parou** (pico 0,0032) e a soneca **tocou de novo** (pico 1,0000, 53 s depois) |

> O detector de botão do E2E procurava a cor antiga do botão vermelho
> (`232,76,76`). Com o redesenho (`198,40,40`) ele achava só um pixel e o
> clique caía no espaço entre os botões — o teste falhava por causa do teste,
> não do app. Corrigido no `scripts/e2e.py`.

### 11.6 Correções depois do redesenho

Duas correções nasceram da revisão do próprio código (o quality gate externo
não rodou — ver 11.7):

1. **As Configurações gravavam o arquivo de dados a cada quadro.** O `dados.json`
   era reescrito 60 vezes por segundo só por o dono olhar a tela. Agora só
   grava quando o valor **muda** (`settings != estado.settings`), conferido pelo
   `mtime` do arquivo: olhar não grava; alternar grava; voltar ao valor
   original volta a gravar.
2. **Digitação por cima do horário.** Ao ganhar o foco, o campo de hora
   seleciona o texto inteiro (`ui::selecionar_todo_ao_ganhar_foco`), então o
   primeiro dígito já substitui o horário anterior — sem apagar antes. Vale no
   editor do despertador e no despertador padrão das Configurações.

### 11.7 Quality gate (Dedé) desta rodada

O quality gate é o **Dedé** (`hermes -p dede chat --query-file ... --oneshot`).
As duas primeiras tentativas **não rodaram**: o provedor do Hermes ficou
indisponível ("Provider temporarily unavailable — retrying") em cinco ciclos, e
o fallback (`qwen3:14b` via Ollama) também deu "Connection error". Na terceira
tentativa o provedor voltou e o gate rodou: **PASS_WITH_WARNING**, com os
**8 critérios de aceitação todos atendidos**.

**O que o Dedé apontou, e o que foi feito**

| Achado (severidade) | O que foi feito |
|---|---|
| symphonia (MPL-2.0) pode conflitar com GPL/AGPL (MÉDIA) | **Auditoria feita** com `cargo tree` (251 dependências): nenhuma GPL/AGPL-only; a única dupla é `self_cell` (Apache-2.0 OU GPL-2.0-only), consumida sob a opção permissiva. Inventário completo no [`README.md`](../README.md#licenças) e em `docs/decisoes/README.md` |
| Enumeração de saídas por nome, e não por id (BAIXA) | **Volume salvo e devolvido pelo id do endpoint** (`IMMDevice::GetId`), e não pelo nome: duas saídas com o mesmo nome não se atrapalham mais e nenhuma fica presa em 100%. A *escolha* do dono continua pelo nome (é o que o mixer mostra) e o resumo avisa quando o nome se repete. Risco D2 passa de "pendente" para "mitigado" |
| `Alarm::a_partir_do_padrao` ignora `id` e histórico (MÉDIA) | **Não é perda, é intenção** — e agora está escrito: o `id` tem que ser novo (o padrão é só um modelo) e o histórico não vem porque o alarme nasceria "já tocou hoje" com as sonecas contadas. Novo teste prova que **todos** os metadados (nome, hora, repetição, som, volume, fade, força do volume, saídas e política de desligar) chegam inteiros |
| Aviso: `tests/audio_mod.rs` não cobre áudio corrompido | **Teste novo**: um MP3 com cabeçalho ID3 e conteúdo lixo tem que dar **erro claro**, sem derrubar o app |
| Aviso: R5 (ícone na bandeja) não verificável no código | Segue **pendente e documentado** — precisa de teste manual do dono |

**O que o gate não substitui:** a revisão adversarial do próprio executor
sobre o diff e o comportamento medido, o **E2E real** (som e soneca medidos no
Core Audio) e as capturas de tela continuam valendo.

O pedido ao Dedé está versionado em
`docs/arquitetura/pedido-dede-quality-gate-rodada-interface.md`: quando o
provedor cair de novo, é só rodar o comando de novo.

## 12. Bandeja do Windows e instalador

O dono perguntou se o projeto já tinha **instalador**, se o app **salva no
lugar definido**, se os despertadores **sobrevivem a um reinício do Windows**
e se ele **fica na bandeja**. A resposta honesta era: o app sobrevivia a um
reinício (a chave de início automático existia), mas **não tinha instalador
nem ícone na bandeja**. Esta seção é o registro do que foi feito.

### 12.1 O ícone na bandeja (`src/tray.rs`)

O ícone é criado com a biblioteca `tray-icon` (que não depende de `winit` e
não conflita com a versão de `windows-sys` do projeto). O desenho é feito em
código (um despertador laranja de 64×64, em `src/icone.rs`), sem arquivo de
imagem no repositório.

Regras do Windows que a implementação respeita:

| Regra | Como foi resolvida |
|---|---|
| O ícone tem que ser criado **na thread com o event loop** | A criação acontece no primeiro quadro da interface (`app.rs`), ou seja, na thread principal |
| Os **eventos** podem ser lidos de outra thread | A thread `tranal-bandeja` só lê eventos e fala com a interface; nada de som |
| O ícone precisa ficar **vivo** enquanto o app vive | O `TrayIcon` é guardado em `TranalApp.bandeja` (se for solto, o Windows remove o ícone) |

### 12.2 O menu da bandeja (`src/tray_menu.rs`) — e por que é próprio

O menu pronto da biblioteca de bandeja é ancorado **na posição do mouse**. O
mouse está dentro da barra de tarefas (é de lá que o clique veio), e o Windows
não sabe que a barra cobre aquela área: o menu nasce com a base *dentro* da
barra e o último item ("Sair") fica **atrás** dela, invisível.

Foi medido: o popup (classe `#32768`) media 59 px de altura, com a base em
`y=1416` — e a barra de tarefas do monitor comeca em `y=1393`. Os 23 px de
baixo (o item "Sair") ficavam atrás da barra.

A solução é um menu do Windows feito à mão (`CreatePopupMenu` +
`TrackPopupMenuEx`), ancorado na **borda inferior da área de trabalho** do
monitor (`GetMonitorInfo` → `rcWork.bottom`), com a base do menu apoiada nela:
o menu abre para cima e nenhum item fica escondido. Medido depois da
correção: popup em `(2193,1333)-(2362,1392)`, base em `1392` — **acima** da
barra.

Um segundo detalhe: um menu do Windows pertence à thread que está em
primeiro plano. Aberto de outra thread, ele pisca e some. Por isso o pedido
do menu vai para a thread principal por mensagem (`PostMessage` para uma
janela dona, invisível, criada na thread principal) e é ela que abre o menu.

### 12.3 A promoção automática do ícone

O Windows 11 esconde, por padrão, o ícone recém-criado: ele vai para a lista
de "ícones ocultos" (o `^` do lado direito). O dono teria de promovê-lo à mão
em Configurações → Barra de tarefas → Outros ícones da bandeja — e o caminho
muda a cada instalação.

O app faz isso sozinho: grava `IsPromoted = 1` na chave do ícone em
`HKCU\Control Panel\NotifyIconSettings`, procurada pelo caminho **exato** do
executável que está rodando. A gravação é tentada de novo por alguns
segundos (o Windows escreve a chave depois de criar o ícone) e o resultado
fica no log (`%APPDATA%\Tranal Despertador\tray.log`).

**Um `IconGuid` fixo foi tentado e descartado.** A ideia era amarrar a
preferência "sempre mostrar" à identidade do ícone, para ela sobreviver a uma
reinstalação em outro caminho. Medido nesta máquina: com `IconGuid`, o
Windows passa a guardar o ícone pela identidade e **os cliques deixam de
chegar no app** (o Windows continua chamando a janela do registro antigo).
Sem o GUID, os cliques funcionam — e a promoção continua garantida pelo
próprio app, a cada arranque.

### 12.4 O instalador (`scripts/instalar.ps1`)

Instalação **por usuário**: nada de administrador, nada em "Program Files".

| Passo | O que acontece |
|---|---|
| 1 | Compila a versão release (ou reaproveita a que já existe, com `-NaoCompilar`) |
| 2 | Fecha o app que estiver rodando (o arquivo não pode ser copiado aberto) |
| 3 | Copia o executável para `%LOCALAPPDATA%\Programs\Tranal Despertador` e grava o `.ico` dos atalhos (`--exportar-icone`) |
| 4 | Cria o atalho do Menu Iniciar (sempre) e o da Área de trabalho (opcional) |
| 5 | Grava a chave de início automático apontando para o executável **instalado**, com `--minimizado` |
| 6 | Abre o app instalado |

`-Desinstalar` remove os atalhos, a chave de início automático e a pasta do
programa. **Os despertadores e os sons do dono são preservados** (eles ficam
em `%APPDATA%\Tranal Despertador`); só `-ApagarDados` remove esses também.

O ícone dos atalhos é exportado pelo próprio app (`--exportar-icone`), e não
por um `.ico` versionado: o desenho do ícone é o mesmo da bandeja, escrito em
uma só lugar (`src/icone.rs`).

### 12.5 O que foi medido nesta máquina

| Prova | Resultado |
|---|---|
| Ícone na faixa de notificação | Despertador laranja visível na faixa do monitor 1, **promovido sozinho** pelo app |
| Clique esquerdo no ícone | Janela restaurada (`IsIconic` 1 → 0) |
| Clique direito no ícone | Menu abre; popup com base em `1392`, **acima** da barra de tarefas |
| "Abrir o Tranal" | Janela minimizada → restaurada pelo menu |
| "Sair" | App encerrado (o processo some) |
| Controle negativo | Clique numa área vazia da barra **não** restaura a janela (o efeito vem do ícone, não do clique no vazio) |
| Ícones promoted | `IsPromoted = 1` gravado pelo app, sem passo manual do dono |
| Instalador rodado no PC do dono | App rodando do caminho instalado (pid 26848, ~157 MB), atalhos com o ícone, chave `Run` apontando para o exe instalado, `IsPromoted = 1` na chave nova e **dados do dono intactos** (alarme `al8a262d2c` às 07:00, `last_rung` null, `iniciar_com_o_windows=true`) |

### 12.6 Quality gate (Dedé) desta rodada

O quality gate é o **Dedé** (`hermes -p dede chat --query-file ... --oneshot`), e
rodou de primeira: **PASS**, com os **12 critérios de aceitação todos
atendidos**, `required_actions` vazio e `warnings` vazio.

Um único achado, de severidade **LOW**: a sessão de desktop do owner ficou
**não interativa** no fim da rodada (o `SetCursorPos` parou de mover o mouse e a
resolução alternou entre 5120x1441 e 4160x1441), o que impediu a
re-verificação **por clique** do menu **no app instalado**. A recomendação do
Dedé é confirmar com o owner se a verificação via código (sem interação
manual) basta para validar o menu na instalação.

O que foi feito sobre esse limite:

- o comportamento do menu foi provado com o **mesmo código**, rodando de
  `D:\tranal-despertador\target\release` (mesmo binário, outro caminho) —
  inclusive o controle negativo (clique numa área vazia da barra **não**
  restaura a janela);
- no app **instalado** foram conferidos o ícone visível na faixa, o
  `IsPromoted = 1` na chave do caminho novo, a chave `Run`, os atalhos e os
  dados intactos.

O pedido ao Dedé está versionado em
`docs/arquitetura/pedido-dede-quality-gate-rodada-bandeja.md`: quando o
provedor cair de novo, é só rodar o comando de novo.

## 13. O layout da tela principal (referência: Tranal Sentinel)

O owner reclamou: *"o layuout do app ainda é muito feio, esse botão novo
despertador e as informações estão grudadas na borda do app sem nenhum
espaçamento, acho que podemos usar o layout do app tranal sentinel como
inspiração."*

O diagnóstico era exato e medido: o `CentralPanel` era desenhado com
`inner_margin = 0`, então **todo** o conteúdo encostava nas bordas da janela —
o cartão do relógio na esquerda, o título da lista na direita e o botão
"+ Novo despertador" solto embaixo do cartão, sem respiro nenhum.

### 13.1 O que foi adotado do Tranal Sentinel

O Sentinel (`E:\projetos\tranal-sentinel`, WPF/.NET) é a referência de layout
da casa. O que foi trazido:

| Do Sentinel | Aqui |
|---|---|
| `CardBorder` com `Padding 20` | `ui::cartao()` com 20 px de respiro interno (era 18×16) |
| Margem de página em volta do conteúdo | `ui::MARGEM_PAGINA` (32 px) e `MARGEM_PAGINA_VERTICAL` (24 px) — ver §14 |
| Barra de título fina (`Row 44`) com divisória | `topo()` virou uma faixa fina com linha divisória embaixo |
| `H1` 26 semi-bold, `Hero` 34, `Label` 10,5, `Sub` 13 | `ESTILO_TITULO`, `ESTILO_HERO`, `ESTILO_ROTULO`, `ESTILO_SUB` |
| `PrimaryButton` com padding largo (28×11) | `botao_primario_grande` na linha do título da página |
| Números em fonte monoespaçada | Relógio e hora do próximo alarme em monoespaçada |

**O que não foi adotado, de propósito:**

- **A paleta.** O Sentinel é monocromático; o Tranal Despertador tem laranja
  como acento, já validado na WCAG AA (7,1:1 no texto e 5,2:1 no
  preenchimento de botão) e aprovado pelo owner. A *estrutura* do layout foi
  adotada, não a paleta — trocar a cor mudaria a identidade do app sem
  resolver a reclamação (que era de espaçamento).
- **A barra lateral de navegação.** O Sentinel tem seis abas; o Despertador
  tem uma tela principal e duas Configurações. Uma barra lateral com dois
  itens seria ruído, não util.

### 13.2 A estrutura nova

```
┌──────────────────────────────────────────────────────────┐
│ Tranal Despertador                        [Configurações]│  <- faixa fina (topo)
├──────────────────────────────────────────────────────────┤
│  (24 px de margem de página)                              │
│  Despertadores                        [+ Novo despertador]│  <- cabeçalho
│  1 ativo · próximo às 07:00                              │
│                                                          │
│  ┌────────────────┐   ┌──────────────────────────────┐  │
│  │ 18:54          │   │ 07:00  Despertador      [ON] │  │
│  │ quarta-feira…  │   │ [Uma vez][Onda][100%][desafio]│  │
│  │ ─────────────  │   │        Editar  Testar  ×      │  │
│  │ PRÓXIMO        │   └──────────────────────────────┘  │
│  │ 07:00          │                                     │
│  │ Despertador    │                                     │
│  │ [Uma vez]      │                                     │
│  │ ─────────────  │                                     │
│  │ dica (dentro)  │                                     │
│  └────────────────┘                                     │
└──────────────────────────────────────────────────────────┘
```

O botão "+ Novo despertador" saiu de baixo do cartão do relógio e foi para a
linha do título da página: era ele que o owner via "grudado na borda". A dica
("Fechar a janela minimiza o app…") foi para **dentro** do cartão, com
separador — antes ela era um texto solto na borda de baixo da janela.

### 13.3 Três armadilhas do egui que custaram tempo (e ficam registradas)

1. **`available_height()` dentro de um `horizontal` devolve 0** no primeiro
   quadro. Com a altura tirada de dentro do `horizontal_top`, **as duas
   colunas sumiram da tela**. A altura agora é calculada **antes** do
   `horizontal` e passada por parâmetro.
2. **O `Align` do egui alinha no eixo transversal do layout.** Num layout
   horizontal, `Align::Center` centraliza **verticalmente**: o botão do
   cabeçalho descia para o meio da janela. A correção é `Align::Min` (topo).
3. **`PrintWindow` com `PW_RENDERFULLCONTENT` sai 1:1 com o
   `GetWindowRect`.** Somar a borda da janela e a barra de título no clique
   deslocava o clique em ~39 px, e o botão "parecia quebrado". Era erro do
   **script de teste**, não do app — e o mesmo tipo de erro (ler a posição do
   mouse no lugar errado) já tinha acontecido antes com o `mouseData >> 16`.

### 13.4 Verificação

- `cargo test`: **72 verdes**; `cargo clippy --all-targets -- -D warnings`:
  limpo; `cargo build --release`: OK.
- Capturas do **app instalado**
  (`%LOCALAPPDATA%\Programs\Tranal Despertador\tranal-despertador.exe`):
  `docs/captura-layout-v3.png` (tela principal), `docs/captura-editor-v3.png`
  (editor) e `docs/captura-config-v3.png` (Configurações). O "antes" está em
  `docs/captura-interface-nova.png`.
- **Cliques reais** no app rodando: "Configurações" abre, "Novo despertador"
  abre o editor, "Fechar" fecha o modal, "Fechar configurações" fecha, "Editar"
  abre o editor do alarme existente.
- Nenhum dado do owner foi mexido: o `last_rung = 2026-11-18` é o alarme das
  07:00 **tocando de verdade** hoje, e o card passou a oferecer "Repetir
  amanhã" — rearmar é decisão do owner.

### 13.5 Quality gate (Dedé) desta rodada

O **Dedé** (`hermes -p dede chat --query-file ... --oneshot --in D:\project-ai`)
rodou de primeira: **PASS**, os **12 critérios de aceitação todos atendidos**,
`required_actions` vazio e `warnings` vazio.

Um único achado, de severidade **LOW**:

> *"O relatório menciona corrigir o slider para mostrar '100%' e '12s', mas não
> há verificação visual da mudança"* — recomendação: *"Incluir uma captura do
> slider atualizado ou confirmar que o código reflete a alteração"*.

Resposta: a captura **`docs/captura-config-v3.png`**, tirada do app
instalado, mostra o slider do volume com **"100%"** (sem o espaço, era
"100 %"). O outro caso ("12 s" → "12s") é o slider do *fade-in*, que só
aparece com valor acima de zero — a correção está no código
(`src/app.rs`, sufixo `"s"` no lugar de `" s"`), no mesmo lugar do `"%"`.

O pedido ao Dedé está versionado em
`docs/arquitetura/pedido-dede-quality-gate-rodada-layout.md`.

## 14. O refactor do layout (rodada "100% do layout")

O owner reclamou, com a captura da tela na mão:

> *"os botões de repetir, editar e testar sobrepõe as informações do despertador,
> o botão novo despertador completamente fora de posição com o texto sem
> centralizar. Acho que precisamos usar as skills de UX e UI disponíveis neste
> computador para refatorar 100% do layout desse sistema, baseando-se no layout
> do Tranal Sentinel"*.

As skills usadas nesta rodada foram a **UI Designer**
(`design-ui-designer/skill.md`) e a **UX Architect**
(`design-ux-architect/skill.md`), mais o roteiro de revisão de interface do
Projeto IA (`skills/ui-ux/interface-review.yaml`): usabilidade →
acessibilidade → responsividade → consistência.

### 14.1 As três causas-raiz (medidas, não deduzidas)

| Defeito visto pelo owner | Causa real (lida na fonte do egui 0.37) |
|---|---|
| Botões por cima das etiquetas | `cartao_alarme()` colocava as etiquetas (`horizontal_wrapped`) e os botões (`with_layout(right_to_left)`) **na mesma linha**. O `with_layout` cria um escopo que ocupa **todo** o espaço do pai: como as etiquetas já tinham tomado a largura toda, o escopo dos botões ficou com ~0 px e os botões foram desenhados **por cima** delas |
| "+ Novo despertador" fora de posição e com o texto torto | O mesmo `with_layout`: em `Layout::right_to_left`, o `main_align` é `Center` (o alinhamento principal é ignorado no eixo da linha) e o `cross_align` é o vertical — o botão ficava **centralizado** no espaço restante, não na borda. E o texto dentro do botão é posicionado pelo sistema de **atoms** do egui 0.37, que não garante centralização |
| Lista encostando na borda direita | A coluna da lista era desenhada com a largura toda do pai (`available_width()`), sem respiro próprio; com a margem de página pequena (24 px) e o scrollbar por fora, o cartão chegava a ~22 px da borda da janela |

### 14.2 O design system refeito (`src/ui.rs`)

**Um componente só de botão**, desenhado à mão (como o chip e o interruptor já
eram), em vez do `egui::Button` do 0.37:

- `EstiloBotao::{Primario, Neutro, Fantasma, Perigo, Icone}` — mesma forma,
  mesmo respiro, mesmo texto centralizado em qualquer tela;
- medidas tiradas do Sentinel: botão principal com `Padding 28,11` e canto de
  10 px, cartão com `Padding 20` e canto de 16 px, etiqueta com `Padding 13,5`
  e canto de 14 px **fundo transparente com borda de 1 px** (o chip do
  Sentinel), interruptor de 46×26;
- **a largura do botão é medida antes de desenhá-lo**
  (`ui::largura_botao`), com a mesma fonte com que o texto vai ser pintado —
  é o que permite alinhar um grupo de botões na borda direita;
- `ui::acoes_a_direita` desenha um grupo de botões encostado na borda
  direita: mede cada botão, reserva o espaço que sobra com `add_space` e
  desenha. **Sem `with_layout`**, que é a origem do defeito;
- `ui::empurra_para_direita` para quando o item da direita tem largura
  conhecida (o interruptor do cartão).

Outros ajustes: margem de página **32 px** (era 24) e **24 px** na vertical
(era 20) — o Sentinel usa 36/30 na página; relógio em 48 px (era 56) para
competir com a hora do alarme (34 px); o seletor segmentado e o múltiplo
passaram a **descontar o espaço entre os segmentos** da largura (eles
estouravam a borda e esticavam o painel que os contém).

### 14.3 O cartão do despertador

Antes as etiquetas e os botões disputavam a mesma linha. Agora são **três
linhas**:

```
┌────────────────────────────────────────────────────────┐
│ 07:00  Café da manhã                          [ ●─── ]  │  linha 1: hora, nome, interruptor
│ [Todo dia][Clássico][100% no aparelho][desafio]        │  linha 2: as etiquetas, só elas
│                              Editar  Testar  ×         │  linha 3: as ações, à direita
└────────────────────────────────────────────────────────┘
```

### 14.4 O cabeçalho da página

O H1 e o subtítulo à esquerda, a ação principal à direita — o mesmo arranjo
do Sentinel (`Grid` com `H1`+`Sub` à esquerda e `OutlineButton` com
`HorizontalAlignment=Right`). O botão fica **centrado na altura do H1**: o
egui alinha as *caixas*, e a caixa da linha tem o ascendente da fonte acima da
letra maiúscula; centralizar na linha é o que põe o texto do botão na mesma
altura das letras do título.

A barra do topo ganhou a forma do Sentinel: `TRANAL` forte + `DESPERTADOR`
fraco + o nome da página, e a ação à direita.

### 14.5 Armadilhas do egui 0.37 que ficam registradas

1. **`with_layout` ocupa todo o espaço do pai** — por isso ele nunca mais é
   usado para alinhar alguma coisa à direita.
2. **`Layout::right_to_left` centraliza no eixo principal** — o
   `main_align` é `Center` e só o eixo transversal é controlado pelo
   argumento. Alinhar à direita com ele é enganoso.
3. **O `Button` do 0.37 posiciona o texto pelo sistema de atoms** — por isso
   os botões são desenhados à mão.
4. **`available_height()` dentro de `horizontal` = 0** (continua valendo).
5. **O `seletor_segmentado` não descontava o espaço entre os segmentos** — o
   conteúdo passava da borda e o `ScrollArea` do editor esticava o modal
   em 35 px (achado medindo as capturas, não adivinhando).

### 14.6 Verificação

- `cargo test`: **74 verdes** (2 novos, que medem o defeito);
  `cargo clippy --all-targets -- -D warnings`: limpo;
  `cargo build --release`: OK.
- **Dois testes novos medem o defeito** (`src/ui.rs`):
  - `o_texto_do_botao_fica_centralizado` — o retângulo do texto desenhado
    tem que ter o mesmo centro do retângulo do botão (tolerância de 1 px);
  - `o_grupo_de_botoes_para_na_direita_sem_invadir_o_conteudo` — o grupo de
    botões encosta na borda direita da linha (1 px) e **não invade** a
    etiqueta que vem antes.
- Capturas: `docs/captura-layout-v4.png` (tela principal com dois
  despertadores), `docs/captura-editor-v4.png`, `docs/captura-config-v4.png`
  e `docs/captura-instalado-v4.png` (o app instalado, com os dados reais do
  owner: o card das 07:00 com "Repetir amanhã", "Editar", "Testar" e "×" na
  linha de baixo, **sem sobrepor** as etiquetas).
- Nenhum dado do owner foi mexido: o `last_rung = 2026-11-18` continua lá, e
  rearmar o alarme de "uma vez" segue sendo decisão do owner.
