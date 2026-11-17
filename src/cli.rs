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
    if args.iter().any(|a| a == "--forcar-volume") {
        return forcar_volume(args);
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
