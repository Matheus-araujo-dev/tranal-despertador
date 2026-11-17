//! Sons do alarme: presets sintetizados e leitura de arquivos de áudio.
//!
//! Os presets são gerados por conta própria (nada de material de terceiros
//! dentro do app). O dono também pode **adicionar os próprios sons**: o
//! arquivo é copiado para a pasta do app e decodificado por uma via única
//! (`symphonia`), que entende WAV, MP3, FLAC e OGG.

use std::path::{Path, PathBuf};

use crate::model::Preset;

/// Duração máxima aceita para o arquivo de áudio do dono (o som repete em laco).
pub const DURACAO_MAXIMA_S: f32 = 120.0;

/// Extensões que o app sabe decodificar.
pub const EXTENSOES: &[&str] = &["wav", "mp3", "flac", "ogg", "oga"];

/// Uma faixa de áudio em ponto flutuante, com 1 ou 2 canais.
#[derive(Clone, Debug)]
pub struct Faixa {
    pub taxa: u32,
    pub canais: Vec<Vec<f32>>,
}

impl Faixa {
    pub fn duracao_s(&self) -> f32 {
        self.canais
            .first()
            .map(|c| c.len() as f32 / self.taxa as f32)
            .unwrap_or(0.0)
    }

    /// Reamostra para outra taxa (linear, suficiente para alarme).
    pub fn reamostrar(&self, nova: u32) -> Faixa {
        if nova == self.taxa || self.canais.is_empty() {
            return self.clone();
        }
        let n = self.canais[0].len();
        let novo_n = ((n as f64) * (nova as f64) / (self.taxa as f64)).round() as usize;
        let mut canais = Vec::with_capacity(self.canais.len());
        for canal in &self.canais {
            let mut saida = Vec::with_capacity(novo_n);
            for i in 0..novo_n {
                let pos = (i as f64) * (self.taxa as f64) / (nova as f64);
                let i0 = pos.floor() as usize;
                let i1 = (i0 + 1).min(n.saturating_sub(1));
                let frac = (pos - i0 as f64) as f32;
                let a = canal.get(i0).copied().unwrap_or(0.0);
                let b = canal.get(i1).copied().unwrap_or(0.0);
                saida.push(a + (b - a) * frac);
            }
            canais.push(saida);
        }
        Faixa {
            taxa: nova,
            canais,
        }
    }
}

/// Envelope suave nas bordas para não estalar.
fn suavizar(canais: &mut [Vec<f32>], amostras: usize) {
    for canal in canais.iter_mut() {
        let n = canal.len();
        if n == 0 {
            continue;
        }
        let m = amostras.min(n / 2);
        for i in 0..m {
            let g = i as f32 / m as f32;
            canal[i] *= g;
            let j = n - 1 - i;
            canal[j] *= g;
        }
    }
}

