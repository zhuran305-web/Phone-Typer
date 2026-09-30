# -*- coding: utf-8 -*-
"""
Phone-Typer 系统托盘后台模式（v1.6）。

无控制台窗口：托盘图标 + 右键菜单 + 小状态窗口。
  - 显示主界面：tkinter 小窗（IP / 二维码 / 端口 / PIN）
  - 显示二维码：生成 PNG → 系统默认看图器弹窗
  - 复制访问地址 / 在浏览器打开
  - 退出

预留开机自启接口（set_autostart），暂不接入菜单，便于以后扩展。
"""
import logging
import os
import sys
import tempfile
import threading
import webbrowser

import pyperclip

from server import (
    APP_DIR,
    APP_VERSION,
    CONFIG,
    ServiceRunner,
    get_lan_ip,
    save_config,
    send_clipboard_to_phones,
    setup_logging,
)

log = logging.getLogger("phonetyper")

ICON_PATH = os.path.join(APP_DIR, "web", "icons", "icon-256.png")
ICON_PATH_512 = os.path.join(APP_DIR, "web", "icons", "icon-512.png")
BUNDLED_ICON = os.path.join(getattr(sys, "_MEIPASS", ""), "web", "icons", "icon-256.png")
BUNDLED_ICON_512 = os.path.join(getattr(sys, "_MEIPASS", ""), "web", "icons", "icon-512.png")


def _load_icon_image(size: int | None = None):
    """加载图标：优先 exe 旁 web\\icons\\，其次内嵌资源，最后纯代码画一个。"""
    from PIL import Image, ImageDraw

    img = None
    for path in (ICON_PATH, ICON_PATH_512, BUNDLED_ICON, BUNDLED_ICON_512):
        if path and os.path.isfile(path):
            try:
                img = Image.open(path)
                break
            except Exception as exc:
                log.warning("图标加载失败 %s: %s", path, exc)
    if img is None:
        img = Image.new("RGBA", (64, 64), (40, 120, 200, 255))
        d = ImageDraw.Draw(img)
        d.text((22, 14), "P", fill=(255, 255, 255, 255))
    if size:
        img = img.resize((size, size), Image.LANCZOS)
    return img


def _access_url() -> str:
    return f"http://{get_lan_ip()}:{CONFIG['http_port']}"


def _pair_uri() -> str:
    """App 配对 URI（手机端 PtQrParser 约定的格式 A）。"""
    return (f"ptyper://connect?ip={get_lan_ip()}"
            f"&http={CONFIG['http_port']}&ws={CONFIG['ws_port']}&pin={CONFIG['pin']}")


def _qr_png_path(name: str = "phone-typer-qr.png") -> str:
    return os.path.join(tempfile.gettempdir(), name)


def show_qrcode() -> None:
    """生成二维码 PNG 并用系统默认看图器打开。"""
    import qrcode

    url = _access_url()
    try:
        img = qrcode.make(url)
        path = _qr_png_path()
        img.save(path)
        os.startfile(path)
        log.info("二维码已弹出: %s", url)
    except Exception as exc:
        log.error("二维码弹窗失败: %s", exc)


def show_pair_qrcode() -> None:
    """生成 App 配对二维码（ptyper:// URI）并用系统看图器打开。"""
    import qrcode

    uri = _pair_uri()
    try:
        img = qrcode.make(uri)
        path = _qr_png_path("phone-typer-pair-qr.png")
        img.save(path)
        os.startfile(path)
        log.info("App 配对二维码已弹出: %s", uri)
    except Exception as exc:
        log.error("配对二维码弹窗失败: %s", exc)


def copy_address() -> None:
    try:
        pyperclip.copy(_access_url())
        log.info("已复制访问地址到剪贴板")
    except Exception as exc:
        log.error("复制地址失败: %s", exc)


def open_in_browser() -> None:
    try:
        webbrowser.open(_access_url())
        log.info("已在浏览器打开 %s", _access_url())
    except Exception as exc:
        log.error("打开浏览器失败: %s", exc)


# ---- 小状态窗口（tkinter，子线程） --------------------------------
_window_lock = threading.Lock()


def show_status_window() -> None:
    """点托盘'显示主界面'：子线程里跑一个 tkinter 小窗。防多开。"""
    if not _window_lock.acquire(blocking=False):
        return
    try:
        _run_status_window()
    except Exception as exc:
        log.error("状态窗口异常: %s", exc)
    finally:
        _window_lock.release()


