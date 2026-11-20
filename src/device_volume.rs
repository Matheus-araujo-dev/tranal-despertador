//! Controle do volume do **aparelho** (o que o Windows mostra no mixer).
//!
//! O app consegue controlar o volume do próprio som, mas o Windows ainda
//! atenua o áudio conforme o volume da saída. Para o alarme não perder
//! força, este módulo força 100% nas saídas escolhidas e devolve o valor
//! anterior quando o alarme termina.
//!
//! Usa a interface oficial do Windows (Core Audio / `IAudioEndpointVolume`).

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
use windows::Win32::Media::Audio::{
    eConsole, eRender, IMMDevice, IMMDeviceEnumerator, MMDeviceEnumerator,
    DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoTaskMemFree, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
};
use windows::Win32::System::Com::StructuredStorage::PropVariantToStringAlloc;

/// Volume salvo por dispositivo, para devolver ao valor original no fim.
///
/// A chave é o **id do endpoint** (o identificador único do Core Audio), e
/// não o nome: duas saídas podem ter o mesmo nome (o Windows deixa), e com
/// o nome como chave o volume de uma delas se perderia — ela ficaria em 100%
/// para sempre.
fn salvos() -> &'static Mutex<HashMap<String, f32>> {
    static SALVOS: OnceLock<Mutex<HashMap<String, f32>>> = OnceLock::new();
    SALVOS.get_or_init(|| Mutex::new(HashMap::new()))
}

/// O COM precisa estar pronto na thread. Cada thread do app inicializa uma vez.
fn garantir_com() {
    thread_local! {
        static INICIADO: bool = {
            unsafe {
                // O HRESULT é ignorado de propósito: S_FALSE significa que o COM
                // já estava pronto nesta thread, o que também serve.
                let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            }
            true
        };
    }
    INICIADO.with(|v| *v);
}

/// Uma saída de áudio vista pelo Windows (Core Audio).
struct Endpoint {
    /// Identificador único do endpoint (o `id` do Core Audio). É o que
    /// diferencia duas saídas com o mesmo nome.
    id: String,
    nome: String,
    volume: IAudioEndpointVolume,
}

fn enumerar() -> Result<Vec<Endpoint>, String> {
    garantir_com();
    unsafe {
        let enumerador: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
        let colecao = enumerador
            .EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)
            .map_err(|e| e.to_string())?;
        let total = colecao.GetCount().map_err(|e| e.to_string())?;

        let mut saidas = Vec::new();
        for indice in 0..total {
            let Ok(device) = colecao.Item(indice) else {
                continue;
            };
            let Some(nome) = nome_amigavel(&device) else {
                continue;
            };
            let Ok(volume) = device.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None) else {
                continue;
            };
            let id = id_do_endpoint(&device).unwrap_or_else(|| nome.clone());
            saidas.push(Endpoint { id, nome, volume });
        }
        Ok(saidas)
    }
}

/// Identificador único do endpoint (o `id` do Core Audio).
unsafe fn id_do_endpoint(device: &IMMDevice) -> Option<String> {
    let id = unsafe { device.GetId() }.ok()?;
    let texto = unsafe { id.to_string() }.ok()?;
    Some(texto)
}

/// Nome amigável da saída (o mesmo que aparece no mixer do Windows).
unsafe fn nome_amigavel(device: &IMMDevice) -> Option<String> {
    let store = unsafe { device.OpenPropertyStore(windows::Win32::System::Com::STGM_READ) }.ok()?;
    let valor = unsafe { store.GetValue(&PKEY_Device_FriendlyName) }.ok()?;
    let texto = unsafe { PropVariantToStringAlloc(&valor) }.ok()?;
    let resultado = unsafe { texto.to_string() }.ok();
    unsafe { CoTaskMemFree(Some(texto.as_ptr() as *const core::ffi::c_void)) };
    resultado
}

