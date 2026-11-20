//! Sistema de design do Tranal Despertador.
//!
//! Tudo aqui é desenhado à mão (sem CSS, sem navegador): o app é uma janela
//! nativa, e o objetivo é ficar bonito sem pesar na memória.
//!
//! A referência de **estrutura** é o Tranal Sentinel (WPF): barra de topo
//! fina, cabeçalho de página com o H1 à esquerda e a ação principal à
//! direita, cartões com 20 px de respiro e cantos de 16 px, e a hierarquia
//! tipográfica H1 26 / Hero 34 / Label 10,5 / Sub 13. A paleta continua
//! sendo a do app (o laranja é a identidade do Tranal), mas segue a regra de
//! contraste da WCAG (AA): texto normal precisa de 4,5:1, texto grande de
//! 3:1. Por isso existem duas variações do laranja e do vermelho: uma para
//! **texto e bordas** (clara, 7:1 ou mais no fundo) e outra para
//! **preenchimento de botão** (escura, para o texto branco em cima dela
//! chegar a 4,5:1).
//!
//! Duas lições do egui 0.37 estão gravadas aqui, porque as duas custaram
//! uma rodada de depuração:
//!
//! 1. `ui.with_layout(...)` cria um escopo que ocupa **todo** o espaço do
//!    pai. Por isso, um grupo de botões alinhado à direita com `with_layout`
//!    acaba desenhado **por cima** do conteúdo que vem antes (era o que
//!    happening com os botões do cartão do despertador). Aqui o grupo é
//!    medido antes e o espaço que sobra é reservado com `add_space`.
//! 2. O `egui::Button` do 0.37 posiciona o texto pelo sistema de atoms, e
//!    o resultado não é o mesmo em todas as situações. Por isso os botões
//!    aqui são **desenhados à mão** (como o chip e o interruptor): o texto
//!    fica sempre centralizado no retângulo.

use egui::{Align2, Color32, CornerRadius, FontId, Response, Sense, Stroke, Ui, Vec2};

/// Paleta do app.
pub struct Tema;

impl Tema {
    /// Fundo da janela (Sentinel Bg #000).
    pub const FUNDO: Color32 = Color32::from_rgb(0, 0, 0);
    /// Fundo dos cartões e painéis (Sentinel Card #0E0E0E).
    pub const PAINEL: Color32 = Color32::from_rgb(14, 14, 14);
    /// Fundo dos controles (Sentinel Hover #181818).
    pub const PAINEL_ALTO: Color32 = Color32::from_rgb(24, 24, 24);
    /// Borda discreta (Sentinel Line #1F1F1F).
    pub const BORDA: Color32 = Color32::from_rgb(31, 31, 31);
    /// Borda de destaque (Sentinel LineStrong #3A3A3A).
    pub const BORDA_FORTE: Color32 = Color32::from_rgb(58, 58, 58);
    /// Texto principal (19,3:1 no fundo — Sentinel Fg #F5F5F5).
    pub const TEXTO: Color32 = Color32::from_rgb(245, 245, 245);
    /// Texto secundário (6,3:1 no fundo — Sentinel Muted #8C8C8C).
    pub const TEXTO_FRACO: Color32 = Color32::from_rgb(140, 140, 140);
    /// Preto para texto sobre fundo claro (Sentinel Ink #000).
    pub const TINTA: Color32 = Color32::from_rgb(0, 0, 0);
    /// Cinza fraco (3,1:1 no cartão — Sentinel Faint, ajustado para WCAG).
    #[allow(dead_code)]
    pub const APAGADO: Color32 = Color32::from_rgb(95, 95, 95);
    /// Laranja de destaque: texto, bordas e ícones (7,5:1 no fundo).
    pub const ACCENT: Color32 = Color32::from_rgb(255, 138, 40);
    /// Laranja de preenchimento: fundo de botão com texto branco (5,2:1).
    pub const ACCENT_FORTE: Color32 = Color32::from_rgb(194, 65, 12);
    /// Laranja do botão primário no hover (4,7:1 com texto branco).
    ///
    /// Antes era `rgb(214, 82, 16)`, que dava **4,14:1** com o texto branco —
    /// abaixo dos 4,5:1 da WCAG AA. O gate de qualidade achou isso; o valor
    /// agora é o mais claro que ainda passa no AA.
    pub const ACCENT_CLARO: Color32 = Color32::from_rgb(203, 72, 9);
    /// Verde de confirmação (8,2:1 no fundo).
    pub const VERDE: Color32 = Color32::from_rgb(56, 190, 118);
    /// Vermelho de alerta (6,6:1 no fundo).
    pub const VERMELHO: Color32 = Color32::from_rgb(240, 110, 110);
    /// Vermelho de preenchimento (5,6:1 com texto branco).
    pub const VERMELHO_FORTE: Color32 = Color32::from_rgb(198, 40, 40);
    /// Vermelho do botão de perigo no hover (5,0:1 com texto branco).
    pub const VERMELHO_CLARO: Color32 = Color32::from_rgb(206, 54, 54);
    /// Âmbar de informação (7,2:1 no fundo).
    pub const AMBAR: Color32 = Color32::from_rgb(217, 171, 72);
}

