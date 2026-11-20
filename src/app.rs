//! Aplicação: janela principal, lista de despertadores, editor e o painel de
//! alarme tocando.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::{DateTime, Datelike, Local, Timelike};
use eframe::egui;

use crate::audio::{self, Saida};
use crate::arquivos;
use crate::estado;
use crate::hora;
use crate::model::{self, Alarm, Desafio, Preset, RepeatRule, SoundChoice};
use crate::schedule;
use crate::storage;
use crate::tones::{self, AudioDaBiblioteca};
use crate::tray;
use crate::ui::{self, Tema};
use crate::device_volume;

/// Eventos que o agendador manda para a interface.
enum EventoAgendador {
    /// O alarme disparou. `tocou` diz se o som realmente saiu.
    ///
    /// O alarme vem pelo **id**, e nao pelo indice: a lista muda enquanto o
    /// app roda (o dono edita, apaga, soneca) e um indice apontaria para o
    /// despertador errado.
    Disparo {
        id: String,
        tocou: bool,
        avisos: Vec<String>,
    },
}

struct Agendador {
    eventos: Receiver<EventoAgendador>,
}

/// Estado do painel de edição.
struct Editor {
    /// `None` = alarme novo.
    id_original: Option<String>,
    rascunho: Alarm,
    hora: u8,
    minuto: u8,
    /// O que o dono digitou no campo de hora.
    texto_hora: String,
    /// Erro de digitação da hora (quando existe).
    erro_hora: Option<String>,
    dias: Vec<u8>,
    /// Duração do arquivo de som escolhido, em segundos.
    duracao_som: Option<f32>,
    /// Volume que o Windows mostra hoje em cada saída (em %).
    volumes_saidas: Vec<(String, u8)>,
    /// Os áudios que o dono adicionou ao app.
    biblioteca: Vec<AudioDaBiblioteca>,
}

/// Para onde vai o arquivo escolhido no seletor.
#[derive(Clone, Copy, PartialEq)]
enum DestinoSelecao {
    /// Tocar no despertador que está sendo editado.
    Editor,
    /// Só adicionar à biblioteca de sons.
    Biblioteca,
}

/// Estado do alarme tocando.
struct Tocando {
    alarme: Alarm,
    sonecas_usadas: u32,
    inicio: Instant,
    palavra: String,
    conta: (i32, i32),
    resposta: String,
}

/// Respostas do cartão de um despertador.
struct AcoesAlarme {
    editar: bool,
    remover: bool,
    alternar: bool,
    testar: bool,
    rearmar: bool,
}

/// Uma ação do cartão de um despertador.
///
/// O cartão tem um grupo de botões cujo conteúdo muda (o "Repetir amanhã"
/// só existe em alarme de uma vez que já tocou), então cada botão precisa
/// saber o que é.
#[derive(Clone, Copy)]
enum AcaoCartao {
    Repetir,
    Editar,
    Testar,
    Excluir,
}

/// Aplicação do Tranal Despertador.
pub struct TranalApp {
    /// Estado compartilhado com a thread do agendador: uma copia so.
    comp: Arc<estado::Compartilhado>,
    saidas: Vec<Saida>,
    editor: Option<Editor>,
    tocando: Option<Tocando>,
    /// Proximo despertador (copia) e a hora em que ele toca.
    proximo: Option<(Alarm, DateTime<Local>)>,
    agendador: Agendador,
    mostrar_config: bool,
    mensagem: Option<String>,
    mensagem_erro: bool,
    acoes_pendentes: Vec<(String, AcoesAlarme)>,
    piscar: f32,
    /// Aviso que fica na tela enquanto o alarme não conseguir tocar.
    alerta: Option<String>,
    /// O seletor de arquivos (aberto por cima de tudo).
    seletor: Option<arquivos::Seletor>,
    /// Para onde vai o arquivo escolhido no seletor.
    destino_selecao: DestinoSelecao,
    /// Os áudios que o dono adicionou ao app (tela de configurações).
    biblioteca: Vec<AudioDaBiblioteca>,
    /// Cache dos volumes das saídas de áudio (evita enumeração COM ao abrir o editor).
    volumes_saidas: Vec<(String, u8)>,
    /// Última geração lida do estado compartilhado.
    geracao_lida: u64,
    /// Cache dos alarmes (evita mutex+clone a cada frame).
    alarmes_cache: Vec<Alarm>,
    /// Geração em que o proximo_disparo foi calculado.
    proximo_geracao: u64,
    /// Instante em que o proximo_disparo foi calculado (o schedule depende da hora).
    proximo_calculado_em: Instant,
    /// O ícone na bandeja do Windows (vive enquanto o app vive).
    ///
    /// O `TrayIcon` tem que ficar guardado: se for solto, o Windows remove o
    /// ícone da área de notificação.
    bandeja: Option<tray_icon::TrayIcon>,
}

impl TranalApp {
    /// Cria o app carregando os dados salvos.
    pub fn novo(cc: &eframe::CreationContext) -> Self {
        ui::aplicar(&cc.egui_ctx);
        let (mut alarms, settings) = storage::carregar();
        let saidas = audio::listar_saidas();

        // Modo demo: dois exemplos em memória, para ver a lista preenchida
        // sem cadastrar nada. Não grava nada.
        if std::env::args().any(|a| a == "--demo") {
            alarms = alarmes_de_exemplo();
        }

        // Interface e agendador olham para o mesmo estado: uma copia so.
        let comp = Arc::new(estado::Compartilhado::novo(alarms.clone(), settings));
        let agendador = iniciar_agendador(cc.egui_ctx.clone(), comp.clone());

        Self {
            comp,
            saidas,
            editor: None,
            tocando: None,
            proximo: None,
            agendador,
            mostrar_config: false,
            mensagem: None,
            mensagem_erro: false,
            acoes_pendentes: Vec::new(),
            piscar: 0.0,
            alerta: None,
            seletor: None,
            destino_selecao: DestinoSelecao::Editor,
            biblioteca: tones::biblioteca(),
            bandeja: None,
            volumes_saidas: device_volume::saidas_com_volume()
                .into_iter()
                .map(|s| {
                    let volume = s.volume.unwrap_or(0.0);
                    (s.nome, (volume * 100.0).round() as u8)
                })
                .collect(),
            geracao_lida: 1,
            alarmes_cache: alarms,
            proximo_geracao: 0,
            proximo_calculado_em: Instant::now(),
        }
    }

    /// Os despertadores do dono (o que a lista mostra).
    /// Usa o cache local — só trava o mutex se a geração mudou.
    fn alarms(&self) -> &[Alarm] {
        &self.alarmes_cache
    }

    /// Atualiza os caches se o estado compartilhado mudou.
    fn sincronizar_cache(&mut self) {
        let g = self.comp.geracao();
        if g != self.geracao_lida {
            let estado = self.comp.travar();
            self.alarmes_cache = estado.alarms.clone();
            self.geracao_lida = g;
        }
    }

    /// Avisa o agendador que o estado mudou: ele recalcula a proxima hora.
    fn recarregar_agendador(&self) {
        self.comp.notificar();
    }

    /// Grava o arquivo de dados (so os despertadores do dono entram nele).
    fn salvar(&mut self) {
        if let Err(e) = self.comp.salvar() {
            self.mensagem = Some(format!("Não consegui salvar: {e}"));
            self.mensagem_erro = true;
        }
    }

    // ---------------------------------------------------------------- interface

    fn desenhar(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        if self.tocando.is_some() {
            self.painel_tocando(ui);
            return;
        }

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(Tema::FUNDO)
                    .inner_margin(egui::Margin::same(0)),
            )
            .show(ui, |ui| {
                self.topo(ui);
                // O respiro da página: nada encosta na borda da janela. E o
                // que o dono pediu ("grudados na borda do app sem nenhum
                // espaçamento").
                egui::Frame::new()
                    .inner_margin(egui::Margin::symmetric(
                        ui::MARGEM_PAGINA,
                        ui::MARGEM_PAGINA_VERTICAL,
                    ))
                    .show(ui, |ui| {
                        self.cabecalho_pagina(ui);
                        ui.add_space(ui::espaco::XL);
                        // A altura do conteudo e calculada ANTES do horizontal:
                        // dentro de um `horizontal`, o egui ainda nao sabe
                        // quanto espaco vertical sobrou (no primeiro quadro ele
                        // devolve zero, e a coluna inteira sumia da tela).
                        let altura = ui.available_height();
                        ui.horizontal_top(|ui| {
                            self.coluna_relogio(ui, altura);
                            ui.add_space(ui::espaco::L);
                            self.coluna_alarmes(ui, altura);
                        });
                    });
            });

