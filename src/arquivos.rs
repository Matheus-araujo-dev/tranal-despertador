//! Escolha de arquivo **dentro do app**.
//!
//! O dono reclamou que o explorer do Windows demorava demais para abrir
//! (é o `IFileDialog`, via COM, e ele demora mesmo). Em vez de chamar o
//! explorador, o app lista os arquivos ele mesmo: abre na hora, tem a cara do
//! app e não depende do Windows.
//!
//! O que o dono escolhe é **copiado** para a pasta de sons do app (veja
//! `tones::importar_audio`): assim o despertador continua tocando mesmo que o
//! arquivo original seja movido ou apagado.

use std::path::{Path, PathBuf};

use eframe::egui;
use eframe::egui::Color32;

use crate::ui::{self, Tema};

/// Estado do seletor de arquivos.
pub struct Seletor {
    /// Pasta que está aberta na lista.
    pub pasta: PathBuf,
    /// Arquivo marcado pelo dono.
    pub selecionado: Option<PathBuf>,
    /// Mostra todo tipo de arquivo (padrão: só os que o app sabe tocar).
    pub mostrar_todos: bool,
    /// Filtro de texto sobre o nome do arquivo.
    pub busca: String,
    /// Ainda está aberto?
    pub aberto: bool,
}

/// Um item da lista.
struct Entrada {
    nome: String,
    caminho: PathBuf,
    pasta: bool,
}

impl Seletor {
    /// Abre o seletor em uma pasta.
    pub fn novo(pasta: PathBuf) -> Self {
        Self {
            pasta,
            selecionado: None,
            mostrar_todos: false,
            busca: String::new(),
            aberto: true,
        }
    }

    /// Os itens da pasta atual (pastas primeiro, depois os arquivos).
    fn listar(&self) -> Vec<Entrada> {
        let Ok(entradas) = std::fs::read_dir(&self.pasta) else {
            return Vec::new();
        };
        let mut lista: Vec<Entrada> = Vec::new();
        for entrada in entradas.flatten() {
            let caminho = entrada.path();
            let Some(nome) = caminho.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let eh_pasta = caminho.is_dir();
            if !eh_pasta && !self.mostrar_todos && !crate::tones::EXTENSOES.contains(
                &caminho
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .as_str(),
            ) {
                continue;
            }
            if !self.busca.is_empty()
                && !nome.to_lowercase().contains(&self.busca.to_lowercase())
            {
                continue;
            }
            lista.push(Entrada {
                nome: nome.to_string(),
                caminho,
                pasta: eh_pasta,
            });
        }
        lista.sort_by(|a, b| match (a.pasta, b.pasta) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.nome.to_lowercase().cmp(&b.nome.to_lowercase()),
        });
        lista
    }
}

