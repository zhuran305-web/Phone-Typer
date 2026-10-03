# -*- coding: utf-8 -*-
"""
Phone-Typer 电脑端主程序

功能：
  1. HTTP 服务  : 在 http_port 上把页面发给手机浏览器
  2. WebSocket  : 接收手机消息
       - {"pin","type":"send","text"} : 整段键入（旧模式）
       - {"pin","type":"sync","text"} : 实时同步——与该连接基线做全文 diff，
                                        自动"退格 N 格 + 键入增量"
  3. 模拟键入   : 键入走剪贴板注入 + Ctrl+V（中文可靠）；删除走模拟 Backspace

运行形态：
  - 开发期：python server.py            → 控制台模式（横幅 + ASCII 二维码）
  - 分发期：双击 PhoneTyper.exe         → 系统托盘后台模式（无控制台窗口）
            PhoneTyper.exe --console   → 仍走控制台模式（排障用）
"""
import asyncio
import json
import logging
import os
import socket
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

import pyperclip
import qrcode
import websockets
from pynput.keyboard import Controller, Key

APP_VERSION = "1.0.0"

FROZEN = getattr(sys, "frozen", False)
BASE_DIR = os.path.dirname(os.path.abspath(__file__))
APP_DIR = os.path.dirname(sys.executable) if FROZEN else BASE_DIR  # exe/脚本所在目录

DEFAULT_CONFIG = {
    "http_port": 8766,
    "ws_port": 8767,
    "pin": "1234",
    "restore_clipboard": True,
    "paste_delay_ms": 60,
    "auto_enter": False,
}

log = logging.getLogger("phonetyper")


def setup_logging(console: bool = False) -> None:
    """配置日志：始终写 exe 旁 logs\\phone-typer.log；console=True 时同时输出到 stdout。"""
    log_dir = os.path.join(APP_DIR, "logs")
    os.makedirs(log_dir, exist_ok=True)
    handlers = [logging.FileHandler(os.path.join(log_dir, "phone-typer.log"), encoding="utf-8")]
    if console and sys.stdout is not None:
        handlers.append(logging.StreamHandler(sys.stdout))
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s [%(levelname)s] %(message)s",
        handlers=handlers,
        force=True,
    )


# 模块级先配一次（frozen → 仅文件；开发 → stdout+文件），后续 run_* 可重配
setup_logging(console=not FROZEN)


def load_config() -> dict:
    cfg = dict(DEFAULT_CONFIG)
    config_path = os.path.join(APP_DIR, "config.json")
    try:
        with open(config_path, "r", encoding="utf-8") as f:
            cfg.update(json.load(f))
    except FileNotFoundError:
        log.warning("未找到 %s，使用默认配置", config_path)
    except Exception as exc:
        log.warning("读取配置失败（%s），使用默认配置", exc)
    return cfg


CONFIG = load_config()
keyboard = Controller()


def save_config(updates: dict) -> None:
    """更新配置：写回 config.json 并热更新内存 CONFIG（无需重启服务）。"""
    config_path = os.path.join(APP_DIR, "config.json")
    try:
        with open(config_path, "r", encoding="utf-8") as f:
            disk = json.load(f)
    except Exception:
        disk = dict(DEFAULT_CONFIG)
    disk.update(updates)
    with open(config_path, "w", encoding="utf-8") as f:
        json.dump(disk, f, indent=2, ensure_ascii=False)
    CONFIG.update(updates)
    log.info("配置已更新: %s", list(updates.keys()))


_phones = set()           # 已连接的手机 WebSocket（用于电脑→手机反向推送）
_service_loop = None      # 服务线程的 asyncio loop（供跨线程提交协程）


def _safe_join(root: str, rel: str) -> str:
    full = os.path.normpath(os.path.join(root, rel))
    return full if full.startswith(root) else ""


def resolve_page(rel: str) -> str:
    """按优先级解析页面文件：exe 旁外部 web\\ > 内嵌资源 > 源码目录 web\\。"""
    rel = rel.lstrip("/\\")
    external = _safe_join(os.path.join(APP_DIR, "web"), rel)
    if external and os.path.isfile(external):
        return external
    bundled_root = os.path.join(getattr(sys, "_MEIPASS", BASE_DIR), "web")
    internal = _safe_join(bundled_root, rel)
    if internal and os.path.isfile(internal):
        return internal
    return ""


def get_lan_ip() -> str:
    """取本机局域网 IP（不会真正发包）。"""
    s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    try:
        s.connect(("8.8.8.8", 80))
        return s.getsockname()[0]
    except Exception:
        return "127.0.0.1"
    finally:
        s.close()