def _run_status_window() -> None:
    import tkinter as tk

    import qrcode
    from PIL import Image, ImageTk

    url = _access_url()
    root = tk.Tk()
    root.title(f"Phone-Typer v{APP_VERSION}")
    root.configure(bg="#0a0a0c")
    root.resizable(False, False)
    root.attributes("-topmost", True)

    photos = []

    try:
        icon_photo = ImageTk.PhotoImage(_load_icon_image(72))
        photos.append(icon_photo)
        tk.Label(root, image=icon_photo, bg="#0a0a0c").pack(pady=(22, 6))
    except Exception as exc:
        log.warning("窗口图标加载失败: %s", exc)

    tk.Label(root, text="Phone-Typer", fg="#ededef", bg="#0a0a0c",
             font=("Segoe UI", 17, "bold")).pack()
    tk.Label(root, text=f"v{APP_VERSION}  ·  服务运行中", fg="#8a8f98", bg="#0a0a0c",
             font=("Segoe UI", 9)).pack(pady=(0, 14))

    tk.Label(root, text="手机访问地址", fg="#8a8f98", bg="#0a0a0c",
             font=("Segoe UI", 10)).pack()
    tk.Label(root, text=url, fg="#ededef", bg="#0a0a0c",
             font=("Consolas", 14, "bold")).pack(pady=(2, 10))

    try:
        row = tk.Frame(root, bg="#0a0a0c")
        row.pack(pady=(2, 8))

        def qr_cell(parent, content, caption):
            img = qrcode.make(content).convert("RGB").resize((150, 150), Image.NEAREST)
            photo = ImageTk.PhotoImage(img)
            photos.append(photo)
            cell = tk.Frame(parent, bg="#0a0a0c")
            tk.Label(cell, image=photo, bg="#0a0a0c",
                     highlightbackground="#26262c", highlightthickness=1).pack()
            tk.Label(cell, text=caption, fg="#8a8f98", bg="#0a0a0c",
                     font=("Segoe UI", 9)).pack(pady=(4, 0))
            return cell

        qr_cell(row, url, "网页版入口（浏览器扫）").pack(side="left", padx=10)
        qr_cell(row, _pair_uri(), "App 配对（App 扫）").pack(side="left", padx=10)
    except Exception as exc:
        log.warning("窗口二维码生成失败: %s", exc)

    info = f"HTTP {CONFIG['http_port']}   ·   WS {CONFIG['ws_port']}   ·   PIN {CONFIG['pin']}"
    tk.Label(root, text=info, fg="#a1a1aa", bg="#0a0a0c",
             font=("Segoe UI", 10)).pack(pady=(4, 2))
    tk.Label(root, text="手机 App 扫配对码 / 浏览器扫网页码（同一 WiFi）", fg="#565b64", bg="#0a0a0c",
             font=("Segoe UI", 9)).pack(pady=(0, 18))

    root.eval('tk::PlaceWindow . center')
    root.mainloop()


_clip_lock = threading.Lock()


def send_clipboard_to_phone() -> None:
    """点托盘'发送剪贴板到手机'：读电脑剪贴板 → 推送给已连接手机。"""
    if not _clip_lock.acquire(blocking=False):
        return
    try:
        import tkinter as tk
        from tkinter import messagebox

        count, preview = send_clipboard_to_phones()
        root = tk.Tk(); root.withdraw(); root.attributes("-topmost", True)
        if count < 0:
            messagebox.showerror("发送失败", "读取剪贴板失败", parent=root)
        elif count == 0:
            messagebox.showinfo("提示", "剪贴板为空，或没有手机连接", parent=root)
        else:
            tip = preview + ("…" if len(preview) >= 50 else "")
            messagebox.showinfo("已发送", f"已推送到 {count} 台手机\n内容预览：{tip}", parent=root)
        root.destroy()
    except Exception as exc:
        log.error("发送剪贴板失败: %s", exc)
    finally:
        _clip_lock.release()


_settings_lock = threading.Lock()


def show_settings_window() -> None:
    """点托盘'设置'：子线程里跑一个 tkinter 设置小窗。防多开。"""
    if not _settings_lock.acquire(blocking=False):
        return
    try:
        _run_settings_window()
    except Exception as exc:
        log.error("设置窗口异常: %s", exc)
    finally:
        _settings_lock.release()


