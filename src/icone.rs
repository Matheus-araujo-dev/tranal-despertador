//! O desenho do ícone do Tranal (o despertador laranja).
//!
//! O desenho é feito em código, sem arquivo de imagem: o app não carrega
//! imagem nenhuma, e um PNG embutido seria um arquivo a mais para manter.
//!
//! O mesmo desenho tem dois destinos:
//!
//! - a **bandeja** do Windows e a **barra de título** da janela, que aceitam
//!   pixels crus (RGBA);
//! - o arquivo `.ico` que o instalador usa nos **atalhos** do Menu Iniciar e
//!   da Área de trabalho — ali é preciso um arquivo de verdade, porque o
//!   Windows não lê o ícone de dentro do executável (o app não tem recurso
//!   de ícone embutido).

use std::path::Path;

/// Desenha o despertador e devolve os pixels em RGBA (4 bytes por pixel).
///
/// `lado` é o tamanho do quadrado (32 ou 64 pixels).
pub fn desenhar(lado: u32) -> Vec<u8> {
    let n = lado as usize;

    let laranja: [u8; 4] = [255, 138, 40, 255];
    let laranja_escuro: [u8; 4] = [194, 65, 12, 255];
    let claro: [u8; 4] = [245, 245, 245, 255];

    // As medidas do desenho são escritas para 64 pixels; em um quadrado
    // menor elas encolhem na mesma proporção.
    let escala = lado as f32 / 64.0;

    let mut rgba = vec![0u8; n * n * 4];
    let c = lado as f32 / 2.0;

    for y in 0..n {
        for x in 0..n {
            let dx = x as f32 - c + 0.5;
            let dy = y as f32 - c + 0.5;
            let d = (dx * dx + dy * dy).sqrt();
            let angulo = dy.atan2(dx).to_degrees();

            let mut cor: Option<[u8; 4]> = None;

            // Corpo do relógio.
            if d <= 22.0 * escala {
                cor = Some(laranja);
            }
            // Moldura um pouco mais escura na borda.
            if d > 18.0 * escala && d <= 22.0 * escala {
                cor = Some(laranja_escuro);
            }
            // Ponteiros: um para cima, outro para a direita.
            let upwards = dx.abs() < 1.6 * escala && dy < 0.0 && d > 3.0 * escala;
            let right = dy.abs() < 1.6 * escala && dx > 0.0 && d > 3.0 * escala;
            if upwards || right {
                cor = Some(claro);
            }
            // Os dois sinhos, em cima, um de cada lado.
            let badalo = (22.0..=27.5).contains(&(d / escala))
                && (35.0..=80.0).contains(&angulo.abs());
            if badalo {
                cor = Some(laranja_escuro);
            }
            // Os dois pés.
            let pes = (20.0..=26.0).contains(&(dy / escala))
                && (11.0..=17.0).contains(&(dx.abs() / escala));
            if pes {
                cor = Some(laranja_escuro);
            }

            if let Some([r, g, b, a]) = cor {
                let i = (y * n + x) * 4;
                rgba[i] = r;
                rgba[i + 1] = g;
                rgba[i + 2] = b;
                rgba[i + 3] = a;
            }
        }
    }
    rgba
}

/// Escreve o desenho em um arquivo `.ico` (o formato dos atalhos do Windows).
///
/// O arquivo é um ICO de 32×32 com um bitmap de 32 bits (com a máscara de
/// transparência). Não depende de biblioteca externa: o formato é pequeno o
/// bastante para escrever na mão, e assim o app continua sem mais uma
/// dependência.
pub fn exportar_ico(caminho: &Path) -> Result<(), String> {
    let lado = 32u32;
    let rgba = desenhar(lado);
    let arquivo = montar_ico(&rgba, lado);

    if let Some(pasta) = caminho.parent() {
        std::fs::create_dir_all(pasta).map_err(|e| format!("criei a pasta do icone: {e}"))?;
    }
    std::fs::write(caminho, arquivo).map_err(|e| format!("escrevi o icone: {e}"))
}