def type_text(text: str) -> None:
    """把文字键入到当前光标处：写剪贴板 -> Ctrl+V -> 可选恢复剪贴板 / 补回车。"""
    text = text.replace("\r\n", "\n").replace("\n", "\r\n")  # Windows 剪贴板惯例
    old = None
    if CONFIG.get("restore_clipboard"):
        try:
            old = pyperclip.paste()
        except Exception:
            old = None

    try:
        pyperclip.copy(text)
        time.sleep(max(CONFIG.get("paste_delay_ms", 60), 0) / 1000.0)

        with keyboard.pressed(Key.ctrl):
            keyboard.press("v")
            keyboard.release("v")

        if CONFIG.get("auto_enter"):
            time.sleep(0.05)
            keyboard.press(Key.enter)
            keyboard.release(Key.enter)
    finally:
        if old is not None:
            time.sleep(0.15)
            try:
                pyperclip.copy(old)
            except Exception:
                pass


def _press_key(key, n: int, delay: float = 0.01) -> None:
    """模拟按 n 次指定按键（n<0 视为 0）。"""
    for _ in range(max(n, 0)):
        keyboard.press(key)
        keyboard.release(key)
        time.sleep(delay)


def backspace(n: int) -> None:
    """模拟按 n 次 Backspace（实时同步时的删格回删）。"""
    _press_key(Key.backspace, n)


def move_left(n: int) -> None:
    """模拟按 n 次 ←（中间编辑时把光标从末尾移到编辑点）。"""
    _press_key(Key.left, n)


def move_right(n: int) -> None:
    """模拟按 n 次 →（中间编辑后把光标移回末尾）。"""
    _press_key(Key.right, n)


def diff_sync(old: str, new: str):
    """全文 diff：返回 (应退格数, 应键入增量, 共同后缀长度)。

    电脑端操作序列（假设光标在 old 末尾）：
        左移 s → 退格 bs → 键入 insert → 右移 s（回 new 末尾）
    s=0 时退化为末尾编辑（向后兼容 v1.3/1.4）。
    """
    p = 0
    m = min(len(old), len(new))
    while p < m and old[p] == new[p]:
        p += 1
    s = 0
    while (s < len(old) - p and s < len(new) - p
           and old[len(old) - 1 - s] == new[len(new) - 1 - s]):
        s += 1
    old_mid = old[p:len(old) - s]
    new_mid = new[p:len(new) - s]
    return len(old_mid), new_mid, s


class PageHandler(BaseHTTPRequestHandler):
    server_version = f"PhoneTyper/{APP_VERSION}"

    def _send(self, code: int, body: bytes, ctype: str) -> None:
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):
        if self.path in ("/api/config", "/api/config/"):
            body = json.dumps({"ws_port": CONFIG["ws_port"], "version": APP_VERSION}).encode("utf-8")
            self._send(200, body, "application/json; charset=utf-8")
            return

        rel = self.path.split("?", 1)[0].split("#", 1)[0]
        rel = "index.html" if rel in ("/", "") else rel
        page = resolve_page(rel)

        if not page:
            self._send(404, b"404 Not Found", "text/plain; charset=utf-8")
            return

        with open(page, "rb") as f:
            body = f.read()
        ctype = "text/html; charset=utf-8" if page.endswith(".html") else "application/octet-stream"
        self._send(200, body, ctype)

    def log_message(self, *args):
        pass


class LocalHTTPServer(ThreadingHTTPServer):
    # 关闭 SO_REUSEADDR：Windows 下端口被占用时让它直接报错，而非"假绑定成功"串包
    allow_reuse_address = False
    daemon_threads = True


def _fatal_port_error(field: str, port: int, exc: Exception) -> None:
    """端口冲突致命错误：写日志 + 弹窗提示（exe）或打印（控制台）后退出。"""
    msg = (f"端口 {port} 被占用或不可用：{exc}\n\n"
           f"请用记事本打开 exe 旁的 config.json，\n"
           f"把 \"{field}\" 改成其他端口（如 {port + 2}）后重试。")
    log.error("%s 端口 %s 被占用：%s", field, port, exc)
    if FROZEN:
        try:
            import tkinter as tk
            from tkinter import messagebox
            root = tk.Tk(); root.withdraw(); root.attributes("-topmost", True)
            messagebox.showerror("Phone-Typer 无法启动", msg, parent=root)
            root.destroy()
        except Exception:
            pass
    else:
        print(f"[FATAL] {msg}")
    os._exit(1)


def start_http_server() -> None:
    try:
        httpd = LocalHTTPServer(("0.0.0.0", CONFIG["http_port"]), PageHandler)
    except OSError as exc:
        _fatal_port_error("http_port", CONFIG["http_port"], exc)
    httpd.serve_forever()


