//! Cálculo de quando cada alarme dispara.
//!
//! Estas funções são puras (só dependem da data de referência) e é aqui que
//! mora a regra de repetição. Sem estado, sem relógio interno: o chamador
//! passa "agora" e recebe o próximo disparo.

use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Weekday};

use crate::model::{Alarm, RepeatRule};

/// Nome curto do dia da semana (1 = segunda ... 7 = domingo).
pub fn nome_dia(dia: u8) -> Option<&'static str> {
    match dia {
        1 => Some("Seg"),
        2 => Some("Ter"),
        3 => Some("Qua"),
        4 => Some("Qui"),
        5 => Some("Sex"),
        6 => Some("Sáb"),
        7 => Some("Dom"),
        _ => None,
    }
}

/// A repetição aceita este dia?
pub fn repeticao_aceita(regra: &RepeatRule, data: NaiveDate) -> bool {
    match regra {
        RepeatRule::Once | RepeatRule::Daily => true,
        RepeatRule::Weekdays => !matches!(data.weekday(), Weekday::Sat | Weekday::Sun),
        RepeatRule::Weekly { days } => days.contains(&(data.weekday().number_from_monday() as u8)),
    }
}

/// Próximo disparo do alarme depois de `depois` (estrito).
///
/// Devolve `None` quando o alarme não tem mais disparo (por exemplo, "uma
/// vez" que já tocou).
pub fn proximo_disparo(alarme: &Alarm, depois: DateTime<Local>) -> Option<DateTime<Local>> {
    // Alarme "uma vez" que já tocou não tem mais disparo.
    if matches!(alarme.repeat, RepeatRule::Once) && alarme.last_rung.is_some() {
        return None;
    }
    let hoje = depois.date_naive();
    // 9 dias cobrem a semana inteira + folga para alarme "uma vez" já disparado.
    for offset in 0..=8i64 {
        let data = hoje + Duration::days(offset);
        if !repeticao_aceita(&alarme.repeat, data) {
            continue;
        }
        if let Some(ultimo) = &alarme.last_rung
            && ultimo == &data.format("%Y-%m-%d").to_string()
        {
            continue;
        }
        let hora = data.and_hms_opt(alarme.hour as u32, alarme.minute as u32, 0)?;
        if hora <= depois.naive_local() {
            continue;
        }
        return Local.from_local_datetime(&hora).earliest();
    }
    None
}

/// Próximo disparo entre todos os alarmes ativos.
///
/// Devolve o índice do alarme e o horário do disparo.
pub fn proximo_disparo_global(alarmes: &[Alarm], agora: DateTime<Local>) -> Option<(usize, DateTime<Local>)> {
    let mut melhor: Option<(usize, DateTime<Local>)> = None;
    for (i, alarme) in alarmes.iter().enumerate() {
        if !alarme.enabled {
            continue;
        }
        if let Some(quando) = proximo_disparo(alarme, agora)
            && melhor.map(|(_, t)| quando < t).unwrap_or(true)
        {
            melhor = Some((i, quando));
        }
    }
    melhor
}

/// O alarme já passou do horário agora e ainda não tocou hoje?
///
/// Serve para pegar o alarme depois de o PC voltar do suspensão ou de o app
/// ficar fechado por alguns minutos.
pub fn atrasado_para_disparar(alarme: &Alarm, agora: DateTime<Local>, janela_min: i64) -> bool {
    if !alarme.enabled {
        return false;
    }
    // D13: alarme "uma vez" que ja tocou nao tem mais disparo -- a mesma
    // guarda que `proximo_disparo` usa.
    if matches!(alarme.repeat, RepeatRule::Once) && alarme.last_rung.is_some() {
        return false;
    }
    let hoje = agora.date_naive();
    if !repeticao_aceita(&alarme.repeat, hoje) {
        return false;
    }
    if let Some(ultimo) = &alarme.last_rung
        && ultimo == &hoje.format("%Y-%m-%d").to_string()
    {
        return false;
    }
    let hora = hoje.and_hms_opt(alarme.hour as u32, alarme.minute as u32, 0);
    let Some(hora) = hora else { return false };
    let Some(hora) = Local.from_local_datetime(&hora).earliest() else {
        return false;
    };
    let atraso = agora - hora;
    atraso >= Duration::zero() && atraso <= Duration::minutes(janela_min)
}

