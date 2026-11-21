//! Reprodução do som do alarme.
//!
//! Regras que vieram da medição na máquina do dono:
//! - os streams são abertos **um por vez** (o driver do PC recusa abertura
//!   paralela com `WdmSyncIoctl ... 0x490`);
//! - cada dispositivo toca em sua própria thread, em paralelo, para o som
//!   sair em todos os monitores ao mesmo tempo;
//! - o ganho é controlado por um contador atômico, o que permite o fade in.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{BufferSize, Device, FromSample, SampleFormat, SizedSample, Stream};

use crate::model::SoundChoice;
use crate::tones::{self, i16_para_f32, Faixa};

/// Uma saída de áudio disponível no sistema.
#[derive(Clone, Debug, PartialEq)]
pub struct Saida {
    pub nome: String,
    /// Verdadeiro se for a saída padrão do Windows.
    pub padrao: bool,
}

/// Nome legível de um dispositivo de áudio.
fn nome_do_dispositivo(device: &Device) -> String {
    match device.description() {
        Ok(descricao) => descricao.name().to_string(),
        Err(_) => device.to_string(),
    }
}

/// Lista as saídas de áudio disponíveis.
pub fn listar_saidas() -> Vec<Saida> {
    let host = cpal::default_host();
    let padrao = host
        .default_output_device()
        .map(|d| nome_do_dispositivo(&d))
        .unwrap_or_default();

    let mut saidas = Vec::new();
    let Ok(devices) = host.output_devices() else {
        return saidas;
    };
    for device in devices {
        let nome = nome_do_dispositivo(&device);
        saidas.push(Saida {
            padrao: nome == padrao,
            nome,
        });
    }
    saidas
}

/// Controlador de uma reprodução em andamento.
pub struct Tocador {
    parar: Arc<AtomicBool>,
    threads: Vec<std::thread::JoinHandle<()>>,
    tocando: Arc<Mutex<Vec<String>>>,
    erros: Arc<Mutex<Vec<String>>>,
}

impl Tocador {
    /// Nomes das saídas que estão tocando agora.
    pub fn tocando(&self) -> Vec<String> {
        self.tocando.lock().map(|t| t.clone()).unwrap_or_default()
    }

    /// Mensagens de erro por dispositivo que falhou.
    pub fn erros(&self) -> Vec<String> {
        self.erros.lock().map(|e| e.clone()).unwrap_or_default()
    }

    /// Para a reprodução e espera as threads terminarem.
    pub fn parar(&mut self) {
        self.parar.store(true, Ordering::SeqCst);
        for t in self.threads.drain(..) {
            let _ = t.join();
        }
        if let Ok(mut t) = self.tocando.lock() {
            t.clear();
        }
    }
}

impl Drop for Tocador {
    fn drop(&mut self) {
        self.parar.store(true, Ordering::SeqCst);
    }
}

/// Prepara a faixa de áudio conforme a escolha do alarme.
pub fn preparar_faixa(escolha: &SoundChoice, taxa: u32) -> Result<Faixa, String> {
    match escolha {
        SoundChoice::Preset(p) => Ok(tones::gerar_preset(*p, taxa)),
        SoundChoice::Arquivo { caminho } => {
            // Uma via só para todo arquivo de áudio (WAV, MP3, FLAC, OGG).
            let faixa = tones::ler_audio(caminho)?;
            Ok(faixa.reamostrar(taxa))
        }
    }
}

