//! O menu de contexto do ícone da bandeja.
//!
//! Por que este arquivo existe, e por que ele não usa o menu pronto da
//! biblioteca de bandeja:
//!
//! - O menu padrão é ancorado **na posição do mouse**. O mouse está dentro da
//!   barra de tarefas (é de lá que o clique veio), e o Windows não sabe que a
//!   barra cobre aquela área: o menu é desenhado com a base dentro da barra e
//!   o último item ("Sair") fica **atrás** dela, invisível.
//! - Aqui o menu é ancorado na **borda inferior da área de trabalho** do
//!   monitor (a área que sobra depois da barra de tarefas), com a base do menu
//!   apoiada nela. O menu abre para cima e nenhum item fica escondido.
//!
//! E por que o menu é mostrado na thread principal: um menu do Windows
//! pertence à thread que está em primeiro plano. Quando ele é aberto de
//! outra thread, o Windows o fecha na hora (ele pisca e some). A thread
//! principal é a do event loop, e é ela que tem a janela dona do menu.
//!
//! O menu é o do Windows mesmo (`CreatePopupMenu`), então o visual e o
//! comportamento são os de sempre: só muda onde ele nasce.

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::OnceLock;

use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, PostMessageW,
    RegisterClassW, TrackPopupMenuEx, HMENU, MENU_ITEM_FLAGS, MF_SEPARATOR, MF_STRING,
    TRACK_POPUP_MENU_FLAGS, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RETURNCMD, WINDOW_EX_STYLE,
    WINDOW_STYLE, WM_APP, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_OVERLAPPED,
};

/// A mensagem que pede a abertura do menu (vai para a thread principal).
const WM_MENU_BANDEJA: u32 = WM_APP + 42;

/// Os comandos do menu (ids que o Windows devolve quando o item é escolhido).
const CMD_ABRIR: usize = 1001;
const CMD_SAIR: usize = 1002;

/// O que o dono escolheu no menu da bandeja.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcao {
    /// "Abrir o Tranal": trazer a janela para a frente.
    Abrir,
    /// "Sair": encerrar o app.
    Sair,
    /// O menu foi fechado sem escolher nada (clique fora, tecla Esc).
    Nenhuma,
}

/// O que o app faz com a escolha do dono.
static ACAO: OnceLock<fn(Opcao)> = OnceLock::new();

/// Onde o clique aconteceu (a posição do menu é calculada a partir dela).
static POSICAO_X: AtomicI32 = AtomicI32::new(0);
static POSICAO_Y: AtomicI32 = AtomicI32::new(0);

/// Registrar o que fazer com a escolha do dono.
pub fn definir_acao(acao: fn(Opcao)) {
    let _ = ACAO.set(acao);
}

/// Cria a janela invisível que serve de dona do menu.
///
/// O menu do Windows precisa de uma janela dona: é ela que faz o menu
/// fechar quando o clique cai fora dele. A janela do app não serve, porque
/// quando o dono clica no ícone da bandeja ela está minimizada — e acordar a
/// janela do app só para mostrar um menu é um efeito colateral que ninguém
/// pediu.
///
/// A janela é criada na thread principal (a do event loop) e nunca aparece.
pub fn criar_janela_auxiliar() -> Option<isize> {
    unsafe {
        let hinstance = GetModuleHandleW(None).ok()?;
        let classe = WNDCLASSW {
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance.into(),
            lpszClassName: w!("TranalBandejaAuxiliar"),
            ..Default::default()
        };
        // Se a classe ja existe (o app recriou a bandeja), RegisterClassW
        // falha com "classe ja registrada" — e isso nao e problema: a classe
        // que importa e a da janela, e ela ja esta valendo.
        let _ = RegisterClassW(&classe);

        CreateWindowExW(
            WINDOW_EX_STYLE(WS_EX_TOOLWINDOW.0 | WS_EX_NOACTIVATE.0),
            w!("TranalBandejaAuxiliar"),
            w!(""),
            WINDOW_STYLE(WS_OVERLAPPED.0),
            0,
            0,
            0,
            0,
            None,
            None,
            Some(hinstance.into()),
            None,
        )
        .ok()
        .map(|hwnd| hwnd.0 as isize)
    }
}

