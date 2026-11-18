"""Teste fim a fim do Tranal Despertador (prova real, na maquina do dono).

Fases:
  fase1  app MINIMIZADO: o alarme dispara sozinho, o som sai (pico medido)
         e o `last_rung` fica gravado no arquivo de dados.
  fase2  app VISIVEL: o alarme dispara, o dono clica em "Soneca" (clique
         real de mouse, pelo pixel do botao vermelho "Parar o alarme") e a
         soneca TOCA de novo -- a regressao que o refactor corrigiu.

O som e medido pelo `IAudioMeterInformation` do Core Audio (pico de
amostra): so tem som se o pico sair de zero.

Uso:
  python scripts/_e2e.py fase1
  python scripts/_e2e.py fase2
"""

import ctypes
import json
import os
import subprocess
import sys
import time
from ctypes import wintypes

APP = r"D:\tranal-despertador\target\release\tranal-despertador.exe"
DADOS = os.path.join(os.environ["APPDATA"], "Tranal Despertador", "dados.json")
DOCS = r"D:\tranal-despertador\docs"

user32 = ctypes.windll.user32
gdi32 = ctypes.windll.gdi32
kernel32 = ctypes.windll.kernel32
EnumWindowsProc = ctypes.WINFUNCTYPE(ctypes.c_bool, wintypes.HWND, wintypes.LPARAM)

# Cor de preenchimento do botao "Parar o alarme" (Tema::VERMELHO_FORTE do
# design system). Antes era a cor antiga, (232, 76, 76), e o detector parava
# de achar o botao depois do redesenho.
VERMELHO = (198, 40, 40)


def focar(hwnd):
    """Traz a janela para o primeiro plano.

    O Windows bloqueia a troca de foco entre processos: um script em
    background nao consegue roubar o foco com SetForegroundWindow sozinho.
    A solucao classica e apertar ALT (o sistema solta o bloqueio) e, se
    ainda nao der, usar AttachThreadInput com a thread do foco atual.
    """
    user32.ShowWindow(hwnd, 9)  # SW_RESTORE
    time.sleep(0.2)

    user32.keybd_event(0x12, 0, 0, 0)  # ALT down
    time.sleep(0.05)
    user32.SetForegroundWindow(hwnd)
    time.sleep(0.05)
    user32.keybd_event(0x12, 0, 2, 0)  # ALT up
    time.sleep(0.3)

    if user32.GetForegroundWindow() != hwnd:
        fg = user32.GetForegroundWindow()
        tid_fg = user32.GetWindowThreadProcessId(fg, None)
        tid_me = kernel32.GetCurrentThreadId()
        user32.AttachThreadInput(tid_me, tid_fg, True)
        user32.BringWindowToTop(hwnd)
        user32.SetForegroundWindow(hwnd)
        user32.AttachThreadInput(tid_me, tid_fg, False)
        time.sleep(0.3)
    return user32.GetForegroundWindow() == hwnd


# ------------------------------------------------------------------ audio

def _medidor():
    """Prepara o Core Audio para medir pico de amostra (thread principal)."""
    import comtypes

    comtypes.CoInitialize()
    from pycaw.utils import AudioUtilities
    from pycaw.constants import EDataFlow
    from pycaw.api.endpointvolume import IAudioMeterInformation

    dispositivos = AudioUtilities.GetAllDevices(data_flow=EDataFlow.eRender.value)
    medidores = []
    for d in dispositivos:
        try:
            m = d._dev.Activate(
                IAudioMeterInformation._iid_, comtypes.CLSCTX_ALL, None
            ).QueryInterface(IAudioMeterInformation)
        except Exception:
            continue
        medidores.append((d.FriendlyName, m))
    return medidores


def pico(medidores):
    """Maior pico de amostra entre as saidas de audio."""
    melhor = 0.0
    for _, m in medidores:
        try:
            v = m.GetPeakValue()
        except Exception:
            continue
        if v > melhor:
            melhor = v
    return melhor


# --------------------------------------------------------------- dados