/// Monta os bytes de um arquivo `.ico` com um quadro de 32×32.
fn montar_ico(rgba: &[u8], lado: u32) -> Vec<u8> {
    let n = lado as usize;

    // O bitmap do ICO é BGRA e de baixo para cima (a ultima linha primeiro).
    let mut xor = Vec::with_capacity(n * n * 4);
    for y in (0..n).rev() {
        for x in 0..n {
            let i = (y * n + x) * 4;
            xor.push(rgba[i + 2]); // B
            xor.push(rgba[i + 1]); // G
            xor.push(rgba[i]); // R
            xor.push(rgba[i + 3]); // A
        }
    }

    // A máscara de 1 bit: 1 = transparente. Cada linha é completada até
    // múltiplo de 4 bytes.
    let bytes_por_linha = n.div_ceil(8);
    let bytes_por_linha = bytes_por_linha.div_ceil(4) * 4;
    let mut mascara = vec![0u8; bytes_por_linha * n];
    for y in 0..n {
        for x in 0..n {
            let i = (y * n + x) * 4;
            if rgba[i + 3] == 0 {
                mascara[y * bytes_por_linha + x / 8] |= 0x80 >> (x % 8);
            }
        }
    }

    // BITMAPINFOHEADER: a altura vale o dobro porque o bitmap tem o quadro
    // de cor e a máscara (a máscara anda de baixo para cima).
    let tamanho_bitmap = 40 + xor.len() + mascara.len();
    let mut arquivo = Vec::with_capacity(22 + tamanho_bitmap);

    // ICONDIR
    arquivo.extend_from_slice(&0u16.to_le_bytes()); // reservado
    arquivo.extend_from_slice(&1u16.to_le_bytes()); // tipo: icone
    arquivo.extend_from_slice(&1u16.to_le_bytes()); // um quadro

    // ICONDIRENTRY
    arquivo.push(lado as u8); // largura
    arquivo.push(lado as u8); // altura
    arquivo.push(0); // cores da paleta
    arquivo.push(0); // reservado
    arquivo.extend_from_slice(&1u16.to_le_bytes()); // planos
    arquivo.extend_from_slice(&32u16.to_le_bytes()); // bits por pixel
    arquivo.extend_from_slice(&(tamanho_bitmap as u32).to_le_bytes());
    arquivo.extend_from_slice(&22u32.to_le_bytes()); // onde o quadro comeca

    // BITMAPINFOHEADER
    arquivo.extend_from_slice(&40u32.to_le_bytes()); // tamanho do cabecalho
    arquivo.extend_from_slice(&(n as i32).to_le_bytes()); // largura
    arquivo.extend_from_slice(&((n * 2) as i32).to_le_bytes()); // altura (cor + mascara)
    arquivo.extend_from_slice(&1u16.to_le_bytes()); // planos
    arquivo.extend_from_slice(&32u16.to_le_bytes()); // bits por pixel
    arquivo.extend_from_slice(&0u32.to_le_bytes()); // compressao: BI_RGB
    arquivo.extend_from_slice(&(tamanho_bitmap as u32).to_le_bytes());
    arquivo.extend_from_slice(&0u32.to_le_bytes()); // pixels por metro (x)
    arquivo.extend_from_slice(&0u32.to_le_bytes()); // pixels por metro (y)
    arquivo.extend_from_slice(&0u32.to_le_bytes()); // cores usadas
    arquivo.extend_from_slice(&0u32.to_le_bytes()); // cores importantes

    arquivo.extend_from_slice(&xor);
    arquivo.extend_from_slice(&mascara);
    arquivo
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_desenho_tem_o_tamanho_pedido() {
        for lado in [32u32, 64] {
            let rgba = desenhar(lado);
            assert_eq!(rgba.len(), (lado * lado * 4) as usize);
        }
    }

    #[test]
    fn o_desenho_tem_o_laranja_do_app() {
        let rgba = desenhar(64);
        let laranja = rgba
            .chunks(4)
            .filter(|p| p[0] == 255 && p[1] == 138 && p[2] == 40 && p[3] == 255)
            .count();
        assert!(laranja > 300, "o corpo do despertador sumiu: {laranja} pixels");
    }

    #[test]
    fn o_ico_tem_o_cabecalho_certo() {
        let rgba = desenhar(32);
        let ico = montar_ico(&rgba, 32);
        // ICONDIR (6) + ICONDIRENTRY (16) + BITMAPINFOHEADER (40) + 32*32*4
        // + mascara (32 linhas de 4 bytes).
        let esperado = 6 + 16 + 40 + 32 * 32 * 4 + 32 * 4;
        assert_eq!(ico.len(), esperado);
        assert_eq!(&ico[0..2], &0u16.to_le_bytes());
        assert_eq!(&ico[2..4], &1u16.to_le_bytes());
        assert_eq!(&ico[4..6], &1u16.to_le_bytes());
        // Largura e altura do quadro.
        assert_eq!(ico[6], 32);
        assert_eq!(ico[7], 32);
        // 32 bits por pixel.
        assert_eq!(&ico[12..14], &32u16.to_le_bytes());
    }

    #[test]
    fn a_mascara_marca_o_que_e_transparente() {
        let rgba = desenhar(32);
        let ico = montar_ico(&rgba, 32);
        // O canto do quadro e transparente (o desenho e um circulo).
        let mascara = &ico[6 + 16 + 40 + 32 * 32 * 4..];
        assert_eq!(mascara.len(), 32 * 4);
        // O bit mais alto da primeira linha e o pixel (0,0), que esta vazio.
        assert_eq!(mascara[0] & 0x80, 0x80);
    }
}