/// Toca a faixa nas saídas pedidas, em paralelo.
///
/// `ganho_final` é de 0 a 100 (volume do alarme). `fade_in_s` faz o volume
/// subir do zero até o alvo. `nomes_alvo` são os nomes das saídas; lista
/// vazia significa a saída padrão do sistema.
pub fn tocar(
    escolha: &SoundChoice,
    nomes_alvo: &[String],
    ganho_final: u8,
    fade_in_s: u32,
) -> Result<Tocador, String> {
    let host = cpal::default_host();

    let alvos: Vec<Device> = if nomes_alvo.is_empty() {
        match host.default_output_device() {
            Some(d) => vec![d],
            None => return Err("Não há saída de áudio disponível.".to_string()),
        }
    } else {
        let devices: Vec<Device> = host.output_devices().map(|d| d.collect()).unwrap_or_default();
        let mut escolhidas = Vec::new();
        for nome in nomes_alvo {
            if let Some(d) = devices
                .iter()
                .find(|d| nome_do_dispositivo(d) == *nome)
            {
                escolhidas.push(d.clone());
            }
        }
        if escolhidas.is_empty() {
            return Err("Nenhuma das saídas escolhidas está disponível agora.".to_string());
        }
        escolhidas
    };

    let parar = Arc::new(AtomicBool::new(false));
    let ganho = Arc::new(AtomicU32::new(0));
    let erros: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let tocando: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let alvo_ganho = (ganho_final.clamp(0, 100) as f32 / 100.0) * 1000.0;

    // Fade in: sobe o ganho devagar, em uma thread só.
    if fade_in_s > 0 {
        let parar_f = parar.clone();
        let ganho_f = ganho.clone();
        std::thread::spawn(move || {
            let passos = (fade_in_s * 10).max(1);
            for i in 0..passos {
                if parar_f.load(Ordering::SeqCst) {
                    return;
                }
                let g = (alvo_ganho * (i + 1) as f32 / passos as f32) as u32;
                ganho_f.store(g, Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            ganho_f.store(alvo_ganho as u32, Ordering::SeqCst);
        });
    } else {
        ganho.store(alvo_ganho as u32, Ordering::SeqCst);
    }

    let mut threads = Vec::new();

    // Cache de faixas por taxa: dispositivos com a mesma taxa compartilham o
    // mesmo Arc<Faixa> em vez de clonar/reamostrar uma cópia para cada um.
    let mut faixas_por_taxa: std::collections::HashMap<u32, Arc<Faixa>> =
        std::collections::HashMap::new();

    // Abertura sequencial: um dispositivo por vez, no thread principal.
    for device in alvos {
        let nome = nome_do_dispositivo(&device);
        let suportado = match device.default_output_config() {
            Ok(c) => c,
            Err(e) => {
                registrar(&erros, format!("{nome}: {e}"));
                continue;
            }
        };
        let config = cpal::StreamConfig {
            channels: suportado.channels(),
            sample_rate: suportado.sample_rate(),
            buffer_size: tamanho_de_buffer(suportado.buffer_size()),
        };
        let taxa_hz = config.sample_rate;
        let faixa = if let Some(cached) = faixas_por_taxa.get(&taxa_hz) {
            Arc::clone(cached)
        } else {
            match preparar_faixa(escolha, taxa_hz) {
                Ok(f) => {
                    let arc = Arc::new(f);
                    faixas_por_taxa.insert(taxa_hz, Arc::clone(&arc));
                    arc
                }
                Err(e) => {
                    registrar(&erros, format!("{nome}: {e}"));
                    continue;
                }
            }
        };

        let stream = match suportado.sample_format() {
            SampleFormat::I16 => abrir_com_reserva::<i16>(&device, config, Arc::clone(&faixa), ganho.clone(), parar.clone()),
            SampleFormat::U16 => abrir_com_reserva::<u16>(&device, config, Arc::clone(&faixa), ganho.clone(), parar.clone()),
            SampleFormat::F32 => abrir_com_reserva::<f32>(&device, config, Arc::clone(&faixa), ganho.clone(), parar.clone()),
            SampleFormat::I32 => abrir_com_reserva::<i32>(&device, config, Arc::clone(&faixa), ganho.clone(), parar.clone()),
            outro => Err(format!("formato de áudio não suportado: {outro:?}")),
        };
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                registrar(&erros, format!("{nome}: {e}"));
                continue;
            }
        };

        let parar_t = parar.clone();
        let tocando_t = tocando.clone();
        let erros_t = erros.clone();
        let nome_t = nome.clone();
        let t = std::thread::spawn(move || {
            if let Err(e) = stream.play() {
                registrar(&erros_t, format!("{nome_t}: {e}"));
                return;
            }
            if let Ok(mut v) = tocando_t.lock() {
                v.push(nome_t.clone());
            }
            while !parar_t.load(Ordering::SeqCst) {
                std::thread::sleep(std::time::Duration::from_millis(120));
            }
            let _ = stream.pause();
        });
        threads.push(t);
    }

    Ok(Tocador {
        parar,
        threads,
        tocando,
        erros,
    })
}

/// Converte o tamanho de buffer suportado em um tamanho usável.
///
/// D15: se o driver recusar o tamanho fixo, o app tenta o tamanho padrão em
/// vez de ficar sem som.
fn tamanho_de_buffer(suportado: &cpal::SupportedBufferSize) -> BufferSize {
    match *suportado {
        cpal::SupportedBufferSize::Range { min, .. } => BufferSize::Fixed(min),
        _ => BufferSize::Default,
    }
}

fn registrar(erros: &Arc<Mutex<Vec<String>>>, mensagem: String) {
    if let Ok(mut v) = erros.lock() {
        v.push(mensagem);
    }
}

/// Abre o stream e, se o driver recusar o tamanho de buffer fixo, tenta de
/// novo com o tamanho padrão (D15). Sem isso, um driver teimoso deixaria o
/// alarme sem som nenhum.
fn abrir_com_reserva<T: SizedSample + FromSample<f32> + Send + 'static>(
    device: &Device,
    config: cpal::StreamConfig,
    faixa: Arc<Faixa>,
    ganho: Arc<AtomicU32>,
    parar: Arc<AtomicBool>,
) -> Result<Stream, String> {
    match abrir_stream::<T>(device, config, Arc::clone(&faixa), ganho.clone(), parar.clone()) {
        Ok(s) => Ok(s),
        Err(primeiro) => {
            let mut config2 = config;
            config2.buffer_size = BufferSize::Default;
            abrir_stream::<T>(device, config2, faixa, ganho, parar)
                .map_err(|segundo| format!("{primeiro} (e com buffer padrão: {segundo})"))
        }
    }
}

