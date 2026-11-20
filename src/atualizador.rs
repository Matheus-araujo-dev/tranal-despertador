//! Atualização automática via GitHub Releases (DEC-APPS-DISTRIBUICAO-001).
//!
//! O fluxo é o padrão de todo app Tranal:
//!
//! 1. Ao abrir e de hora em hora, o app lista as releases do repositório.
//! 2. Escolhe a maior tag `vX.Y.Z` (tag que não é versão é ignorada) que tem
//!    o instalador `Tranal-Despertador-Setup.exe` como asset.
//! 3. Se ela é maior que a versão gravada no binário, baixa o Setup e confere
//!    o tamanho.
//! 4. Roda o Setup em modo silencioso e fecha o app. O Setup (Inno) troca os
//!    arquivos e reabre o app no fim. Os dados do dono ficam em
//!    `%APPDATA%\Tranal Despertador` e não são tocados.
//!
//! Instalação antiga (a do `scripts\instalar.ps1`, em
//! `%LOCALAPPDATA%\Programs\Tranal Despertador`) aceita a release da MESMA
//! versão: é assim que ela passa para o instalador na primeira checagem, em
//! vez de esperar a próxima versão.

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
/// O nome do instalador publicado em cada release (o `OutputBaseFilename` do `.iss`).
pub const NOME_SETUP: &str = "Tranal-Despertador-Setup.exe";
const USER_AGENT: &str = "tranal-despertador";
const TIMEOUT_CHECAGEM: Duration = Duration::from_secs(10);
const TIMEOUT_DOWNLOAD: Duration = Duration::from_secs(900);
/// De quanto em quanto tempo a lista de releases é conferida de novo.
pub const INTERVALO_CHECAGEM: Duration = Duration::from_secs(60 * 60);

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
    /// A versão gravada no binário (o CI sobrescreve a do `Cargo.toml` com a
    /// da release antes de compilar).
    pub fn do_app() -> Self {
        Self::parse(env!("CARGO_PKG_VERSION")).unwrap_or(Versao {
            maior: 0,
            menor: 0,
            correcao: 0,
        })
    }

    /// Lê `X.Y.Z` (com ou sem o `v`). Qualquer outra coisa não é versão.
    pub fn parse(texto: &str) -> Option<Self> {
        let limpo = texto.trim();
        let limpo = limpo.strip_prefix(['v', 'V']).unwrap_or(limpo);
        let mut partes = limpo.split('.');
        let mut numero = || -> Option<u32> {
            let p = partes.next()?;
            if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            p.parse().ok()
        };
        let maior = numero()?;
        let menor = numero()?;
        let correcao = numero()?;
        if partes.next().is_some() {
            return None;
        }
        Some(Versao {
            maior,
            menor,
            correcao,
        })
    }

    /// Uma tag de release é versão só no formato `vX.Y.Z`.
    pub fn da_tag(tag: &str) -> Option<Self> {
        let resto = tag.strip_prefix('v')?;
        Self::parse(resto)
    }

    pub fn completa(&self) -> String {
        self.to_string()
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
    pub url_asset: String,
    pub tamanho: u64,
}

/// Escolhe, na lista de releases da API do GitHub, a que deve ser instalada.
///
/// Parte pura do atualizador (sem rede): pega a maior tag `vX.Y.Z` que não é
/// rascunho nem pré-release e que tem o Setup como asset. Ela só vale se for
/// maior que `atual` — ou igual, quando `aceitar_mesma` (instalação antiga
/// que precisa passar para o instalador).
pub fn escolher_release(
    releases: &serde_json::Value,
    atual: &Versao,
    aceitar_mesma: bool,
) -> Option<(Versao, ReleaseGitHub)> {
    let melhor = releases
        .as_array()?
        .iter()
        .filter_map(extrair_release)
        .max_by(|(a, _), (b, _)| a.cmp(b))?;
    let serve = match melhor.0.cmp(atual) {
        Ordering::Greater => true,
        Ordering::Equal => aceitar_mesma,
        Ordering::Less => false,
    };
    serve.then_some(melhor)
}

