//! Modelo de dados do Tranal Despertador.
//!
//! Tudo que é gravado em disco vive aqui. A forma é simples e legível em JSON,
//! porque o dono pode precisar abrir e conferir o arquivo um dia.

use serde::{Deserialize, Serialize};

/// Um despertador.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Alarm {
    pub id: String,
    pub label: String,
    pub hour: u8,
    pub minute: u8,
    pub enabled: bool,
    pub repeat: RepeatRule,
    pub sound: SoundChoice,
    /// Volume do app, de 0 a 100.
    pub volume: u8,
    /// Segundos para o volume chegar no alvo (fade in).
    pub fade_in_secs: u32,
    /// Garante 100% no dispositivo de saída do Windows enquanto toca.
    pub force_device_volume: bool,
    /// Nomes das saídas de áudio escolhidas. Vazio = saída padrão do sistema.
    #[serde(default)]
    pub output_devices: Vec<String>,
    pub dismiss: DismissPolicy,
    /// Data (YYYY-MM-DD) do último disparo, para o alerta "uma vez" não repetir.
    #[serde(default)]
    pub last_rung: Option<String>,
    /// Quantas sonecas já foram usadas nesta sequência. Carried pela soneca
    /// para que o limite de sonecas do alarme original seja respeitado.
    #[serde(default)]
    pub sonecas_usadas: u32,
}

impl Alarm {
    pub fn new(hour: u8, minute: u8) -> Self {
        Self {
            id: crate::model::novo_id(),
            label: "Despertador".to_string(),
            hour,
            minute,
            enabled: true,
            repeat: RepeatRule::Once,
            sound: SoundChoice::Preset(Preset::Classico),
            volume: 100,
            fade_in_secs: 0,
            force_device_volume: true,
            output_devices: Vec::new(),
            dismiss: DismissPolicy::default(),
            last_rung: None,
            sonecas_usadas: 0,
        }
    }

    /// Cria um despertador novo a partir do modelo das Configurações.
    ///
    /// O modelo (o "despertador padrão") é copiado inteiro, menos o que só
    /// faz sentido em um alarme de verdade: o `id` (que precisa ser novo) e o
    /// histórico de disparo e de sonecas.
    pub fn a_partir_do_padrao(padrao: &Alarm) -> Alarm {
        let mut novo = padrao.clone();
        novo.id = crate::model::novo_id();
        novo.last_rung = None;
        novo.sonecas_usadas = 0;
        novo
    }

    /// "14:30" no formato do relógio.
    pub fn time_label(&self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }
}

impl Default for Alarm {
    fn default() -> Self {
        Alarm::new(7, 0)
    }
}

/// Como o despertador repete.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepeatRule {
    /// Dispara uma vez e não repete.
    Once,
    /// Todos os dias.
    Daily,
    /// Segunda a sexta.
    Weekdays,
    /// Nos dias marcados (1 = segunda ... 7 = domingo).
    Weekly { days: Vec<u8> },
}

impl RepeatRule {
    /// Texto curto para a interface.
    pub fn describe(&self) -> String {
        match self {
            RepeatRule::Once => "Uma vez".to_string(),
            RepeatRule::Daily => "Todo dia".to_string(),
            RepeatRule::Weekdays => "Dias úteis".to_string(),
            RepeatRule::Weekly { days } => {
                if days.is_empty() {
                    return "Nunca".to_string();
                }
                let nomes: Vec<&str> = days
                    .iter()
                    .filter_map(|d| crate::schedule::nome_dia(*d))
                    .collect();
                nomes.join(", ")
            }
        }
    }
}

/// De onde vem o som do alarme.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SoundChoice {
    /// Som sintetizado pelo próprio app (sem material de terceiros).
    Preset(Preset),
    /// Arquivo WAV escolhido pelo dono.
    Arquivo { caminho: String },
}

/// Sons embutidos.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Preset {
    Classico,
    Digital,
    Sirene,
    Cuco,
    Onda,
}

impl Preset {
    pub fn all() -> [Preset; 5] {
        [
            Preset::Classico,
            Preset::Digital,
            Preset::Sirene,
            Preset::Cuco,
            Preset::Onda,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Preset::Classico => "Clássico",
            Preset::Digital => "Digital",
            Preset::Sirene => "Sirene",
            Preset::Cuco => "Cuco",
            Preset::Onda => "Onda",
        }
    }

