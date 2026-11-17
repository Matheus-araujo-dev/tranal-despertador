//! Leitura e gravação do arquivo de dados do app.
//!
//! Um único JSON em `%APPDATA%\Tranal Despertador\dados.json`. A gravação é
//! atômica (escreve num .tmp e renomeia) para o arquivo nunca ficar pela
//! metade se o PC desligar no meio da escrita.
//!
//! Duas threads gravam neste arquivo: a interface, quando o dono edita um
//! despertador, e o agendador, quando um alarme toca. Por isso a escrita é
//! serializada por uma trava: duas gravações ao mesmo tempo no mesmo
//! arquivo .tmp produziriam um JSON pela metade.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use crate::model::{Alarm, BancoDeDados, Settings};

/// Trava de escrita: serializa as gravações do arquivo de dados.
static ESCRITA: Mutex<()> = Mutex::new(());

/// Pega a trava de escrita, sem morrer se outra thread envenenou o mutex.
fn trava() -> MutexGuard<'static, ()> {
    ESCRITA.lock().unwrap_or_else(|e| e.into_inner())
}

/// Pasta de dados do app no perfil do usuário.
pub fn pasta_dados() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("Tranal Despertador")
}

/// Caminho do arquivo de dados.
pub fn caminho_dados() -> PathBuf {
    pasta_dados().join("dados.json")
}

/// Pasta onde ficam os WAVs do dono.
pub fn pasta_sons() -> PathBuf {
    pasta_dados().join("sons")
}

/// Carrega os dados. Se o arquivo não existir ou estiver quebrado, devolve o
/// estado padrão sem quebrar o app.
pub fn carregar() -> (Vec<Alarm>, Settings) {
    let caminho = caminho_dados();
    let Ok(texto) = fs::read_to_string(&caminho) else {
        return (Vec::new(), Settings::default());
    };
    match serde_json::from_str::<BancoDeDados>(&texto) {
        Ok(bd) => (bd.alarms, bd.settings),
        Err(e) => {
            // Não apaga o arquivo: o dono pode querer recuperar. Deixa um
            // lado fora do caminho para o caso do JSON estar consertável.
            let resgate = caminho.with_extension(format!("corrompido-{}.json", e.to_string().len()));
            let _ = fs::copy(&caminho, resgate);
            (Vec::new(), Settings::default())
        }
    }
}

/// Grava os dados no arquivo padrão do app.
pub fn salvar(alarms: &[Alarm], settings: &Settings) -> Result<(), String> {
    let pasta = pasta_dados();
    fs::create_dir_all(&pasta).map_err(|e| format!("Não consegui criar {}: {e}", pasta.display()))?;
    let _ = fs::create_dir_all(pasta_sons());
    salvar_em(&caminho_dados(), alarms, settings)
}

/// Grava os dados em um caminho específico.
///
/// A gravação é atômica: escreve num .tmp e renomeia por cima. Com a trava de
/// escrita, duas threads não se atropelam no mesmo .tmp.
pub fn salvar_em(caminho: &Path, alarms: &[Alarm], settings: &Settings) -> Result<(), String> {
    let _t = trava();

    if let Some(pasta) = caminho.parent() {
        fs::create_dir_all(pasta).map_err(|e| format!("Não consegui criar {}: {e}", pasta.display()))?;
    }

    let bd = BancoDeDados {
        alarms: alarms.to_vec(),
        settings: settings.clone(),
    };
    let texto = serde_json::to_string_pretty(&bd).map_err(|e| e.to_string())?;
    let tmp = caminho.with_extension("json.tmp");
    fs::write(&tmp, texto).map_err(|e| format!("Falha ao gravar: {e}"))?;
    fs::rename(&tmp, caminho).map_err(|e| format!("Falha ao finalizar a gravação: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::model::BancoDeDados;

    /// O arquivo gravado pelo app tem que voltar sem perda: o dono pode
    /// copiar, editar e devolver. Este teste trava o formato.
    #[test]
    fn o_arquivo_gravado_volta_com_tudo() {
        let (alarms, settings) = (vec![crate::model::Alarm::new(7, 30)], Settings::default());
        let bd = BancoDeDados {
            alarms: alarms.clone(),
            settings: settings.clone(),
        };
        let texto = serde_json::to_string_pretty(&bd).unwrap();
        let lido: BancoDeDados = serde_json::from_str(&texto)
            .unwrap_or_else(|e| panic!("o arquivo gravado nao voltou: {e}\n{texto}"));
        assert_eq!(lido.alarms.len(), 1);
        assert_eq!(lido.alarms[0].hour, 7);
        assert_eq!(lido.alarms[0].minute, 30);
        assert_eq!(lido.settings.sair_ao_fechar, settings.sair_ao_fechar);
    }

    /// O formato exato do arquivo, para saber o que o app espera.
    #[test]
    fn formato_do_arquivo_de_dados() {
        let bd = BancoDeDados {
            alarms: vec![crate::model::Alarm::new(7, 30)],
            settings: Settings::default(),
        };
        println!("{}", serde_json::to_string_pretty(&bd).unwrap());
    }

    /// Gravar e reler o arquivo devolve o mesmo conteúdo.
    #[test]
    fn gravar_e_reler_mantem_o_conteudo() {
        let caminho = std::env::temp_dir().join("tranal-teste-storage.json");
        let mut alarme = crate::model::Alarm::new(6, 45);
        alarme.label = "Café".to_string();
        alarme.volume = 70;
        salvar_em(&caminho, &[alarme.clone()], &Settings::default()).unwrap();

        let texto = fs::read_to_string(&caminho).unwrap();
        let lido: BancoDeDados = serde_json::from_str(&texto).unwrap();
        assert_eq!(lido.alarms.len(), 1);
        assert_eq!(lido.alarms[0].label, "Café");
        assert_eq!(lido.alarms[0].volume, 70);
        let _ = fs::remove_file(&caminho);
    }
}
