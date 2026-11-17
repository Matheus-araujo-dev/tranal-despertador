//! Tranal Despertador — aplicativo desktop em Rust.
//!
//! Janela nativa (sem navegador embarcado), tema escuro e consumo mínimo de
//! memória e processador. O app fica rodando em segundo plano e dispara os
//! alarmes na hora, mesmo com a janela fechada.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod arquivos;
mod audio;
mod cli;
mod device_volume;
mod estado;
mod hora;
mod instancia;
mod model;
mod schedule;
mod storage;
mod tones;
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

    let opcoes = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1040.0, 700.0])
            .with_min_inner_size([920.0, 620.0])
            .with_title("Tranal Despertador")
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