fn extrair_release(item: &serde_json::Value) -> Option<(Versao, ReleaseGitHub)> {
    if item.get("draft").and_then(|v| v.as_bool()).unwrap_or(false)
        || item.get("prerelease").and_then(|v| v.as_bool()).unwrap_or(false)
    {
        return None;
    }
    let tag = item.get("tag_name")?.as_str()?;
    let versao = Versao::da_tag(tag)?;
    let titulo = item
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or(tag)
        .to_string();

    let asset = item.get("assets")?.as_array()?.iter().find(|a| {
        a.get("name")
            .and_then(|n| n.as_str())
            .is_some_and(|n| n.eq_ignore_ascii_case(NOME_SETUP))
    })?;
    let url_asset = asset.get("browser_download_url")?.as_str()?.to_string();
    let tamanho = asset.get("size").and_then(|v| v.as_u64()).unwrap_or(0);

    Some((
        versao,
        ReleaseGitHub {
            tag: tag.to_string(),
            titulo,
            url_asset,
            tamanho,
        },
    ))
}

// ---------------------------------------------------------------------------
// Instalação atual
// ---------------------------------------------------------------------------

/// O app está rodando da instalação antiga do `scripts\instalar.ps1`?
pub fn instalacao_legada(exe: &Path) -> bool {
    let texto = exe.to_string_lossy().to_ascii_lowercase().replace('/', "\\");
    texto.contains("\\programs\\tranal despertador\\") && !texto.contains("\\tranal group\\")
}

/// O executável em uso está numa pasta de instalação (antiga ou do Setup)?
///
/// Rodando de `target\` (desenvolvimento), o app não se atualiza sozinho:
/// rodar o Setup ali instalaria por cima do que o dono está testando.
pub fn instalado(exe: &Path) -> bool {
    let texto = exe.to_string_lossy().to_ascii_lowercase().replace('/', "\\");
    instalacao_legada(exe) || texto.contains("\\tranal group\\tranal despertador\\")
}

// ---------------------------------------------------------------------------
// Checagem
// ---------------------------------------------------------------------------

/// Consulta o GitHub e devolve a release a instalar (se houver).
pub fn checar() -> Result<Option<(Versao, ReleaseGitHub)>, String> {
    let atual = Versao::do_app();
    let aceitar_mesma = std::env::current_exe()
        .map(|e| instalacao_legada(&e))
        .unwrap_or(false);

    let url = format!("https://api.github.com/repos/{REPOSITORIO}/releases?per_page=30");
    let resp = match ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .set("Accept", "application/vnd.github+json")
        .timeout(TIMEOUT_CHECAGEM)
        .call()
    {
        Ok(r) => r,
        Err(ureq::Error::Status(404, _)) => return Ok(None),
        Err(e) => return Err(mensagem_de_erro(&e)),
    };
    let corpo = resp
        .into_string()
        .map_err(|e| format!("Resposta do GitHub ilegível: {e}"))?;
    let json: serde_json::Value =
        serde_json::from_str(&corpo).map_err(|e| format!("JSON do GitHub inválido: {e}"))?;
    Ok(escolher_release(&json, &atual, aceitar_mesma))
}

// ---------------------------------------------------------------------------
// Download
// ---------------------------------------------------------------------------

/// Pasta onde o Setup baixado fica até rodar.
pub fn pasta_update() -> PathBuf {
    let local = dirs::data_local_dir().unwrap_or_else(std::env::temp_dir);
    local.join("Tranal Despertador").join("update")
}

pub fn caminho_setup() -> PathBuf {
    pasta_update().join(NOME_SETUP)
}

pub struct Progresso {
    pub baixados: Arc<Mutex<u64>>,
    pub total: u64,
    pub concluido: Arc<AtomicBool>,
    pub erro: Arc<Mutex<Option<String>>>,
}