/// Luminância relativa de uma cor, como a WCAG 2.1 define.
///
/// A cor é linearizada (a curva sRGB) antes do cálculo — sem isso o número
/// sai otimista demais e o teste passa semverdade.
#[cfg(test)]
pub fn luminancia(cor: Color32) -> f64 {
    fn linear(canal: u8) -> f64 {
        let c = canal as f64 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * linear(cor.r()) + 0.7152 * linear(cor.g()) + 0.0722 * linear(cor.b())
}

/// A razão de contraste entre duas cores, de `1:1` a `21:1`.
///
/// É o número que a WCAG 2.1 usa: **4,5:1** para texto normal e **3:1**
/// para texto grande (≥ 18,66 px, ou ≥ 14 px em negrito).
#[cfg(test)]
pub fn razao_contraste(a: Color32, b: Color32) -> f64 {
    let (la, lb) = (luminancia(a), luminancia(b));
    let (claro, escuro) = if la >= lb { (la, lb) } else { (lb, la) };
    (claro + 0.05) / (escuro + 0.05)
}

/// Escala de espaçamento (multiplos de 4, como manda a casa).
pub mod espaco {
    pub const XS: f32 = 4.0;
    pub const S: f32 = 8.0;
    pub const M: f32 = 12.0;
    pub const L: f32 = 16.0;
    /// Respiro entre um bloco e outro (cartão e título).
    pub const XL: f32 = 24.0;
}

/// Respiro da página: nada encosta na borda da janela.
///
/// O dono reclamou exatamente disso — *"as informações estão grudadas na
/// borda do app sem nenhum espaçamento"*. O Sentinel usa 36 px de margem
/// lateral e 30 px de vertical na página; aqui é 32 e 24, um pouco menos
/// porque a janela do despertador é menor.
pub const MARGEM_PAGINA: i8 = 32;
/// Respiro da página na vertical (em cima e embaixo).
pub const MARGEM_PAGINA_VERTICAL: i8 = 24;
/// Respiro dentro de um cartão (o `Padding 20` do Sentinel).
pub const PADDING_CARTAO: i8 = 20;
/// Altura da barra do topo (a barra de título do Sentinel tem 44 px).
pub const ALTURA_BARRA: f32 = 44.0;
/// Largura da coluna do relógio (à esquerda da lista).
pub const LARGURA_RELOGIO: f32 = 320.0;
/// Largura do interruptor (para empurrar o resto da linha para a direita).
pub const LARGURA_INTERRUPTOR: f32 = 46.0;

/// Raios de canto.
pub mod raio {
    /// Controle pequeno (botão, chip).
    pub const CONTROLE: u8 = 10;
    /// Cartão.
    pub const CARTAO: u8 = 16;
    /// Pílula (etiqueta).
    pub const PILULA: u8 = 14;
}

/// Raio padrão dos cartões.
pub fn raio(n: u8) -> CornerRadius {
    CornerRadius::same(n)
}

/// Nome do estilo de texto grande usado no relógio e na hora digitável.
pub const ESTILO_HORA: &str = "hora";
/// Nome do estilo de texto do relógio grande da tela inicial.
pub const ESTILO_RELOGIO: &str = "relogio";
/// Título da página (26 px, semi-bold): o `H1` do Tranal Sentinel.
pub const ESTILO_TITULO: &str = "titulo-pagina";
/// Rótulo de seção (10,5 px, semi-bold): o `Label` do Tranal Sentinel.
pub const ESTILO_ROTULO: &str = "rotulo";
/// Subtítulo (13 px): o `Sub` do Tranal Sentinel.
pub const ESTILO_SUB: &str = "sub";
/// Número em destaque (34 px): o `Hero` do Tranal Sentinel.
pub const ESTILO_HERO: &str = "hero";

/// Aplica o visual dark do app.
pub fn aplicar(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Tema::FUNDO;
    visuals.window_fill = Tema::PAINEL;
    visuals.extreme_bg_color = Tema::FUNDO;
    visuals.faint_bg_color = Tema::PAINEL;
    visuals.override_text_color = Some(Tema::TEXTO);
    visuals.widgets.noninteractive.bg_fill = Tema::PAINEL_ALTO;
    visuals.widgets.noninteractive.weak_bg_fill = Tema::PAINEL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, Tema::BORDA);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Tema::TEXTO_FRACO);
    visuals.widgets.inactive.bg_fill = Tema::PAINEL_ALTO;
    visuals.widgets.inactive.weak_bg_fill = Tema::PAINEL;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Tema::BORDA);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Tema::TEXTO);
    visuals.widgets.hovered.bg_fill = Tema::PAINEL_ALTO;
    visuals.widgets.hovered.weak_bg_fill = Tema::PAINEL_ALTO;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Tema::BORDA_FORTE);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Tema::TEXTO);
    visuals.widgets.active.bg_fill = Tema::ACCENT_FORTE;
    visuals.widgets.active.weak_bg_fill = Tema::ACCENT_FORTE;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, Tema::ACCENT_FORTE);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    visuals.widgets.open.bg_fill = Tema::PAINEL_ALTO;
    visuals.widgets.open.weak_bg_fill = Tema::PAINEL;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, Tema::BORDA);
    visuals.selection.bg_fill = Tema::ACCENT.gamma_multiply(0.35);
    visuals.selection.stroke = Stroke::new(1.0, Tema::ACCENT);
    visuals.hyperlink_color = Tema::ACCENT;
    visuals.error_fg_color = Tema::VERMELHO;
    visuals.warn_fg_color = Tema::ACCENT;
    visuals.window_stroke = Stroke::new(1.0, Tema::BORDA);
    visuals.window_corner_radius = raio(raio::CARTAO);
    ctx.set_visuals(visuals);

    ctx.all_styles_mut(|s| {
        s.spacing.item_spacing = Vec2::new(espaco::M, espaco::S);
        s.spacing.button_padding = Vec2::new(14.0, 8.0);
        s.spacing.window_margin = egui::Margin::same(16);
        s.spacing.interact_size.y = 34.0;
        s.text_styles.insert(egui::TextStyle::Heading, FontId::proportional(22.0));
        s.text_styles.insert(egui::TextStyle::Body, FontId::proportional(14.0));
        s.text_styles.insert(egui::TextStyle::Small, FontId::proportional(12.0));
        s.text_styles.insert(egui::TextStyle::Button, FontId::proportional(14.0));
        s.text_styles.insert(egui::TextStyle::Monospace, FontId::monospace(14.0));
        // Estilos próprios: o relógio grande e o campo de hora digitável.
        s.text_styles.insert(
            egui::TextStyle::Name(ESTILO_HORA.into()),
            FontId::monospace(36.0),
        );
        s.text_styles.insert(
            egui::TextStyle::Name(ESTILO_RELOGIO.into()),
            FontId::proportional(72.0),
        );
        // Hierarquia de página, no mesmo padrão do Tranal Sentinel:
        // H1 26, Hero 34, Label 10,5 e Sub 13.
        s.text_styles.insert(
            egui::TextStyle::Name(ESTILO_TITULO.into()),
            FontId::proportional(26.0),
        );
        s.text_styles.insert(
            egui::TextStyle::Name(ESTILO_HERO.into()),
            FontId::proportional(34.0),
        );
        s.text_styles.insert(
            egui::TextStyle::Name(ESTILO_ROTULO.into()),
            FontId::proportional(10.5),
        );
        s.text_styles.insert(
            egui::TextStyle::Name(ESTILO_SUB.into()),
            FontId::proportional(13.0),
        );
    });
}

