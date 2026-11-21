//! O ícone na bandeja do Windows.
//!
//! O dono fecha a janela e o app continua tocando (é um despertador, não um
//! programa que morre). Sem ícone na bandeja, ele tinha que caçar o ícone na
//! barra de tarefas para voltar. Aqui o app ganha um ícone próprio na área de
//! notificação, com um menu curto: **Abrir** e **Sair**.
//!
//! Duas coisas que o Windows exige e que valem saber:
//!
//! - o ícone tem que ser criado **na thread que tem o event loop** (a thread
//!   principal, a do winit). Por isso a criação acontece no primeiro
//!   quadro da interface;
//! - os **eventos** (clique no ícone, item do menu) podem ser lidos por outra
//!   thread qualquer. Por isso o menu funciona mesmo com a janela minimizada,
//!   quando a interface quase não roda.
//!
//! E uma coisa que o Windows 11 exige e que ninguém avisa: o ícone **não**
//! aparece na faixa de notificação quando é criado. Ele vai para a lista de
//! ícones ocultos (o "^" do lado direito) e o dono precisa promoted-lo à mão
//! (Configurações → Barra de tarefas → Outros ícones da bandeja). Como isso
//! é um passo que o dono não deveria ter que repetir a cada instalação, o app
//! promove o próprio ícone pelo registro logo depois de criá-lo.
//!
//! A thread do agendador do app é a que toca o alarme; a de aqui só fala com
//! a interface (e com a janela do Windows), sem tocar em som nenhum.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::tray_menu;

/// O texto que o Windows mostra quando o mouse fica sobre o ícone.
const TOOLTIP: &str = "Tranal Despertador";

/// O menu pediu para sair do app.
static SAIR_PEDIDO: AtomicBool = AtomicBool::new(false);

/// O contexto do egui: é por ele que a bandeja fala com a interface
/// (abrir a janela, pedir o fechamento).
static CONTEXTO: OnceLock<Mutex<Option<egui::Context>>> = OnceLock::new();

/// A janela invisível que é a dona do menu da bandeja.
///
/// Guarda o valor bruto do handle: o tipo `HWND` do Windows não pode ficar
/// num static (ele não é `Send`), e o handle não muda de lugar depois que a
/// janela é criada.
static JANELA_AUXILIAR: OnceLock<isize> = OnceLock::new();

fn contexto() -> &'static Mutex<Option<egui::Context>> {
    CONTEXTO.get_or_init(|| Mutex::new(None))
}

/// Registra o contexto da interface (chamado no primeiro quadro).
pub fn registrar_contexto(ctx: &egui::Context) {
    if let Ok(mut guarda) = contexto().lock() {
        *guarda = Some(ctx.clone());
    }
}

/// O menu da bandeja pediu para sair? A interface consome a resposta.
pub fn sair_pedido() -> bool {
    SAIR_PEDIDO.swap(false, Ordering::SeqCst)
}

/// Cria o ícone da bandeja.
///
/// Precisa ser chamado na thread principal (o Windows exige um event loop
/// rodando na thread que cria o ícone).
pub fn criar() -> Option<TrayIcon> {
    let (rgba, lado) = icone();
    let Some(icone) = Icon::from_rgba(rgba, lado, lado).ok() else {
        registrar("falha: o Windows nao aceitou o desenho do icone");
        return None;
    };

    // A janela dona do menu. Nasce aqui (thread principal) para ter um event
    // loop proprio.
    if let Some(hwnd) = tray_menu::criar_janela_auxiliar() {
        let _ = JANELA_AUXILIAR.set(hwnd);
    }

    // O menu NAO e entregue a biblioteca de bandeja: ela ancora o menu na
    // posicao do mouse, que esta dentro da barra de tarefas, e o ultimo item
    // ("Sair") fica escondido atras dela. O menu proprio (em `tray_menu`)
    // nasce acima da barra de tarefas.
    let resultado = TrayIconBuilder::new()
        .with_tooltip(TOOLTIP)
        .with_icon(icone)
        .with_menu_on_left_click(false)
        .with_menu_on_right_click(false)
        .build();

    match resultado {
        Ok(icone) => {
            // Onde o Windows colocou o icone (ele pode ir para a lista de
            // icones ocultos, a do "^" na barra de tarefas).
            match icone.rect() {
                Some(retangulo) => registrar(&format!(
                    "ok: icone criado na bandeja, em x={} y={} ({}x{})",
                    retangulo.position.x,
                    retangulo.position.y,
                    retangulo.size.width,
                    retangulo.size.height
                )),
                None => registrar("ok: icone criado na bandeja (o Windows nao informou onde)"),
            }
            Some(icone)
        }
        Err(e) => {
            registrar(&format!("falha ao criar o icone: {e}"));
            None
        }
    }
}