pub fn baixar_em_segundo_plano(release: &ReleaseGitHub) -> Progresso {
    let destino = caminho_setup();
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
            *e.lock().unwrap_or_else(|p| p.into_inner()) = Some(msg);
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

    // Baixa num .part e só renomeia completo: um Setup pela metade nunca roda.
    let parcial = destino.with_extension("part");
    let mut reader = resp.into_reader();
    let mut arquivo =
        fs::File::create(&parcial).map_err(|e| format!("Não consegui criar o arquivo: {e}"))?;

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
        *baixados.lock().unwrap_or_else(|p| p.into_inner()) = lidos_total;
    }
    arquivo
        .flush()
        .map_err(|e| format!("Erro ao finalizar: {e}"))?;
    drop(arquivo);

    if total_esperado == 0 || lidos_total != total_esperado {
        let _ = fs::remove_file(&parcial);
        return Err(format!(
            "Arquivo incompleto: {lidos_total} de {total_esperado} bytes"
        ));
    }
    let _ = fs::remove_file(destino);
    fs::rename(&parcial, destino).map_err(|e| format!("Não consegui finalizar o Setup: {e}"))
}

// ---------------------------------------------------------------------------
// Instalação
// ---------------------------------------------------------------------------

/// Argumentos do Setup em modo silencioso (DEC-APPS-DISTRIBUICAO-001).
///
/// `/MINIMIZADO=1` é lido pelo `.iss`: o app reaberto no fim volta escondido
/// na bandeja, como estava, em vez de aparecer na frente do dono.
pub fn argumentos_setup(minimizado: bool) -> Vec<&'static str> {
    let mut args = vec![
        "/VERYSILENT",
        "/SUPPRESSMSGBOXES",
        "/NORESTART",
        "/CLOSEAPPLICATIONS",
    ];
    if minimizado {
        args.push("/MINIMIZADO=1");
    }
    args
}

/// Lança o Setup baixado. Quem chama fecha o app logo em seguida.
pub fn rodar_setup(setup: &Path, minimizado: bool) -> io::Result<()> {
    if !setup.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "O Setup baixado não existe",
        ));
    }
    Command::new(setup)
        .args(argumentos_setup(minimizado))
        .spawn()?;
    Ok(())
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

// ---------------------------------------------------------------------------
// Testes
// ---------------------------------------------------------------------------

#[cfg(test)]
mod testes {
    use super::*;
    use serde_json::json;

    fn v(texto: &str) -> Versao {
        Versao::parse(texto).unwrap()
    }

    fn release(tag: &str, asset: &str) -> serde_json::Value {
        json!({
            "tag_name": tag,
            "name": tag,
            "draft": false,
            "prerelease": false,
            "assets": [
                { "name": "tranal-despertador.exe", "browser_download_url": "https://x/exe", "size": 10 },
                { "name": asset, "browser_download_url": format!("https://x/{tag}/{asset}"), "size": 1234 }
            ]
        })
    }

    #[test]
    fn versao_so_aceita_tres_numeros() {
        assert_eq!(v("0.4.1"), Versao { maior: 0, menor: 4, correcao: 1 });
        assert_eq!(v("v1.2.3"), Versao { maior: 1, menor: 2, correcao: 3 });
        assert!(Versao::parse("1.2").is_none());
        assert!(Versao::parse("1.2.3.4").is_none());
        assert!(Versao::parse("1.2.x").is_none());
        assert!(Versao::parse("latest").is_none());
        assert!(Versao::parse("1.2.3-beta").is_none());
    }

    #[test]
    fn tag_de_versao_exige_v() {
        assert_eq!(Versao::da_tag("v0.10.2"), Some(v("0.10.2")));
        assert!(Versao::da_tag("0.10.2").is_none());
        assert!(Versao::da_tag("latest").is_none());
        assert!(Versao::da_tag("v1.0").is_none());
    }

    #[test]
    fn comparacao_e_numerica_e_nao_textual() {
        assert!(v("0.10.0") > v("0.9.9"));
        assert!(v("1.0.0") > v("0.99.99"));
        assert!(v("0.4.10") > v("0.4.9"));
    }