        if self.editor.is_some() {
            self.painel_editor(&ctx);
        }
        if self.mostrar_config {
            self.painel_config(&ctx);
        }
        self.seletor_de_arquivos(&ctx);
        if let Some(msg) = self.mensagem.clone() {
            self.mensagem_flutuante(&ctx, msg);
        }
    }

    /// A barra do topo: fina, como a barra de título do Tranal Sentinel
    /// (44 px), com o nome do app à esquerda e as ações à direita.
    ///
    /// Não é mais um cartão: é uma faixa com uma linha divisória embaixo,
    /// que separa a navegação do conteúdo.
    fn topo(&mut self, ui: &mut egui::Ui) {
        // A barra tem 44 px (a barra de título do Sentinel): o respiro de
        // cima e de baixo sobra da altura do botão.
        let respiro = ((ui::ALTURA_BARRA - 34.0) / 2.0).round() as i8;
        egui::Frame::new()
            .fill(Tema::FUNDO)
            .inner_margin(egui::Margin::symmetric(ui::MARGEM_PAGINA, respiro))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // O nome do app, como o "TRANAL SENTINEL" do Sentinel.
                    ui.label(
                        egui::RichText::new("TRANAL")
                            .color(Tema::TEXTO)
                            .size(13.0)
                            .strong(),
                    );
                    ui.label(
                        egui::RichText::new(" DESPERTADOR")
                            .color(Tema::TEXTO_FRACO)
                            .size(13.0),
                    );
                    // A página atual, como o `TitlePage` do Sentinel.
                    ui.add_space(ui::espaco::M);
                    ui.label(
                        egui::RichText::new("Despertadores")
                            .color(Tema::TEXTO_FRACO)
                            .size(13.0),
                    );
                    // As ações vão para a ponta direita: o grupo é medido
                    // antes de ser desenhado (ver `ui::acoes_a_direita`).
                    let clicou = ui::acoes_a_direita(
                        ui,
                        &[ui::Acao::nova("Configurações", ui::EstiloBotao::Neutro)],
                    );
                    if clicou.first().copied().unwrap_or(false) {
                        self.mostrar_config = !self.mostrar_config;
                        if self.mostrar_config {
                            self.biblioteca = tones::biblioteca();
                        }
                    }
                });
            });
        // A linha que separa a barra do conteúdo.
        let (linha, _) =
            ui.allocate_exact_size(egui::Vec2::new(ui.available_width(), 1.0), egui::Sense::hover());
        ui.painter().rect_filled(linha, 0.0, Tema::BORDA);
    }

    /// O cabeçalho da página: título, subtítulo e a ação principal.
    ///
    /// O botão "Novo despertador" mora **aqui**, na linha do título, e não
    /// solto embaixo do cartão: era exatamente o que o owner achou "grudado
    /// na borda do app". No Sentinel a página faz o mesmo — H1 e `Sub` à
    /// esquerda, ação principal à direita.
    fn cabecalho_pagina(&mut self, ui: &mut egui::Ui) {
        let alarmes = self.alarms();
        let ativos = alarmes.iter().filter(|a| a.enabled).count();
        let ativos_texto = if ativos == 1 {
            "1 ativo".to_string()
        } else {
            format!("{ativos} ativos")
        };
        let proximo_texto = match self.proximo.as_ref() {
            Some((alarme, _)) => format!("{ativos_texto} · próximo às {}", alarme.time_label()),
            None => format!("{ativos_texto} · nenhum despertador programado"),
        };

        ui.vertical(|ui| {
            // A linha do H1 com a ação principal na ponta direita. O botão
            // fica **centrado** na altura do H1 (o `horizontal` do egui
            // centra os itens na linha), que é o que deixa o texto do botão
            // na mesma altura das letras do título.
            ui.horizontal(|ui| {
                ui::titulo_pagina(ui, "Despertadores");
                ui::empurra_para_direita(
                    ui,
                    ui::largura_botao(ui, "+ Novo despertador", ui::EstiloBotao::Primario),
                );
                if ui::botao_primario(ui, "+ Novo despertador").clicked() {
                    self.abrir_editor(None);
                }
            });
            ui::sub(ui, &proximo_texto);
        });
    }

    fn coluna_relogio(&mut self, ui: &mut egui::Ui, altura: f32) {
        ui.allocate_ui_with_layout(
            egui::Vec2::new(ui::LARGURA_RELOGIO, altura),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui::cartao(ui, |ui| {
                    let agora = Local::now();
                    ui.label(
                        egui::RichText::new(agora.format("%H:%M").to_string())
                            .color(Tema::TEXTO)
                            .size(48.0)
                            .strong()
                            .monospace(),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new(format!(
                            "{}, {} de {}",
                            dia_semana(agora.weekday()),
                            agora.day(),
                            nome_mes(agora.month())
                        ))
                        .color(Tema::TEXTO_FRACO)
                        .size(13.0),
                    );

                    ui.add_space(ui::espaco::M);
                    ui::linha_separadora(ui);
                    ui.add_space(ui::espaco::M);

                    ui::titulo_secao(ui, "próximo despertador");
                    match self.proximo.as_ref() {
                        Some((alarme, quando)) => {
                            ui.label(
                                egui::RichText::new(alarme.time_label())
                                    .color(Tema::ACCENT)
                                    .size(30.0)
                                    .strong()
                                    .monospace(),
                            );
                            ui.add_space(2.0);
                            ui.label(
                                egui::RichText::new(nome_do_alarme(alarme))
                                    .color(Tema::TEXTO)
                                    .size(15.0),
                            );
                            ui.label(
                                egui::RichText::new(schedule::texto_ate(*quando, Local::now()))
                                    .color(Tema::TEXTO_FRACO)
                                    .size(13.0),
                            );
                            ui.add_space(ui::espaco::S);
                            ui::chip(ui, &alarme.repeat.describe(), Tema::ACCENT);
                        }
                        None => {
                            ui.label(
                                egui::RichText::new("Nenhum despertador ativo")
                                    .color(Tema::TEXTO_FRACO)
                                    .size(14.0),
                            );
                            ui::ajuda(ui, "Crie um despertador e ele aparece aqui.");
                        }
                    }

                    // A dica fica dentro do cartão, com respiro: antes ela
                    // encostava na borda de baixo da janela.
                    ui.add_space(ui::espaco::M);
                    ui::linha_separadora(ui);
                    ui.add_space(ui::espaco::S);
                    ui::ajuda(
                        ui,
                        "Fechar a janela minimiza o app e ele continua tocando. Para abrir de novo, clique no ícone dele na bandeja.",
                    );
                });
            },
        );
    }

    fn coluna_alarmes(&mut self, ui: &mut egui::Ui, altura: f32) {
        // Copia para desenhar: a closure precisa de &mut self para montar
        // os cartões, então copiamos o cache (barato: já é local).
        let alarms = self.alarmes_cache.clone();

        ui.allocate_ui_with_layout(
            egui::Vec2::new(ui.available_width(), altura),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([true, false])
                    .show(ui, |ui| {
                        if alarms.is_empty() {
                            ui::cartao(ui, |ui| {
                                ui.label(
                                    egui::RichText::new("Nenhum despertador ainda")
                                        .color(Tema::TEXTO)
                                        .size(15.0)
                                        .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(
                                        "Clique em “Novo despertador” para criar o primeiro.",
                                    )
                                    .color(Tema::TEXTO_FRACO)
                                    .size(13.0),
                                );
                            });
                        }
                        // As ações saem da lista e são aplicadas depois do
                        // ScrollArea, para não segurar o empréstimo de self.
                        let mut acoes: Vec<(String, AcoesAlarme)> = Vec::new();
                        for alarme in alarms.iter() {
                            let resposta = self.cartao_alarme(ui, alarme);
                            acoes.push((alarme.id.clone(), resposta));
                            ui.add_space(ui::espaco::M);
                        }
                        self.acoes_pendentes = acoes;
                    });
            },
        );

        // Aplica o que foi pedido na lista (fora do ScrollArea).
        let acoes = std::mem::take(&mut self.acoes_pendentes);
        for (id, acao) in acoes {
            if acao.alternar {
                let mudou = {
                    let mut g = self.comp.travar();
                    match g.alarms.iter_mut().find(|a| a.id == id) {
                        Some(a) => {
                            a.enabled = !a.enabled;
                            true
                        }
                        None => false,
                    }
                };
                if mudou {
                    self.recarregar_agendador();
                    self.salvar();
                }
            }
            if acao.editar {
                self.abrir_editor(Some(id.clone()));
            }
            if acao.remover {
                {
                    let mut g = self.comp.travar();
                    g.alarms.retain(|a| a.id != id);
                }
                self.recarregar_agendador();
                self.salvar();
                self.mensagem = Some("Despertador apagado.".to_string());
                self.mensagem_erro = false;
            }
            if acao.testar {
                let som = {
                    let g = self.comp.travar();
                    g.alarms
                        .iter()
                        .find(|a| a.id == id)
                        .map(|a| (a.sound.clone(), a.volume))
                };
                if let Some((som, volume)) = som {
                    self.testar_som(som, volume);
                }
            }
            if acao.rearmar {
                {
                    let mut g = self.comp.travar();
                    if let Some(a) = g.alarms.iter_mut().find(|a| a.id == id) {
                        a.last_rung = None;
                        a.enabled = true;
                    }
                }
                self.recarregar_agendador();
                self.salvar();
                self.mensagem = Some("Despertador rearmado para o próximo dia.".to_string());
                self.mensagem_erro = false;
            }
        }
    }

    fn cartao_alarme(&self, ui: &mut egui::Ui, alarme: &Alarm) -> AcoesAlarme {
        let mut acoes = AcoesAlarme {
            editar: false,
            remover: false,
            alternar: false,
            testar: false,
            rearmar: false,
        };
        let destaque = if alarme.enabled {
            Tema::TEXTO
        } else {
            Tema::TEXTO_FRACO
        };

        ui::cartao(ui, |ui| {
            // Linha 1: hora, nome e interruptor.
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(alarme.time_label())
                        .color(if alarme.enabled {
                            Tema::ACCENT
                        } else {
                            Tema::TEXTO_FRACO
                        })
                        .size(34.0)
                        .strong()
                        .monospace(),
                );
                ui.add_space(ui::espaco::S);
                ui.label(
                    egui::RichText::new(nome_do_alarme(alarme))
                        .color(destaque)
                        .size(16.0),
                );
                // O interruptor fica na ponta direita da linha.
                ui::empurra_para_direita(ui, ui::LARGURA_INTERRUPTOR);
                let mut ligado = alarme.enabled;
                if ui::interruptor(ui, &mut ligado).changed() {
                    acoes.alternar = true;
                }
            });

            // Linha 2: as etiquetas, em uma linha **só delas**.
            //
            // Antes elas dividiam a linha com os botões, e o `with_layout`
            // do egui (que ocupa o espaço todo do pai) acabava desenhando os
            // botões **por cima** das etiquetas — era o defeito que o dono
            // apontou na captura.
            ui.add_space(ui::espaco::M);
            ui.horizontal_wrapped(|ui| {
                ui::chip(ui, &alarme.repeat.describe(), Tema::ACCENT);
                ui::chip(ui, &nome_som(&alarme.sound), Tema::TEXTO_FRACO);
                if alarme.force_device_volume {
                    ui::chip(ui, "100% no aparelho", Tema::TEXTO_FRACO);
                }
                if alarme.dismiss.desafio != Desafio::Nenhum {
                    ui::chip(ui, "desafio", Tema::TEXTO_FRACO);
                }
            });

            // Linha 3: as ações, encostadas na borda direita do cartão.
            ui.add_space(ui::espaco::M);
            let mut alvos: Vec<(AcaoCartao, ui::Acao)> = Vec::new();
            // D9: alarme "uma vez" que ja tocou pode ser rearmado
            // para o dia seguinte sem precisar recriar o alarme.
            if matches!(alarme.repeat, RepeatRule::Once) && alarme.last_rung.is_some() {
                alvos.push((
                    AcaoCartao::Repetir,
                    ui::Acao::nova("Repetir amanhã", ui::EstiloBotao::Neutro),
                ));
            }
            alvos.push((
                AcaoCartao::Editar,
                ui::Acao::nova("Editar", ui::EstiloBotao::Neutro),
            ));
            alvos.push((
                AcaoCartao::Testar,
                ui::Acao::nova("Testar", ui::EstiloBotao::Neutro),
            ));
            alvos.push((
                AcaoCartao::Excluir,
                ui::Acao::nova("×", ui::EstiloBotao::Icone).com_dica("Excluir"),
            ));
            let lista: Vec<ui::Acao> = alvos.iter().map(|(_, a)| a.clone()).collect();
            // O grupo fica numa **linha horizontal**: é nela que o espaço
            // sobrando empurra os botões para a direita do cartão.
            ui.horizontal(|ui| {
                let clicados = ui::acoes_a_direita(ui, &lista);
                for ((alvo, _), clicado) in alvos.iter().zip(clicados.iter()) {
                    if !*clicado {
                        continue;
                    }
                    match alvo {
                        AcaoCartao::Repetir => acoes.rearmar = true,
                        AcaoCartao::Editar => acoes.editar = true,
                        AcaoCartao::Testar => acoes.testar = true,
                        AcaoCartao::Excluir => acoes.remover = true,
                    }
                }
            });
        });
        acoes
    }

    fn mensagem_flutuante(&mut self, ctx: &egui::Context, texto: String) {
        let cor = if self.mensagem_erro {
            Tema::VERMELHO
        } else {
            Tema::VERDE
        };
        egui::Window::new("aviso")
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-16.0, 70.0))
            .show(ctx, |ui| {
                ui::cartao(ui, |ui| {
                    ui.label(egui::RichText::new(texto).color(cor).size(13.0));
                });
            });
        self.mensagem = None;
    }

    // ------------------------------------------------------------------ editor

    fn abrir_editor(&mut self, id: Option<String>) {
        let alarms = self.alarms();
        let padrao = self.comp.travar().settings.alarme_padrao.clone();
        let (rascunho, dias) = match &id {
            Some(id) => match alarms.iter().find(|a| a.id == *id) {
                Some(a) => {
                    let dias = match &a.repeat {
                        RepeatRule::Weekly { days } => days.clone(),
                        _ => vec![1, 2, 3, 4, 5, 6, 7],
                    };
                    (a.clone(), dias)
                }
                None => return,
            },
            // Alarme novo: já vem preenchido com o despertador padrão das
            // Configurações, e o dono edita o que quiser.
            None => {
                let novo = Alarm::a_partir_do_padrao(&padrao);
                (novo, vec![1, 2, 3, 4, 5, 6, 7])
            }
        };

        let volumes_saidas = self.volumes_saidas.clone();

        let hora = rascunho.hour;
        let minuto = rascunho.minute;
        self.editor = Some(Editor {
            id_original: id,
            texto_hora: hora::formatar(hora, minuto),
            hora,
            minuto,
            rascunho,
            dias,
            erro_hora: None,
            duracao_som: None,
            volumes_saidas,
            biblioteca: self.biblioteca.clone(),
        });
    }

    /// O editor: uma janela no centro da tela, com tudo que o dono precisa.
    fn painel_editor(&mut self, ctx: &egui::Context) {
        // O editor sai do `self` durante o desenho: assim os widgets podem
        // ler as saídas e escrever no rascunho sem brigar com o empréstimo.
        let Some(mut editor) = self.editor.take() else {
            return;
        };
        let saidas = self.saidas.clone();

        let mut fechar = false;
        let mut salvar = false;
        let mut excluir = false;
        let mut testar = false;
        let mut salvar_como_padrao = false;
        let mut abrir_seletor = false;

        egui::Modal::new(egui::Id::unique("tranal-editor-de-despertador"))
            .frame(
                egui::Frame::new()
                    .fill(Tema::PAINEL)
                    .stroke(egui::Stroke::new(1.0, Tema::BORDA))
                    .corner_radius(ui::raio(ui::raio::CARTAO))
                    .inner_margin(egui::Margin::same(20)),
            )
            .show(ctx, |ui| {
                ui.set_max_width(600.0);

                // Cabeçalho do editor.
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(
                            if editor.id_original.is_some() {
                                "Editar despertador"
                            } else {
                                "Novo despertador"
                            },
                        )
                        .color(Tema::TEXTO)
                        .size(19.0)
                        .strong(),
                    );
                    // O "Fechar" fica na ponta direita da linha do título.
                    ui::empurra_para_direita(
                        ui,
                        ui::largura_botao(ui, "Fechar", ui::EstiloBotao::Fantasma),
                    );
                    if ui::botao_fantasma(ui, "Fechar").clicked() {
                        fechar = true;
                    }
                });
                ui.add_space(ui::espaco::M);

                egui::ScrollArea::vertical()
                    .max_height(500.0)
                    .auto_shrink([true, false])
                    .show(ui, |ui| {
                        // ------------------------------------------------ nome
                        ui::titulo_secao(ui, "nome");
                        ui.add_space(ui::espaco::XS);
                        ui::campo_texto(
                            ui,
                            &mut editor.rascunho.label,
                            "Ex.: acordar para o café",
                        );

                        // ---------------------------------------------- horário
                        ui.add_space(ui::espaco::L);
                        ui::titulo_secao(ui, "horário");
                        ui.add_space(ui::espaco::XS);
                        ui.horizontal(|ui| {
                            campo_hora_editavel(
                                ui,
                                &mut editor.hora,
                                &mut editor.minuto,
                                &mut editor.texto_hora,
                                &mut editor.erro_hora,
                                egui::Id::unique("campo-hora-editor"),
                            );
                            ui.add_space(ui::espaco::S);
                            ui.vertical(|ui| {
                                ui.horizontal(|ui| {
                                    if ui::botao_neutro(ui, "−1h").on_hover_text("Uma hora a menos").clicked() {
                                        mover_hora(&mut editor, -60);
                                    }
                                    if ui::botao_neutro(ui, "+1h").on_hover_text("Uma hora a mais").clicked() {
                                        mover_hora(&mut editor, 60);
                                    }
                                });
                                ui.horizontal(|ui| {
                                    if ui::botao_neutro(ui, "−5m").on_hover_text("Cinco minutos a menos").clicked()
                                    {
                                        mover_hora(&mut editor, -5);
                                    }
                                    if ui::botao_neutro(ui, "+5m").on_hover_text("Cinco minutos a mais").clicked()
                                    {
                                        mover_hora(&mut editor, 5);
                                    }
                                });
                            });
                        });
                        if let Some(erro) = &editor.erro_hora {
                            ui.label(egui::RichText::new(erro).color(Tema::VERMELHO).size(12.0));
                        } else {
                            ui::ajuda(ui, "Digite o horário por cima (0730 ou 07:30) ou use os botões.");
                        }

                        // -------------------------------------------- repetição
                        ui.add_space(ui::espaco::L);
                        ui::titulo_secao(ui, "repetição");
                        ui.add_space(ui::espaco::XS);
                        let opcoes = [
                            ui::Segmento::novo("Uma vez"),
                            ui::Segmento::novo("Todo dia"),
                            ui::Segmento::novo("Dias úteis")
                                .com_dica("Segunda a sexta, sem marcar nada"),
                            ui::Segmento::novo("Semanal").com_dica("Você marca os dias da semana"),
                        ];
                        let atual = repeticao_indice(&editor.rascunho.repeat);
                        if let Some(escolha) = ui::seletor_segmentado(ui, &opcoes, atual) {
                            editor.rascunho.repeat = match escolha {
                                0 => RepeatRule::Once,
                                1 => RepeatRule::Daily,
                                2 => RepeatRule::Weekdays,
                                _ => RepeatRule::Weekly {
                                    days: editor.dias.clone(),
                                },
                            };
                        }
                        ui.add_space(ui::espaco::S);
                        match &editor.rascunho.repeat {
                            RepeatRule::Weekdays => ui::ajuda(
                                ui,
                                "Dias úteis = de segunda a sexta, todo dia. Não precisa marcar nada.",
                            ),
                            RepeatRule::Weekly { .. } => {
                                ui::ajuda(
                                    ui,
                                    "Semanal = só nos dias que você marcar abaixo.",
                                );
                                ui.add_space(ui::espaco::S);
                                let nomes = ["Seg", "Ter", "Qua", "Qui", "Sex", "Sáb", "Dom"];
                                let marcados: Vec<bool> = (1u8..=7)
                                    .map(|d| editor.dias.contains(&d))
                                    .collect();
                                if let Some(dia) = ui::seletor_multiplo(ui, &nomes, &marcados) {
                                    let dia = dia as u8 + 1;
                                    if let Some(pos) = editor.dias.iter().position(|d| *d == dia) {
                                        editor.dias.remove(pos);
                                    } else {
                                        editor.dias.push(dia);
                                    }
                                    editor.dias.sort_unstable();
                                    editor.rascunho.repeat = RepeatRule::Weekly {
                                        days: editor.dias.clone(),
                                    };
                                }
                            }
                            _ => ui::ajuda(ui, "Dispara uma vez só, na hora marcada."),
                        }

                        // ---------------------------------------------------- som
                        ui.add_space(ui::espaco::L);
                        ui::titulo_secao(ui, "som");
                        ui.add_space(ui::espaco::XS);
                        let presets = Preset::all();
                        let som_atual = match &editor.rascunho.sound {
                            SoundChoice::Preset(p) => presets.iter().position(|x| x == p),
                            SoundChoice::Arquivo { .. } => None,
                        };
                        let segmentos: Vec<ui::Segmento> = presets
                            .iter()
                            .map(|p| ui::Segmento::novo(p.label()))
                            .collect();
                        if let Some(escolha) =
                            ui::seletor_segmentado(ui, &segmentos, som_atual.unwrap_or(usize::MAX))
                        {
                            editor.rascunho.sound = SoundChoice::Preset(presets[escolha]);
                        }
                        if let SoundChoice::Preset(p) = editor.rascunho.sound {
                            ui::ajuda(ui, p.description());
                        }

                        ui.add_space(ui::espaco::S);
                        ui.horizontal_wrapped(|ui| {
                            if ui::botao_neutro(ui, "»  Ouvir o som").clicked() {
                                testar = true;
                            }
                            if ui::botao_neutro(ui, "+  Adicionar um áudio").clicked() {
                                abrir_seletor = true;
                            }
                        });

                        // Os áudios que o dono já adicionou.
                        if !editor.biblioteca.is_empty() {
                            ui.add_space(ui::espaco::S);
                            ui::titulo_secao(ui, "meus áudios");
                            ui.add_space(ui::espaco::XS);
                            for audio in editor.biblioteca.clone() {
                                let marcado = matches!(
                                    &editor.rascunho.sound,
                                    SoundChoice::Arquivo { caminho } if *caminho == audio.caminho
                                );
                                ui.horizontal(|ui| {
                                    let resp = if marcado {
                                        ui::botao_primario(ui, &audio.nome)
                                    } else {
                                        ui::botao_neutro(ui, &audio.nome)
                                    };
                                    if resp.clicked() {
                                        editor.rascunho.sound = SoundChoice::Arquivo {
                                            caminho: audio.caminho.clone(),
                                        };
                                        editor.duracao_som = crate::tones::ler_audio(&audio.caminho)
                                            .ok()
                                            .map(|f| f.duracao_s());
                                    }
                                });
                            }
                        }
                        if let SoundChoice::Arquivo { caminho } = &editor.rascunho.sound {
                            let nome = PathBuf::from(caminho)
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            match editor.duracao_som {
                                Some(d) => ui::ajuda(ui, &format!("Som escolhido: {nome} ({d:.1}s)")),
                                None => ui::ajuda(ui, &format!("Som escolhido: {nome}")),
                            }
                        }

                        // ------------------------------------------------ volume
                        ui.add_space(ui::espaco::L);
                        ui::titulo_secao(ui, "volume");
                        ui.add_space(ui::espaco::XS);
                        let mut volume = editor.rascunho.volume as i32;
                        ui::slider_inteiro(ui, &mut volume, 0, 100, "volume do alarme", "%");
                        editor.rascunho.volume = volume.clamp(0, 100) as u8;

                        let mut fade = editor.rascunho.fade_in_secs as i32;
                        ui::slider_inteiro(ui, &mut fade, 0, 120, "subir o volume devagar", "s");
                        editor.rascunho.fade_in_secs = fade.clamp(0, 120) as u32;
                        ui::ajuda(
                            ui,
                            "Com 0 s o som começa no volume cheio. Com 30 s ele sobe aos poucos.",
                        );

                        ui.horizontal(|ui| {
                            let mut forcar = editor.rascunho.force_device_volume;
                            ui::interruptor(ui, &mut forcar);
                            ui.label(
                                egui::RichText::new("Forçar 100% no aparelho (saída do Windows)")
                                    .color(Tema::TEXTO)
                                    .size(13.0),
                            );
                            editor.rascunho.force_device_volume = forcar;
                        });

                        // -------------------------------------------- onde tocar
                        ui.add_space(ui::espaco::L);
                        ui::titulo_secao(ui, "onde tocar");
                        ui.add_space(ui::espaco::XS);
                        ui.horizontal_wrapped(|ui| {
                            for saida in saidas.clone() {
                                let marcado = editor.rascunho.output_devices.contains(&saida.nome);
                                let rotulo = if saida.padrao {
                                    format!("{} (padrão)", saida.nome)
                                } else {
                                    saida.nome.clone()
                                };
                                let resp = if marcado {
                                    ui::botao_primario(ui, &rotulo)
                                } else {
                                    ui::botao_neutro(ui, &rotulo)
                                };
                                if resp.clicked() {
                                    if let Some(pos) = editor
                                        .rascunho
                                        .output_devices
                                        .iter()
                                        .position(|n| *n == saida.nome)
                                    {
                                        editor.rascunho.output_devices.remove(pos);
                                    } else {
                                        editor.rascunho.output_devices.push(saida.nome.clone());
                                    }
                                }
                            }
                        });
                        if editor.rascunho.output_devices.is_empty() {
                            ui::ajuda(
                                ui,
                                "Nada marcado: o alarme toca na saída padrão do Windows (a que o Windows usa). Marque mais saídas para tocar em várias ao mesmo tempo.",
                            );
                        } else {
                            ui::ajuda(
                                ui,
                                "O alarme toca só nas saídas marcadas. Desmarque todas para voltar à saída padrão.",
                            );
                            for (nome, volume) in editor.volumes_saidas.clone() {
                                if editor.rascunho.output_devices.contains(&nome) {
                                    ui::ajuda(
                                        ui,
                                        &format!("{nome}: volume no Windows hoje está em {volume}%"),
                                    );
                                }
                            }
                        }

                        // ------------------------------------------ como desligar
                        ui.add_space(ui::espaco::L);
                        ui::titulo_secao(ui, "como desligar");
                        ui.add_space(ui::espaco::XS);
                        ui.horizontal(|ui| {
                            let mut soneca = editor.rascunho.dismiss.soneca;
                            ui::interruptor(ui, &mut soneca);
                            ui.label(
                                egui::RichText::new("Liberar soneca")
                                    .color(Tema::TEXTO)
                                    .size(13.0),
                            );
                            editor.rascunho.dismiss.soneca = soneca;
                        });
                        if editor.rascunho.dismiss.soneca {
                            let mut minutos = editor.rascunho.dismiss.soneca_minutos as i32;
                            ui::slider_inteiro(
                                ui,
                                &mut minutos,
                                1,
                                60,
                                "minutos de soneca",
                                " min",
                            );
                            editor.rascunho.dismiss.soneca_minutos = minutos.clamp(1, 60) as u32;
                            let mut max = editor.rascunho.dismiss.max_sonecas as i32;
                            ui::slider_inteiro(ui, &mut max, 1, 10, "quantas sonecas", " vezes");
                            editor.rascunho.dismiss.max_sonecas = max.clamp(1, 10) as u32;
                        }
                        ui.add_space(ui::espaco::S);
                        ui.label(
                            egui::RichText::new("Desafio para desligar")
                                .color(Tema::TEXTO)
                                .size(13.0),
                        );
                        let desafios = [
                            Desafio::Nenhum,
                            Desafio::Digitar,
                            Desafio::Conta,
                        ];
                        let segmentos: Vec<ui::Segmento> =
                            desafios.iter().map(|d| ui::Segmento::novo(d.label())).collect();
                        let atual = desafios
                            .iter()
                            .position(|d| *d == editor.rascunho.dismiss.desafio)
                            .unwrap_or(0);
                        if let Some(escolha) = ui::seletor_segmentado(ui, &segmentos, atual) {
                            editor.rascunho.dismiss.desafio = desafios[escolha];
                        }
                    });
                ui.add_space(ui::espaco::L);
                ui::linha_separadora(ui);
                ui.add_space(ui::espaco::M);
                ui.horizontal(|ui| {
                    if ui::botao_primario(ui, "Salvar despertador").clicked() {
                        salvar = true;
                    }
                    if ui::botao_neutro(ui, "Cancelar").clicked() {
                        fechar = true;
                    }
                    if ui::botao_fantasma(ui, "Salvar como padrão").clicked() {
                        salvar_como_padrao = true;
                    }
                    // O "Excluir" fica na ponta direita da linha.
                    if editor.id_original.is_some() {
                        ui::empurra_para_direita(
                            ui,
                            ui::largura_botao(ui, "Excluir", ui::EstiloBotao::Fantasma),
                        );
                        if ui::botao_fantasma(ui, "Excluir").clicked() {
                            excluir = true;
                        }
                    }
                });
            });

        if abrir_seletor {
            self.destino_selecao = DestinoSelecao::Editor;
            self.seletor = Some(arquivos::Seletor::novo(pasta_inicial()));
        }
        if salvar_como_padrao {
            let mut padrao = editor.rascunho.clone();
            padrao.enabled = true;
            {
                let mut g = self.comp.travar();
                g.settings.alarme_padrao = padrao;
            }
            self.salvar();
            self.mensagem = Some("Este despertador virou o padrão dos novos.".to_string());
            self.mensagem_erro = false;
        }
        if salvar {
            self.salvar_editor(editor);
            return;
        }
        if excluir {
            if let Some(id) = editor.id_original.clone() {
                {
                    let mut g = self.comp.travar();
                    g.alarms.retain(|a| a.id != id);
                }
                self.recarregar_agendador();
                self.salvar();
                self.mensagem = Some("Despertador apagado.".to_string());
                self.mensagem_erro = false;
            }
            return;
        }
        if testar {
            let som = editor.rascunho.sound.clone();
            // D14: a previa usa o volume que o dono ajustou no editor.
            let volume = editor.rascunho.volume;
            self.testar_som(som, volume);
        }
        if fechar {
            return;
        }
        self.editor = Some(editor);
    }

    fn salvar_editor(&mut self, mut editor: Editor) {
        editor.rascunho.hour = editor.hora.clamp(0, 23);
        editor.rascunho.minute = editor.minuto.clamp(0, 59);
        if matches!(editor.rascunho.repeat, RepeatRule::Weekly { .. }) {
            editor.rascunho.repeat = RepeatRule::Weekly {
                days: editor.dias.clone(),
            };
        }
        {
            let mut g = self.comp.travar();
            match &editor.id_original {
                Some(id) => {
                    if let Some(a) = g.alarms.iter_mut().find(|a| a.id == *id) {
                        *a = editor.rascunho.clone();
                    } else {
                        g.alarms.push(editor.rascunho.clone());
                    }
                }
                None => g.alarms.push(editor.rascunho.clone()),
            }
        }
        self.recarregar_agendador();
        self.salvar();
        self.mensagem = Some("Despertador salvo.".to_string());
        self.mensagem_erro = false;
    }

    /// O seletor de arquivos do app (não o explorer do Windows).
    fn seletor_de_arquivos(&mut self, ctx: &egui::Context) {
        let Some(mut seletor) = self.seletor.take() else {
            return;
        };
        let escolha = arquivos::mostrar(ctx, &mut seletor);
        match escolha {
            Some(origem) => {
                match tones::importar_audio(&origem) {
                    Ok(caminho) => {
                        self.biblioteca = tones::biblioteca();
                        match self.destino_selecao {
                            DestinoSelecao::Editor => {
                                if let Some(editor) = self.editor.as_mut() {
                                    editor.biblioteca = self.biblioteca.clone();
                                    editor.rascunho.sound = SoundChoice::Arquivo {
                                        caminho: caminho.clone(),
                                    };
                                    editor.duracao_som =
                                        tones::ler_audio(&caminho).ok().map(|f| f.duracao_s());
                                }
                            }
                            DestinoSelecao::Biblioteca => {}
                        }
                        self.mensagem = Some("Áudio adicionado à biblioteca do app.".to_string());
                        self.mensagem_erro = false;
                    }
                    Err(e) => {
                        self.mensagem = Some(e);
                        self.mensagem_erro = true;
                    }
                }
            }
            None => {
                if seletor.aberto {
                    self.seletor = Some(seletor);
                }
            }
        }
    }

    // ------------------------------------------------------------------ som

    fn testar_som(&mut self, som: SoundChoice, volume: u8) {
        match audio::testar(&som, volume) {
            Ok(mut tocador) => {
                std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_secs(4));
                    tocador.parar();
                });
                self.mensagem = Some("Testando o som...".to_string());
                self.mensagem_erro = false;
            }
            Err(e) => {
                self.mensagem = Some(format!("Não consegui tocar: {e}"));
                self.mensagem_erro = true;
            }
        }
    }

    /// Mostra o painel do alarme. O som ja comecou na thread do agendador.
    ///
    /// O "last_rung" e gravado por quem disparou (o agendador), e so quando o
    /// som realmente saiu: se falhou, o alarme continua pendente e o
    /// agendador tenta de novo.
    fn mostrar_alarme(&mut self, id: &str, tocou: bool, avisos: Vec<String>) {
        let alarme = match self.comp.travar().por_id(id) {
            Some(a) => a.clone(),
            None => return,
        };

        self.tocando = Some(Tocando {
            palavra: "ACORDAR".to_string(),
            alarme: alarme.clone(),
            sonecas_usadas: alarme.sonecas_usadas,
            inicio: Instant::now(),
            conta: (7, 5),
            resposta: String::new(),
        });
        self.alerta = if tocou {
            None
        } else {
            Some(format!(
                "O som ainda nao saiu. O app tenta de novo a cada 5 segundos.\n{}",
                avisos.join(" · ")
            ))
        };

        if !avisos.is_empty() {
            self.mensagem = Some(avisos.join(" · "));
            self.mensagem_erro = !tocou;
        }
    }

    fn parar_alarme(&mut self, restaurar_volume: bool) {
        audio::parar_global();
        // O dono dispensou: o agendador para de tentar tocar e o alarme fica
        // marcado como atendido (nao toca de novo hoje).
        let id = self.tocando.as_ref().map(|t| t.alarme.id.clone());
        match id {
            Some(id) => self.comp.dispensar(&id),
            None => self.comp.pedir_parada(),
        }
        if restaurar_volume
            && let Some(t) = &self.tocando
            && t.alarme.force_device_volume
        {
            device_volume::restaurar(&t.alarme.output_devices);
        }
        self.tocando = None;
        self.alerta = None;
    }

    fn sonecar(&mut self) {
        let Some(tocando) = self.tocando.take() else {
            return;
        };
        audio::parar_global();
        let minutos = tocando.alarme.dismiss.soneca_minutos.max(1);
        let quando = Local::now() + chrono::Duration::minutes(minutos as i64);
        let mut soneca = tocando.alarme.clone();
        soneca.id = model::novo_id();
        soneca.hour = quando.hour() as u8;
        soneca.minute = quando.minute() as u8;
        soneca.repeat = RepeatRule::Once;
        soneca.last_rung = None;
        // D8: a soneca carrega quantas sonecas ja foram usadas, senao o limite
        // do alarme original nunca seria alcancado.
        soneca.sonecas_usadas = tocando.sonecas_usadas;
        soneca.label = format!("{} (soneca)", soneca.label);
        // A soneca entra no estado compartilhado: e o agendador que ve a
        // fila nova, e ela fica na sessao (nao vai para o arquivo).
        self.comp.adicionar_soneca(soneca, &tocando.alarme.id);
        self.mensagem = Some(format!(
            "Soneca: toca de novo às {}.",
            quando.format("%H:%M")
        ));
        self.mensagem_erro = false;
    }

    // -------------------------------------------------------- painel tocando

    fn painel_tocando(&mut self, ui: &mut egui::Ui) {
        self.piscar += 0.06;
        let pulso = (self.piscar.sin() + 1.0) / 2.0;

        let Some(tocando) = &mut self.tocando else {
            return;
        };
        let alarme = tocando.alarme.clone();
        let sonecas_usadas = tocando.sonecas_usadas;
        let pode_sonecar = alarme.dismiss.soneca && sonecas_usadas < alarme.dismiss.max_sonecas;
        let desafio = alarme.dismiss.desafio;
        let palavra = tocando.palavra.clone();
        let conta = tocando.conta;
        let resposta = &mut tocando.resposta;
        let segundos = segundos_decorridos(tocando.inicio);
        let alerta = self.alerta.clone();

        let mut parar = false;
        let mut sonecar = false;

        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Tema::FUNDO))
            .show(ui, |ui| {
                let painter = ui.painter();
                let centro = ui.max_rect().center();
                // Anel pulsante ao centro.
                for i in 0..3 {
                    let raio = 150.0 + i as f32 * 26.0 + pulso * 8.0;
                    painter.circle_stroke(
                        centro,
                        raio,
                        egui::Stroke::new(
                            2.0,
                            Tema::ACCENT.gamma_multiply(0.55 - i as f32 * 0.15),
                        ),
                    );
                }

                ui.vertical_centered(|ui| {
                    ui.add_space(60.0);
                    ui.label(
                        egui::RichText::new("DESPERTADOR")
                            .color(Tema::ACCENT)
                            .size(13.0)
                            .strong(),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(alarme.time_label())
                            .color(Tema::TEXTO)
                            .size(96.0)
                            .strong()
                            .monospace(),
                    );
                    ui.label(
                        egui::RichText::new(nome_do_alarme(&alarme))
                            .color(Tema::TEXTO_FRACO)
                            .size(16.0),
                    );
                    ui.add_space(18.0);

                    // Aviso persistente: o alarme esta na tela mas o som nao
                    // saiu. Fica visivel ate o som funcionar ou o dono parar.
                    if let Some(alerta) = &alerta {
                        egui::Frame::new()
                            .fill(Tema::VERMELHO.gamma_multiply(0.16))
                            .stroke(egui::Stroke::new(1.0, Tema::VERMELHO))
                            .inner_margin(egui::Margin::symmetric(14, 10))
                            .corner_radius(8.0)
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(alerta)
                                        .color(Tema::VERMELHO)
                                        .size(13.0),
                                );
                            });
                        ui.add_space(12.0);
                    }

                    match desafio {
                        Desafio::Nenhum => {}
                        Desafio::Digitar => {
                            ui.label(
                                egui::RichText::new(format!("Digite a palavra: {palavra}"))
                                    .color(Tema::TEXTO)
                                    .size(14.0),
                            );
                            ui.add(
                                egui::TextEdit::singleline(resposta)
                                    .desired_width(320.0)
                                    .hint_text("digite aqui"),
                            );
                        }
                        Desafio::Conta => {
                            ui.label(
                                egui::RichText::new(format!("Quanto é {} + {}?", conta.0, conta.1))
                                    .color(Tema::TEXTO)
                                    .size(15.0),
                            );
                            ui.add(
                                egui::TextEdit::singleline(resposta)
                                    .desired_width(220.0)
                                    .hint_text("resposta"),
                            );
                        }
                    }

                    ui.add_space(18.0);

                    // Botoes empilhados e centralizados: em layout horizontal
                    // o egui FORCA o alinhamento a esquerda (o `main_align` e
                    // ignorado em `next_frame_ignore_wrap`), e a linha de
                    // botoes ficava colada no canto da janela. Empilhados, cada
                    // botao e centralizado como os rotulos.
                    if pode_sonecar
                        && ui::botao_neutro(
                            ui,
                            &format!("Soneca ({})", minutos_soneca(&alarme)),
                        )
                        .clicked()
                    {
                        sonecar = true;
                    }
                    let precisa_desafio = desafio != Desafio::Nenhum;
                    let resposta_certa = match desafio {
                        Desafio::Nenhum => true,
                        Desafio::Digitar => resposta.trim().eq_ignore_ascii_case(&palavra),
                        Desafio::Conta => {
                            resposta.trim().parse::<i32>().ok() == Some(conta.0 + conta.1)
                        }
                    };
                    if precisa_desafio && !resposta_certa {
                        ui::botao_neutro(ui, "Responda o desafio para parar");
                    } else if ui::botao_perigo(ui, "Parar o alarme").clicked() {
                        parar = true;
                    }

                    ui.add_space(8.0);
                    // Quantas saidas estao tocando de verdade agora.
                    let saidas_tocando = audio::tocando_global().len();
                    ui.label(
                        egui::RichText::new(format!(
                            "Tocando há {segundos} segundos em {} saída(s)",
                            if saidas_tocando > 0 {
                                saidas_tocando
                            } else if alarme.output_devices.is_empty() {
                                1
                            } else {
                                alarme.output_devices.len()
                            }
                        ))
                        .color(Tema::TEXTO_FRACO)
                        .size(12.0),
                    );
                });
            });

        if parar {
            self.parar_alarme(true);
        }
        if sonecar {
            if let Some(t) = &mut self.tocando {
                t.sonecas_usadas += 1;
            }
            self.sonecar();
        }
        // A tela do alarme fica animando enquanto toca. Repintar a 20 fps
        // (em vez de continuously) mantem o anel suave e nao queima CPU.
        ui.ctx().request_repaint_after(Duration::from_millis(50));
    }

    // --------------------------------------------------------- configurações

    fn painel_config(&mut self, ctx: &egui::Context) {
        // As configurações são editadas numa cópia e gravadas no estado
        // compartilhado no fim: a interface nunca segura o travamento do
        // estado enquanto desenha.
        let mut settings = self.comp.travar().settings.clone();
        let biblioteca = self.biblioteca.clone();
        let saidas = self.saidas.clone();

        let mut fechar = false;
        let mut abrir_seletor = false;

        egui::Modal::new(egui::Id::unique("tranal-configuracoes"))
            .frame(
                egui::Frame::new()
                    .fill(Tema::PAINEL)
                    .stroke(egui::Stroke::new(1.0, Tema::BORDA))
                    .corner_radius(ui::raio(ui::raio::CARTAO))
                    .inner_margin(egui::Margin::same(20)),
            )
            .show(ctx, |ui| {
                ui.set_max_width(600.0);
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("Configurações")
                            .color(Tema::TEXTO)
                            .size(19.0)
                            .strong(),
                    );
                    // O "Fechar" fica na ponta direita da linha do título.
                    ui::empurra_para_direita(
                        ui,
                        ui::largura_botao(ui, "Fechar", ui::EstiloBotao::Fantasma),
                    );
                    if ui::botao_fantasma(ui, "Fechar").clicked() {
                        fechar = true;
                    }
                });
                ui.add_space(ui::espaco::M);

                egui::ScrollArea::vertical()
                    .max_height(470.0)
                    .auto_shrink([true, false])
                    .show(ui, |ui| {
                        // ------------------------------------------------- geral
                        ui::titulo_secao(ui, "geral");
                        ui.add_space(ui::espaco::S);
                        linha_de_interruptor(
                            ui,
                            &mut settings.iniciar_com_o_windows,
                            "Iniciar junto com o Windows",
                        );
                        linha_de_interruptor(
                            ui,
                            &mut settings.iniciar_minimizado,
                            "Abrir minimizado quando o PC liga",
                        );
                        linha_de_interruptor(
                            ui,
                            &mut settings.sair_ao_fechar,
                            "Encerrar o app quando a janela é fechada",
                        );
                        ui::ajuda(
                            ui,
                            "Desligado: fechar a janela minimiza o app, que continua tocando. Para abrir de novo, clique no ícone dele na barra de tarefas.",
                        );

                        // ---------------------------------- despertador padrão
                        ui.add_space(ui::espaco::L);
                        ui::titulo_secao(ui, "despertador padrão");
                        ui.add_space(ui::espaco::S);
                        ui::ajuda(
                            ui,
                            "É o que já vem preenchido quando você clica em “Novo despertador”. Você pode mudar tudo depois.",
                        );

                        let mut hora = settings.alarme_padrao.hour;
                        let mut minuto = settings.alarme_padrao.minute;
                        let mut texto_hora = hora::formatar(hora, minuto);
                        let mut erro_hora: Option<String> = None;
                        ui.horizontal(|ui| {
                            campo_hora_editavel(
                                ui,
                                &mut hora,
                                &mut minuto,
                                &mut texto_hora,
                                &mut erro_hora,
                                egui::Id::unique("campo-hora-padrao"),
                            );
                            ui.add_space(ui::espaco::S);
                            let presets = Preset::all();
                            let som_atual = match &settings.alarme_padrao.sound {
                                SoundChoice::Preset(p) => presets.iter().position(|x| x == p),
                                SoundChoice::Arquivo { .. } => None,
                            };
                            let segmentos: Vec<ui::Segmento> = presets
                                .iter()
                                .map(|p| ui::Segmento::novo(p.label()))
                                .collect();
                            if let Some(escolha) = ui::seletor_segmentado(
                                ui,
                                &segmentos,
                                som_atual.unwrap_or(usize::MAX),
                            ) {
                                settings.alarme_padrao.sound = SoundChoice::Preset(presets[escolha]);
                            }
                        });
                        if let Some(erro) = &erro_hora {
                            ui.label(egui::RichText::new(erro).color(Tema::VERMELHO).size(12.0));
                        }
                        settings.alarme_padrao.hour = hora;
                        settings.alarme_padrao.minute = minuto;

                        let mut volume = settings.alarme_padrao.volume as i32;
                        ui::slider_inteiro(ui, &mut volume, 0, 100, "volume do alarme", "%");
                        settings.alarme_padrao.volume = volume.clamp(0, 100) as u8;

                        let mut soneca = settings.alarme_padrao.dismiss.soneca;
                        linha_de_interruptor(ui, &mut soneca, "Liberar soneca");
                        settings.alarme_padrao.dismiss.soneca = soneca;
                        if soneca {
                            let mut minutos = settings.alarme_padrao.dismiss.soneca_minutos as i32;
                            ui::slider_inteiro(
                                ui,
                                &mut minutos,
                                1,
                                60,
                                "minutos de soneca",
                                " min",
                            );
                            settings.alarme_padrao.dismiss.soneca_minutos =
                                minutos.clamp(1, 60) as u32;
                        }

                        // ------------------------------------------ biblioteca
                        ui.add_space(ui::espaco::L);
                        ui::titulo_secao(ui, "biblioteca de sons");
                        ui.add_space(ui::espaco::S);
                        ui::ajuda(
                            ui,
                            "Os áudios que você adiciona ficam guardados na pasta do app (o arquivo é copiado, então o despertador continua tocando mesmo se você mover o original).",
                        );
                        ui.add_space(ui::espaco::S);
                        if biblioteca.is_empty() {
                            ui::ajuda(ui, "Nenhum áudio seu por aqui. Use os sons do app ou adicione o seu.");
                        } else {
                            for audio in biblioteca.iter() {
                                ui.label(
                                    egui::RichText::new(format!("  {}", audio.nome))
                                        .color(Tema::TEXTO_FRACO)
                                        .size(13.0),
                                );
                            }
                        }
                        ui.add_space(ui::espaco::S);
                        ui.horizontal(|ui| {
                            if ui::botao_neutro(ui, "+  Adicionar um áudio").clicked() {
                                abrir_seletor = true;
                            }
                            if ui::botao_fantasma(ui, "Abrir a pasta").clicked() {
                                let pasta = storage::pasta_sons();
                                let _ = std::process::Command::new("explorer")
                                    .arg(&pasta)
                                    .spawn();
                            }
                        });

                        // ------------------------------------------------- dados
                        ui.add_space(ui::espaco::L);
                        ui::titulo_secao(ui, "dados");
                        ui.add_space(ui::espaco::S);
                        ui::ajuda(
                            ui,
                            &format!("Arquivo: {}", storage::caminho_dados().display()),
                        );
                        ui::ajuda(ui, &format!("Sons: {}", storage::pasta_sons().display()));
                        ui::ajuda(
                            ui,
                            &format!("Saídas de áudio encontradas: {}", saidas.len()),
                        );
                    });

                ui.add_space(ui::espaco::M);
                ui.horizontal(|ui| {
                    if ui::botao_primario(ui, "Fechar configurações").clicked() {
                        fechar = true;
                    }
                });
            });

        if abrir_seletor {
            self.destino_selecao = DestinoSelecao::Biblioteca;
            self.seletor = Some(arquivos::Seletor::novo(pasta_inicial()));
        }

        // Grava as configurações no estado compartilhado -- **só quando
        // mudaram**. Gravar a cada quadro escribava o arquivo de dados
        // dezenas de vezes por segundo com a janela de configurações aberta.
        let mudou = {
            let g = self.comp.travar();
            g.settings != settings
        };
        if mudou {
            {
                let mut g = self.comp.travar();
                g.settings = settings.clone();
            }
            self.salvar();
        }
        if fechar {
            self.mostrar_config = false;
            let _ = device_volume::configurar_inicio_com_windows(
                self.comp.travar().settings.iniciar_com_o_windows,
            );
        }
    }
}

