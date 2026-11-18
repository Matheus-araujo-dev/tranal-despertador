//! Sistema de design do Tranal Despertador.
//!
//! Tudo aqui é desenhado à mão (sem CSS, sem navegador): o app é uma janela
//! nativa, e o objetivo é ficar bonito sem pesar na memória.
//!
//! As cores foram conferidas contra o fundo escuro pela regra de contraste
//! da WCAG (AA): texto normal precisa de 4.5:1, texto grande de 3:1.
//! Por isso existem duas variações do laranja e do vermelho: uma para
//! **texto e bordas** (clara, 7:1 ou mais no fundo) e outra para
//! **preenchimento de botão** (escura, para o texto branco em cima dela
//! chegar a 4.5:1).

use egui::{Align2, Color32, CornerRadius, FontId, Response, Sense, Stroke, Ui, Vec2};

/// Paleta do app.
pub struct Tema;

impl Tema {
    /// Fundo da janela.
    pub const FUNDO: Color32 = Color32::from_rgb(11, 13, 18);
    /// Fundo dos cartões e painéis.
    pub const PAINEL: Color32 = Color32::from_rgb(21, 25, 34);
    /// Fundo dos controles (campos, botões).
    pub const PAINEL_ALTO: Color32 = Color32::from_rgb(29, 34, 46);
    /// Borda discreta.
    pub const BORDA: Color32 = Color32::from_rgb(42, 49, 64);
    /// Borda de destaque (campo em foco).
    pub const BORDA_FORTE: Color32 = Color32::from_rgb(62, 72, 92);
    /// Texto principal (15:1 no fundo).
    pub const TEXTO: Color32 = Color32::from_rgb(232, 236, 243);
    /// Texto secundário (6.5:1 no fundo).
    pub const TEXTO_FRACO: Color32 = Color32::from_rgb(146, 155, 174);
    /// Laranja de destaque: texto, bordas e ícones (8.3:1 no fundo).
    pub const ACCENT: Color32 = Color32::from_rgb(255, 138, 40);
    /// Laranja de preenchimento: fundo de botão com texto branco (5.2:1).
    pub const ACCENT_FORTE: Color32 = Color32::from_rgb(194, 65, 12);
    /// Verde de confirmação (8.2:1 no fundo).
    pub const VERDE: Color32 = Color32::from_rgb(56, 190, 118);
    /// Vermelho de alerta (6.6:1 no fundo).
    pub const VERMELHO: Color32 = Color32::from_rgb(240, 110, 110);
    /// Vermelho de preenchimento (5.6:1 com texto branco).
    pub const VERMELHO_FORTE: Color32 = Color32::from_rgb(198, 40, 40);
    /// Azul de informação (7.1:1 no fundo).
    pub const AZUL: Color32 = Color32::from_rgb(88, 156, 255);
}

/// Escala de espaçamento (multiplos de 4, como manda a casa).
///
/// Os dois últimos existem para o **respiro da página**: o dono reclamou que
/// o botão e as informações estavam "grudados na borda do app", e a
/// referência de layout (o Tranal Sentinel) trabalha com cartões de 20 px de
/// respiro e 24 px de margem de página.
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
/// O dono reclamou exatamente disso — *"o botão novo despertador e as
/// informações estão grudadas na borda do app sem nenhum espaçamento"*. É a
/// margem que o Tranal Sentinel usa entre a borda da janela e o conteúdo
/// (14 px na barra lateral, 20 px dentro dos cartões).
pub const MARGEM_PAGINA: i8 = 24;
/// Respiro da página na vertical (em cima e embaixo).
pub const MARGEM_PAGINA_VERTICAL: i8 = 20;
/// Respiro dentro de um cartão (o `Padding 20` do Sentinel).
pub const PADDING_CARTAO: i8 = 20;