// ------------------------------------------------------------------ botões

/// O visual de um botão.
///
/// É o **único** botão do app: as variantes são o mesmo componente com
/// outras cores e medidas. Um componente só significa que um botão em
/// qualquer tela tem a mesma forma, o mesmo respiro e o mesmo texto
/// centralizado — e que dá para medir o botão antes de desenhá-lo.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EstiloBotao {
    /// Ação principal da página (preenchida, laranja).
    Primario,
    /// Ação secundária (contorno).
    Neutro,
    /// Ação discreta (sem fundo).
    Fantasma,
    /// Ação destrutiva (vermelha).
    Perigo,
    /// Botão só de ícone (quadrado, sem texto).
    Icone,
}

/// As medidas de um botão.
struct Medidas {
    /// Tamanho da fonte do rótulo.
    fonte: f32,
    /// Respiro horizontal (de cada lado).
    respiro_x: f32,
    /// Respiro vertical (de cada lado).
    respiro_y: f32,
    /// Altura mínima do botão.
    altura: f32,
    /// Largura fixa (só nos botões de ícone).
    largura_fixa: Option<f32>,
}

fn medidas(estilo: EstiloBotao) -> Medidas {
    match estilo {
        // O `PrimaryButton` do Sentinel: Padding 28,11 e CornerRadius 10.
        EstiloBotao::Primario => Medidas {
            fonte: 15.0,
            respiro_x: 28.0,
            respiro_y: 11.0,
            altura: 44.0,
            largura_fixa: None,
        },
        EstiloBotao::Neutro => Medidas {
            fonte: 13.0,
            respiro_x: 14.0,
            respiro_y: 9.0,
            altura: 34.0,
            largura_fixa: None,
        },
        EstiloBotao::Fantasma => Medidas {
            fonte: 13.0,
            respiro_x: 10.0,
            respiro_y: 7.0,
            altura: 30.0,
            largura_fixa: None,
        },
        EstiloBotao::Perigo => Medidas {
            fonte: 15.0,
            respiro_x: 28.0,
            respiro_y: 11.0,
            altura: 44.0,
            largura_fixa: None,
        },
        EstiloBotao::Icone => Medidas {
            fonte: 16.0,
            respiro_x: 0.0,
            respiro_y: 0.0,
            altura: 26.0,
            largura_fixa: Some(26.0),
        },
    }
}

/// A largura de um botão, **medida**.
///
/// O egui não sabe a largura de um texto antes de desenhá-lo, e é por isso
/// que alinhar um grupo de botões à direita é delicado: o grupo é medido
/// antes (com a mesma fonte com que o texto vai ser desenhado) e só depois
/// é desenhado.
pub fn largura_botao(ui: &Ui, texto: &str, estilo: EstiloBotao) -> f32 {
    let m = medidas(estilo);
    if let Some(l) = m.largura_fixa {
        return l;
    }
    let galeria = ui.painter().layout_no_wrap(
        texto.to_string(),
        FontId::proportional(m.fonte),
        Tema::TEXTO,
    );
    galeria.rect.width() + 2.0 * m.respiro_x
}

/// Desenha um botão e devolve a resposta dele (clique, hover, dica).
///
/// O botão é desenhado à mão (como o chip e o interruptor) por dois motivos:
/// o texto fica **sempre** centralizado no retângulo, e o tamanho segue o
/// Sentinel (`Padding 28,11` no botão principal, canto de 10 px).
pub fn botao(ui: &mut Ui, texto: &str, estilo: EstiloBotao) -> Response {
    let m = medidas(estilo);
    let galeria = ui.painter().layout_no_wrap(
        texto.to_string(),
        FontId::proportional(m.fonte),
        Tema::TEXTO,
    );
    let largura = m
        .largura_fixa
        .unwrap_or(galeria.rect.width() + 2.0 * m.respiro_x);
    let altura = m.altura.max(galeria.rect.height() + 2.0 * m.respiro_y);
    let (ret, resposta) = ui.allocate_exact_size(Vec2::new(largura, altura), Sense::click());
    pintar_botao(ui, ret, texto, estilo, resposta.hovered());
    resposta
}