// ------------------------------------------------------------------ ajudantes

/// Uma linha com interruptor e rótulo.
fn linha_de_interruptor(ui: &mut egui::Ui, valor: &mut bool, rotulo: &str) {
    ui.horizontal(|ui| {
        ui::interruptor(ui, valor);
        ui.label(egui::RichText::new(rotulo).color(Tema::TEXTO).size(13.0));
    });
}

/// Move a hora do editor em alguns minutos (e atualiza o texto digitado).
fn mover_hora(editor: &mut Editor, minutos: i32) {
    let total = editor.hora as i32 * 60 + editor.minuto as i32 + minutos;
    let total = total.rem_euclid(24 * 60);
    editor.hora = (total / 60) as u8;
    editor.minuto = (total % 60) as u8;
    editor.texto_hora = hora::formatar(editor.hora, editor.minuto);
    editor.erro_hora = None;
}

/// Campo de hora que o dono **digita**.
///
/// O texto digitado é interpretado na hora ("0730", "07:30" e "730" valem
/// 07:30). Quando o campo perde o foco, o texto volta ao formato canônico
/// "HH:MM".
fn campo_hora_editavel(
    ui: &mut egui::Ui,
    hora_atual: &mut u8,
    minuto_atual: &mut u8,
    texto: &mut String,
    erro: &mut Option<String>,
    id: egui::Id,
) {
    let tinha_foco = ui.ctx().memory(|m| m.has_focus(id));
    let resposta = ui::campo_hora(ui, texto, id);
    let tem_foco = ui.ctx().memory(|m| m.has_focus(id));

    // Ao ganhar o foco, seleciona o texto inteiro: o dono digita **por
    // cima** do horário, sem precisar apagar antes.
    ui::selecionar_todo_ao_ganhar_foco(ui.ctx(), id, texto, !tinha_foco && tem_foco);

    if resposta.changed() {
        let limpo = hora::sanear(texto);
        *texto = limpo;
        match hora::interpretar(texto, *minuto_atual) {
            Ok((h, m)) => {
                *hora_atual = h;
                *minuto_atual = m;
                *erro = None;
            }
            Err(e) => {
                *erro = Some(e);
            }
        }
    }

    // Perdeu o foco: mostra o horário no formato de relógio.
    if tinha_foco && !tem_foco && erro.is_none() && !texto.is_empty() {
        *texto = hora::formatar(*hora_atual, *minuto_atual);
    }
}