def escrever_alarme(quando, soneca_minutos=1, force_device_volume=False):
    hora, minuto = quando.hour, quando.minute
    dados = {
        "alarms": [
            {
                "id": "al-e2e-%d" % int(time.time()),
                "label": "E2E",
                "hour": hora,
                "minute": minuto,
                "enabled": True,
                "repeat": "once",
                "sound": {"preset": "classico"},
                "volume": 100,
                "fade_in_secs": 0,
                "force_device_volume": force_device_volume,
                "output_devices": [],
                "dismiss": {
                    "soneca": True,
                    "soneca_minutos": soneca_minutos,
                    "max_sonecas": 3,
                    "desafio": "nenhum",
                },
                "last_rung": None,
                "sonecas_usadas": 0,
            }
        ],
        "settings": {
            "iniciar_com_o_windows": False,
            "iniciar_minimizado": False,
            "forcar_volume_dispositivo": True,
            "volume_padrao": 100,
            "som_padrao": {"preset": "classico"},
            "sair_ao_fechar": False,
        },
    }
    os.makedirs(os.path.dirname(DADOS), exist_ok=True)
    with open(DADOS, "w", encoding="utf-8") as f:
        json.dump(dados, f, indent=2, ensure_ascii=False)
    return dados["alarms"][0]["id"]


def ler_dados():
    with open(DADOS, encoding="utf-8") as f:
        return json.load(f)


# -------------------------------------------------------------- janela

def janelas_do_pid(pid):
    """Todas as janelas de topo que pertencem ao processo (hwnd, titulo, rect)."""
    achados = []

    def cb(hwnd, lparam):
        p = wintypes.DWORD()
        user32.GetWindowThreadProcessId(hwnd, ctypes.byref(p))
        if p.value == pid:
            r = wintypes.RECT()
            user32.GetWindowRect(hwnd, ctypes.byref(r))
            buf = ctypes.create_unicode_buffer(256)
            user32.GetWindowTextW(hwnd, buf, 256)
            achados.append((hwnd, buf.value, r))
        return True

    user32.EnumWindows(EnumWindowsProc(cb), 0)
    return achados


def janela_do_app(pid):
    """hwnd e rect da janela principal do app.

    A janela principal e a que tem o titulo do app; o filtro por tamanho
    sozinho falha quando ela esta minimizada (o rect vira o do icone).
    """
    janelas = janelas_do_pid(pid)
    for hwnd, titulo, r in janelas:
        if titulo.startswith("Tranal"):
            return hwnd, titulo, r
    for hwnd, titulo, r in janelas:
        if r.right - r.left > 400 and r.bottom - r.top > 300:
            return hwnd, titulo, r
    return None


def capturar(hwnd, rect, saida):
    """Captura a janela em PNG (PrintWindow)."""
    from PIL import Image

    largura = rect.right - rect.left
    altura = rect.bottom - rect.top
    hdc = user32.GetWindowDC(hwnd)
    mdc = gdi32.CreateCompatibleDC(hdc)
    bmp = gdi32.CreateCompatibleBitmap(hdc, largura, altura)
    gdi32.SelectObject(mdc, bmp)
    user32.PrintWindow(hwnd, mdc, 2)

    class BITMAPINFOHEADER(ctypes.Structure):
        _fields_ = [
            ("biSize", wintypes.DWORD), ("biWidth", ctypes.c_long),
            ("biHeight", ctypes.c_long), ("biPlanes", wintypes.WORD),
            ("biBitCount", wintypes.WORD), ("biCompression", wintypes.DWORD),
            ("biSizeImage", wintypes.DWORD), ("biXPelsPerMeter", ctypes.c_long),
            ("biYPelsPerMeter", ctypes.c_long), ("biClrUsed", wintypes.DWORD),
            ("biClrImportant", wintypes.DWORD),
        ]

    class BITMAPINFO(ctypes.Structure):
        _fields_ = [("bmiHeader", BITMAPINFOHEADER), ("bmiColors", wintypes.DWORD * 3)]

    bi = BITMAPINFO()
    bi.bmiHeader.biSize = ctypes.sizeof(BITMAPINFOHEADER)
    bi.bmiHeader.biWidth = largura
    bi.bmiHeader.biHeight = -altura
    bi.bmiHeader.biPlanes = 1
    bi.bmiHeader.biBitCount = 32
    bi.bmiHeader.biCompression = 0

    buf = ctypes.create_string_buffer(largura * altura * 4)
    gdi32.GetDIBits(mdc, bmp, 0, altura, buf, ctypes.byref(bi), 0)
    img = Image.frombuffer("RGBA", (largura, altura), buf, "raw", "BGRA", 0, 1)
    img.save(saida)

    gdi32.DeleteObject(bmp)
    gdi32.DeleteDC(mdc)
    user32.ReleaseDC(hwnd, hdc)
    return img