/// Pinta um botão no retângulo dado (usado por `botao`).
fn pintar_botao(ui: &mut Ui, ret: egui::Rect, texto: &str, estilo: EstiloBotao, hover: bool) {
    let m = medidas(estilo);
    let (fundo, fundo_hover, borda, borda_hover, cor_texto, cor_texto_hover) = match estilo {
        EstiloBotao::Primario => (
            Tema::ACCENT_FORTE,
            Tema::ACCENT_CLARO,
            None,
            None,
            Color32::WHITE,
            Color32::WHITE,
        ),
        EstiloBotao::Neutro => (
            Color32::TRANSPARENT,
            Tema::BORDA,
            Some(Tema::TEXTO),
            Some(Tema::BORDA_FORTE),
            Tema::TEXTO,
            Tema::TEXTO,
        ),
        EstiloBotao::Fantasma => (
            Color32::TRANSPARENT,
            Tema::PAINEL_ALTO,
            None,
            None,
            Tema::TEXTO_FRACO,
            Tema::TEXTO,
        ),
        EstiloBotao::Perigo => (
            Tema::VERMELHO_FORTE,
            Tema::VERMELHO_CLARO,
            None,
            None,
            Color32::WHITE,
            Color32::WHITE,
        ),
        EstiloBotao::Icone => (
            Color32::TRANSPARENT,
            Tema::VERMELHO.gamma_multiply(0.18),
            None,
            None,
            Tema::TEXTO_FRACO,
            Tema::VERMELHO,
        ),
    };
    let canto = raio(raio::CONTROLE);
    if hover {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        ui.painter().rect_filled(ret, canto, fundo_hover);
    } else if fundo != Color32::TRANSPARENT {
        ui.painter().rect_filled(ret, canto, fundo);
    }
    if let Some(b) = borda {
        let cor = if hover { borda_hover.unwrap_or(b) } else { b };
        ui.painter().rect_stroke(
            ret,
            canto,
            Stroke::new(1.0, cor),
            egui::StrokeKind::Middle,
        );
    }
    ui.painter().text(
        ret.center(),
        Align2::CENTER_CENTER,
        texto,
        FontId::proportional(m.fonte),
        if hover { cor_texto_hover } else { cor_texto },
    );
}

/// Botão principal (laranja, preenchido).
pub fn botao_primario(ui: &mut Ui, texto: &str) -> Response {
    botao(ui, texto, EstiloBotao::Primario)
}

/// Botão neutro (contorno).
pub fn botao_neutro(ui: &mut Ui, texto: &str) -> Response {
    botao(ui, texto, EstiloBotao::Neutro)
}

/// Botão fantasma (sem fundo, só o texto).
pub fn botao_fantasma(ui: &mut Ui, texto: &str) -> Response {
    botao(ui, texto, EstiloBotao::Fantasma)
}

/// Botão de perigo (usado para "Parar").
pub fn botao_perigo(ui: &mut Ui, texto: &str) -> Response {
    botao(ui, texto, EstiloBotao::Perigo)
}

/// Uma ação (botão) de um grupo de ações.
#[derive(Clone)]
pub struct Acao {
    /// O rótulo do botão.
    pub texto: String,
    /// O visual do botão.
    pub estilo: EstiloBotao,
    /// A dica que aparece quando o mouse passa por cima.
    pub dica: Option<String>,
}

impl Acao {
    /// Cria uma ação.
    pub fn nova(texto: impl Into<String>, estilo: EstiloBotao) -> Self {
        Self {
            texto: texto.into(),
            estilo,
            dica: None,
        }
    }

    /// Junta uma dica à ação.
    pub fn com_dica(mut self, dica: impl Into<String>) -> Self {
        self.dica = Some(dica.into());
        self
    }
}

/// Desenha um grupo de botões **alinhado à borda direita** da linha.
///
/// O egui não sabe a largura de um botão antes de desenhá-lo, e é por isso
/// que alinhar um grupo à direita com `with_layout` é frágil: o escopo do
/// `with_layout` ocupa o espaço todo do pai, e o grupo acaba desenhado por
/// cima do conteúdo que vem antes (era o que acontecia com os botões do
/// cartão do despertador).
///
/// Aqui o grupo é **medido antes** — com a mesma fonte com que o texto vai
/// ser desenhado — e o espaço que sobra é reservado com `add_space`. O
/// grupo cai certinho na borda direita e nunca invade o conteúdo anterior.
///
/// Devolve, para cada ação, se ela foi clicada.
pub fn acoes_a_direita(ui: &mut Ui, acoes: &[Acao]) -> Vec<bool> {
    if acoes.is_empty() {
        return Vec::new();
    }
    let larguras: Vec<f32> = acoes
        .iter()
        .map(|a| largura_botao(ui, &a.texto, a.estilo))
        .collect();
    let espacamento = ui.spacing().item_spacing.x;
    let total: f32 = larguras.iter().sum::<f32>() + espacamento * (acoes.len() - 1) as f32;
    let disponivel = ui.available_width();

    if total > disponivel {
        // Não cabe em uma linha: os botões vão para a esquerda e, se ainda
        // assim não couberem, o egui os quebra em mais linhas. Melhor uma
        // linha quebrada do que um botão por cima do outro.
        let mut clicados = Vec::with_capacity(acoes.len());
        ui.horizontal_wrapped(|ui| {
            for a in acoes {
                clicados.push(desenhar_acao(ui, a));
            }
        });
        return clicados;
    }

    // O espaço que sobra entre o conteúdo anterior e o grupo.
    ui.add_space(disponivel - total);
    let mut clicados = Vec::with_capacity(acoes.len());
    for a in acoes {
        clicados.push(desenhar_acao(ui, a));
    }
    clicados
}

/// Desenha uma ação e devolve se ela foi clicada.
fn desenhar_acao(ui: &mut Ui, acao: &Acao) -> bool {
    let mut resposta = botao(ui, &acao.texto, acao.estilo);
    if let Some(dica) = &acao.dica {
        resposta = resposta.on_hover_text(dica.clone());
    }
    resposta.clicked()
}