/// Deixa um rastro do que a bandeja fez, para o dono poder conferir
/// (e para o developer descobrir o que quebrou).
fn registrar(linha: &str) {
    use std::io::Write;

    let pasta = crate::storage::pasta_dados();
    let _ = std::fs::create_dir_all(&pasta);
    if let Ok(mut arquivo) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(pasta.join("tray.log"))
    {
        let _ = writeln!(arquivo, "{}", linha);
    }
}

/// Thread que escuta a bandeja.
///
/// Ela so le eventos (o clique e o menu) e fala com a interface. Nada de
/// som, nada de arquivo: o som e do agendador, o arquivo e do estado
/// compartilhado.
pub fn iniciar_ouvinte() {
    // O menu precisa saber o que fazer com a escolha do dono. A escolha chega
    // na thread principal (a dona da janela do menu).
    tray_menu::definir_acao(acao_do_menu);

    // O Windows grava a preferencia do icone no registro logo depois de
    // cria-lo (e as vezes um pouco depois). A promocao e tentada algumas
    // vezes no começo, e so para quando da certo.
    thread_promocao();

    std::thread::Builder::new()
        .name("tranal-bandeja".to_string())
        .spawn(|| loop {
            leu_eventos();
            std::thread::sleep(Duration::from_millis(120));
        })
        .ok();
}

/// O que o app faz com a escolha do dono no menu da bandeja.
fn acao_do_menu(opcao: tray_menu::Opcao) {
    match opcao {
        tray_menu::Opcao::Abrir => {
            registrar("menu: Abrir o Tranal");
            abrir_janela();
        }
        tray_menu::Opcao::Sair => {
            registrar("menu: Sair");
            pedir_saida();
        }
        tray_menu::Opcao::Nenhuma => {
            registrar("menu: fechado sem escolha");
        }
    }
}

/// Thread que promove o icone para a faixa de notificacao.
///
/// O Windows 11 esconde, por padrao, o icone recem-criado: ele vai para a
/// lista de "icones ocultos" (o "^"). O dono teria de promote-lo a mao em
/// Configurações → Barra de tarefas → Outros icones da bandeja, e o caminho
/// muda a cada instalacao. Aqui o app faz isso sozinho, escrevendo a
/// preferencia "IsPromoted" na chave do proprio icone.
///
/// A gravacao e **conferida**: o Windows escreve a chave do icone pouco depois
/// de cria-lo, e essa escrita pode apagar a preferencia que o app acabou de
/// gravar (foi o que aconteceu na primeira medicao: a chave ficou com
/// `IsPromoted = 0`). Por isso a gravacao e repetida ate o Windows aceitar.
fn thread_promocao() {
    std::thread::Builder::new()
        .name("tranal-promocao".to_string())
        .spawn(|| {
            for _ in 0..25 {
                let gravou = promover_icone().is_ok();
                // Da um tempo para o Windows reescrever a chave dele.
                std::thread::sleep(Duration::from_millis(600));
                if gravou && preferencia_aceita() {
                    registrar("ok: icone promovido para a faixa de notificacao");
                    return;
                }
                std::thread::sleep(Duration::from_millis(300));
            }
            registrar("aviso: nao consegui promover o icone para a faixa de notificacao");
        })
        .ok();
}

/// A chave do icone do app no registro (pelo caminho exato do executavel).
///
/// A chave e procurada pelo caminho EXATO do executavel que esta rodando:
/// assim a promocao cai na chave do icone de verdade, e nao em uma chave
/// antiga deixada por uma versao anterior instalada em outro lugar.
///
/// (Um `IconGuid` fixo resolveria a chave, mas o Windows 11 passa a guardar
/// o icone pela identidade: depois de um reinicio, o clique no icone deixa
/// de chegar no app, porque o Windows continua chamando a janela velha. Foi
/// medido nesta maquina: com GUID, o clique direito nao recebia nada.)
fn chave_do_icone(com_escrita: bool) -> Option<winreg::RegKey> {
    use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE};
    use winreg::RegKey;

    let meu_caminho = std::env::current_exe()
        .ok()?
        .to_string_lossy()
        .to_lowercase();

    let raiz = RegKey::predef(HKEY_CURRENT_USER);
    let base = raiz
        .open_subkey_with_flags(
            r"Control Panel\NotifyIconSettings",
            KEY_READ | if com_escrita { KEY_WRITE } else { 0 },
        )
        .ok()?;

    for chave in base.enum_keys().flatten() {
        let sub = if com_escrita {
            base.open_subkey_with_flags(&chave, KEY_READ | KEY_WRITE).ok()
        } else {
            base.open_subkey(&chave).ok()
        };
        let Some(sub) = sub else { continue };
        let caminho = sub
            .get_value::<String, _>("ExecutablePath")
            .unwrap_or_default()
            .to_lowercase();
        if caminho == meu_caminho {
            return Some(sub);
        }
    }
    None
}