/// Pede a abertura do menu.
///
/// O pedido vai para a thread principal (a dona da janela) por mensagem: é
/// lá que o menu do Windows pode ficar aberto.
pub fn pedir_menu(dono: isize, cursor_x: i32, cursor_y: i32) {
    POSICAO_X.store(cursor_x, Ordering::SeqCst);
    POSICAO_Y.store(cursor_y, Ordering::SeqCst);
    let hwnd = HWND(dono as *mut core::ffi::c_void);
    // SAFETY: a janela dona foi criada por este app e vive ate o fim do
    // processo. Se a mensagem nao chegar, o menu simplesmente nao abre.
    unsafe {
        let _ = PostMessageW(Some(hwnd), WM_MENU_BANDEJA, WPARAM(0), LPARAM(0));
    }
}

/// A janela é invisível, mas não é muda: ela é a dona do menu.
unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    mensagem: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if mensagem == WM_MENU_BANDEJA {
        let x = POSICAO_X.load(Ordering::SeqCst);
        let y = POSICAO_Y.load(Ordering::SeqCst);
        let opcao = mostrar(hwnd, x, y);
        if let Some(acao) = ACAO.get() {
            acao(opcao);
        }
        return LRESULT(0);
    }
    unsafe { DefWindowProcW(hwnd, mensagem, wparam, lparam) }
}

/// Mostra o menu da bandeja e diz o que o dono escolheu.
///
/// `cursor_x`/`cursor_y` é onde o clique aconteceu: o menu nasce na coluna do
/// ícone, mas na borda inferior da área de trabalho (acima da barra de
/// tarefas), para nenhum item ficar escondido atrás dela.
fn mostrar(dono: HWND, cursor_x: i32, cursor_y: i32) -> Opcao {
    let Some(hmenu) = criar_menu() else {
        return Opcao::Nenhuma;
    };

    let (x, y) = ancora(cursor_x, cursor_y);
    let flags: TRACK_POPUP_MENU_FLAGS = TPM_LEFTALIGN | TPM_BOTTOMALIGN | TPM_RETURNCMD;
    // SAFETY: o menu foi criado agora e a janela dona esta viva.
    let escolhido = unsafe { TrackPopupMenuEx(hmenu, flags.0, x, y, dono, None).0 as usize };
    // SAFETY: o menu e nosso; o Windows nao fica com a mao nele depois do
    // TrackPopupMenuEx.
    unsafe {
        let _ = DestroyMenu(hmenu);
    }

    match escolhido {
        CMD_ABRIR => Opcao::Abrir,
        CMD_SAIR => Opcao::Sair,
        _ => Opcao::Nenhuma,
    }
}

/// Monta o menu (dois itens e um separador entre eles).
fn criar_menu() -> Option<HMENU> {
    unsafe {
        let hmenu = CreatePopupMenu().ok()?;
        AppendMenuW(hmenu, MENU_ITEM_FLAGS(MF_STRING.0), CMD_ABRIR, w!("Abrir o Tranal")).ok()?;
        AppendMenuW(hmenu, MENU_ITEM_FLAGS(MF_SEPARATOR.0), 0, w!("")).ok()?;
        AppendMenuW(hmenu, MENU_ITEM_FLAGS(MF_STRING.0), CMD_SAIR, w!("Sair")).ok()?;
        Some(hmenu)
    }
}

/// Onde o menu nasce: mesma coluna do clique, mas na borda inferior da área
/// de trabalho do monitor (a barra de tarefas fica fora dela).
fn ancora(x: i32, y: i32) -> (i32, i32) {
    unsafe {
        let monitor = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO {
            cbSize: std::mem::size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };
        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            (x, info.rcWork.bottom)
        } else {
            // Sem informação do monitor: usa o próprio clique (o Windows
            // vira o menu para cima se não couber embaixo).
            (x, y)
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn os_itens_do_menu_tem_ids_proprios() {
        // Os ids não podem colidir: o Windows devolve o id do item escolhido.
        assert_ne!(CMD_ABRIR, CMD_SAIR);
    }

    #[test]
    fn o_menu_tem_os_dois_itens() {
        // Criar o menu e uma chamada de Win32; se ela passar, os dois itens
        // foram montados (o item separador nao devolve id).
        let menu = criar_menu();
        assert!(menu.is_some());
        if let Some(hmenu) = menu {
            unsafe {
                let _ = DestroyMenu(hmenu);
            }
        }
    }

    #[test]
    fn o_menu_nasce_acima_da_barra_de_tarefas() {
        // A ancora e a borda inferior da area de trabalho: e o que impede o
        // ultimo item ("Sair") de ficar atras da barra de tarefas.
        let (x, y) = ancora(2193, 1416);
        assert_eq!(x, 2193);
        assert!(
            y < 1416,
            "o menu nasce em {y}, que esta dentro da barra de tarefas"
        );
    }
}