/// Abre o stream de saída, já no formato pedido pelo dispositivo.
fn abrir_stream<T: SizedSample + FromSample<f32> + Send + 'static>(
    device: &Device,
    config: cpal::StreamConfig,
    faixa: Arc<Faixa>,
    ganho: Arc<AtomicU32>,
    parar: Arc<AtomicBool>,
) -> Result<Stream, String> {
    let canais = config.channels as usize;
    let dados = faixa;
    let pos = Arc::new(AtomicUsize::new(0));
    let pos_f = pos.clone();
    let ganho_f = ganho.clone();
    let parar_f = parar.clone();

    let callback = move |saida: &mut [T], _: &cpal::OutputCallbackInfo| {
        if parar_f.load(Ordering::SeqCst) {
            for s in saida.iter_mut() {
                *s = T::from_sample(0.0);
            }
            return;
        }
        let g = ganho_f.load(Ordering::SeqCst) as f32 / 1000.0;
        let canais_fonte = dados.canais.len();
        let tamanho_fonte = dados.canais.first().map(|c| c.len()).unwrap_or(0);
        if tamanho_fonte == 0 || canais == 0 {
            return;
        }
        let quadros = saida.len() / canais;
        let inicio = pos_f.load(Ordering::SeqCst);
        for q in 0..quadros {
            for c in 0..canais {
                let idx = q * canais + c;
                if idx >= saida.len() {
                    break;
                }
                let canal_fonte = if canais_fonte == 1 { 0 } else { c.min(canais_fonte - 1) };
                // `.get` em vez de indexar: uma faixa malformada nao pode
                // estourar a thread de audio do Windows.
                let amostra = dados
                    .canais
                    .get(canal_fonte)
                    .and_then(|canal| canal.get((inicio + q) % tamanho_fonte))
                    .copied()
                    .map(i16_para_f32)
                    .unwrap_or(0.0);
                saida[idx] = T::from_sample(amostra * g);
            }
        }
        pos_f.store((inicio + quadros) % tamanho_fonte, Ordering::SeqCst);
    };

    let erro = move |e: cpal::Error| {
        eprintln!("[audio] erro de stream: {e}");
    };

    device
        .build_output_stream(config, callback, erro, None)
        .map_err(|e| e.to_string())
}

/// Toca uma faixa só para teste (usa a saída padrão do sistema).
pub fn testar(escolha: &SoundChoice, ganho_final: u8) -> Result<Tocador, String> {
    tocar(escolha, &[], ganho_final, 0)
}

/// O tocador do alarme em curso, guardado fora da interface.
///
/// O alarme precisa tocar **mesmo com a janela minimizada** (o Windows não
/// desenha uma janela minimizada e a interface pode não rodar). Por isso o
/// som fica aqui, numa variável global, e não dentro da struct da interface.
fn slot() -> &'static Mutex<Option<Tocador>> {
    static SLOT: OnceLock<Mutex<Option<Tocador>>> = OnceLock::new();
    SLOT.get_or_init(|| Mutex::new(None))
}

/// Toca o alarme e guarda o tocador no slot global.
///
/// Devolve as saídas que começaram a tocar. Se não tocou nenhuma, o tocador
/// é descartado e quem chamou decide se tenta de novo.
pub fn tocar_global(
    escolha: &SoundChoice,
    nomes_alvo: &[String],
    ganho_final: u8,
    fade_in_s: u32,
) -> Result<Vec<String>, String> {
    let mut tocador = tocar(escolha, nomes_alvo, ganho_final, fade_in_s)?;
    std::thread::sleep(std::time::Duration::from_millis(200));
    let tocando = tocador.tocando();
    if tocando.is_empty() {
        tocador.parar();
        return Err("Nenhuma saída começou a tocar.".to_string());
    }
    let mut slot = slot().lock().unwrap();
    if let Some(mut antigo) = slot.take() {
        antigo.parar();
    }
    *slot = Some(tocador);
    Ok(tocando)
}

/// Saídas que estão tocando agora.
pub fn tocando_global() -> Vec<String> {
    match slot().lock().ok() {
        Some(slot) => slot.as_ref().map(|t| t.tocando()).unwrap_or_default(),
        None => Vec::new(),
    }
}

/// Para o alarme que estiver tocando.
pub fn parar_global() {
    if let Ok(mut slot) = slot().lock()
        && let Some(mut tocador) = slot.take()
    {
        tocador.parar();
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::model::Preset;

    #[test]
    fn preparar_faixa_de_preset_funciona() {
        let faixa = preparar_faixa(&SoundChoice::Preset(Preset::Sirene), 44100).unwrap();
        assert_eq!(faixa.taxa, 44100);
        assert!(faixa.canais[0].len() > 1000);
    }

    #[test]
    fn listar_saidas_nao_panca() {
        // Só garante que a chamada não entra em pânico (pode não ter áudio).
        let _ = listar_saidas();
    }
}