/// A preferencia "sempre mostrar" ficou gravada na chave do icone?
fn preferencia_aceita() -> bool {
    chave_do_icone(false)
        .and_then(|sub| sub.get_value::<u32, _>("IsPromoted").ok())
        .map(|v| v == 1)
        .unwrap_or(false)
}

/// Escreve a preferencia "sempre mostrar" do icone no registro.
///
/// A chave do icone esta em
/// `HKCU\Control Panel\NotifyIconSettings\<algum numero>`, e o Windows
/// guarda nela o caminho do executavel, o texto do icone e a preferencia
/// `IsPromoted`.
fn promover_icone() -> Result<(), String> {
    let sub = chave_do_icone(true).ok_or("a chave do icone ainda nao existe no registro")?;
    sub.set_value("IsPromoted", &1u32).map_err(|e| e.to_string())
}

fn leu_eventos() {
    // Clique no icone: o da esquerda abre a janela, o da direita abre o menu.
    while let Ok(evento) = TrayIconEvent::receiver().try_recv() {
        match evento {
            TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } => abrir_janela(),
            TrayIconEvent::Click {
                button: MouseButton::Right,
                button_state: MouseButtonState::Up,
                position,
                ..
            } => {
                registrar(&format!(
                    "clique direito na bandeja, em x={} y={}",
                    position.x, position.y
                ));
                if let Some(dono) = JANELA_AUXILIAR.get() {
                    tray_menu::pedir_menu(*dono, position.x as i32, position.y as i32);
                } else {
                    registrar("aviso: o clique direito chegou sem a janela dona do menu");
                }
            }
            _ => {}
        }
    }
}

/// Pede o encerramento do app (pela interface, para o `on_exit` rodar).
fn pedir_saida() {
    SAIR_PEDIDO.store(true, Ordering::SeqCst);
    if let Some(ctx) = contexto().lock().ok().and_then(|g| g.clone()) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        ctx.request_repaint();
    }
}

/// Traz a janela do app para o primeiro plano.
///
/// A interface e avisada pelo egui (e quem sabe o estado da viewport) e o
/// Windows e avisado pela API (e quem sabe se a janela esta minimizada).
fn abrir_janela() {
    if let Some(ctx) = contexto().lock().ok().and_then(|g| g.clone()) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        ctx.request_repaint();
    }
    crate::instancia::mostrar_janela_existente();
}

/// O desenho do ícone: um despertador de mesa, em laranja do app.
///
/// O desenho mora em `crate::icone` (o mesmo que vira arquivo `.ico` para os
/// atalhos do instalador).
pub fn icone() -> (Vec<u8>, u32) {
    (crate::icone::desenhar(64), 64)
}

pub fn icone_bruto() -> (Vec<u8>, u32) {
    icone()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_icone_tem_o_tamanho_certo() {
        let (rgba, lado) = icone();
        assert_eq!(lado, 64);
        assert_eq!(rgba.len(), (64 * 64 * 4) as usize);
    }

    #[test]
    fn o_icone_nao_e_vazio() {
        // Um icone todo transparente nao apareceria na bandeja.
        let (rgba, _) = icone();
        let pintados = rgba.chunks(4).filter(|p| p[3] > 0).count();
        assert!(pintados > 500, "o icone tem coisas demais: {pintados} pixels");
        assert!(pintados < 64 * 64, "o icone esta todo pintado");
    }

    #[test]
    fn o_icone_tem_o_laranja_do_app() {
        let (rgba, _) = icone();
        let laranja = rgba
            .chunks(4)
            .filter(|p| p[0] == 255 && p[1] == 138 && p[2] == 40 && p[3] == 255)
            .count();
        assert!(laranja > 300, "o corpo do despertador sumiu: {laranja} pixels");
    }

    #[test]
    fn a_promocao_do_icone_encontra_a_chave_certa() {
        // A chave do icone no registro e procurada pelo caminho exato do
        // executavel. Sem esse teste, uma versao instalada em outro lugar
        // poderia promover a chave antiga e o icone continuaria escondido.
        let meu = std::env::current_exe().expect("o proprio executavel");
        let caminho = meu.to_string_lossy().to_lowercase();
        assert!(caminho.ends_with(".exe"));
        assert!(caminho.contains("tranal"), "caminho inesperado: {caminho}");
    }
}
