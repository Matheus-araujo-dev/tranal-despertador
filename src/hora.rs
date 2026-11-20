//! Digitação do horário.
//!
//! O dono pediu para **digitar a hora por cima do relógio**, em vez de
//! ficar clicando nos botões de + e -. Este módulo é a interpretação do que
//! foi digitado: uma função só, testada, que decide o que "0730", "07:30" ou
//! "730" significam.

/// O texto canônico do horário: "07:30".
pub fn formatar(hora: u8, minuto: u8) -> String {
    format!("{hora:02}:{minuto:02}")
}

/// Só os dígitos do texto digitado (o resto, como ":", é ignorado).
pub fn digitos(texto: &str) -> String {
    texto.chars().filter(|c| c.is_ascii_digit()).collect()
}

/// Limpa o texto digitado: mantém dígitos e dois-pontos, corta o resto.
pub fn sanear(texto: &str) -> String {
    texto.chars()
        .filter(|c| c.is_ascii_digit() || *c == ':')
        .take(5)
        .collect()
}

/// Interpreta o que o dono digitou.
///
/// - 1 ou 2 dígitos: a hora (`7` -> 07:00). O minuto continua como estava.
/// - 3 dígitos: hora com 2 dígitos e a dezena do minuto (`730` -> 07:30).
/// - 4 dígitos: hora e minuto (`0730` -> 07:30).
///
/// Quando a leitura natural não cabe em um horário válido, tenta a outra
/// leitura (`730` -> 07:30 em vez de hora 73).
pub fn interpretar(texto: &str, minuto_anterior: u8) -> Result<(u8, u8), String> {
    let d = digitos(texto);
    match d.len() {
        0 => Err("Digite a hora, por exemplo 0730.".to_string()),
        1 | 2 => {
            let hora: u8 = d.parse().map_err(|_| "Hora inválida.".to_string())?;
            if hora > 23 {
                return Err("A hora vai de 00 a 23.".to_string());
            }
            Ok((hora, minuto_anterior))
        }
        3 => {
            let hora2: u8 = d[0..2].parse().unwrap_or(0);
            let minuto2: u8 = d[2..3].parse().unwrap_or(0) * 10;
            if hora2 <= 23 && minuto2 <= 59 {
                return Ok((hora2, minuto2));
            }
            // Outra leitura: "730" -> 07:30 (hora de um dígito).
            let hora: u8 = d[0..1].parse().unwrap_or(0);
            let minuto: u8 = d[1..3].parse().unwrap_or(0);
            if minuto > 59 {
                return Err("O minuto vai de 00 a 59.".to_string());
            }
            Ok((hora, minuto))
        }
        4 => {
            let hora: u8 = d[0..2].parse().unwrap_or(0);
            let minuto: u8 = d[2..4].parse().unwrap_or(0);
            if hora > 23 {
                return Err("A hora vai de 00 a 23.".to_string());
            }
            if minuto > 59 {
                return Err("O minuto vai de 00 a 59.".to_string());
            }
            Ok((hora, minuto))
        }
        _ => Err("Digite no máximo 4 dígitos (ex.: 0730).".to_string()),
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn quatro_digitos_sao_hora_e_minuto() {
        assert_eq!(interpretar("0730", 0).unwrap(), (7, 30));
        assert_eq!(interpretar("2359", 0).unwrap(), (23, 59));
        assert_eq!(interpretar("0000", 0).unwrap(), (0, 0));
    }

    #[test]
    fn dois_pontos_sao_ignorados() {
        assert_eq!(interpretar("07:30", 0).unwrap(), (7, 30));
        assert_eq!(interpretar("7:30", 0).unwrap(), (7, 30));
    }

    #[test]
    fn um_ou_dois_digitos_mudam_so_a_hora() {
        assert_eq!(interpretar("7", 45).unwrap(), (7, 45));
        assert_eq!(interpretar("06", 45).unwrap(), (6, 45));
    }

    #[test]
    fn tres_digitos_aceitam_as_duas_leituras() {
        // "730" nao cabe como hora 73: vira 07:30.
        assert_eq!(interpretar("730", 0).unwrap(), (7, 30));
        // "129" nao cabe como minuto 90: vira 01:29.
        assert_eq!(interpretar("129", 0).unwrap(), (1, 29));
        // "930" cabe dos dois jeitos: 09:30.
        assert_eq!(interpretar("930", 0).unwrap(), (9, 30));
    }

    #[test]
    fn horario_impossivel_vira_erro() {
        assert!(interpretar("2500", 0).is_err());
        assert!(interpretar("0060", 0).is_err());
        assert!(interpretar("", 0).is_err());
        assert!(interpretar("12345", 0).is_err());
    }

    #[test]
    fn sanear_tira_o_que_nao_e_digito() {
        assert_eq!(sanear("07:30"), "07:30");
        assert_eq!(sanear("ab 07 cd"), "07");
        assert_eq!(sanear("073012"), "07301");
    }

    #[test]
    fn formatar_mostra_dois_digitos() {
        assert_eq!(formatar(7, 5), "07:05");
        assert_eq!(formatar(23, 59), "23:59");
    }
}