/// Nome da saída padrão do Windows (a que o mixer mostra em primeiro lugar).
pub fn saida_padrao() -> Option<String> {
    garantir_com();
    unsafe {
        let enumerador: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).ok()?;
        let device = enumerador.GetDefaultAudioEndpoint(eRender, eConsole).ok()?;
        nome_amigavel(&device)
    }
}

/// Uma saída de áudio com o volume que o Windows mostra hoje.
pub struct SaidaComVolume {
    pub nome: String,
    /// De 0 a 1. `None` quando o Windows não respondeu.
    pub volume: Option<f32>,
}

/// Lista as saídas de áudio com o volume de cada uma.
///
/// **Uma única enumeração.** Antes, a interface chamava `volume_atual` para
/// cada saída e cada chamada enumerava o Core Audio inteiro: com 12 saídas
/// eram 12 enumerações COM, e o editor demorava uma eternidade para abrir.
/// Aqui as saídas são enumeradas uma vez só e os volumes vêm na mesma volta.
pub fn saidas_com_volume() -> Vec<SaidaComVolume> {
    let Ok(saidas) = enumerar() else {
        return Vec::new();
    };
    saidas
        .into_iter()
        .map(|saida| {
            let volume = unsafe { saida.volume.GetMasterVolumeLevelScalar() }.ok();
            SaidaComVolume {
                nome: saida.nome,
                volume,
            }
        })
        .collect()
}

/// Força 100% nas saídas pedidas e guarda o volume anterior.
///
/// Lista vazia = **a saída padrão do Windows** (e não todas as saídas: um
/// alarme sem saída marcada não deve mexer no volume de aparelho nenhum).
/// Devolve um resumo do que aconteceu, para o app poder mostrar ao dono.
///
/// O volume anterior é guardado pelo **id do endpoint**, não pelo nome: duas
/// saídas podem ter o mesmo nome, e nesse caso o volume de uma delas se
/// perderia na hora de devolver.
pub fn forcar_100(nomes: &[String]) -> Result<String, String> {
    let saidas = enumerar()?;
    let alvo: Vec<&Endpoint> = if nomes.is_empty() {
        // Sem lista: só a saída padrão.
        match saida_padrao() {
            Some(nome) => saidas.iter().filter(|s| s.nome == nome).collect(),
            None => Vec::new(),
        }
    } else {
        saidas
            .iter()
            .filter(|s| nomes.iter().any(|n| n == &s.nome))
            .collect()
    };
    if alvo.is_empty() {
        return Err("Nenhuma das saídas escolhidas foi encontrada no Windows.".to_string());
    }
    let mut salvos = salvos().lock().unwrap();
    let mut ajustadas = 0usize;
    let mut antes_min = 1.0f32;
    // O Windows permite duas saídas com o mesmo nome. O dono escolheu pelo
    // nome, então todas as homônimas são ajustadas — e o resumo avisa, para
    // o app não mexer onde o dono não esperava sem dizer.
    let repetidos = nomes
        .iter()
        .filter(|nome| alvo.iter().filter(|s| &s.nome == *nome).count() > 1)
        .count();
    for saida in &alvo {
        let Ok(atual) = (unsafe { saida.volume.GetMasterVolumeLevelScalar() }) else {
            continue;
        };
        salvos.entry(saida.id.clone()).or_insert(atual);
        if unsafe { saida.volume.SetMasterVolumeLevelScalar(1.0, std::ptr::null()) }.is_ok() {
            ajustadas += 1;
            antes_min = antes_min.min(atual);
        }
    }
    if ajustadas == 0 {
        return Err("O Windows não deixou mudar o volume das saídas.".to_string());
    }
    let mut resumo = format!(
        "Volume do aparelho: {:.0}% para 100% em {ajustadas} saída(s)",
        antes_min * 100.0
    );
    if repetidos > 0 {
        resumo.push_str(&format!(
            " (atenção: {repetidos} nome(s) de saída se repetem no Windows; todas as homônimas foram ajustadas)"
        ));
    }
    Ok(resumo)
}