/// Texto do próximo disparo para a interface ("em 2 h 15 min", "amanhã 07:00").
pub fn texto_ate(quando: DateTime<Local>, agora: DateTime<Local>) -> String {
    let diff = quando - agora;
    let minutos = diff.num_minutes();
    if minutos < 0 {
        return "agora".to_string();
    }
    if minutos < 1 {
        return "em menos de 1 min".to_string();
    }
    if minutos < 60 {
        return format!("em {} min", minutos);
    }
    let horas = diff.num_hours();
    let resto = minutos % 60;
    if horas < 24 && quando.date_naive() == agora.date_naive() {
        if resto == 0 {
            return format!("em {} h", horas);
        }
        return format!("em {} h {} min", horas, resto);
    }
    let amanha = agora.date_naive() + Duration::days(1);
    if quando.date_naive() == amanha {
        return format!("amanhã {}", quando.format("%H:%M"));
    }
    format!("{}", quando.format("%a %d/%m %H:%M"))
}

#[cfg(test)]
mod testes {
    use super::*;
    use chrono::TimeZone;

    fn em(ano: i32, mes: u32, dia: u32, hora: u32, minuto: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(ano, mes, dia, hora, minuto, 0).unwrap()
    }

    fn alarme_hora(h: u8, m: u8) -> Alarm {
        Alarm::new(h, m)
    }

    #[test]
    fn uma_vez_dispara_hoje_se_falta_horario() {
        let a = alarme_hora(14, 30);
        let agora = em(2026, 11, 17, 13, 0);
        let proximo = proximo_disparo(&a, agora).unwrap();
        assert_eq!(proximo, em(2026, 11, 17, 14, 30));
    }

    #[test]
    fn uma_vez_hoje_ja_passou_va_para_amanha() {
        let a = alarme_hora(14, 30);
        let agora = em(2026, 11, 17, 15, 0);
        let proximo = proximo_disparo(&a, agora).unwrap();
        assert_eq!(proximo, em(2026, 11, 18, 14, 30));
    }

    #[test]
    fn uma_vez_que_ja_toquei_nao_repete() {
        let mut a = alarme_hora(14, 30);
        a.last_rung = Some("2026-11-17".to_string());
        let agora = em(2026, 11, 17, 15, 0);
        // Passou o dia de hoje: não existe mais disparo.
        assert!(proximo_disparo(&a, agora).is_none());
    }

    #[test]
    fn todo_dia_repete_amanha() {
        let mut a = alarme_hora(14, 30);
        a.repeat = RepeatRule::Daily;
        let agora = em(2026, 11, 17, 15, 0);
        let proximo = proximo_disparo(&a, agora).unwrap();
        assert_eq!(proximo, em(2026, 11, 18, 14, 30));
    }

    #[test]
    fn dias_uteis_pula_fim_de_semana() {
        let mut a = alarme_hora(7, 0);
        a.repeat = RepeatRule::Weekdays;
        // 2026-11-21 é sábado.
        let agora = em(2026, 11, 20, 23, 0);
        let proximo = proximo_disparo(&a, agora).unwrap();
        assert_eq!(proximo, em(2026, 11, 23, 7, 0));
        assert_eq!(proximo.weekday(), Weekday::Mon);
    }

    #[test]
    fn semanal_usa_os_dias_marcados() {
        let mut a = alarme_hora(7, 0);
        // segunda (1) e quarta (3)
        a.repeat = RepeatRule::Weekly { days: vec![1, 3] };
        // 2026-11-17 é terça.
        let agora = em(2026, 11, 17, 10, 0);
        let proximo = proximo_disparo(&a, agora).unwrap();
        assert_eq!(proximo, em(2026, 11, 18, 7, 0));
        assert_eq!(proximo.weekday(), Weekday::Wed);
    }

