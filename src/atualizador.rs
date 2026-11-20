//! Atualização automática via GitHub Releases.
//!
//! O fluxo segue o mesmo padrão do Tranal Sentinel:
//!
//! 1. Consulta a lista de releases do repositório no GitHub.
//! 2. Escolhe a maior versão que tenha um asset `.exe`.
//! 3. Se for mais nova que a versão em execução, avisa a UI.
//! 4. O dono clica "Atualizar" → o exe novo é baixado para a pasta `update`.
//! 5. O app lança uma cópia de si mesmo com `--atualizar <caminho>` e avisa
//!    que a troca acontece quando o app fechar.
//! 6. O processo filho espera o app pai sair, troca o executável e reabre.

use std::cmp::Ordering;
use std::fmt;
use std::fs;
use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering as AO};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const REPOSITORIO: &str = "Matheus-araujo-dev/tranal-despertador";
const NOME_EXE: &str = "tranal-despertador.exe";
const USER_AGENT: &str = "tranal-despertador";
const TIMEOUT_CHECAGEM: Duration = Duration::from_secs(10);
const TIMEOUT_DOWNLOAD: Duration = Duration::from_secs(900);

// ---------------------------------------------------------------------------
// Versão
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Versao {
    pub maior: u32,
    pub menor: u32,
    pub correcao: u32,
}

impl Versao {
    pub fn do_app() -> Self {
        Self::parse(env!("CARGO_PKG_VERSION")).unwrap_or(Versao {
            maior: 0,
            menor: 0,
            correcao: 0,
        })
    }

    pub fn parse(texto: &str) -> Option<Self> {
        let limpo = texto.trim().trim_start_matches(['v', 'V']);
        let partes: Vec<&str> = limpo.split('.').collect();
        if partes.is_empty() {
            return None;
        }
        let maior = partes.first().and_then(|p| p.parse().ok()).unwrap_or(0);
        let menor = partes.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
        let correcao = partes.get(2).and_then(|p| p.parse().ok()).unwrap_or(0);
        Some(Versao {
            maior,
            menor,
            correcao,
        })
    }

    pub fn completa(&self) -> String {
        format!("{}.{}.{}", self.maior, self.menor, self.correcao)
    }
}

impl Ord for Versao {
    fn cmp(&self, outro: &Self) -> Ordering {
        self.maior
            .cmp(&outro.maior)
            .then(self.menor.cmp(&outro.menor))
            .then(self.correcao.cmp(&outro.correcao))
    }
}

impl PartialOrd for Versao {
    fn partial_cmp(&self, outro: &Self) -> Option<Ordering> {
        Some(self.cmp(outro))
    }
}

impl fmt::Display for Versao {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.maior, self.menor, self.correcao)
    }
}

// ---------------------------------------------------------------------------
// Release do GitHub
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ReleaseGitHub {
    pub tag: String,
    pub titulo: String,
    pub url_pagina: String,
    pub nome_asset: String,
    pub url_asset: String,
    pub tamanho: u64,
}

// ---------------------------------------------------------------------------
// Resultado da checagem
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ResultadoChecagem {
    pub ha_versao_nova: bool,
    pub versao_atual: Versao,
    pub versao_nova: Option<Versao>,
    pub release: Option<ReleaseGitHub>,
    pub erro: Option<String>,
}

// ---------------------------------------------------------------------------
// Caminhos
// ---------------------------------------------------------------------------

pub fn pasta_update() -> PathBuf {
    let local = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    local.join("Programs").join("Tranal Despertador").join("update")
}

pub fn caminho_arquivo_novo() -> PathBuf {
    pasta_update().join("tranal-despertador.exe.new")
}

// ---------------------------------------------------------------------------
// Checagem
// ---------------------------------------------------------------------------