/// Devolve o volume que estava antes de `forcar_100`.
///
/// A entrada é removida depois de restaurada: sem isso, o segundo alarme do
/// dia devolveria o volume de antes do primeiro, e não o que o dono ajustou
/// no meio.
pub fn restaurar(nomes: &[String]) {
    let Ok(saidas) = enumerar() else {
        return;
    };
    let alvos: Vec<&Endpoint> = if nomes.is_empty() {
        match saida_padrao() {
            Some(nome) => saidas.iter().filter(|s| s.nome == nome).collect(),
            None => Vec::new(),
        }
    } else {
        saidas
            .iter()
            .filter(|s| nomes.iter().any(|n| n == &s.nome))
            .collect()
    };
    let mut salvos = salvos().lock().unwrap();
    for saida in alvos {
        if let Some(anterior) = salvos.remove(&saida.id) {
            let _ = unsafe { saida.volume.SetMasterVolumeLevelScalar(anterior, std::ptr::null()) };
        }
    }
}

/// Liga ou desliga a inicialização junto com o Windows.
///
/// Grava em `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, que só
/// precisa do usuário logado (sem pedir administrador).
pub fn configurar_inicio_com_windows(ativar: bool) -> Result<(), String> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (chave, _) = hkcu
        .create_subkey(r"Software\Microsoft\Windows\CurrentVersion\Run")
        .map_err(|e| e.to_string())?;

    if ativar {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        // "--minimizado": o app comeca escondido, com o agendador rodando.
        let comando = format!("\"{}\" --minimizado", exe.display());
        chave
            .set_value("TranalDespertador", &comando)
            .map_err(|e| e.to_string())
    } else {
        match chave.delete_value("TranalDespertador") {
            Ok(()) => Ok(()),
            // Já não existia: tudo bem.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}

/// Nome do valor de "iniciar com o Windows" gravado pelo `scripts\instalar.ps1`
/// antigo e pelo instalador (Inno) numa instalação nova. O app usa outro nome
/// (`TranalDespertador`); este aqui é só um pedido: "o dono quer iniciar com
/// o Windows".
const VALOR_LEGADO: &str = "Tranal Despertador";

/// Converte o valor legado em ajuste do app.
///
/// O `instalar.ps1` sempre ligava o início automático, por fora do ajuste do
/// app. O instalador repete isso numa instalação nova. Para a opção da tela e
/// o registro não divergirem, o valor legado é apagado e vira
/// `iniciar_com_o_windows = true`. Devolve `true` quando havia o valor.
pub fn migrar_inicio_legado() -> bool {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let Ok(chave) = hkcu.open_subkey_with_flags(
        r"Software\Microsoft\Windows\CurrentVersion\Run",
        KEY_READ | KEY_SET_VALUE,
    ) else {
        return false;
    };
    if chave.get_raw_value(VALOR_LEGADO).is_err() {
        return false;
    }
    let _ = chave.delete_value(VALOR_LEGADO);
    true
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn enumerar_nao_panca() {
        // Em máquina sem áudio pode falhar; o importante é não entrar em pânico.
        let _ = enumerar();
    }

    #[test]
    fn os_ids_das_saidas_sao_distintos() {
        // Duas saídas com o mesmo nome continuam sendo duas saídas diferentes:
        // o volume de cada uma é guardado pelo id, e não pelo nome.
        let Ok(saidas) = enumerar() else {
            return; // máquina sem áudio: nada a conferir
        };
        let mut ids: Vec<&str> = saidas.iter().map(|s| s.id.as_str()).collect();
        let total = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), total, "cada saída tem que ter o id próprio");
        assert!(
            !ids.iter().any(|i| i.is_empty()),
            "o id da saída não pode vir vazio"
        );
    }
}