    #[test]
    fn semanal_sem_dias_nao_dispara() {
        let mut a = alarme_hora(7, 0);
        a.repeat = RepeatRule::Weekly { days: vec![] };
        let agora = em(2026, 11, 17, 10, 0);
        assert!(proximo_disparo(&a, agora).is_none());
    }

    #[test]
    fn global_escolhe_o_mais_proximo() {
        let mut a1 = alarme_hora(9, 0);
        a1.repeat = RepeatRule::Daily;
        let mut a2 = alarme_hora(7, 0);
        a2.repeat = RepeatRule::Daily;
        let agora = em(2026, 11, 17, 6, 0);
        let (idx, quando) = proximo_disparo_global(&[a1, a2], agora).unwrap();
        assert_eq!(idx, 1);
        assert_eq!(quando, em(2026, 11, 17, 7, 0));
    }

    #[test]
    fn global_ignora_alarme_desligado() {
        let mut a1 = alarme_hora(9, 0);
        a1.repeat = RepeatRule::Daily;
        a1.enabled = false;
        let a2 = alarme_hora(7, 0);
        let agora = em(2026, 11, 17, 6, 0);
        let (idx, _) = proximo_disparo_global(&[a1, a2], agora).unwrap();
        assert_eq!(idx, 1);
    }

    #[test]
    fn atrasado_detecta_janela_de_tolerancia() {
        let a = alarme_hora(14, 30);
        // 5 minutos depois do horário, dentro da janela de 15.
        let agora = em(2026, 11, 17, 14, 35);
        assert!(atrasado_para_disparar(&a, agora, 15));
        // 40 minutos depois, fora da janela.
        let agora = em(2026, 11, 17, 15, 10);
        assert!(!atrasado_para_disparar(&a, agora, 15));
    }

    #[test]
    fn atrasado_ignora_quem_ja_toqueu_hoje() {
        let mut a = alarme_hora(14, 30);
        a.last_rung = Some("2026-11-17".to_string());
        let agora = em(2026, 11, 17, 14, 35);
        assert!(!atrasado_para_disparar(&a, agora, 15));
    }

    #[test]
    fn atrasado_ignora_uma_vez_que_ja_toqueu_ontem() {
        // D13: a mesma guarda do `proximo_disparo`. Um "uma vez" que tocou
        // ontem nao pode tocar de novo hoje quando o app sobe atrasado.
        let mut a = alarme_hora(14, 30);
        a.last_rung = Some("2026-11-16".to_string());
        let agora = em(2026, 11, 17, 14, 35);
        assert!(!atrasado_para_disparar(&a, agora, 15));
    }

    #[test]
    fn todo_dia_pode_tocar_depois_de_ter_tocado() {
        // O contrario do teste anterior: "todo dia" continua valendo.
        let mut a = alarme_hora(14, 30);
        a.repeat = RepeatRule::Daily;
        a.last_rung = Some("2026-11-16".to_string());
        let agora = em(2026, 11, 17, 14, 35);
        assert!(atrasado_para_disparar(&a, agora, 15));
    }

    #[test]
    fn texto_ate_fala_em_horas_e_minutos() {
        let agora = em(2026, 11, 17, 12, 0);
        let quando = em(2026, 11, 17, 14, 30);
        assert_eq!(texto_ate(quando, agora), "em 2 h 30 min");
    }

    #[test]
    fn texto_ate_fala_amanha() {
        let agora = em(2026, 11, 17, 23, 0);
        let quando = em(2026, 11, 18, 7, 0);
        assert_eq!(texto_ate(quando, agora), "amanhã 07:00");
    }

    #[test]
    fn descricao_da_repeticao_em_portugues() {
        assert_eq!(RepeatRule::Once.describe(), "Uma vez");
        assert_eq!(RepeatRule::Daily.describe(), "Todo dia");
        assert_eq!(RepeatRule::Weekdays.describe(), "Dias úteis");
        let regra = RepeatRule::Weekly { days: vec![1, 3, 5] };
        assert_eq!(regra.describe(), "Seg, Qua, Sex");
    }
}
