//! Tranal Despertador — aplicativo desktop em Rust.
//!
//! Janela nativa (sem navegador embarcado), tema escuro e consumo mínimo de
//! memória e processador. O app fica rodando em segundo plano e dispara os
//! alarmes na hora, mesmo com a janela fechada.

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
mod schedule;
mod storage;
mod tones;
mod tray;
mod tray_menu;
mod ui;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if cli::executar(&args) {
        return Ok(());
    }

    // Só uma instância por vez: se já existe um app rodando, traz a janela
    // dele para o primeiro plano e sai. Sem isso, fechar a janela mataria o
    // despertador e abrir de novo criaria duas copias rodando.
    if !instancia::iniciar() {
        instancia::mostrar_janela_existente();
        return Ok(());
    }

    // A chave de "iniciar junto com o Windows" e o que faz o despertador
    // sobreviver a um reinicio do PC. A tela e o registro podem divergir (o
    // dono liga a opcao e o registro nao grava, ou o exe muda de lugar), e
    // essa divergencia e silenciosa: o app parece la, mas sobe depois do
    // reinicio. Por isso o registro e reconciliado com o ajuste salvo toda
    // vez que o app arranca.
    let (_, ajustes) = storage::carregar();
    if let Err(e) = device_volume::configurar_inicio_com_windows(ajustes.iniciar_com_o_windows) {
        eprintln!("Nao consegui sincronizar o inicio automatico: {e}");
    }

    let opcoes = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 700.0])
            .with_min_inner_size([920.0, 620.0])
            .with_title("Tranal Despertador")
            // O mesmo desenho do icone da bandeja, na barra de titulo.
            .with_icon(crate::tray::icone_da_janela())
            // "--minimizado" e usado pela chave de inicio automatico: o app
            // comeca escondido, so com o agendador rodando.
            .with_visible(!args.iter().any(|a| a == "--minimizado")),
        // OpenGL em vez de wgpu/DX12: mesma aparencia, muito menos recurso.
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };

    eframe::run_native(
        "Tranal Despertador",
        opcoes,
        Box::new(|cc| Ok(Box::new(app::TranalApp::novo(cc)))),
    )
}