/// Reserva o espaço que falta para o próximo item encostar na borda direita.
///
/// É o botão irmão de [`acoes_a_direita`], para quando o item da direita
/// tem largura conhecida (o interruptor, por exemplo).
pub fn empurra_para_direita(ui: &mut Ui, largura: f32) {
    let disponivel = ui.available_width();
    if disponivel > largura {
        ui.add_space(disponivel - largura);
    }
}

// -------------------------------------------------------------------- cartões

/// Cartão com fundo, borda e cantos arredondados.
///
/// O respiro interno é o `Padding 20` do Tranal Sentinel: é ele que tira o
/// conteúdo da borda do cartão e o deixa "respirar".
pub fn cartao<R>(ui: &mut Ui, adicionar: impl FnOnce(&mut Ui) -> R) -> R {
    egui::Frame::new()
        .fill(Tema::PAINEL)
        .stroke(Stroke::new(1.0, Tema::BORDA))
        .corner_radius(raio(raio::CARTAO))
        .inner_margin(egui::Margin::symmetric(PADDING_CARTAO, PADDING_CARTAO - 2))
        .show(ui, adicionar)
        .inner
}

/// Etiqueta arredondada (ex.: "Todo dia").
///
/// É o `Chip` do Sentinel: fundo transparente, Padding 13,5, canto de 14 px
/// e borda de 1 px. Aqui a borda e o texto usam a cor da informação (repete,
/// som, 100%, desafio), e o texto é a cor cheia — sobre o fundo do cartão
/// ela passa de 6:1, como manda a WCAG.
///
/// Devolve o retângulo desenhado (para os testes medirem o respiro).
pub fn chip(ui: &mut Ui, texto: &str, cor: Color32) -> egui::Rect {
    let galeria = ui.painter().layout_no_wrap(
        texto.to_string(),
        FontId::proportional(12.0),
        cor,
    );
    let largura = galeria.rect.width() + 26.0;
    let altura = galeria.rect.height() + 10.0;
    let (ret, resposta) = ui.allocate_exact_size(Vec2::new(largura, altura), Sense::hover());
    if resposta.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
    }
    ui.painter().rect_stroke(
        ret,
        raio(raio::PILULA),
        Stroke::new(1.0, cor.gamma_multiply(0.7)),
        egui::StrokeKind::Middle,
    );
    ui.painter().text(
        ret.center(),
        Align2::CENTER_CENTER,
        texto,
        FontId::proportional(12.0),
        cor,
    );
    ret
}

/// Interruptor liga/desliga desenhado à mão.
pub fn interruptor(ui: &mut Ui, ligado: &mut bool) -> Response {
    let tamanho = Vec2::new(46.0, 26.0);
    let (ret, mut resposta) = ui.allocate_exact_size(tamanho, Sense::click());
    if resposta.clicked() {
        *ligado = !*ligado;
        resposta.mark_changed();
    }
    let cor = if *ligado { Tema::ACCENT } else { Tema::BORDA_FORTE };
    ui.painter().rect_filled(ret, raio(13), cor);
    let centro_x = if *ligado { ret.right() - 13.0 } else { ret.left() + 13.0 };
    let bolinha = egui::Rect::from_center_size(
        egui::pos2(centro_x, ret.center().y),
        Vec2::new(20.0, 20.0),
    );
    ui.painter().rect_filled(
        bolinha,
        raio(10),
        if *ligado { Color32::WHITE } else { Tema::TEXTO_FRACO },
    );
    resposta
}

/// Título de seção (rótulo em maiúsculas, com respiro embaixo).
///
/// É o `Label` do Tranal Sentinel: 10,5 px, semi-bold, cinza, com 8 px de
/// respiro antes do conteúdo que ele anuncia.
pub fn titulo_secao(ui: &mut Ui, texto: &str) {
    ui.label(
        egui::RichText::new(texto.to_uppercase())
            .color(Tema::TEXTO_FRACO)
            .size(10.5)
            .strong(),
    );
    ui.add_space(espaco::S);
}

/// Título da página (26 px, semi-bold): o `H1` do Tranal Sentinel.
pub fn titulo_pagina(ui: &mut Ui, texto: &str) {
    ui.label(
        egui::RichText::new(texto)
            .color(Tema::TEXTO)
            .size(26.0)
            .strong(),
    );
}

/// Subtítulo (13 px, cinza): o `Sub` do Tranal Sentinel.
pub fn sub(ui: &mut Ui, texto: &str) {
    ui.label(egui::RichText::new(texto).color(Tema::TEXTO_FRACO).size(13.0));
}

/// Texto de ajuda (cinza, pequeno).
pub fn ajuda(ui: &mut Ui, texto: &str) {
    ui.label(egui::RichText::new(texto).color(Tema::TEXTO_FRACO).size(12.0));
}

/// Linha divisória fina.
pub fn linha_separadora(ui: &mut Ui) {
    ui.add_space(espaco::XS);
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 1.0), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, Tema::BORDA);
    ui.add_space(espaco::XS);
}

// ------------------------------------------------------------------ seletores

/// Uma opção de um seletor segmentado.
pub struct Segmento<'a> {
    pub texto: &'a str,
    /// Texto mostrado quando o mouse passa por cima.
    pub dica: Option<&'a str>,
}

impl<'a> Segmento<'a> {
    pub fn novo(texto: &'a str) -> Self {
        Self { texto, dica: None }
    }

    pub fn com_dica(mut self, dica: &'a str) -> Self {
        self.dica = Some(dica);
        self
    }
}

