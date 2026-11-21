//! Tranal Despertador — aplicativo desktop em Rust.
//!
//! Janela nativa (sem navegador embarcado), tema escuro e consumo mínimo de
//! memória e processador. O app fica rodando em segundo plano e dispara os
//! alarmes na hora, mesmo com a janela fechada.
//!
//! Renderização 100% software: sem OpenGL, sem wgpu, sem driver de vídeo.
//! O egui desenha a interface, o `renderizador.rs` rasteriza os triângulos
//! em um buffer de pixels, e o `softbuffer` exibe na janela via GDI.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod arquivos;
mod atualizador;
mod audio;
mod cli;
mod device_volume;
mod estado;
mod hora;
mod icone;
mod instancia;
mod model;
mod renderizador;
mod schedule;
mod storage;
mod tones;
mod tray;
mod tray_menu;
mod ui;

use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Instant;

use egui::ViewportCommand;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::window::{Window, WindowId};

#[derive(Debug)]
enum Evento {
    Repintar,
}

struct Aplicacao {
    janela: Option<Arc<Window>>,
    surface: Option<softbuffer::Surface<Arc<Window>, Arc<Window>>>,
    egui_ctx: egui::Context,
    egui_state: Option<egui_winit::State>,
    rend: renderizador::Renderizador,
    app: Option<app::TranalApp>,
    minimizado: bool,
    proximo_repaint: Instant,
    proxy: EventLoopProxy<Evento>,
}

impl Aplicacao {
    fn novo(minimizado: bool, proxy: EventLoopProxy<Evento>) -> Self {
        Self {
            janela: None,
            surface: None,
            egui_ctx: egui::Context::default(),
            egui_state: None,
            rend: renderizador::Renderizador::novo(1040, 700),
            app: None,
            minimizado,
            proximo_repaint: Instant::now(),
            proxy,
        }
    }

    fn pintar(&mut self) {
        let Some(janela) = &self.janela else { return };
        let Some(surface) = &mut self.surface else { return };
        let Some(state) = &mut self.egui_state else { return };
        let Some(app) = &mut self.app else { return };

        let tamanho = janela.inner_size();
        if tamanho.width == 0 || tamanho.height == 0 {
            return;
        }

        let escala = janela.scale_factor() as f32;
        self.rend.redimensionar(tamanho.width, tamanho.height);

        let input = state.take_egui_input(janela);
        let mut saida = self.egui_ctx.run_ui(input, |ui| {
            app.atualizar(ui);
        });

        // Texturas do egui (atlas de fontes, imagens).
        self.rend.aplicar_texturas(&saida.textures_delta);
        saida.textures_delta.clear();

        // Tessela as formas em triângulos e rasteriza em software.
        let primitivas = self
            .egui_ctx
            .tessellate(saida.shapes, saida.pixels_per_point);
        self.rend.renderizar(&primitivas, escala);

        // Exibe o buffer na janela via softbuffer (GDI no Windows).
        let _ = surface.resize(
            NonZeroU32::new(tamanho.width).unwrap(),
            NonZeroU32::new(tamanho.height).unwrap(),
        );
        if let Ok(mut buf) = surface.buffer_mut() {
            buf.copy_from_slice(self.rend.pixels());
            let _ = buf.present();
        }

        // Plataforma: clipboard, IME, cursor.
        state.handle_platform_output(janela, saida.platform_output);

        // Comandos de viewport (fechar, esconder, focar).
        for (_, vp) in saida.viewport_output {
            for cmd in vp.commands {
                match cmd {
                    ViewportCommand::Close => {
                        if let Some(a) = &mut self.app {
                            a.ao_sair();
                        }
                        self.app = None;
                        std::process::exit(0);
                    }
                    ViewportCommand::CancelClose => {}
                    ViewportCommand::Visible(v) => janela.set_visible(v),
                    ViewportCommand::Minimized(m) => janela.set_minimized(m),
                    ViewportCommand::Focus => janela.focus_window(),
                    _ => {}
                }
            }
            self.proximo_repaint = Instant::now() + vp.repaint_delay;
        }
    }
}