/// Índice da regra de repetição no seletor segmentado.
fn repeticao_indice(repeat: &RepeatRule) -> usize {
    match repeat {
        RepeatRule::Once => 0,
        RepeatRule::Daily => 1,
        RepeatRule::Weekdays => 2,
        RepeatRule::Weekly { .. } => 3,
    }
}

/// O nome do despertador (ou "Despertador", quando o dono não deu nome).
fn nome_do_alarme(alarme: &Alarm) -> String {
    if alarme.label.is_empty() {
        "Despertador".to_string()
    } else {
        alarme.label.clone()
    }
}

/// O nome do som escolhido.
fn nome_som(som: &SoundChoice) -> String {
    match som {
        SoundChoice::Preset(p) => p.label().to_string(),
        SoundChoice::Arquivo { caminho } => std::path::Path::new(caminho)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Áudio".to_string()),
    }
}

/// A pasta onde o seletor de arquivos abre.
fn pasta_inicial() -> PathBuf {
    dirs::audio_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(|| PathBuf::from("C:\\"))
}

/// Dia da semana em português.
fn dia_semana(dia: chrono::Weekday) -> &'static str {
    match dia {
        chrono::Weekday::Mon => "segunda-feira",
        chrono::Weekday::Tue => "terça-feira",
        chrono::Weekday::Wed => "quarta-feira",
        chrono::Weekday::Thu => "quinta-feira",
        chrono::Weekday::Fri => "sexta-feira",
        chrono::Weekday::Sat => "sábado",
        chrono::Weekday::Sun => "domingo",
    }
}