/// Desenha o seletor como janela modal.
///
/// Devolve o caminho escolhido quando o dono confirma, e `None` enquanto o
/// seletor continua aberto (ou quando ele cancela).
pub fn mostrar(ctx: &egui::Context, seletor: &mut Seletor) -> Option<String> {
    let mut escolhido: Option<String> = None;
    let mut cancelar = false;
    let mut entrou_em: Option<PathBuf> = None;

    egui::Modal::new(egui::Id::unique("tranal-seletor-de-arquivo"))
        .frame(
            egui::Frame::new()
                .fill(Tema::PAINEL)
                .stroke(egui::Stroke::new(1.0, Tema::BORDA))
                .corner_radius(ui::raio(ui::raio::CARTAO))
                .inner_margin(egui::Margin::same(18)),
        )
        .show(ctx, |ui| {
            ui.set_max_width(620.0);

            // Cabeçalho.
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("Escolher um áudio")
                        .color(Tema::TEXTO)
                        .size(18.0)
                        .strong(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui::botao_fantasma(ui, "Fechar").clicked() {
                        cancelar = true;
                    }
                });
            });
            ui::ajuda(
                ui,
                "O arquivo é copiado para a pasta do app: o despertador continua tocando mesmo que você mova o original.",
            );
            ui.add_space(ui::espaco::M);

            // Caminho e navegação.
            ui.horizontal(|ui| {
                let pode_subir = seletor.pasta.parent().is_some();
                if ui
                    .add_enabled(
                        pode_subir,
                        egui::Button::new("Subir").corner_radius(ui::raio(ui::raio::CONTROLE)),
                    )
                    .clicked()
                    && let Some(pai) = seletor.pasta.parent()
                {
                    entrou_em = Some(pai.to_path_buf());
                }
                ui.label(
                    egui::RichText::new(caminho_curto(&seletor.pasta))
                        .color(Tema::TEXTO_FRACO)
                        .size(12.0),
                );
            });
            ui.add_space(ui::espaco::S);

            // Atalhos: unidades, a pasta do app e a pasta de músicas.
            ui.horizontal_wrapped(|ui| {
                for unidade in unidades() {
                    if ui::botao_neutro(ui, &unidade).clicked() {
                        entrou_em = Some(PathBuf::from(unidade));
                    }
                }
                if ui::botao_neutro(ui, "Sons do app").clicked() {
                    entrou_em = Some(crate::storage::pasta_sons());
                }
                if let Some(musicas) = dirs::audio_dir()
                    && ui::botao_neutro(ui, "Músicas").clicked()
                {
                    entrou_em = Some(musicas);
                }
            });
            ui.add_space(ui::espaco::S);

            // Busca e filtro.
            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut seletor.busca)
                        .hint_text("filtrar pelo nome...")
                        .desired_width(240.0)
                        .text_color(Tema::TEXTO),
                );
                let mut todos = seletor.mostrar_todos;
                if ui.checkbox(&mut todos, "mostrar todos").changed() {
                    seletor.mostrar_todos = todos;
                }
            });
            ui.add_space(ui::espaco::S);

            // A lista.
            egui::ScrollArea::vertical()
                .max_height(320.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let itens = seletor.listar();
                    if itens.is_empty() {
                        ui::ajuda(ui, "Nada por aqui. Suba de pasta ou escolha outro atalho acima.");
                        return;
                    }
                    for item in itens {
                        let marcado = seletor.selecionado.as_ref() == Some(&item.caminho);
                        let cor = if item.pasta {
                            Tema::ACCENT
                        } else if marcado {
                            Tema::TEXTO
                        } else {
                            Tema::TEXTO_FRACO
                        };
                        let prefixo = if item.pasta { "▸ " } else { "   " };
                        let texto = egui::RichText::new(format!("{prefixo}{}", item.nome))
                            .color(cor)
                            .size(13.0);
                        let resposta = ui.add(
                            egui::Button::new(texto)
                                .fill(if marcado {
                                    Tema::ACCENT_FORTE.gamma_multiply(0.35)
                                } else {
                                    Color32::TRANSPARENT
                                })
                                .corner_radius(ui::raio(8))
                                .stroke(egui::Stroke::NONE)
                                .min_size(egui::Vec2::new(ui.available_width(), 28.0)),
                        );
                        if resposta.clicked() {
                            if item.pasta {
                                entrou_em = Some(item.caminho.clone());
                            } else {
                                seletor.selecionado = Some(item.caminho.clone());
                            }
                        }
                        if resposta.double_clicked() && !item.pasta {
                            seletor.selecionado = Some(item.caminho.clone());
                            escolhido = seletor.selecionado.as_ref().map(|p| p.to_string_lossy().to_string());
                        }
                    }
                });
            ui.add_space(ui::espaco::M);

            // Rodapé.
            ui.horizontal(|ui| {
                match &seletor.selecionado {
                    Some(c) => ui.label(
                        egui::RichText::new(c.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())
                            .color(Tema::TEXTO)
                            .size(13.0),
                    ),
                    None => ui.label(
                        egui::RichText::new("Escolha um arquivo na lista")
                            .color(Tema::TEXTO_FRACO)
                            .size(13.0),
                    ),
                };
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let tem_arquivo = seletor.selecionado.is_some();
                    if ui
                        .add_enabled(
                            tem_arquivo,
                            egui::Button::new(
                                egui::RichText::new("Escolher")
                                    .color(Color32::WHITE)
                                    .size(15.0)
                                    .strong(),
                            )
                            .fill(Tema::ACCENT_FORTE)
                            .corner_radius(ui::raio(ui::raio::CONTROLE))
                            .stroke(egui::Stroke::NONE)
                            .min_size(egui::Vec2::new(0.0, 40.0)),
                        )
                        .clicked()
                    {
                        escolhido = seletor
                            .selecionado
                            .as_ref()
                            .map(|p| p.to_string_lossy().to_string());
                    }
                    if ui::botao_neutro(ui, "Cancelar").clicked() {
                        cancelar = true;
                    }
                });
            });
        });

    if let Some(destino) = entrou_em {
        seletor.pasta = destino;
        seletor.selecionado = None;
    }
    if cancelar {
        seletor.aberto = false;
    }
    if escolhido.is_some() {
        seletor.aberto = false;
    }
    escolhido
}

/// As unidades do Windows (C:, D:, ...).
fn unidades() -> Vec<String> {
    let mascara = unsafe { windows::Win32::Storage::FileSystem::GetLogicalDrives() };
    let mut lista = Vec::new();
    for i in 0..26u32 {
        if mascara & (1 << i) != 0 {
            let letra = (b'A' + i as u8) as char;
            lista.push(format!("{letra}:\\"));
        }
    }
    lista
}

/// Caminho curto para caber na barra (as três últimas partes).
fn caminho_curto(caminho: &Path) -> String {
    let partes: Vec<String> = caminho
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    if partes.len() <= 3 {
        return partes.join("\\");
    }
    let ultimas = &partes[partes.len() - 3..];
    format!("…\\{}", ultimas.join("\\"))
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_seletor_lista_a_pasta_atual() {
        let seletor = Seletor::novo(crate::storage::pasta_sons());
        // Pode não existir ainda: o app tem que aguentar isso sem pânico.
        let _ = seletor.listar();
    }

    #[test]
    fn as_unidades_do_windows_sao_lidas() {
        let lista = unidades();
        assert!(!lista.is_empty(), "o Windows sempre tem pelo menos C:");
    }

    #[test]
    fn caminho_curto_mantem_o_fim() {
        let curto = caminho_curto(Path::new("C:\\Users\\Matheus\\Music\\Songs"));
        assert!(curto.contains("Songs"), "{curto}");
    }
}
