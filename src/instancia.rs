//! Instância única e reabertura da janela.
//!
//! O dono fecha a janela e espera que o app continue de pé (é assim que um
//! despertador deve funcionar). Para isso o app precisa:
//!
//! 1. detectar que já existe outra instância rodando e, em vez de abrir uma
//!    segunda janela, trazer a que existe para o primeiro plano;
//! 2. esconder a janela ao fechar, em vez de matar o processo.

use std::sync::OnceLock;

use windows::core::w;
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, SetForegroundWindow, ShowWindow, SW_RESTORE,
};

/// O mutex fica vivo durante todo o processo: se o handle for solto, o nome
/// fica livre e a próxima instância acharia que é a primeira.
static INSTANCIA: OnceLock<isize> = OnceLock::new();

/// Tenta ser a única instância do app.
///
/// Devolve `true` se esta execução pode seguir (primeira instância) e
/// `false` se já existe um app rodando.
pub fn iniciar() -> bool {
    let nome = w!("Local\\TranalDespertadorUnico");
    let handle = unsafe { CreateMutexW(None, true, nome) }.map(|h| h.0 as isize).unwrap_or(0);
    if handle == 0 {
        // Não conseguiu criar o mutex: deixa passar, para não travar o app.
        return true;
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        return false;
    }
    let _ = INSTANCIA.set(handle);
    true
}

/// Traz a janela da instância que já está rodando para o primeiro plano.
pub fn mostrar_janela_existente() {
    unsafe {
        let Ok(hwnd) = FindWindowW(None, w!("Tranal Despertador")) else {
            return;
        };
        let _ = ShowWindow(hwnd, SW_RESTORE);
        let _ = SetForegroundWindow(hwnd);
    }
}