/// Nome do mês em português.
fn nome_mes(mes: u32) -> &'static str {
    match mes {
        1 => "janeiro",
        2 => "fevereiro",
        3 => "março",
        4 => "abril",
        5 => "maio",
        6 => "junho",
        7 => "julho",
        8 => "agosto",
        9 => "setembro",
        10 => "outubro",
        11 => "novembro",
        _ => "dezembro",
    }
}

fn segundos_decorridos(inicio: Instant) -> u64 {
    inicio.elapsed().as_secs()
}

/// Dois despertadores de exemplo (usados no modo `--demo`).
fn alarmes_de_exemplo() -> Vec<Alarm> {
    let mut cafe = Alarm::new(7, 0);
    cafe.label = "Café da manhã".to_string();
    cafe.repeat = RepeatRule::Daily;
    cafe.sound = SoundChoice::Preset(Preset::Classico);
    cafe.fade_in_secs = 20;

    let mut treino = Alarm::new(18, 30);
    treino.label = "Treino".to_string();
    treino.repeat = RepeatRule::Weekly { days: vec![1, 3, 5] };
    treino.sound = SoundChoice::Preset(Preset::Sirene);
    treino.volume = 80;
    treino.dismiss.soneca = true;
    treino.dismiss.soneca_minutos = 5;

    vec![cafe, treino]
}