pub fn checar() -> ResultadoChecagem {
    let atual = Versao::do_app();

    let url = format!(
        "https://api.github.com/repos/{REPOSITORIO}/releases?per_page=20"
    );

    let resp = match ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/vnd.github+json")
        .timeout(TIMEOUT_CHECAGEM)
        .call()
    {
        Ok(r) => r,
        Err(ureq::Error::Status(404, _)) => {
            return ResultadoChecagem {
                ha_versao_nova: false,
                versao_atual: atual,
                versao_nova: None,
                release: None,
                erro: None,
            };
        }
        Err(e) => {
            return ResultadoChecagem {
                ha_versao_nova: false,
                versao_atual: atual,
                versao_nova: None,
                release: None,
                erro: Some(mensagem_de_erro(&e)),
            };
        }
    };

    let corpo = match resp.into_string() {
        Ok(s) => s,
        Err(_) => {
            return ResultadoChecagem {
                ha_versao_nova: false,
                versao_atual: atual,
                versao_nova: None,
                release: None,
                erro: None,
            };
        }
    };
    let json: serde_json::Value = match serde_json::from_str(&corpo) {
        Ok(v) => v,
        Err(_) => {
            return ResultadoChecagem {
                ha_versao_nova: false,
                versao_atual: atual,
                versao_nova: None,
                release: None,
                erro: None,
            };
        }
    };

    let mut melhor: Option<(Versao, ReleaseGitHub)> = None;

    if let Some(arr) = json.as_array() {
        for item in arr {
            if let Some((versao, release)) = extrair_release(item) {
                match &melhor {
                    None => melhor = Some((versao, release)),
                    Some((v, _)) if versao > *v => melhor = Some((versao, release)),
                    _ => {}
                }
            }
        }
    }

    match melhor {
        None => ResultadoChecagem {
            ha_versao_nova: false,
            versao_atual: atual,
            versao_nova: None,
            release: None,
            erro: None,
        },
        Some((versao, release)) => {
            let nova = versao > atual;
            ResultadoChecagem {
                ha_versao_nova: nova,
                versao_atual: atual,
                versao_nova: Some(versao),
                release: Some(release),
                erro: None,
            }
        }
    }
}

fn extrair_release(item: &serde_json::Value) -> Option<(Versao, ReleaseGitHub)> {
    let tag = item.get("tag_name")?.as_str()?.to_string();
    let versao = Versao::parse(&tag)?;

    let titulo = item
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(&tag)
        .to_string();
    let url_pagina = item
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let assets = item.get("assets")?.as_array()?;

    let mut primeiro_exe: Option<ReleaseGitHub> = None;
    for asset in assets {
        let nome = asset.get("name")?.as_str()?;
        let url = asset
            .get("browser_download_url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let tamanho = asset
            .get("size")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        if nome.eq_ignore_ascii_case(NOME_EXE) {
            return Some((
                versao,
                ReleaseGitHub {
                    tag,
                    titulo,
                    url_pagina,
                    nome_asset: nome.to_string(),
                    url_asset: url,
                    tamanho,
                },
            ));
        }

        if primeiro_exe.is_none() && nome.to_ascii_lowercase().ends_with(".exe") {
            primeiro_exe = Some(ReleaseGitHub {
                tag: tag.clone(),
                titulo: titulo.clone(),
                url_pagina: url_pagina.clone(),
                nome_asset: nome.to_string(),
                url_asset: url,
                tamanho,
            });
        }
    }

    primeiro_exe.map(|r| (versao, r))
}

// ---------------------------------------------------------------------------
// Download
// ---------------------------------------------------------------------------

pub struct Progresso {
    pub baixados: Arc<Mutex<u64>>,
    pub total: u64,
    pub concluido: Arc<AtomicBool>,
    pub erro: Arc<Mutex<Option<String>>>,
}

pub fn baixar_em_segundo_plano(release: &ReleaseGitHub) -> Progresso {
    let destino = caminho_arquivo_novo();
    let url = release.url_asset.clone();
    let total = release.tamanho;

    let baixados = Arc::new(Mutex::new(0u64));
    let concluido = Arc::new(AtomicBool::new(false));
    let erro = Arc::new(Mutex::new(None));

    let b = Arc::clone(&baixados);
    let c = Arc::clone(&concluido);
    let e = Arc::clone(&erro);

    thread::spawn(move || {
        if let Err(msg) = baixar_bloqueante(&url, &destino, total, &b) {
            *e.lock().unwrap() = Some(msg);
        }
        c.store(true, AO::Release);
    });

    Progresso {
        baixados,
        total,
        concluido,
        erro,
    }
}

fn baixar_bloqueante(
    url: &str,
    destino: &Path,
    total_esperado: u64,
    baixados: &Arc<Mutex<u64>>,
) -> Result<(), String> {
    if let Some(pasta) = destino.parent() {
        fs::create_dir_all(pasta).map_err(|e| format!("Não consegui criar a pasta: {e}"))?;
    }

    let resp = ureq::get(url)
        .set("User-Agent", USER_AGENT)
        .timeout(TIMEOUT_DOWNLOAD)
        .call()
        .map_err(|e| format!("Falha no download: {e}"))?;

    let mut reader = resp.into_reader();
    let mut arquivo =
        fs::File::create(destino).map_err(|e| format!("Não consegui criar o arquivo: {e}"))?;

    let mut buf = [0u8; 64 * 1024];
    let mut lidos_total: u64 = 0;

    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("Erro de leitura: {e}"))?;
        if n == 0 {
            break;
        }
        arquivo
            .write_all(&buf[..n])
            .map_err(|e| format!("Erro de escrita: {e}"))?;
        lidos_total += n as u64;
        *baixados.lock().unwrap() = lidos_total;
    }

    arquivo
        .flush()
        .map_err(|e| format!("Erro ao finalizar: {e}"))?;

    if total_esperado > 0 && lidos_total != total_esperado {
        return Err(format!(
            "Arquivo incompleto: {} de {} bytes",
            lidos_total, total_esperado
        ));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Troca do executável
// ---------------------------------------------------------------------------

pub fn iniciar_troca(caminho_novo: &Path) -> io::Result<()> {
    if !caminho_novo.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "O arquivo novo não existe",
        ));
    }

    let exe_atual = std::env::current_exe()?;

    Command::new(&exe_atual)
        .arg("--atualizar")
        .arg(caminho_novo)
        .spawn()?;

    Ok(())
}

