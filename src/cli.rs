//! Modo de linha de comando.
//!
//! `--testar-som` toca o alarme sem abrir a janela. Serve para conferir se o
//! som sai nas saídas escolhidas (útil quando o app está rodando em segundo
//! plano) e para o dono testar sem abrir a interface.

use std::time::Duration;

use crate::audio;
use crate::device_volume;
use crate::model::{Preset, SoundChoice};

/// Executa os modos de linha de comando. Devolve `true` se algum modo foi pedido.
pub fn executar(args: &[String]) -> bool {
    if args.iter().any(|a| a == "--atualizar") {
        return crate::atualizador::executar_troca(args);
    }
    if args.iter().any(|a| a == "--forcar-volume") {
        return forcar_volume(args);
    }
    if args.iter().any(|a| a == "--exportar-icone") {
        return exportar_icone(args);
    }
    if !args.iter().any(|a| a == "--testar-som") {
        return false;
    }

    let saidas = valor(args, "--saidas")
        .map(|v| {
            v.split(';')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    let segundos = valor(args, "--segundos")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(6);

    let alvos = if saidas.is_empty() {
        "a saida padrao do Windows".to_string()
    } else {
        saidas.join(" | ")
    };
    println!("Tranal Despertador - teste de som");
    println!("Saidas: {alvos}");
    println!("Duracao: {segundos}s");

    match audio::tocar(&SoundChoice::Preset(Preset::Classico), &saidas, 100, 0) {
        Ok(mut tocador) => {
            std::thread::sleep(Duration::from_millis(300));
            let tocando = tocador.tocando();
            let erros = tocador.erros();
            if tocando.is_empty() {
                println!("Aviso: nenhuma saida comecou a tocar.");
            } else {
                for nome in &tocando {
                    println!("Tocando em: {nome}");
                }
            }
            for erro in erros {
                println!("Erro: {erro}");
            }
            std::thread::sleep(Duration::from_secs(segundos));
            tocador.parar();
            println!("Fim do teste.");
        }
        Err(e) => {
            eprintln!("Nao consegui tocar: {e}");
            std::process::exit(1);
        }
    }
    true
}

/// Lê o valor de uma chave de linha de comando (`--chave valor`).
fn valor(args: &[String], chave: &str) -> Option<String> {
    let pos = args.iter().position(|a| a == chave)?;
    args.get(pos + 1).cloned()
}

/// Modo `--exportar-icone <arquivo.ico>`: grava o desenho do despertador em
/// um arquivo `.ico`.
///
/// O instalador usa isso para os atalhos do Menu Iniciar e da Area de
/// trabalho: o executavel nao tem recurso de icone embutido (o icone do app
/// e desenhado em codigo, em tempo de execucao), e o Windows nao adivinha o
/// icone de um executavel que nao o tem.
fn exportar_icone(args: &[String]) -> bool {
    let Some(caminho) = valor(args, "--exportar-icone") else {
        eprintln!("Uso: tranal-despertador --exportar-icone <arquivo.ico>");
        std::process::exit(1);
    };
    let caminho = std::path::PathBuf::from(caminho);
    match crate::icone::exportar_ico(&caminho) {
        Ok(()) => {
            println!("Icone gravado em: {}", caminho.display());
            true
        }
        Err(e) => {
            eprintln!("Nao consegui gravar o icone: {e}");
            std::process::exit(1);
        }
    }
}

/// Modo `--forcar-volume`: exercita o caminho Rust do Core Audio.
///
/// Sobe o volume do aparelho para 100% nas saídas pedidas, segura um tempo
/// e devolve o volume anterior. Serve para conferir que o botão "Forçar 100%
/// no aparelho" do editor funciona, sem precisar abrir a janela.
fn forcar_volume(args: &[String]) -> bool {
    let saidas = valor(args, "--saidas")
        .map(|v| {
            v.split(';')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<String>>()
        })
        .unwrap_or_default();
    let segundos = valor(args, "--segundos")
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(5);

    let alvo = if saidas.is_empty() {
        "todas as saidas de audio".to_string()
    } else {
        saidas.join(" | ")
    };
    println!("Tranal Despertador - forcar volume do aparelho");
    println!("Alvo: {alvo}");

    match device_volume::forcar_100(&saidas) {
        Ok(resumo) => {
            println!("{resumo}");
            std::thread::sleep(Duration::from_secs(segundos));
            device_volume::restaurar(&saidas);
            println!("Volume anterior restaurado.");
        }
        Err(e) => {
            eprintln!("Nao consegui forcar o volume: {e}");
            std::process::exit(1);
        }
    }
    true
}