fn minutos_soneca(alarme: &Alarm) -> String {
    format!("{} min", alarme.dismiss.soneca_minutos)
}

/// Inicia a thread do agendador.
///
/// A thread dorme ate o proximo disparo (sem polling) e acorda quando o
/// estado muda. O som e tocado aqui, e nao na interface: o Windows nao
/// desenha uma janela minimizada, e o app precisa continuar tocando
/// escondido.
fn iniciar_agendador(ctx: egui::Context, comp: Arc<estado::Compartilhado>) -> Agendador {
    let (eventos_tx, eventos_rx) = mpsc::channel::<EventoAgendador>();

    let comp_thread = comp.clone();
    std::thread::spawn(move || {
        loop {
            let agora = Local::now();

            // Alarme que passou enquanto o PC estava dormindo.
            let atrasado = comp_thread.travar().atrasado(agora, 15);
            if let Some(alarme) = atrasado {
                disparar(&comp_thread, &alarme.id, &eventos_tx, &ctx);
                continue;
            }

            // Proximo disparo: dorme ate la, ou ate o estado mudar.
            //
            // A trava fica presa do calculo ate a espera. Se fosse solta
            // entre "qual e a proxima hora" e "vou dormir", um aviso do dono
            // nesse instante se perderia e o agendador dormiria ate a hora
            // antiga -- o alarme novo, mais cedo, so tocaria horas depois.
            let g = comp_thread.travar();
            let proximo = g.proximo_disparo(agora);
            let Some((alarme, quando)) = proximo else {
                // Nada agendado: espera o dono mexer no estado.
                let _g = match comp_thread.condvar.wait(g) {
                    Ok(guardia) => guardia,
                    Err(envenenado) => envenenado.into_inner(),
                };
                continue;
            };

            let espera = (quando - agora).num_milliseconds().max(0) as u64;
            let (g, resultado) = match comp_thread
                .condvar
                .wait_timeout(g, Duration::from_millis(espera))
            {
                Ok(r) => r,
                Err(envenenado) => envenenado.into_inner(),
            };
            drop(g);
            if !resultado.timed_out() {
                // O estado mudou: recalcula.
                continue;
            }
            disparar(&comp_thread, &alarme.id, &eventos_tx, &ctx);
        }
    });

    Agendador { eventos: eventos_rx }
}