/// Seletor segmentado: um grupo de opções, das quais só uma vale.
///
/// Devolve o índice da opção clicada (ou `None` se ninguém clicou).
pub fn seletor_segmentado(ui: &mut Ui, segmentos: &[Segmento], selecionado: usize) -> Option<usize> {
    let total = segmentos.len();
    if total == 0 {
        return None;
    }
    let altura = 36.0;
    // O espaco entre os segmentos conta na largura: sem descontar ele, os
    // ultimos segmentos passam da borda e esticam o painel que os contem.
    let espacamento = 4.0;
    let largura =
        (ui.available_width() - espacamento * (total as f32 - 1.0)) / total as f32;
    let mut escolhido = None;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
        for (i, segmento) in segmentos.iter().enumerate() {
            let (ret, resposta) = ui.allocate_exact_size(Vec2::new(largura, altura), Sense::click());
            let ativo = i == selecionado;
            let fundo = if ativo {
                Tema::TEXTO
            } else if resposta.hovered() {
                Tema::PAINEL_ALTO
            } else {
                Tema::PAINEL
            };
            ui.painter().rect_filled(ret, raio(raio::CONTROLE), fundo);
            if !ativo {
                ui.painter().rect_stroke(
                    ret,
                    raio(raio::CONTROLE),
                    Stroke::new(1.0, Tema::BORDA),
                    egui::StrokeKind::Middle,
                );
            }
            let cor = if ativo { Tema::TINTA } else { Tema::TEXTO };
            ui.painter().text(
                ret.center(),
                Align2::CENTER_CENTER,
                segmento.texto,
                FontId::proportional(13.0),
                cor,
            );
            let clicado = resposta.clicked();
            if let Some(dica) = segmento.dica {
                resposta.on_hover_text(dica);
            }
            if clicado {
                escolhido = Some(i);
            }
        }
    });
    escolhido
}

/// Controle deslizante de número inteiro.
///
/// O egui, com número decimal, mostrava "3.0 vezes". Aqui o valor é inteiro
/// de verdade: a tela mostra "3 vezes".
pub fn slider_inteiro(
    ui: &mut Ui,
    valor: &mut i32,
    minimo: i32,
    maximo: i32,
    texto: &str,
    sufixo: &str,
) -> Response {
    ui.add(
        egui::Slider::new(valor, minimo..=maximo)
            .integer()
            .text(texto)
            .suffix(sufixo)
            .trailing_fill(true),
    )
}

/// Campo de texto do app (com a moldura do tema).
pub fn campo_texto(ui: &mut Ui, texto: &mut String, dica: &str) -> Response {
    ui.add(
        egui::TextEdit::singleline(texto)
            .desired_width(f32::INFINITY)
            .hint_text(dica.to_string())
            .text_color(Tema::TEXTO),
    )
}

/// Campo da hora: grande, em monoespaçada, para o dono **digitar** o horário.
pub fn campo_hora(ui: &mut Ui, texto: &mut String, id: egui::Id) -> Response {
    ui.add(
        egui::TextEdit::singleline(texto)
            .id(id)
            .font(egui::TextStyle::Name(ESTILO_HORA.into()))
            .desired_width(190.0)
            .align(Align2::CENTER_CENTER)
            .hint_text("07:30".to_string())
            .text_color(Tema::TEXTO)
            .char_limit(5),
    )
}

/// Seleciona o texto inteiro do campo quando ele **ganha o foco**.
///
/// O dono pediu para **digitar por cima** do horário: com o texto todo
/// selecionado, o primeiro dígito já substitui o valor antigo, sem precisar
/// apagar antes.
///
/// A seleção fica no estado do `TextEdit` (o egui guarda isso entre quadros),
/// então ela vale no quadro seguinte, que é o que o dono vê.
pub fn selecionar_todo_ao_ganhar_foco(
    ctx: &egui::Context,
    id: egui::Id,
    texto: &str,
    ganhou_foco: bool,
) {
    if !ganhou_foco || texto.is_empty() {
        return;
    }
    let selecao = egui::text_selection::CCursorRange::select_all_str(texto);
    if let Some(mut estado) = egui::widgets::text_edit::TextEditState::load(ctx, id) {
        estado.cursor.set_char_range(Some(selecao));
        estado.store(ctx, id);
    }
}

/// Grupo de opções em que **mais de uma** pode estar marcada (os dias da
/// semana).
///
/// Devolve o índice da opção clicada (ou `None` se ninguém clicou).
pub fn seletor_multiplo(ui: &mut Ui, opcoes: &[&str], marcados: &[bool]) -> Option<usize> {
    let total = opcoes.len();
    if total == 0 {
        return None;
    }
    let altura = 34.0;
    // O espaco entre os botoes conta na largura (mesmo cuidado do seletor
    // segmentado).
    let espacamento = 4.0;
    let largura =
        (ui.available_width() - espacamento * (total as f32 - 1.0)) / total as f32;
    let mut escolhido = None;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
        for (i, texto) in opcoes.iter().enumerate() {
            let (ret, resposta) = ui.allocate_exact_size(Vec2::new(largura, altura), Sense::click());
            let ativo = marcados.get(i).copied().unwrap_or(false);
            let fundo = if ativo {
                Tema::ACCENT_FORTE
            } else if resposta.hovered() {
                Tema::PAINEL_ALTO
            } else {
                Tema::PAINEL
            };
            ui.painter().rect_filled(ret, raio(raio::CONTROLE), fundo);
            if !ativo {
                ui.painter().rect_stroke(
                    ret,
                    raio(raio::CONTROLE),
                    Stroke::new(1.0, Tema::BORDA),
                    egui::StrokeKind::Middle,
                );
            }
            let cor = if ativo { Color32::WHITE } else { Tema::TEXTO_FRACO };
            ui.painter().text(
                ret.center(),
                Align2::CENTER_CENTER,
                *texto,
                FontId::proportional(13.0),
                cor,
            );
            if resposta.clicked() {
                escolhido = Some(i);
            }
        }
    });
    escolhido
}