    /// Descrição curta do timbre, mostrada na lista.
    pub fn description(&self) -> &'static str {
        match self {
            Preset::Classico => "Dois tons suaves, como despertador de mesa",
            Preset::Digital => "Bipes rápidos e secos",
            Preset::Sirene => "Sinal subindo e descendo",
            Preset::Cuco => "Canto de ave em duas notas",
            Preset::Onda => "Tom grave com onda lenta",
        }
    }
}

/// Como o alarme pode ser desligado.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DismissPolicy {
    /// Libera a soneca.
    pub soneca: bool,
    /// Minutos de soneca.
    pub soneca_minutos: u32,
    /// Quantas sonecas são permitidas.
    pub max_sonecas: u32,
    /// Desafio para desligar de verdade.
    pub desafio: Desafio,
}

impl Default for DismissPolicy {
    fn default() -> Self {
        Self {
            soneca: true,
            soneca_minutos: 9,
            max_sonecas: 3,
            desafio: Desafio::Nenhum,
        }
    }
}

/// Desafio para desligar o alarme.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Desafio {
    Nenhum,
    /// Digitar a palavra mostrada.
    Digitar,
    /// Resolver uma conta simples.
    Conta,
}

impl Desafio {
    pub fn label(&self) -> &'static str {
        match self {
            Desafio::Nenhum => "Sem desafio",
            Desafio::Digitar => "Digitar a palavra",
            Desafio::Conta => "Conta de somar",
        }
    }
}

/// Configurações do app (não de cada alarme).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub iniciar_com_o_windows: bool,
    pub iniciar_minimizado: bool,
    /// O **despertador padrão**: o modelo que vem preenchido quando o dono
    /// clica em "Novo despertador".
    ///
    /// É um alarme como qualquer outro, e não uma configuração à parte: uma
    /// única interpretação do que é um despertador. O `id` e o histórico são
    /// trocados quando o modelo vira um alarme de verdade
    /// (`Alarm::a_partir_do_padrao`).
    #[serde(default)]
    pub alarme_padrao: Alarm,
    /// Fecha o app quando a janela é fechada (falso = continua rodando).
    pub sair_ao_fechar: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let mut alarme_padrao = Alarm::new(7, 0);
        alarme_padrao.label = String::new();
        Self {
            iniciar_com_o_windows: false,
            iniciar_minimizado: false,
            alarme_padrao,
            // Padrao: fechar a janela NAO encerra o app. Um despertador que
            // morre quando a janela fecha nao serve para nada.
            sair_ao_fechar: false,
        }
    }
}

/// Estado completo salvo em disco.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BancoDeDados {
    #[serde(default)]
    pub alarms: Vec<Alarm>,
    #[serde(default)]
    pub settings: Settings,
}

/// Identificador único curto.
pub fn novo_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("al{:x}", nanos & 0xffff_ffff)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_despertador_padrao_vem_preenchido() {
        let settings = Settings::default();
        assert_eq!(settings.alarme_padrao.hour, 7);
        assert_eq!(settings.alarme_padrao.minute, 0);
        assert_eq!(settings.alarme_padrao.sound, SoundChoice::Preset(Preset::Classico));
        assert!(settings.alarme_padrao.volume <= 100);
    }

    #[test]
    fn o_despertador_padrao_da_um_alarme_novo_com_id_novo() {
        let settings = Settings::default();
        let mut padrao = settings.alarme_padrao.clone();
        padrao.last_rung = Some("2026-11-17".to_string());
        padrao.sonecas_usadas = 2;

        let novo = Alarm::a_partir_do_padrao(&padrao);
        assert_ne!(novo.id, padrao.id, "o novo alarme precisa de id proprio");
        assert_eq!(novo.last_rung, None, "o historico nao vem junto");
        assert_eq!(novo.sonecas_usadas, 0, "as sonecas nao vem junto");
        assert_eq!(novo.hour, padrao.hour);
        assert_eq!(novo.volume, padrao.volume);
        assert_eq!(novo.repeat, padrao.repeat);
    }

    #[test]
    fn as_configuracoes_antigas_ainda_carregam() {
        // Um arquivo gravado antes do despertador padrao existir tem que
        // continuar abrindo: o campo novo entra com o valor padrao.
        let antigo = r#"{
            "alarms": [],
            "settings": {
                "iniciar_com_o_windows": true,
                "iniciar_minimizado": false,
                "forcar_volume_dispositivo": true,
                "volume_padrao": 100,
                "som_padrao": "classico",
                "sair_ao_fechar": false
            }
        }"#;
        let bd: BancoDeDados = serde_json::from_str(antigo).expect("o arquivo antigo tem que abrir");
        assert!(bd.settings.iniciar_com_o_windows);
        assert_eq!(bd.settings.alarme_padrao.hour, 7);
    }
}