def achar_botao_vermelho(img):
    """Bounding box do botao vermelho ('Parar o alarme') na captura."""
    from PIL import Image

    px = img.convert("RGB").load()
    largura, altura = img.size
    xs, ys = [], []
    for y in range(0, altura, 2):
        for x in range(0, largura, 2):
            r, g, b = px[x, y]
            if abs(r - VERMELHO[0]) < 24 and abs(g - VERMELHO[1]) < 24 and abs(b - VERMELHO[2]) < 24:
                xs.append(x)
                ys.append(y)
    if not xs:
        return None
    return min(xs), min(ys), max(xs), max(ys)


def cor_de_fundo(img):
    """Cor de fundo do painel: a cor mais comum na faixa da esquerda.

    A janela tem uma borda propria (uns 8 px) entao a amostragem comeca
    depois dela: a faixa esquerda, de x=12 em diante, e painel vazio.
    """
    from collections import Counter

    px = img.convert("RGB").load()
    largura, altura = img.size
    conta = Counter()
    for x in (12, 18, 24, 30, 40):
        for y in range(80, altura - 20, 9):
            conta[px[x, y]] += 1
    return conta.most_common(1)[0][0]


def achar_botao_soneca(img, caixa_vermelho):
    """Acha o botao 'Soneca', que fica ACIMA do botao vermelho.

    Os botoes sao empilhados e centralizados (linha horizontal nao centraliza
    no egui). O 'Soneca' e neutro: fundo cinza-azulado. O algoritmo sobe a
    coluna do botao vermelho, pula o respiro e mede a superficie do botao
    neutro.
    """
    px = img.convert("RGB").load()
    largura, altura = img.size
    x0, y0, x1, y1 = caixa_vermelho
    x_meio = (x0 + x1) // 2
    fundo = cor_de_fundo(img)

    y = y0 - 2
    while y > 0 and px[x_meio, y] == fundo:
        y -= 1
    base = y
    while y > 0 and px[x_meio, y] != fundo:
        y -= 1
    topo = y + 1
    if base <= topo or (base - topo) > 120:
        return None
    return topo, base, x_meio


def clicar(x, y, hwnd=None):
    """Clique de mouse de verdade: move, espera o egui ver o hover e clica."""
    if hwnd and not focar(hwnd):
        print("AVISO: a janela do app nao esta em foco (foco=%s)" % user32.GetForegroundWindow())
    user32.SetCursorPos(int(x) - 30, int(y))
    time.sleep(0.2)
    user32.SetCursorPos(int(x), int(y))
    time.sleep(0.3)
    user32.mouse_event(0x0002, 0, 0, 0, 0)  # LEFTDOWN
    time.sleep(0.10)
    user32.mouse_event(0x0004, 0, 0, 0, 0)  # LEFTUP
    time.sleep(0.2)


# ----------------------------------------------------------------- fases

def esperar_disparo(medidores, segundos, rotulo, quando=None, limiar=0.30):
    """Espera o som comecar de verdade.

    O alarme real chega a pico 1.0. Qualquer coisa abaixo de 0.30 e ruido de
    outro programa (notificacao, chat), e nao o despertador: por isso o som
    precisa passar do limiar em Amostras CONSECUTIVAS, e nao em uma so.
    """
    inicio = time.time()
    melhor = 0.0
    acima = 0
    while time.time() - inicio < segundos:
        v = pico(medidores)
        if v > melhor:
            melhor = v
        if v > limiar:
            acima += 1
        else:
            acima = 0
        if acima >= 3:
            atraso = ""
            if quando is not None:
                atraso = " (%.0fs apos o horario)" % (time.time() - quando)
            print("[%s] SOM: pico %.4f em %.1fs%s" % (rotulo, v, time.time() - inicio, atraso))
            return time.time() - inicio, v
        time.sleep(0.25)
    print("[%s] sem som em %ds (pico maximo %.4f)" % (rotulo, segundos, melhor))
    return None, melhor


def esperar_last_rung(alarme_id, segundos=20):
    """Espera o arquivo registrar que o alarme tocou."""
    fim = time.time() + segundos
    while time.time() < fim:
        try:
            dados = ler_dados()
            for a in dados["alarms"]:
                if a["id"] == alarme_id and a.get("last_rung"):
                    return a["last_rung"]
        except Exception:
            pass
        time.sleep(1)
    return None