/// Executado quando o app é chamado com `--atualizar <caminho_novo>`.
///
/// Espera o processo pai fechar (o exe fica travado enquanto roda), troca o
/// executável no lugar e abre o app atualizado.
pub fn executar_troca(args: &[String]) -> bool {
    let Some(caminho_str) = args.get(args.iter().position(|a| a == "--atualizar").unwrap() + 1)
    else {
        eprintln!("Uso: tranal-despertador --atualizar <caminho_do_novo_exe>");
        std::process::exit(1);
    };
    let caminho_novo = PathBuf::from(caminho_str);

    if !caminho_novo.exists() {
        registrar_no_log(&format!(
            "ERRO: arquivo novo não existe: {}",
            caminho_novo.display()
        ));
        std::process::exit(1);
    }

    let exe_destino = destino_instalado();
    registrar_no_log(&format!(
        "Iniciando troca: {} → {}",
        caminho_novo.display(),
        exe_destino.display()
    ));

    // Espera o exe de destino ficar desbloqueado (o app pai está fechando).
    for tentativa in 0..60 {
        if tentativa > 0 {
            thread::sleep(Duration::from_millis(500));
        }
        match fs::copy(&caminho_novo, &exe_destino) {
            Ok(_) => {
                registrar_no_log("Executável trocado com sucesso.");
                let _ = fs::remove_file(&caminho_novo);

                // Abre o app atualizado.
                let _ = Command::new(&exe_destino).spawn();
                registrar_no_log("App atualizado iniciado.");
                return true;
            }
            Err(e) if tentativa < 59 => {
                if tentativa == 0 {
                    registrar_no_log(&format!("Esperando o app fechar... ({e})"));
                }
            }
            Err(e) => {
                registrar_no_log(&format!("ERRO: não consegui trocar após 30s: {e}"));
                std::process::exit(1);
            }
        }
    }

    false
}

/// O caminho do exe instalado: `%LOCALAPPDATA%\Programs\Tranal Despertador\tranal-despertador.exe`.
fn destino_instalado() -> PathBuf {
    let local = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    local
        .join("Programs")
        .join("Tranal Despertador")
        .join(NOME_EXE)
}

// ---------------------------------------------------------------------------
// Log
// ---------------------------------------------------------------------------

pub fn registrar_no_log(mensagem: &str) {
    let pasta = pasta_update();
    let _ = fs::create_dir_all(&pasta);
    let caminho = pasta.join("atualizacao.log");
    let linha = format!(
        "{}  {}\n",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
        mensagem
    );
    let _ = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(caminho)
        .and_then(|mut f| f.write_all(linha.as_bytes()));
}

// ---------------------------------------------------------------------------
// Formatação
// ---------------------------------------------------------------------------

pub fn formatar_tamanho(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

// ---------------------------------------------------------------------------
// Auxiliares
// ---------------------------------------------------------------------------

fn mensagem_de_erro(e: &ureq::Error) -> String {
    match e {
        ureq::Error::Status(403, _) => {
            "O GitHub recusou a consulta (limite de requisições ou acesso negado).".into()
        }
        ureq::Error::Status(401, _) => "O GitHub recusou a consulta (acesso negado).".into(),
        ureq::Error::Status(429, _) => {
            "O GitHub está limitando as consultas. Tente mais tarde.".into()
        }
        ureq::Error::Status(code, _) => format!("O GitHub respondeu {code}."),
        ureq::Error::Transport(t) => format!("Não foi possível falar com o GitHub: {t}"),
    }
}