async def ws_handler(ws):
    peer = getattr(ws, "remote_address", None)
    log.info("手机已连接: %s", peer)
    _phones.add(ws)
    last_text = ""
    try:
        async for raw in ws:
            try:
                msg = json.loads(raw)
                if not isinstance(msg, dict):
                    raise ValueError("not a dict")
            except Exception:
                await ws.send(json.dumps({"ok": False, "err": "bad-json"}))
                continue

            if str(msg.get("pin", "")) != str(CONFIG["pin"]):
                await ws.send(json.dumps({"ok": False, "err": "bad-pin"}))
                log.info("PIN 校验失败，已拒绝")
                continue

            text = msg.get("text", "") or ""
            if not isinstance(text, str):
                text = str(text)

            mtype = msg.get("type", "send")
            if mtype == "sync":
                bs, insert, s = diff_sync(last_text, text)
                if s:
                    await asyncio.to_thread(move_left, s)
                if bs:
                    await asyncio.to_thread(backspace, bs)
                if insert:
                    await asyncio.to_thread(type_text, insert)
                if s:
                    await asyncio.to_thread(move_right, s)
                if bs or insert or s:
                    preview = insert[:30].replace("\n", "\\n")
                    log.info("SYNC 左移%s 退格%s 键入%s字 右移%s: %r", s, bs, len(insert), s, preview)
                last_text = text
                await ws.send(json.dumps({"ok": True, "bs": bs, "typed": len(insert), "s": s}))
            else:
                if text:
                    await asyncio.to_thread(type_text, text)
                    preview = text[:30].replace("\n", "\\n")
                    log.info("TYPE 已键入 %s 字: %r", len(text), preview)
                last_text = ""
                await ws.send(json.dumps({"ok": True}))
    except websockets.ConnectionClosed:
        pass
    except Exception as exc:
        log.error("连接异常: %s", exc)
    finally:
        _phones.discard(ws)
        log.info("手机断开: %s", peer)


def send_clipboard_to_phones() -> tuple[int, str]:
    """读电脑剪贴板，推送给所有已连接手机。返回 (发送条数, 文本预览)；-1 表示读剪贴板失败。"""
    try:
        text = pyperclip.paste()
    except Exception as exc:
        log.error("读剪贴板失败: %s", exc)
        return -1, ""
    if not text:
        return 0, ""
    preview = text[:50].replace("\n", "\\n")
    if _service_loop is None or not _phones:
        return 0, preview
    msg = json.dumps({"type": "clip", "text": text})
    count = 0
    for ws in list(_phones):
        try:
            fut = asyncio.run_coroutine_threadsafe(ws.send(msg), _service_loop)
            fut.result(timeout=5)
            count += 1
        except Exception as exc:
            log.warning("推送剪贴板到手机失败: %s", exc)
    log.info("剪贴板已推送到 %s 台手机: %r", count, preview)
    return count, preview


class ServiceRunner:
    """可启停的服务容器：HTTP + WebSocket 跑在子线程的 asyncio loop 里。"""

    def __init__(self):
        self.loop = None
        self.thread = None

    def start(self) -> None:
        self.thread = threading.Thread(target=self._run, daemon=True, name="pt-service")
        self.thread.start()

    def _run(self) -> None:
        global _service_loop
        self.loop = asyncio.new_event_loop()
        _service_loop = self.loop
        asyncio.set_event_loop(self.loop)
        try:
            self.loop.run_until_complete(self._serve())
        except Exception as exc:
            log.error("服务异常: %s", exc)

    async def _serve(self) -> None:
        ip = get_lan_ip()
        threading.Thread(target=start_http_server, daemon=True, name="pt-http").start()
        log.info("HTTP 监听 %s:%s", ip, CONFIG["http_port"])
        log.info("WebSocket 监听 :%s", CONFIG["ws_port"])
        await _serve_ws_forever()

    def stop(self) -> None:
        if self.loop and self.loop.is_running():
            self.loop.call_soon_threadsafe(self.loop.stop)


def _print_banner(ip: str) -> None:
    print("=" * 56)
    print(f"  Phone-Typer v{APP_VERSION} 已启动")
    print(f"  手机浏览器打开:  http://{ip}:{CONFIG['http_port']}")
    print(f"  PIN: {CONFIG['pin']}")
    print(f"  WebSocket 端口: {CONFIG['ws_port']}")
    print("  手机扫码直达（同一 WiFi）:")
    print("=" * 56)


def run_console() -> None:
    """开发/排障模式：控制台横幅 + ASCII 二维码。"""
    setup_logging(console=True)
    ip = get_lan_ip()
    threading.Thread(target=start_http_server, daemon=True).start()
    _print_banner(ip)
    try:
        qr = qrcode.QRCode(border=1)
        qr.add_data(f"http://{ip}:{CONFIG['http_port']}")
        qr.make(fit=True)
        qr.print_ascii(invert=True)
    except Exception as exc:
        print(f"[WARN] 二维码生成失败（{exc}），请手动输入网址")

    try:
        asyncio.run(_serve_ws_forever())
    except KeyboardInterrupt:
        print("\n已退出")


async def _serve_ws_forever() -> None:
    try:
        async with websockets.serve(ws_handler, "0.0.0.0", CONFIG["ws_port"]):
            await asyncio.Future()
    except OSError as exc:
        _fatal_port_error("ws_port", CONFIG["ws_port"], exc)


if __name__ == "__main__":
    if FROZEN and "--console" not in sys.argv:
        # 分发态默认：系统托盘后台模式
        from tray import run
        run()
    else:
        run_console()