/// Gera o preset pedido, com 2 canais, na taxa pedida.
pub fn gerar_preset(preset: Preset, taxa: u32) -> Faixa {
    let t = taxa as f32;
    let mut esquerda: Vec<f32> = Vec::new();
    let mut direita: Vec<f32> = Vec::new();

    let push = |duracao_s: f32, freq: f32, tipo: Onda, gain: f32, esq: &mut Vec<f32>, dir: &mut Vec<f32>| {
        let n = (t * duracao_s) as usize;
        for i in 0..n {
            let s = t * i as f32 / taxa as f32;
            let v = match tipo {
                Onda::Seno => (2.0 * std::f32::consts::PI * freq * s).sin(),
                Onda::Quadrada => {
                    if (2.0 * std::f32::consts::PI * freq * s).sin() >= 0.0 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                Onda::Varredura => {
                    // Ida e volta entre 500 Hz e 1400 Hz ao longo do trecho.
                    let fase = (s / duracao_s.max(0.001)) * 2.0;
                    let tri = if fase <= 1.0 { fase } else { 2.0 - fase };
                    let f = 500.0 + 900.0 * tri;
                    (2.0 * std::f32::consts::PI * f * s).sin()
                }
                Onda::Envelope => {
                    let env = 0.5 - 0.5 * (2.0 * std::f32::consts::PI * s / duracao_s.max(0.001)).cos();
                    (2.0 * std::f32::consts::PI * freq * s).sin() * env
                }
            };
            let v = v * gain;
            esq.push(v);
            dir.push(v);
        }
    };

    match preset {
        Preset::Classico => {
            push(0.35, 880.0, Onda::Seno, 0.85, &mut esquerda, &mut direita);
            push(0.18, 0.0, Onda::Seno, 0.0, &mut esquerda, &mut direita);
            push(0.35, 660.0, Onda::Seno, 0.85, &mut esquerda, &mut direita);
            push(0.62, 0.0, Onda::Seno, 0.0, &mut esquerda, &mut direita);
        }
        Preset::Digital => {
            for _ in 0..4 {
                push(0.09, 1000.0, Onda::Quadrada, 0.55, &mut esquerda, &mut direita);
                push(0.08, 0.0, Onda::Seno, 0.0, &mut esquerda, &mut direita);
            }
            push(0.60, 0.0, Onda::Seno, 0.0, &mut esquerda, &mut direita);
        }
        Preset::Sirene => {
            push(1.30, 0.0, Onda::Varredura, 0.85, &mut esquerda, &mut direita);
            push(0.40, 0.0, Onda::Seno, 0.0, &mut esquerda, &mut direita);
        }
        Preset::Cuco => {
            push(0.13, 1300.0, Onda::Seno, 0.8, &mut esquerda, &mut direita);
            push(0.09, 0.0, Onda::Seno, 0.0, &mut esquerda, &mut direita);
            push(0.13, 1300.0, Onda::Seno, 0.8, &mut esquerda, &mut direita);
            push(0.16, 0.0, Onda::Seno, 0.0, &mut esquerda, &mut direita);
            push(0.26, 1000.0, Onda::Seno, 0.8, &mut esquerda, &mut direita);
            push(0.55, 0.0, Onda::Seno, 0.0, &mut esquerda, &mut direita);
        }
        Preset::Onda => {
            push(0.55, 440.0, Onda::Envelope, 0.9, &mut esquerda, &mut direita);
            push(0.55, 440.0, Onda::Envelope, 0.9, &mut esquerda, &mut direita);
        }
    }

    let mut canais = vec![esquerda, direita];
    suavizar(&mut canais, 128);
    Faixa { taxa, canais }
}

#[derive(Clone, Copy)]
enum Onda {
    Seno,
    Quadrada,
    Varredura,
    Envelope,
}

/// Lê um arquivo de áudio (WAV, MP3, FLAC, OGG) e devolve a faixa.
///
/// Uma via única para todos os formatos: o decodificador (`symphonia`) devolve
/// amostras interlaçadas em `f32`, e aqui elas viram um vetor por canal.
///
/// O canal é preservado como é: um arquivo **mono** fica com um canal só, e o
/// callback de áudio repete esse canal em todas as saídas do aparelho.
pub fn ler_audio(caminho: &str) -> Result<Faixa, String> {
    use symphonia::core::codecs::audio::AudioDecoderOptions;
    use symphonia::core::errors::Error;
    use symphonia::core::formats::probe::Hint;
    use symphonia::core::formats::{FormatOptions, TrackType};
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;

    let arquivo =
        std::fs::File::open(caminho).map_err(|e| format!("Não consegui abrir o arquivo: {e}"))?;
    let mss = MediaSourceStream::new(Box::new(arquivo), Default::default());

    // A dica ajuda o detector a não adivinhar o formato (MP3 e OGG se
    // confundem quando o arquivo não tem cabeçalho).
    let mut hint = Hint::new();
    if let Some(ext) = Path::new(caminho)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
    {
        hint.with_extension(&ext);
    }

    let mut format = symphonia::default::get_probe()
        .probe(
            &hint,
            mss,
            FormatOptions::default(),
            MetadataOptions::default(),
        )
        .map_err(|e| format!("Não consegui ler esse arquivo de áudio: {e}"))?;

    let trilha = format
        .default_track(TrackType::Audio)
        .ok_or_else(|| "Esse arquivo não tem trilha de áudio.".to_string())?;
    let id_trilha = trilha.id;
    let params = trilha
        .codec_params
        .as_ref()
        .ok_or_else(|| "Esse arquivo não tem dados de áudio.".to_string())?;
    let audio = params
        .audio()
        .ok_or_else(|| "Esse arquivo não tem dados de áudio.".to_string())?;

    let mut decoder = symphonia::default::get_codecs()
        .make_audio_decoder(audio, &AudioDecoderOptions::default())
        .map_err(|e| format!("Esse formato de áudio não é suportado: {e}"))?;

    // Amostras de todos os canais, misturadas (intercaladas) enquanto decodifica.
    let mut plano: Vec<f32> = Vec::new();
    let mut canais = audio.channels.as_ref().map(|c| c.count()).unwrap_or(2).max(1);
    let mut taxa = audio.sample_rate.unwrap_or(44_100);

    loop {
        let pacote = match format.next_packet() {
            Ok(Some(p)) => p,
            Ok(None) => break,
            // O fim da trilha: o arquivo acabou.
            Err(Error::ResetRequired) => break,
            Err(Error::IoError(_)) => continue,
            Err(e) => return Err(format!("Erro ao ler o áudio: {e}")),
        };
        if pacote.track_id != id_trilha {
            continue;
        }
        let decodificado = match decoder.decode(&pacote) {
            Ok(d) => d,
            // Pacote ruim: pula e segue (arquivo com um quadro corrompido).
            Err(Error::IoError(_)) | Err(Error::DecodeError(_)) => continue,
            Err(e) => return Err(format!("Erro ao decodificar o áudio: {e}")),
        };

        // A taxa e o número de canais saem do proprio decodificador: e ele
        // quem sabe quantos canais saiu do pacote.
        let spec = decodificado.spec();
        taxa = spec.rate();
        if taxa == 0 {
            continue;
        }
        let limite_quadros = (DURACAO_MAXIMA_S * taxa as f32) as usize;

        let antes = plano.len();
        coletar(&decodificado, &mut plano, &mut canais);
        if plano.len() == antes {
            continue;
        }

        // D5: um audio longo consome centenas de MB e estoura a meta de consumo.
        // O som do alarme e curto e repete em laco, entao um limite e seguro.
        if plano.len() / canais > limite_quadros {
            let duracao = plano.len() as f32 / canais as f32 / taxa as f32;
            return Err(format!(
                "Esse audio tem {duracao:.0}s. O limite do despertador e {DURACAO_MAXIMA_S:.0}s."
            ));
        }
    }

    if plano.is_empty() {
        return Err("Esse arquivo não tem amostras de áudio.".to_string());
    }

    // Um vetor por canal.
    let mut saida: Vec<Vec<f32>> = (0..canais)
        .map(|_| Vec::with_capacity(plano.len() / canais))
        .collect();
    for (i, v) in plano.iter().enumerate() {
        let valor = if v.is_finite() { v.clamp(-1.0, 1.0) } else { 0.0 };
        saida[i % canais].push(valor);
    }

    Ok(Faixa {
        taxa,
        canais: saida,
    })
}

/// Copia as amostras de um buffer decodificado para o vetor intercalado.
///
/// O `symphonia` entrega um buffer generico (o tipo da amostra depende do
/// codec); aqui tudo vira `f32`.
fn coletar(
    decodificado: &symphonia::core::audio::GenericAudioBufferRef<'_>,
    plano: &mut Vec<f32>,
    canais: &mut usize,
) {
    use symphonia::core::audio::conv::IntoSample;
    use symphonia::core::audio::{Audio, GenericAudioBufferRef};

    macro_rules! copia {
        ($buf:expr) => {{
            let buf = $buf;
            let quadros = buf.frames();
            let n = buf.num_planes();
            if n == 0 {
                return;
            }
            *canais = n;
            for i in 0..quadros {
                for p in 0..n {
                    // Sem valor "zero" do tipo da amostra (u24/i24 nao tem
                    // um zero obvious): o que nao existir vira silencio.
                    let v: f32 = match buf.plane(p) {
                        Some(linha) => match linha.get(i) {
                            Some(&amostra) => amostra.into_sample(),
                            None => 0.0,
                        },
                        None => 0.0,
                    };
                    plano.push(v);
                }
            }
        }};
    }

    match decodificado {
        GenericAudioBufferRef::U8(b) => copia!(b),
        GenericAudioBufferRef::U16(b) => copia!(b),
        GenericAudioBufferRef::U24(b) => copia!(b),
        GenericAudioBufferRef::U32(b) => copia!(b),
        GenericAudioBufferRef::S8(b) => copia!(b),
        GenericAudioBufferRef::S16(b) => copia!(b),
        GenericAudioBufferRef::S24(b) => copia!(b),
        GenericAudioBufferRef::S32(b) => copia!(b),
        GenericAudioBufferRef::F32(b) => copia!(b),
        GenericAudioBufferRef::F64(b) => copia!(b),
    }
}

/// Um áudio que o dono adicionou ao app.
#[derive(Clone, Debug)]
pub struct AudioDaBiblioteca {
    /// Nome do arquivo (como aparece na lista).
    pub nome: String,
    /// Caminho completo dentro da pasta do app.
    pub caminho: String,
}

/// Copia um arquivo de áudio para a pasta do app.
///
/// O app passa a **ter** o som: se o dono mover ou apagar o arquivo original,
/// o despertador continua tocando. Devolve o caminho do arquivo copiado.
pub fn importar_audio(origem: &str) -> Result<String, String> {
    let origem = Path::new(origem);
    let nome = origem
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Esse arquivo não tem nome.".to_string())?
        .to_string();
    let ext = origem
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !EXTENSOES.contains(&ext.as_str()) {
        return Err(format!(
            "Esse formato ({} ) não é suportado. Use: {}.",
            ext,
            EXTENSOES.join(", ")
        ));
    }

    let pasta = crate::storage::pasta_sons();
    std::fs::create_dir_all(&pasta)
        .map_err(|e| format!("Não consegui criar {}: {e}", pasta.display()))?;

    // O arquivo ja esta na pasta do app: nao copia de novo (uma copia
    // seria identica, e o dono acabaria com dois arquivos iguais).
    if let (Ok(a), Ok(b)) = (std::fs::canonicalize(origem), std::fs::canonicalize(&pasta)) {
        if a.starts_with(&b) {
            return Ok(origem.to_string_lossy().to_string());
        }
    }

    let destino = caminho_livre(&pasta, &nome);
    std::fs::copy(origem, &destino).map_err(|e| format!("Não consegui copiar o arquivo: {e}"))?;
    Ok(destino.to_string_lossy().to_string())
}

/// Escolhe um nome que ainda não existe na pasta.
fn caminho_livre(pasta: &Path, nome: &str) -> PathBuf {
    let original = pasta.join(nome);
    if !original.exists() {
        return original;
    }
    let sem_ext = Path::new(nome)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("som");
    let ext = Path::new(nome)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    for n in 2..1000 {
        let novo = if ext.is_empty() {
            format!("{sem_ext} ({n})")
        } else {
            format!("{sem_ext} ({n}).{ext}")
        };
        let caminho = pasta.join(novo);
        if !caminho.exists() {
            return caminho;
        }
    }
    original
}

/// Lista os áudios que o dono adicionou ao app.
pub fn biblioteca() -> Vec<AudioDaBiblioteca> {
    let pasta = crate::storage::pasta_sons();
    let Ok(entradas) = std::fs::read_dir(&pasta) else {
        return Vec::new();
    };
    let mut lista: Vec<AudioDaBiblioteca> = entradas
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let caminho = e.path();
            let nome = caminho.file_name()?.to_str()?.to_string();
            let ext = caminho
                .extension()
                .and_then(|x| x.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if !EXTENSOES.contains(&ext.as_str()) {
                return None;
            }
            Some(AudioDaBiblioteca {
                nome,
                caminho: caminho.to_string_lossy().to_string(),
            })
        })
        .collect();
    lista.sort_by_key(|a| a.nome.to_lowercase());
    lista
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn todos_os_presets_geram_audio() {
        for preset in Preset::all() {
            let faixa = gerar_preset(preset, 48000);
            assert!(!faixa.canais.is_empty(), "{:?} sem canais", preset);
            assert!(faixa.canais[0].len() > 1000, "{:?} curto demais", preset);
            let max = faixa.canais[0].iter().cloned().fold(0.0f32, f32::max);
            assert!(max > 0.3, "{:?} sem amplitude", preset);
            assert!(max <= 1.0, "{:?} estourou o limite", preset);
        }
    }

    #[test]
    fn reamostragem_mantem_a_duracao() {
        let faixa = gerar_preset(Preset::Classico, 48000);
        let antes = faixa.duracao_s();
        let nova = faixa.reamostrar(44100);
        let depois = nova.duracao_s();
        assert!((antes - depois).abs() < 0.02, "antes {antes} depois {depois}");
        assert_eq!(nova.taxa, 44100);
    }

    #[test]
    fn arquivo_inexistente_da_erro_claro() {
        let erro = ler_audio("C:\\nao\\existe\\arquivo.wav").unwrap_err();
        assert!(erro.contains("Não consegui abrir"), "erro: {erro}");
    }

    #[test]
    fn arquivo_que_nao_e_audio_da_erro_claro() {
        let caminho = std::env::temp_dir().join(format!("tranal-nao-audio-{}.txt", std::process::id()));
        std::fs::write(&caminho, b"isso nao e audio").expect("falha ao escrever");
        let erro = ler_audio(caminho.to_str().unwrap()).unwrap_err();
        let _ = std::fs::remove_file(&caminho);
        assert!(
            erro.contains("áudio") || erro.contains("audio"),
            "erro inesperado: {erro}"
        );
    }

    #[test]
    fn importar_copia_o_arquivo_para_a_pasta_do_app() {
        let origem = std::env::temp_dir().join(format!("tranal-importa-{}.mp3", std::process::id()));
        std::fs::write(&origem, b"falso mp3, mas a copia e o que importa").expect("falha ao escrever");
        let caminho = importar_audio(origem.to_str().unwrap()).expect("deveria importar");
        assert!(Path::new(&caminho).exists(), "o arquivo copiado tem que existir");
        assert!(
            Path::new(&caminho).starts_with(crate::storage::pasta_sons()),
            "a copia tem que ficar na pasta do app: {caminho}"
        );
        // Um arquivo que ja esta na pasta do app nao e copiado de novo.
        let segunda = importar_audio(&caminho).expect("deveria importar de novo");
        assert_eq!(segunda, caminho, "nao pode criar uma copia identica");
        let _ = std::fs::remove_file(&caminho);
        let _ = std::fs::remove_file(&origem);
    }

    #[test]
    fn formato_desconhecido_e_recusado() {
        let erro = importar_audio("C:\\nao\\existe\\coisa.xyz").unwrap_err();
        assert!(erro.contains("não é suportado"), "erro: {erro}");
    }

    /// Monta um WAV PCM 16 bits mono ou stereo em memoria.
    fn wav_de_teste(canais: u16, quadros: usize) -> Vec<u8> {
        let taxa: u32 = 44100;
        let bits: u16 = 16;
        let dados = canais as usize * quadros * 2;
        let mut b: Vec<u8> = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + dados as u32).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&canais.to_le_bytes());
        b.extend_from_slice(&taxa.to_le_bytes());
        b.extend_from_slice(&(taxa * canais as u32 * 2).to_le_bytes());
        b.extend_from_slice(&(canais * 2).to_le_bytes());
        b.extend_from_slice(&bits.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&(dados as u32).to_le_bytes());
        for i in 0..dados / 2 {
            // onda de teste dentro do limite do i16 (evita overflow no build de debug)
            b.extend_from_slice(&(((i % 100) as i16 - 50) * 100).to_le_bytes());
        }
        b
    }

    fn escrever_wav_temporario(nome: &str, bytes: &[u8]) -> std::path::PathBuf {
        // O nome precisa ser unico: dois arquivos de teste podem ter exatamente o
        // mesmo tamanho, e os testes rodam em paralelo.
        let unico = format!(
            "tranal-teste-{}-{}-{:?}.wav",
            nome,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let caminho = std::env::temp_dir().join(unico);
        std::fs::write(&caminho, bytes).expect("falha ao escrever o wav de teste");
        caminho
    }

    #[test]
    fn wav_mono_preserva_o_canal_e_tem_som() {
        // Um WAV mono fica com UM canal (o callback de audio repete esse
        // canal em todas as saidas do aparelho). O bug antigo era o oposto:
        // o segundo canal ficava vazio e o callback estourava a thread.
        let bytes = wav_de_teste(1, 1000);
        let caminho = escrever_wav_temporario("mono", &bytes);
        let faixa = ler_audio(caminho.to_str().unwrap()).expect("wav mono deveria ser lido");
        let _ = std::fs::remove_file(&caminho);

        assert_eq!(faixa.canais.len(), 1, "o mono continua com um canal");
        assert_eq!(faixa.canais[0].len(), 1000);
        assert!(
            faixa.canais[0].iter().any(|v| *v != 0.0),
            "o canal do mono nao pode ficar mudo"
        );
    }

    #[test]
    fn wav_stereo_mantem_os_dois_canais() {
        let bytes = wav_de_teste(2, 500);
        let caminho = escrever_wav_temporario("stereo", &bytes);
        let faixa = ler_audio(caminho.to_str().unwrap()).expect("wav stereo deveria ser lido");
        let _ = std::fs::remove_file(&caminho);

        assert_eq!(faixa.canais.len(), 2);
        for (i, canal) in faixa.canais.iter().enumerate() {
            assert_eq!(canal.len(), 500, "canal {i} com tamanho errado");
        }
    }

    #[test]
    fn wav_longo_e_recusado() {
        // D5: um audio de mais de DURACAO_MAXIMA_S consome centenas de MB.
        // 121 s a 44100 Hz, mono, 16 bits.
        let bytes = wav_de_teste(1, 121 * 44100);
        let caminho = escrever_wav_temporario("longo", &bytes);
        let erro = ler_audio(caminho.to_str().unwrap()).unwrap_err();
        let _ = std::fs::remove_file(&caminho);
        assert!(
            erro.contains("limite do despertador"),
            "erro inesperado: {erro}"
        );
    }

    #[test]
    fn wav_no_limite_e_aceito() {
        let bytes = wav_de_teste(1, (DURACAO_MAXIMA_S as usize) * 44100);
        let caminho = escrever_wav_temporario("no-limite", &bytes);
        let faixa = ler_audio(caminho.to_str().unwrap()).expect("wav no limite deveria ser lido");
        let _ = std::fs::remove_file(&caminho);
        assert_eq!(faixa.canais.len(), 1);
    }

    /// Fixture real de MP3 (gerado com ffmpeg: 1 s de seno a 440 Hz).
    const MP3_MONO: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/teste-mp3.mp3"
    );
    const MP3_STEREO: &str = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/teste-mp3-stereo.mp3"
    );

    /// O dono perguntou por MP3: o app precisa ler, e não só aceitar WAV.
    #[test]
    fn mp3_mono_e_lido_e_tem_som() {
        let faixa = ler_audio(MP3_MONO).expect("mp3 deveria ser lido");
        assert_eq!(faixa.canais.len(), 1, "o mp3 mono fica com um canal");
        assert!(
            (faixa.duracao_s() - 1.0).abs() < 0.2,
            "duracao inesperada: {}",
            faixa.duracao_s()
        );
        let max = faixa.canais[0].iter().cloned().fold(0.0f32, f32::max);
        assert!(max > 0.5, "o mp3 precisa ter amplitude (max {max})");
    }

    #[test]
    fn mp3_stereo_mantem_os_dois_canais() {
        let faixa = ler_audio(MP3_STEREO).expect("mp3 stereo deveria ser lido");
        assert_eq!(faixa.canais.len(), 2);
        for (i, canal) in faixa.canais.iter().enumerate() {
            assert!(!canal.is_empty(), "canal {i} vazio");
            let max = canal.iter().cloned().fold(0.0f32, f32::max);
            assert!(max > 0.5, "canal {i} sem amplitude (max {max})");
        }
    }
}