def _run_settings_window() -> None:
    import tkinter as tk
    from tkinter import messagebox

    from PIL import ImageTk

    root = tk.Tk()
    root.title(f"Phone-Typer 设置 · v{APP_VERSION}")
    root.configure(bg="#0a0a0c")
    root.resizable(False, False)
    root.attributes("-topmost", True)

    photos = []
    try:
        icon_photo = ImageTk.PhotoImage(_load_icon_image(56))
        photos.append(icon_photo)
        tk.Label(root, image=icon_photo, bg="#0a0a0c").pack(pady=(18, 4))
    except Exception as exc:
        log.warning("设置窗口图标加载失败: %s", exc)

    tk.Label(root, text="设置", fg="#ededef", bg="#0a0a0c",
             font=("Segoe UI", 15, "bold")).pack(pady=(0, 14))

    pin_frame = tk.Frame(root, bg="#0a0a0c")
    pin_frame.pack(padx=30, pady=(0, 6), fill="x")
    tk.Label(pin_frame, text="PIN 码", fg="#8a8f98", bg="#0a0a0c",
             font=("Segoe UI", 10)).pack(anchor="w")
    pin_var = tk.StringVar(value=str(CONFIG.get("pin", "")))
    pin_entry = tk.Entry(pin_frame, textvariable=pin_var, fg="#ededef", bg="#16161a",
                         insertbackground="#ededef", relief="flat",
                         font=("Consolas", 13), width=22)
    pin_entry.pack(fill="x", pady=(4, 0), ipady=6)
    pin_entry.focus_set()

    tk.Label(root, text="手机端需输入相同 PIN 才能键入", fg="#565b64", bg="#0a0a0c",
             font=("Segoe UI", 9)).pack(pady=(2, 14))

    def on_save():
        new_pin = pin_var.get().strip()
        if not new_pin:
            messagebox.showwarning("提示", "PIN 不能为空", parent=root)
            return
        try:
            save_config({"pin": new_pin})
            log.info("PIN 已更新为 %s（热生效，无需重启）", new_pin)
            messagebox.showinfo("已保存",
                                f"PIN 已更新为：{new_pin}\n（立即生效，无需重启）",
                                parent=root)
            root.destroy()
        except Exception as exc:
            log.error("PIN 保存失败: %s", exc)
            messagebox.showerror("保存失败", str(exc), parent=root)

    tk.Button(root, text="保存", command=on_save, fg="#ededef", bg="#3b3b46",
              activebackground="#4b4b56", activeforeground="#ededef",
              relief="flat", font=("Segoe UI", 11), width=12,
              cursor="hand2").pack(pady=(0, 18))

    root.eval('tk::PlaceWindow . center')
    root.mainloop()


def set_autostart(enabled: bool) -> None:
    """写入/删除 HKCU\\...\\Run 注册表项以控制开机自启。

    预留扩展接口，当前版本不调用。以后加菜单项时直接调用本函数即可。
    """
    import winreg

    key_path = r"Software\Microsoft\Windows\CurrentVersion\Run"
    app_name = "PhoneTyper"
    try:
        key = winreg.OpenKey(winreg.HKEY_CURRENT_USER, key_path, 0, winreg.KEY_SET_VALUE)
    except OSError:
        key = winreg.CreateKey(winreg.HKEY_CURRENT_USER, key_path)
    try:
        if enabled:
            target = sys.executable if getattr(sys, "frozen", False) else os.path.abspath("server.py")
            winreg.SetValueEx(key, app_name, 0, winreg.REG_SZ, f'"{target}"')
            log.info("已设置开机自启: %s", target)
        else:
            try:
                winreg.DeleteValue(key, app_name)
                log.info("已取消开机自启")
            except FileNotFoundError:
                pass
    finally:
        winreg.CloseKey(key)


def _build_menu():
    from pystray import Menu, MenuItem

    return Menu(
        MenuItem("显示主界面", lambda icon, item: threading.Thread(
            target=show_status_window, daemon=True).start(), default=True),
        MenuItem("复制访问地址", lambda icon, item: copy_address()),
        MenuItem("在浏览器打开", lambda icon, item: open_in_browser()),
        MenuItem("发送剪贴板到手机", lambda icon, item: threading.Thread(
            target=send_clipboard_to_phone, daemon=True).start()),
        MenuItem("设置", lambda icon, item: threading.Thread(
            target=show_settings_window, daemon=True).start()),
        Menu.SEPARATOR,
        MenuItem("退出", lambda icon, item: _on_exit(icon)),
    )


def _on_exit(icon) -> None:
    log.info("用户从托盘退出")
    try:
        icon.stop()
    except Exception:
        pass
    os._exit(0)


def run() -> None:
    """托盘模式入口：起服务 + 跑托盘（阻塞主线程）。"""
    setup_logging(console=False)
    log.info("Phone-Typer v%s 托盘模式启动", APP_VERSION)

    service = ServiceRunner()
    service.start()

    try:
        from pystray import Icon

        icon = Icon(
            "PhoneTyper",
            _load_icon_image(),
            f"Phone-Typer v{APP_VERSION}",
            _build_menu(),
        )
        icon.run()
    except Exception as exc:
        log.error("托盘启动失败: %s", exc)
        os._exit(1)