    #[test]
    fn escolhe_a_maior_tag_maior_que_a_atual() {
        let lista = json!([
            release("v0.4.2", NOME_SETUP),
            release("v0.10.0", NOME_SETUP),
            release("v0.9.0", NOME_SETUP),
        ]);
        let (versao, rel) = escolher_release(&lista, &v("0.4.0"), false).unwrap();
        assert_eq!(versao, v("0.10.0"));
        assert_eq!(rel.tag, "v0.10.0");
        assert_eq!(rel.tamanho, 1234);
        assert!(rel.url_asset.ends_with(NOME_SETUP));
    }

    #[test]
    fn ignora_tag_que_nao_e_versao() {
        let lista = json!([
            release("latest", NOME_SETUP),
            release("v2.0", NOME_SETUP),
            release("nightly-5", NOME_SETUP),
            release("v0.5.0", NOME_SETUP),
        ]);
        let (versao, _) = escolher_release(&lista, &v("0.4.0"), false).unwrap();
        assert_eq!(versao, v("0.5.0"));
    }

    #[test]
    fn ignora_release_sem_setup_rascunho_e_prerelease() {
        let mut rascunho = release("v0.9.0", NOME_SETUP);
        rascunho["draft"] = json!(true);
        let mut pre = release("v0.8.0", NOME_SETUP);
        pre["prerelease"] = json!(true);
        let lista = json!([
            release("v1.0.0", "outro-arquivo.zip"),
            rascunho,
            pre,
            release("v0.5.0", NOME_SETUP),
        ]);
        let (versao, _) = escolher_release(&lista, &v("0.4.0"), false).unwrap();
        assert_eq!(versao, v("0.5.0"));
    }

    #[test]
    fn nada_a_fazer_quando_ja_esta_na_maior() {
        let lista = json!([release("v0.4.0", NOME_SETUP), release("v0.3.9", NOME_SETUP)]);
        assert!(escolher_release(&lista, &v("0.4.0"), false).is_none());
        assert!(escolher_release(&lista, &v("0.5.0"), false).is_none());
        assert!(escolher_release(&json!([]), &v("0.4.0"), false).is_none());
        assert!(escolher_release(&json!({"message": "x"}), &v("0.4.0"), false).is_none());
    }

    #[test]
    fn instalacao_antiga_aceita_a_mesma_versao() {
        let lista = json!([release("v0.4.0", NOME_SETUP)]);
        let (versao, _) = escolher_release(&lista, &v("0.4.0"), true).unwrap();
        assert_eq!(versao, v("0.4.0"));
        // Mas nunca volta para uma versão menor.
        assert!(escolher_release(&lista, &v("0.5.0"), true).is_none());
    }

    #[test]
    fn reconhece_a_instalacao_antiga_e_a_nova() {
        let antiga = Path::new(
            r"C:\Users\x\AppData\Local\Programs\Tranal Despertador\tranal-despertador.exe",
        );
        let nova = Path::new(
            r"C:\Users\x\AppData\Local\Programs\Tranal Group\Tranal Despertador\tranal-despertador.exe",
        );
        let pf = Path::new(r"C:\Program Files\Tranal Group\Tranal Despertador\tranal-despertador.exe");
        let dev = Path::new(r"E:\Projetos\tranal-despertador\target\release\tranal-despertador.exe");
        assert!(instalacao_legada(antiga));
        assert!(!instalacao_legada(nova));
        assert!(!instalacao_legada(pf));
        assert!(instalado(antiga) && instalado(nova) && instalado(pf));
        assert!(!instalado(dev));
    }

    #[test]
    fn setup_roda_silencioso_e_fecha_o_app() {
        let args = argumentos_setup(false);
        for esperado in ["/VERYSILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/CLOSEAPPLICATIONS"] {
            assert!(args.contains(&esperado), "falta {esperado}");
        }
        assert!(!args.contains(&"/MINIMIZADO=1"));
        assert!(argumentos_setup(true).contains(&"/MINIMIZADO=1"));
    }
}