/// Raios de canto.
pub mod raio {
    /// Controle pequeno (botão, chip).
    pub const CONTROLE: u8 = 10;
    /// Cartão.
    pub const CARTAO: u8 = 16;
    /// Pílula (etiqueta).
    pub const PILULA: u8 = 12;
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

/// Botão principal (laranja).
pub fn botao_primario(ui: &mut Ui, texto: &str) -> Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(texto)
                .color(Color32::WHITE)
                .size(15.0)
                .strong(),
        )
        .fill(Tema::ACCENT_FORTE)
        .corner_radius(raio(raio::CONTROLE))
        .stroke(Stroke::NONE)
        .min_size(Vec2::new(0.0, 40.0)),
    )
}

/// Botão principal **grande**: o da ação principal da página.
///
/// É o `PrimaryButton` do Tranal Sentinel (preenchido, canto 10, padding
/// largo). Ele fica na linha do título da página, longe de qualquer borda.
pub fn botao_primario_grande(ui: &mut Ui, texto: &str) -> Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(texto)
                .color(Color32::WHITE)
                .size(15.0)
                .strong(),
        )
        .fill(Tema::ACCENT_FORTE)
        .corner_radius(raio(raio::CONTROLE))
        .stroke(Stroke::NONE)
        .min_size(Vec2::new(0.0, 44.0)),
    )
}

/// Botão neutro (contorno).
pub fn botao_neutro(ui: &mut Ui, texto: &str) -> Response {
    ui.add(
        egui::Button::new(egui::RichText::new(texto).color(Tema::TEXTO).size(13.0))
            .fill(Tema::PAINEL_ALTO)
            .corner_radius(raio(raio::CONTROLE))
            .stroke(Stroke::new(1.0, Tema::BORDA))
            .min_size(Vec2::new(0.0, 34.0)),
    )
}

/// Botão fantasma (sem fundo, só o texto).
pub fn botao_fantasma(ui: &mut Ui, texto: &str) -> Response {
    ui.add(
        egui::Button::new(egui::RichText::new(texto).color(Tema::TEXTO_FRACO).size(13.0))
            .fill(Color32::TRANSPARENT)
            .corner_radius(raio(raio::CONTROLE))
            .stroke(Stroke::NONE)
            .min_size(Vec2::new(0.0, 30.0)),
    )
}

/// Botão de perigo (usado para "Parar").
pub fn botao_perigo(ui: &mut Ui, texto: &str) -> Response {
    ui.add(
        egui::Button::new(
            egui::RichText::new(texto)
                .color(Color32::WHITE)
                .size(15.0)
                .strong(),
        )
        .fill(Tema::VERMELHO_FORTE)
        .corner_radius(raio(raio::CONTROLE))
        .stroke(Stroke::NONE)
        .min_size(Vec2::new(0.0, 44.0)),
    )
}

/// Etiqueta arredondada (ex.: "Todo dia").
pub fn chip(ui: &mut Ui, texto: &str, cor: Color32) {
    let galeria = ui.painter().layout_no_wrap(
        texto.to_string(),
        FontId::proportional(12.0),
        cor,
    );
    let largura = galeria.rect.width() + 18.0;
    let altura = galeria.rect.height() + 8.0;
    let (ret, resposta) = ui.allocate_exact_size(Vec2::new(largura, altura), Sense::hover());
    if resposta.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
    }
    ui.painter()
        .rect_filled(ret, raio(raio::PILULA), cor.gamma_multiply(0.16));
    ui.painter().rect_stroke(
        ret,
        raio(raio::PILULA),
        Stroke::new(1.0, cor.gamma_multiply(0.45)),
        egui::StrokeKind::Middle,
    );
    ui.painter().text(
        ret.center(),
        Align2::CENTER_CENTER,
        texto,
        FontId::proportional(12.0),
        cor,
    );
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
    let largura = ui.available_width() / total as f32;
    let mut escolhido = None;

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
        for (i, segmento) in segmentos.iter().enumerate() {
            let (ret, resposta) = ui.allocate_exact_size(Vec2::new(largura, altura), Sense::click());
            let ativo = i == selecionado;
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
            let cor = if ativo { Color32::WHITE } else { Tema::TEXTO };
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
    let largura = ui.available_width() / total as f32;
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
}