def fase1():
    from datetime import datetime, timedelta

    print("== FASE 1: app minimizado ==")
    agora = datetime.now()
    quando = (agora + timedelta(minutes=1)).replace(second=0, microsecond=0)
    alarme_id = escrever_alarme(quando, soneca_minutos=1)
    print("alarme gravado para %s (id %s)" % (quando.strftime("%H:%M:%S"), alarme_id))

    medidores = _medidor()
    print("saidas monitoradas: %s" % ", ".join(n for n, _ in medidores))

    proc = subprocess.Popen([APP, "--minimizado"])
    print("app iniciado, pid %d" % proc.pid)
    try:
        espera, v = esperar_disparo(medidores, 300, "fase1")
        if espera is None:
            print("RESULTADO FASE 1: FALHOU (nenhum som)")
            return 1
        last = esperar_last_rung(alarme_id, 30)
        print("last_rung no arquivo: %r" % last)
        hoje = datetime.now().strftime("%Y-%m-%d")
        ok = last == hoje
        print("RESULTADO FASE 1: %s" % ("OK" if ok else "FALHOU"))
        return 0 if ok else 1
    finally:
        proc.kill()
        proc.wait()


def fase2():
    from datetime import datetime, timedelta

    print("== FASE 2: soneca real (app visivel) ==")
    # Alarme no proximo minuto cheio: o app dispara em HH:MM:00.
    agora = datetime.now()
    quando = (agora + timedelta(minutes=1)).replace(second=0, microsecond=0)
    alarme_id = escrever_alarme(quando, soneca_minutos=1)
    print("alarme gravado para %s (id %s)" % (quando.strftime("%H:%M:%S"), alarme_id))

    medidores = _medidor()
    proc = subprocess.Popen([APP])
    print("app iniciado, pid %d" % proc.pid)
    try:
        # A janela precisa existir e estar em foco para o clique funcionar.
        janela = None
        for _ in range(40):
            janela = janela_do_app(proc.pid)
            if janela:
                break
            time.sleep(0.5)
        if not janela:
            print("RESULTADO FASE 2: FALHOU (sem janela)")
            return 1
        hwnd, titulo, rect = janela
        print("janela: hwnd=%s titulo=%r %sx%s" % (hwnd, titulo, rect.right - rect.left, rect.bottom - rect.top))
        user32.ShowWindow(hwnd, 5)  # SW_SHOW
        if not focar(hwnd):
            print("AVISO: nao consegui focar a janela")
        time.sleep(1)

        espera, v = esperar_disparo(medidores, 300, "fase2-disparo")
        if espera is None:
            print("RESULTADO FASE 2: FALHOU (nenhum som)")
            return 1

        time.sleep(2)
        img = capturar(hwnd, rect, os.path.join(DOCS, "_e2e-tocando.png"))
        caixa = achar_botao_vermelho(img)
        if not caixa:
            print("RESULTADO FASE 2: FALHOU (botao vermelho nao encontrado)")
            return 1
        x0, y0, x1, y1 = caixa
        print("botao 'Parar o alarme' em x %d..%d y %d..%d" % (x0, x1, y0, y1))

        achado = achar_botao_soneca(img, caixa)
        if not achado:
            print("RESULTADO FASE 2: FALHOU (botao 'Soneca' nao encontrado)")
            return 1
        topo, base, x_soneca = achado
        y_soneca = (topo + base) // 2
        print("botao 'Soneca' em y %d..%d (clique em %d, %d)" % (topo, base, x_soneca, y_soneca))

        clique_x = rect.left + x_soneca
        clique_y = rect.top + y_soneca
        print("clique em tela (%d, %d)" % (clique_x, clique_y))
        clicar(clique_x, clique_y, hwnd)
        time.sleep(1.5)

        img2 = capturar(hwnd, rect, os.path.join(DOCS, "_e2e-soneca.png"))
        # O clique tem que ter PARADO o som: e isso que prova que a soneca
        # foi aceita (e nao que o botao foi clicado sem efeito).
        pico_apos = 0.0
        for _ in range(20):
            pico_apos = max(pico_apos, pico(medidores))
            time.sleep(0.1)
        print("pico depois do clique (soneca): %.4f" % pico_apos)
        if pico_apos > 0.05:
            print("RESULTADO FASE 2: FALHOU (o som nao parou: o clique nao pegou)")
            return 1

        # A soneca foi criada? O agendador tem que tocar de novo.
        espera2, v2 = esperar_disparo(medidores, 240, "fase2-soneca")
        last = esperar_last_rung(alarme_id, 15)
        print("last_rung no arquivo: %r" % last)
        ok = espera2 is not None
        print("RESULTADO FASE 2: %s" % ("OK" if ok else "FALHOU"))
        return 0 if ok else 1
    finally:
        proc.kill()
        proc.wait()


if __name__ == "__main__":
    qual = sys.argv[1] if len(sys.argv) > 1 else "fase1"
    sys.exit(fase1() if qual == "fase1" else fase2())