#[cfg(test)]
mod testes {
    use super::*;

    /// Contexto do egui sem janela.
    ///
    /// O `Box::leak` é de propósito: o `Context` guarda a textura da fonte e
    /// o epaint reclama (`debug_assert`) quando ela é destruída sem um
    /// renderer, e um teste headless não tem renderer. Vazar o contexto
    /// durante o teste é o jeito limpo de não ter esse assert.
    fn contexto() -> &'static egui::Context {
        Box::leak(Box::new(egui::Context::default()))
    }

    /// Um quadro do egui com o campo de hora na tela.
    fn quadro(
        ctx: &egui::Context,
        id: egui::Id,
        texto: &mut String,
        eventos: Vec<egui::Event>,
    ) {
        let raw = egui::RawInput {
            events: eventos,
            ..Default::default()
        };
        let saida = ctx.run_ui(raw, |ui| {
            ui.add(egui::TextEdit::singleline(texto).id(id).char_limit(5));
        });
        saida.drop_without_applying_deltas();
    }

    fn selecao(ctx: &egui::Context, id: egui::Id) -> Option<(usize, usize)> {
        let estado = egui::widgets::text_edit::TextEditState::load(ctx, id).unwrap();
        estado
            .cursor
            .char_range()
            .map(|r| (r.primary.index.0, r.secondary.index.0))
    }

    /// Roda um quadro e devolve as formas desenhadas (para medir).
    fn medir(mut conteudo: impl FnMut(&mut Ui)) -> Vec<egui::Shape> {
        let ctx = egui::Context::default();
        let saida = ctx.run_ui(Default::default(), |ui| {
            conteudo(ui);
        });
        let formas: Vec<egui::Shape> = saida.shapes.iter().map(|c| c.shape.clone()).collect();
        saida.drop_without_applying_deltas();
        formas
    }

    /// O retângulo preenchido com a cor dada.
    fn retangulo(formas: &[egui::Shape], cor: Color32) -> Option<egui::Rect> {
        formas.iter().find_map(|f| match f {
            egui::Shape::Rect(r) if r.fill == cor => Some(r.rect),
            _ => None,
        })
    }

    /// O retângulo do texto desenhado (na tela).
    ///
    /// O `galley.rect` é o retângulo **local** do texto; na tela ele começa
    /// em `pos`.
    fn texto_desenhado(formas: &[egui::Shape]) -> Option<egui::Rect> {
        formas.iter().find_map(|f| match f {
            egui::Shape::Text(t) => {
                Some(egui::Rect::from_min_size(t.pos, t.galley.rect.size()))
            }
            _ => None,
        })
    }

    #[test]
    fn ganhar_foco_seleciona_o_texto_inteiro() {
        let ctx = contexto();
        let id = egui::Id::unique("campo_hora_1");
        let mut texto = String::from("07:00");
        quadro(ctx, id, &mut texto, vec![]);

        // o dono clicou no campo: o egui deu foco a ele
        ctx.memory_mut(|m| m.request_focus(id));
        selecionar_todo_ao_ganhar_foco(ctx, id, &texto, true);

        assert_eq!(selecao(ctx, id), Some((5, 0)), "o texto inteiro tem que estar selecionado");
    }

    #[test]
    fn digitar_por_cima_substitui_o_horario() {
        let ctx = contexto();
        let id = egui::Id::unique("campo_hora_2");
        let mut texto = String::from("07:00");
        quadro(ctx, id, &mut texto, vec![]);

        ctx.memory_mut(|m| m.request_focus(id));
        selecionar_todo_ao_ganhar_foco(ctx, id, &texto, true);

        // o dono digita "0": tem que REPOR o horario, e nao somar
        quadro(ctx, id, &mut texto, vec![egui::Event::Text("0".to_string())]);
        assert_eq!(texto, "0");
    }

    #[test]
    fn sem_selecao_o_horario_seria_somado() {
        // contra-prova: sem a selecao automatica, o texto seria somado
        let ctx = contexto();
        let id = egui::Id::unique("campo_hora_3");
        let mut texto = String::from("07:00");
        quadro(ctx, id, &mut texto, vec![]);

        ctx.memory_mut(|m| m.request_focus(id));
        quadro(ctx, id, &mut texto, vec![egui::Event::Text("0".to_string())]);
        assert_eq!(texto, "07:00");
    }

    #[test]
    fn texto_vazio_nao_quebra_nada() {
        let ctx = contexto();
        let id = egui::Id::unique("campo_hora_4");
        let mut texto = String::new();
        quadro(ctx, id, &mut texto, vec![]);
        ctx.memory_mut(|m| m.request_focus(id));
        selecionar_todo_ao_ganhar_foco(ctx, id, &texto, true);
        assert_eq!(selecao(ctx, id), None);
    }

    /// O texto do botão tem que ficar **centralizado** dentro dele.
    ///
    /// Era exatamente o defeito que o dono apontou: o texto do "+ Novo
    /// despertador" não batia com o meio do botão.
    #[test]
    fn o_texto_do_botao_fica_centralizado() {
        let formas = medir(|ui| {
            botao(ui, "+  Novo despertador", EstiloBotao::Primario);
        });
        let botao = retangulo(&formas, Tema::ACCENT_FORTE).expect("o botão foi desenhado");
        let texto = texto_desenhado(&formas).expect("o texto foi desenhado");
        assert!(
            (texto.center().x - botao.center().x).abs() < 1.0,
            "o texto não está centralizado no botão: texto {:?}, botão {:?}",
            texto,
            botao
        );
        assert!(
            (texto.center().y - botao.center().y).abs() < 1.0,
            "o texto não está centralizado na altura do botão"
        );
    }

    /// O grupo de botões tem que encostar na borda direita da linha, sem
    /// invadir o conteúdo que vem antes.
    ///
    /// Este é o defeito do cartão do despertador: os botões eram desenhados
    /// por cima das etiquetas.
    #[test]
    fn o_grupo_de_botoes_para_na_direita_sem_invadir_o_conteudo() {
        let mut borda_da_linha = 0.0;
        let mut borda_da_etiqueta = 0.0;
        let formas = medir(|ui| {
            borda_da_linha = ui.max_rect().right();
            ui.horizontal(|ui| {
                // O conteúdo da esquerda (as etiquetas do despertador).
                borda_da_etiqueta = chip(ui, "Uma vez", Tema::ACCENT).right();
                // O grupo de ações, à direita.
                acoes_a_direita(
                    ui,
                    &[
                        Acao::nova("Repetir amanhã", EstiloBotao::Neutro),
                        Acao::nova("Editar", EstiloBotao::Neutro),
                    ],
                );
            });
        });
        // Os dois botões do grupo (a borda outline do neutro).
        let acoes: Vec<egui::Rect> = formas
            .iter()
            .filter_map(|f| match f {
                egui::Shape::Rect(r) if r.stroke.color == Tema::TEXTO => Some(r.rect),
                _ => None,
            })
            .collect();
        assert!(acoes.len() >= 2, "os dois botões do grupo foram desenhados");
        let grupo_esquerda = acoes
            .iter()
            .map(|r| r.left())
            .fold(f32::INFINITY, |a, b| a.min(b));
        let grupo_direita = acoes
            .iter()
            .map(|r| r.right())
            .fold(f32::NEG_INFINITY, |a, b| a.max(b));

        assert!(
            grupo_esquerda > borda_da_etiqueta,
            "o grupo de botões invadiu as etiquetas: grupo em {}, etiqueta em {}",
            grupo_esquerda,
            borda_da_etiqueta
        );
        assert!(
            (grupo_direita - borda_da_linha).abs() < 1.0,
            "o grupo de botões não encostou na borda direita: grupo em {}, linha em {}",
            grupo_direita,
            borda_da_linha
        );
    }

    /// A paleta tem que passar na **WCAG AA**, medido.
    ///
    /// O gate de qualidade pediu uma validação formal do contraste (o
    /// único apontamento que ficou). Este teste é essa validação: ele
    /// calcula a razão de contraste de cada par de cor que o app usa de
    /// verdade e falha se algum ficar abaixo do mínimo da norma.
    ///
    /// * texto normal: **4,5:1** (WCAG 1.4.3);
    /// * borda de elemento informativo: **3:1** (WCAG 1.4.11).
    #[test]
    fn a_paleta_passa_na_wcag_aa() {
        let textos: [(&str, Color32, Color32); 16] = [
            ("texto principal no fundo", Tema::TEXTO, Tema::FUNDO),
            ("texto principal no cartão", Tema::TEXTO, Tema::PAINEL),
            ("texto principal no controle", Tema::TEXTO, Tema::PAINEL_ALTO),
            ("texto fraco no fundo", Tema::TEXTO_FRACO, Tema::FUNDO),
            ("texto fraco no cartão", Tema::TEXTO_FRACO, Tema::PAINEL),
            ("texto fraco no controle", Tema::TEXTO_FRACO, Tema::PAINEL_ALTO),
            ("laranja no fundo", Tema::ACCENT, Tema::FUNDO),
            ("laranja no cartão", Tema::ACCENT, Tema::PAINEL),
            ("verde no cartão", Tema::VERDE, Tema::PAINEL),
            ("vermelho no cartão", Tema::VERMELHO, Tema::PAINEL),
            ("âmbar no cartão", Tema::AMBAR, Tema::PAINEL),
            ("tinta no botão branco", Tema::TINTA, Tema::TEXTO),
            ("branco no botão primário", Color32::WHITE, Tema::ACCENT_FORTE),
            (
                "branco no hover do botão primário",
                Color32::WHITE,
                Tema::ACCENT_CLARO,
            ),
            ("branco no botão de perigo", Color32::WHITE, Tema::VERMELHO_FORTE),
            (
                "branco no hover do botão de perigo",
                Color32::WHITE,
                Tema::VERMELHO_CLARO,
            ),
        ];
        for (nome, frente, fundo) in textos {
            let razao = razao_contraste(frente, fundo);
            assert!(
                razao >= 4.5,
                "{}: contraste de {:.2}:1 está abaixo dos 4,5:1 exigidos pela WCAG AA",
                nome,
                razao
            );
        }

        // A borda de 1 px da etiqueta (a "informação" do chip) tem que
        // aparecer contra o cartão: 3:1 (WCAG 1.4.11).
        let bordas: [(&str, Color32); 5] = [
            ("borda da etiqueta de repetição", Tema::AMBAR),
            ("borda da etiqueta de som", Tema::TEXTO_FRACO),
            ("borda da etiqueta de volume", Tema::VERDE),
            ("borda da etiqueta de desafio", Tema::ACCENT),
            ("borda da etiqueta de perigo", Tema::VERMELHO),
        ];
        for (nome, cor) in bordas {
            let razao = razao_contraste(cor.gamma_multiply(0.7), Tema::PAINEL);
            assert!(
                razao >= 3.0,
                "{}: contraste de {:.2}:1 está abaixo dos 3:1 exigidos pela WCAG 1.4.11",
                nome,
                razao
            );
        }

        let razao = razao_contraste(Tema::APAGADO, Tema::PAINEL);
        assert!(
            razao >= 3.0,
            "apagado no cartão: contraste de {:.2}:1 está abaixo dos 3:1 exigidos pela WCAG 1.4.11",
            razao
        );
    }
}