impl ApplicationHandler<Evento> for Aplicacao {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.janela.is_some() {
            return;
        }

        let (rgba, lado) = crate::tray::icone_bruto();
        let icone = winit::window::Icon::from_rgba(rgba, lado, lado).ok();

        let attrs = Window::default_attributes()
            .with_inner_size(LogicalSize::new(1040.0, 700.0))
            .with_min_inner_size(LogicalSize::new(920.0, 620.0))
            .with_title("Tranal Despertador")
            .with_window_icon(icone)
            .with_visible(!self.minimizado);

        let janela = Arc::new(event_loop.create_window(attrs).expect("criar janela"));

        let context = softbuffer::Context::new(janela.clone()).expect("softbuffer context");
        let surface = softbuffer::Surface::new(&context, janela.clone()).expect("softbuffer surface");

        // Callback de repaint: acorda o event loop de qualquer thread.
        let proxy = self.proxy.clone();
        self.egui_ctx
            .set_request_repaint_callback(move |_info| {
                let _ = proxy.send_event(Evento::Repintar);
            });

        let state = egui_winit::State::new(
            self.egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &janela,
            Some(janela.scale_factor() as f32),
            None,
            Some(8192),
        );

        self.app = Some(app::TranalApp::novo(&self.egui_ctx));
        self.janela = Some(janela);
        self.surface = Some(surface);
        self.egui_state = Some(state);

        // Primeiro quadro.
        self.pintar();
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = &mut self.egui_state else {
            return;
        };
        let Some(janela) = &self.janela else { return };

        let resposta = state.on_window_event(janela, &event);

        if resposta.repaint {
            if let Some(w) = &self.janela {
                w.request_redraw();
            }
        }

        match event {
            WindowEvent::RedrawRequested => {
                self.pintar();
            }
            WindowEvent::CloseRequested => {
                // O egui recebe o evento via on_window_event acima.
                // O app decide se cancela (minimiza pra bandeja) ou fecha.
                // Pinta um frame para processar a decisão.
                self.pintar();
            }
            WindowEvent::Resized(_) => {
                if let Some(w) = &self.janela {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: Evento) {
        if let Some(w) = &self.janela {
            w.request_redraw();
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if Instant::now() >= self.proximo_repaint {
            if let Some(w) = &self.janela {
                w.request_redraw();
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if cli::executar(&args) {
        return;
    }

    if !instancia::iniciar() {
        instancia::mostrar_janela_existente();
        return;
    }

    let (alarmes, mut ajustes) = storage::carregar();
    let dados_legiveis = match std::fs::read_to_string(storage::caminho_dados()) {
        Ok(t) => serde_json::from_str::<model::BancoDeDados>(&t).is_ok(),
        Err(e) => e.kind() == std::io::ErrorKind::NotFound,
    };
    if device_volume::migrar_inicio_legado() && !ajustes.iniciar_com_o_windows {
        ajustes.iniciar_com_o_windows = true;
        if !dados_legiveis {
            eprintln!("Arquivo de dados ilegivel: o inicio automatico vale so nesta execucao.");
        } else if let Err(e) = storage::salvar(&alarmes, &ajustes) {
            eprintln!("Nao consegui gravar o inicio automatico migrado: {e}");
        }
    }
    if let Err(e) = device_volume::configurar_inicio_com_windows(ajustes.iniciar_com_o_windows) {
        eprintln!("Nao consegui sincronizar o inicio automatico: {e}");
    }

    let minimizado = args.iter().any(|a| a == "--minimizado");

    let event_loop = EventLoop::<Evento>::with_user_event()
        .build()
        .expect("criar event loop");
    event_loop.set_control_flow(ControlFlow::Wait);

    let proxy = event_loop.create_proxy();
    let mut aplicacao = Aplicacao::novo(minimizado, proxy);
    let _ = event_loop.run_app(&mut aplicacao);
}