/// Dispara um alarme **na thread do agendador**, sem depender da interface.
///
/// O Windows nao desenha uma janela minimizada, e a interface pode nao
/// rodar: se o som dependesse dela, o alarme silenciaria justo quando o
/// dono dorme com o app minimizado. Aqui o som comeca na hora e a interface
/// so recebe o aviso.
///
/// Se o som nao sair, tenta de novo a cada 5 segundos por ate uma hora. Para
/// um despertador, silenciar e pior do que repetir.
fn disparar(
    comp: &Arc<estado::Compartilhado>,
    id: &str,
    eventos_tx: &Sender<EventoAgendador>,
    ctx: &egui::Context,
) {
    let marcas = comp.paradas.load(std::sync::atomic::Ordering::SeqCst);
    let mut tentativas = 0u32;
    loop {
        // O alarme atual: o dono pode ter mudado o som no meio da espera.
        let Some(alarme) = comp.travar().por_id(id).cloned() else {
            return;
        };

        let (tocou, avisos) = iniciar_som_do_alarme(&alarme);
        let _ = eventos_tx.send(EventoAgendador::Disparo {
            id: id.to_string(),
            tocou,
            avisos: avisos.clone(),
        });
        ctx.request_repaint();

        if tocou {
            // O som saiu de verdade: so agora o alarme fica marcado como
            // tocado. Quem grava e o agendador, porque a interface pode nem
            // estar rodando (janela minimizada).
            let hoje = Local::now().format("%Y-%m-%d").to_string();
            {
                let mut g = comp.travar();
                g.marcar_tocado(id, &hoje);
            }
            let _ = comp.salvar();
            return;
        }

        tentativas += 1;
        if tentativas >= 720 {
            // Uma hora de tentativas: desiste e deixa o alarme pendente.
            return;
        }

        // Espera 5 segundos, ou ate o dono dispensar.
        let g = comp.travar();
        let (g, _resultado) = match comp
            .condvar
            .wait_timeout(g, Duration::from_secs(5))
        {
            Ok(r) => r,
            Err(envenenado) => envenenado.into_inner(),
        };
        drop(g);
        if comp.paradas.load(std::sync::atomic::Ordering::SeqCst) != marcas {
            // O dono dispensou: para de tentar.
            return;
        }
    }
}

/// Começa a tocar o alarme e devolve (tocou, avisos).
///
/// Se a saída escolhida não responder, cai para a saída padrão do Windows:
/// num despertador, degradar é sempre melhor que silenciar.
fn iniciar_som_do_alarme(alarme: &Alarm) -> (bool, Vec<String>) {
    let mut avisos: Vec<String> = Vec::new();

    if alarme.force_device_volume {
        match device_volume::forcar_100(&alarme.output_devices) {
            Ok(msg) => avisos.push(msg),
            Err(e) => avisos.push(format!("Volume do aparelho: {e}")),
        }
    }

    match audio::tocar_global(
        &alarme.sound,
        &alarme.output_devices,
        alarme.volume,
        alarme.fade_in_secs,
    ) {
        Ok(tocando) => {
            avisos.push(format!("Tocando em {} saída(s).", tocando.len()));
            (true, avisos)
        }
        Err(erro) => {
            if alarme.output_devices.is_empty() {
                avisos.push(format!("Não consegui tocar o alarme: {erro}"));
                return (false, avisos);
            }
            // D4: nenhuma saída escolhida respondeu -> tenta o padrão.
            avisos.push(format!("Saídas escolhidas indisponíveis ({erro})."));
            match audio::tocar_global(&alarme.sound, &[], alarme.volume, alarme.fade_in_secs) {
                Ok(tocando) => {
                    avisos.push(format!(
                        "Tocando na saída padrão do Windows ({} saída(s)).",
                        tocando.len()
                    ));
                    (true, avisos)
                }
                Err(erro2) => {
                    avisos.push(format!("Não consegui tocar o alarme: {erro2}"));
                    (false, avisos)
                }
            }
        }
    }
}

impl eframe::App for TranalApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // A bandeja do Windows nasce no primeiro quadro: o Windows exige que
        // o ícone seja criado na thread que tem o event loop (a principal).
        if self.bandeja.is_none() {
            tray::registrar_contexto(&ctx);
            self.bandeja = tray::criar();
            tray::iniciar_ouvinte();
        }

        // Fechar a janela nao encerra o app: a janela minimiza e o agendador
        // continua tocando. A excecao e a opcao "encerrar o app quando a
        // janela e fechada" — e o "Sair" do menu da bandeja.
        let fechar_foi_pedido = ctx.input(|i| i.viewport().close_requested());
        if fechar_foi_pedido {
            // O flag so e consumido quando o fechamento chegou de verdade:
            // ai o "Sair" da bandeja encerra o app em vez de minimiza-lo.
            let saiu_da_bandeja = tray::sair_pedido();
            if !saiu_da_bandeja && !self.comp.travar().settings.sair_ao_fechar {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                self.mensagem = None;
                self.alerta = None;
            }
        }

        // Eventos do agendador. O som ja comecou na thread do agendador: aqui
        // so aparece o painel do alarme.
        let mut disparos: Vec<(String, bool, Vec<String>)> = Vec::new();
        while let Ok(evento) = self.agendador.eventos.try_recv() {
            let EventoAgendador::Disparo { id, tocou, avisos } = evento;
            disparos.push((id, tocou, avisos));
        }
        for (id, tocou, avisos) in disparos {
            self.mostrar_alarme(&id, tocou, avisos);
        }

        // Atualiza os caches se o estado mudou (uma leitura atômica;
        // só trava o mutex quando os alarmes realmente mudaram).
        self.sincronizar_cache();

        // O proximo despertador: recalcula só se os alarmes mudaram
        // ou se passou mais de 1 segundo (o schedule depende da hora).
        let g = self.comp.geracao();
        if g != self.proximo_geracao
            || self.proximo_calculado_em.elapsed() > Duration::from_secs(1)
        {
            self.proximo = self.comp.travar().proximo_disparo(Local::now());
            self.proximo_geracao = g;
            self.proximo_calculado_em = Instant::now();
        }

        self.desenhar(ui);

        // Atualiza o relógio sem trabalho pesado (meio segundo é o bastante
        // para o relógio não "pular").
        ctx.request_repaint_after(Duration::from_millis(500));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        audio::parar_global();
        // Deixa o volume do PC como estava: sem isso, fechar o app durante o
        // alarme deixaria todas as saidas em 100% para sempre.
        if let Some(t) = &self.tocando
            && t.alarme.force_device_volume
        {
            device_volume::restaurar(&t.alarme.output_devices);
        }
        let _ = self.comp.salvar();
    }
}
