# -*- coding: utf-8 -*-
"""
Phone-Typer 系统托盘后台模式。

无控制台窗口：托盘图标 + 右键菜单 + 主界面 / 设置小窗。
  - 显示主界面：窗口内直接生成并展示网页入口 / App 配对二维码
  - 复制访问地址 / 在浏览器打开 / 发送剪贴板到手机
  - 设置：PIN 热更新 + 开机自启开关（HKCU Run 注册表）
  - 退出
"""
import ctypes
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
ICON_ICO = os.path.join(APP_DIR, "web", "icons", "icon.ico")
BUNDLED_ICON_ICO = os.path.join(getattr(sys, "_MEIPASS", ""), "web", "icons", "icon.ico")

_OUTLINE_BTN = dict(fg_color="transparent", border_color="#6366f1", border_width=1,
                    text_color="#6366f1", hover_color="#16161a", corner_radius=8)


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


def _load_icon_ico_path() -> str | None:
    """解析窗口任务栏图标 .ico 路径：优先 exe 旁，其次内嵌，最后用占位图生成临时 ico。

    打包态经 sys._MEIPASS\\web\\icons\\icon.ico 可访问；全缺失时用 _load_icon_image()
    占位兜底生成临时 ico，保证 root.iconbitmap() 始终有可用源。异常记 WARNING 返回 None。
    """
    try:
        for path in (ICON_ICO, BUNDLED_ICON_ICO):
            if path and os.path.isfile(path):
                return path
        img = _load_icon_image()
        tmp_path = os.path.join(tempfile.gettempdir(), "phone-typer-taskbar.ico")
        img.save(tmp_path, format="ICO")
        log.warning("任务栏 ico 资源缺失，已生成临时占位图标: %s", tmp_path)
        return tmp_path
    except Exception as exc:
        log.warning("任务栏 ico 路径解析失败: %s", exc)
        return None


def _set_window_taskbar_icon(root) -> None:
    """设置窗口任务栏图标：调用 CTk 公开重写的 iconbitmap，置位其"用户图标优先"标志，
    抵抗 Windows 平台约 200ms 后 CTk 默认标题栏图标的覆盖；iconphoto 随后收尾设最终 PNG。
    异常仅记 WARNING 不抛出，窗口正常显示（spec 5.10.3 场景 2/4）。
    """
    try:
        ico_path = _load_icon_ico_path()
        if ico_path:
            root.iconbitmap(ico_path)
    except Exception as exc:
        log.warning("窗口任务栏图标设置失败（iconbitmap）: %s", exc)


def _install_raise_poller(root, flag, warn_msg: str) -> None:
    """注册 300ms 周期轮询：flag 置位时前置窗口并清标志，异常记 WARNING 不抛出。"""
    def _poll() -> None:
        if flag.is_set():
            flag.clear()
            try:
                root.deiconify()
                root.lift()
                root.attributes("-topmost", True)
                root.focus_force()
            except Exception as exc:
                log.warning("%s: %s", warn_msg, exc)
        root.after(300, _poll)
    root.after(300, _poll)


def _reapply_icon(root, photos) -> None:
    """CTk 200ms 默认图标回调之后延迟重设窗口图标（路径 c 防护）。"""
    try:
        if not root.winfo_exists():
            return
        from PIL import ImageTk
        taskbar_photo = ImageTk.PhotoImage(_load_icon_image(), master=root)
        photos.append(taskbar_photo)
        _set_window_taskbar_icon(root)
        root.iconphoto(False, taskbar_photo)
    except Exception as exc:
        log.warning("窗口图标延迟重设失败: %s", exc)


def _apply_window_icon(root, photos: list, logo_size: int,
                       logo_pady=(22, 6), warn_msg: str = "窗口图标加载失败") -> None:
    """统一窗口图标设置：Logo + 任务栏 ico + 标题栏 PNG + after(250) 延迟重设防 CTk 覆盖。"""
    import tkinter as tk
    from PIL import ImageTk

    try:
        icon_photo = ImageTk.PhotoImage(_load_icon_image(logo_size), master=root)
        photos.append(icon_photo)
        tk.Label(root, image=icon_photo, bg="#050506").pack(pady=logo_pady)
        taskbar_photo = ImageTk.PhotoImage(_load_icon_image(), master=root)
        photos.append(taskbar_photo)
        _set_window_taskbar_icon(root)
        root.iconphoto(False, taskbar_photo)
        root.after(250, lambda: _reapply_icon(root, photos))
    except Exception as exc:
        log.warning("%s: %s", warn_msg, exc)


def _install_window_icon(root, photos: list, logo_size: int,
                         logo_pady=(22, 6), warn_msg: str = "窗口图标加载失败") -> None:
    """加载 Logo 与任务栏图标入 photos（防 GC）+ 设任务栏 ico + iconphoto 收尾（薄封装）。"""
    _apply_window_icon(root, photos, logo_size, logo_pady, warn_msg)


def _access_url() -> str:
    return f"http://{get_lan_ip()}:{CONFIG['http_port']}"


def _pair_uri() -> str:
    """App 配对 URI（手机端 PtQrParser 约定的格式 A）。"""
    return (f"ptyper://connect?ip={get_lan_ip()}"
            f"&http={CONFIG['http_port']}&ws={CONFIG['ws_port']}&pin={CONFIG['pin']}")


def _qr_fallback(parent, ctk) -> None:
    """二维码生成失败时在 parent 内显示占位提示。"""
    ctk.CTkLabel(parent, text="二维码生成失败", text_color="#8a8f98",
                 fg_color="transparent", font=("Segoe UI", 11)).pack(padx=10, pady=10)


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


def _configure_tabview_purple_theme(tabview) -> None:
    """配置 CTkTabview 紫黑圆角配色（indigo 高亮），兼容不同 customtkinter 版本 API 差异。

    以逐参数 try/except 兜底，任一参数配置失败记 WARNING 后继续，不阻断 tabview 创建。
    最外层 try/except 包裹整个方法体，兼容早期版本参数名差异（如 selected_color 而非
    segmented_button_selected_color）。全部失败时 CTkTabview 呈 CTk 默认主题，需用户目视确认。
    """
    try:
        params = {
            "fg_color": "#16161a",
            "segmented_button_fg_color": "#050506",
            "segmented_button_selected_color": "#6366f1",
            "segmented_button_selected_hover_color": "#a855f7",
            "segmented_button_unselected_color": "#050506",
            "segmented_button_unselected_hover_color": "#16161a",
            "text_color": "#ededef",
            "text_color_disabled": "#565b64",
            "corner_radius": 10,
            "segmented_button_corner_radius": 8,
        }
        for name, value in params.items():
            try:
                tabview.configure(**{name: value})
            except (TypeError, ValueError) as exc:
                log.warning("CTkTabview 配色参数 %s 配置失败: %s", name, exc)
    except Exception as exc:
        log.warning("CTkTabview 配色配置整体失败: %s", exc)


def _run_status_window() -> None:
    import tkinter as tk
    import customtkinter as ctk

    import qrcode
    from PIL import Image, ImageTk

    ctk.set_appearance_mode("dark")
    url = _access_url()
    root = ctk.CTk()
    root.title(f"Phone-Typer v{APP_VERSION}")
    root.configure(fg_color="#050506")
    root.resizable(False, False)
    root.attributes("-topmost", True)

    _install_raise_poller(root, _raise_flag, "窗口前置失败")

    photos = []
    _apply_window_icon(root, photos, 72)

    ctk.CTkLabel(root, text="Phone-Typer", text_color="#6366f1", fg_color="transparent",
                 font=("Segoe UI", 20, "bold")).pack()
    ctk.CTkFrame(root, height=2, width=40, fg_color="#6366f1", corner_radius=0).pack(pady=(4, 8))
    status_row = ctk.CTkFrame(root, fg_color="transparent")
    status_row.pack(pady=(0, 14))
    ctk.CTkLabel(status_row, text="●", text_color="#4ade80", fg_color="transparent",
                 font=("Segoe UI", 11)).pack(side="left")
    ctk.CTkLabel(status_row, text=f"  v{APP_VERSION}  ·  服务运行中", text_color="#8a8f98", fg_color="transparent",
                 font=("Segoe UI", 11)).pack(side="left")

    ctk.CTkLabel(root, text="手机访问地址", text_color="#565b64", fg_color="transparent",
                 font=("Segoe UI", 12)).pack()
    url_card = ctk.CTkFrame(root, fg_color="#16161a", corner_radius=10, border_width=2, border_color="#3b3b46")
    url_card.pack(pady=(4, 10), padx=40, fill="x")
    ctk.CTkLabel(url_card, text=url, text_color="#ededef", fg_color="transparent",
                 font=("Consolas", 14, "bold")).pack(padx=12, pady=8)

    def qr_cell(parent, content, caption):
        img = qrcode.make(content).convert("RGB").resize((150, 150), Image.NEAREST)
        photo = ImageTk.PhotoImage(img, master=root)
        photos.append(photo)
        cell = ctk.CTkFrame(parent, fg_color="#16161a", corner_radius=10, border_width=2, border_color="#3b3b46")
        tk.Label(cell, image=photo, bg="#16161a").pack(padx=10, pady=(10, 4))
        ctk.CTkLabel(cell, text=caption, text_color="#8a8f98", fg_color="transparent",
                     font=("Segoe UI", 11)).pack(pady=(0, 10))
        return cell

    try:
        tabview = ctk.CTkTabview(root, fg_color="#16161a", corner_radius=10)
        _configure_tabview_purple_theme(tabview)
        tabview.pack(pady=(2, 8), padx=20, fill="x")

        tabview.add("网页版")
        try:
            qr_cell(tabview.tab("网页版"), url, "网页版入口（浏览器扫）").pack(padx=10, pady=10)
        except Exception as exc:
            log.warning("网页版二维码生成失败: %s", exc)
            _qr_fallback(tabview.tab("网页版"), ctk)

        tabview.add("App 版")
        try:
            qr_cell(tabview.tab("App 版"), _pair_uri(), "App 配对（App 扫）").pack(padx=10, pady=10)
        except Exception as exc:
            log.warning("App 版二维码生成失败: %s", exc)
            _qr_fallback(tabview.tab("App 版"), ctk)

        tabview.set("网页版")
    except (AttributeError, ImportError) as exc:
        log.warning("CTkTabview 不可用，降级为 ttk.Notebook: %s", exc)
        try:
            import tkinter.ttk as ttk
            style = ttk.Style()
            style.configure("TNotebook", background="#16161a", borderwidth=0)
            style.configure("TNotebook.Tab", background="#050506", foreground="#8a8f98",
                            padding=(12, 6), font=("Segoe UI", 11))
            style.map("TNotebook.Tab", background=[("selected", "#6366f1")],
                      foreground=[("selected", "#ededef")])
            notebook = ttk.Notebook(root)
            notebook.pack(pady=(2, 8), padx=20, fill="x")
            tab_web = tk.Frame(notebook, bg="#16161a")
            tab_app = tk.Frame(notebook, bg="#16161a")
            notebook.add(tab_web, text="网页版")
            notebook.add(tab_app, text="App 版")
            try:
                qr_cell(tab_web, url, "网页版入口（浏览器扫）").pack(padx=10, pady=10)
            except Exception as exc2:
                log.warning("网页版二维码生成失败: %s", exc2)
            try:
                qr_cell(tab_app, _pair_uri(), "App 配对（App 扫）").pack(padx=10, pady=10)
            except Exception as exc2:
                log.warning("App 版二维码生成失败: %s", exc2)
            notebook.select(0)
        except Exception as exc2:
            log.warning("ttk.Notebook 降级也失败: %s", exc2)
    except Exception as exc:
        log.warning("窗口二维码区域构建失败: %s", exc)

    ctk.CTkFrame(root, height=1, fg_color="#26262c", corner_radius=0).pack(fill="x", padx=40, pady=(4, 6))
    info_row = ctk.CTkFrame(root, fg_color="transparent")
    info_row.pack(pady=(0, 2))
    _info_label_style = dict(text_color="#8a8f98", fg_color="transparent", font=("Segoe UI", 12))
    _info_value_style = dict(text_color="#6366f1", fg_color="transparent", font=("Segoe UI", 13, "bold"))
    for label, value in (("HTTP ", CONFIG['http_port']), ("   ·   WS ", CONFIG['ws_port']), ("   ·   PIN ", CONFIG['pin'])):
        ctk.CTkLabel(info_row, text=label, **_info_label_style).pack(side="left")
        ctk.CTkLabel(info_row, text=str(value), **_info_value_style).pack(side="left")
    ctk.CTkLabel(root, text="手机 App 扫配对码 / 浏览器扫网页码（同一 WiFi）", text_color="#565b64", fg_color="transparent",
                 font=("Segoe UI", 11), wraplength=320).pack(pady=(0, 18))

    ctk.CTkButton(root, text="复制剪贴板到手机",
                  command=lambda: threading.Thread(target=send_clipboard_to_phone, daemon=True).start(),
                  **_OUTLINE_BTN, width=160, font=("Segoe UI", 12)).pack(pady=(6, 0))

    ctk.CTkButton(root, text="打开设置",
                  command=lambda: threading.Thread(target=get_or_create_settings_window, daemon=True).start(),
                  **_OUTLINE_BTN, width=160, font=("Segoe UI", 12)).pack(pady=(6, 18))

    root.eval('tk::PlaceWindow . center')
    root.mainloop()
    log.info("主界面窗口已关闭，程序驻留托盘")


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
_settings_raise_flag = threading.Event()
_settings_root = None
_settings_state = "idle"
_settings_pin_entry = None


def _do_focus(root, pin_entry) -> None:
    """设置窗口前置聚焦序列：deiconify → lift → topmost → focus_force → PIN 光标末尾。"""
    try:
        root.deiconify()
        root.lift()
        root.attributes("-topmost", True)
        root.focus_force()
        try:
            if pin_entry is not None:
                pin_entry.icursor("end")
        except Exception:
            pass
    except Exception as exc:
        log.warning("设置窗口前置聚焦失败: %s", exc)


def _on_close(root) -> None:
    """WM_DELETE_WINDOW 回调：销毁设置窗口实例并清空单例引用。"""
    global _settings_root, _settings_state
    try:
        root.destroy()
    except Exception as exc:
        log.warning("设置窗口销毁失败: %s", exc)
    _settings_root = None
    _settings_state = "idle"
    log.info("设置窗口已关闭")


def get_or_create_settings_window() -> None:
    """跨入口统一获取设置窗口：已存活则前置聚焦，不存在则新建（单例）。"""
    global _settings_root, _settings_state
    if not _settings_lock.acquire(blocking=False):
        root = _settings_root
        if root is not None:
            try:
                if root.winfo_exists():
                    try:
                        root.after(0, lambda: _do_focus(root, _settings_pin_entry))
                    except Exception:
                        _settings_raise_flag.set()
                else:
                    _settings_raise_flag.set()
            except Exception:
                _settings_raise_flag.set()
        return
    try:
        root = _settings_root
        if root is not None:
            try:
                if root.winfo_exists():
                    _settings_state = "alive"
                    try:
                        root.after(0, lambda: _do_focus(root, _settings_pin_entry))
                    except Exception:
                        _settings_raise_flag.set()
                    return
            except Exception:
                pass
        _settings_state = "creating"
        _run_settings_window()
    except Exception as exc:
        log.error("设置窗口异常: %s", exc)
    finally:
        _settings_root = None
        _settings_state = "idle"
        _settings_lock.release()


def show_settings_window() -> None:
    """点托盘'设置'：跨入口统一获取设置窗口（薄封装，向后兼容）。"""
    threading.Thread(target=get_or_create_settings_window, daemon=True).start()


def _run_settings_window() -> None:
    global _settings_root, _settings_state, _settings_pin_entry
    import tkinter as tk
    from tkinter import messagebox
    import customtkinter as ctk

    ctk.set_appearance_mode("dark")
    root = ctk.CTk()
    _settings_root = root
    _settings_state = "alive"
    root.title(f"Phone-Typer 设置 · v{APP_VERSION}")
    root.configure(fg_color="#050506")
    root.resizable(False, False)
    root.attributes("-topmost", True)

    _install_raise_poller(root, _settings_raise_flag, "设置窗口前置失败")
    root.protocol("WM_DELETE_WINDOW", lambda: _on_close(root))

    photos = []
    _apply_window_icon(root, photos, 56, logo_pady=(18, 4), warn_msg="设置窗口图标加载失败")

    ctk.CTkLabel(root, text="设置", text_color="#6366f1", fg_color="transparent",
                 font=("Segoe UI", 17, "bold")).pack(pady=(0, 14))

    pin_frame = ctk.CTkFrame(root, fg_color="transparent")
    pin_frame.pack(padx=30, pady=(0, 6), fill="x")
    ctk.CTkLabel(pin_frame, text="PIN 码", text_color="#565b64", fg_color="transparent",
                 font=("Segoe UI", 11), anchor="w").pack(fill="x")
    pin_var = tk.StringVar(value=str(CONFIG.get("pin", "")))
    pin_entry = ctk.CTkEntry(pin_frame, textvariable=pin_var, fg_color="#16161a",
                             border_color="#3b3b46", border_width=2, corner_radius=8,
                             text_color="#ededef", font=("Consolas", 13), width=220)
    pin_entry.pack(fill="x", pady=(4, 0))
    _settings_pin_entry = pin_entry
    pin_entry.focus_set()
    pin_entry.bind("<FocusIn>", lambda e: pin_entry.configure(border_color="#6366f1"))
    pin_entry.bind("<FocusOut>", lambda e: pin_entry.configure(border_color="#3b3b46"))

    ctk.CTkLabel(root, text="手机端需输入相同 PIN 才能键入", text_color="#565b64", fg_color="transparent",
                 font=("Segoe UI", 11), wraplength=260).pack(pady=(2, 14))

    try:
        autostart_initial = is_autostart_enabled()
    except Exception:
        autostart_initial = False
    autostart_var = tk.BooleanVar(value=autostart_initial)

    def on_toggle_autostart():
        desired = autostart_var.get()
        try:
            set_autostart(desired)
        except Exception as exc:
            log.error("设置开机自启失败: %s", exc)
            autostart_var.set(not desired)
            messagebox.showerror("设置失败", "设置开机自启失败，请检查系统权限", parent=root)

    autostart_frame = ctk.CTkFrame(root, fg_color="transparent")
    autostart_frame.pack(padx=30, pady=(0, 14), fill="x")
    ctk.CTkLabel(autostart_frame, text="开机自启", text_color="#565b64", fg_color="transparent",
                 font=("Segoe UI", 11), anchor="w").pack(fill="x")
    ctk.CTkCheckBox(autostart_frame, text="开机自动启动", variable=autostart_var,
                    command=on_toggle_autostart, fg_color="#6366f1", hover_color="#5558e0",
                    text_color="#ededef", corner_radius=6, font=("Segoe UI", 12)).pack(anchor="w", pady=(4, 0))

    def on_save():
        new_pin = pin_var.get().strip()
        if not new_pin:
            messagebox.showwarning("提示", "PIN 不能为空", parent=root)
            return
        try:
            save_config({"pin": new_pin})
        except Exception as exc:
            log.error("PIN 保存失败: %s", exc)
            messagebox.showerror("保存失败", str(exc), parent=root)
            return
        try:
            log.info("PIN 已更新为 %s（热生效，无需重启）", new_pin)
            messagebox.showinfo("已保存",
                                f"PIN 已更新为：{new_pin}\n（立即生效，无需重启）",
                                parent=root)
        except Exception as exc:
            log.warning("PIN 已保存但提示弹窗失败: %s", exc)
        finally:
            _on_close(root)

    ctk.CTkButton(root, text="保存", command=on_save, fg_color="#6366f1",
                  hover_color="#5558e0", text_color="#ffffff", corner_radius=8,
                  font=("Segoe UI", 13), width=200).pack(pady=(0, 18))

    root.eval('tk::PlaceWindow . center')
    root.mainloop()


def set_autostart(enabled: bool) -> None:
    """写入/删除 HKCU\\...\\Run 注册表项以控制开机自启。

    分发态指向 exe、开发态指向 server.py；写入时追加 --autostart 参数，
    使开机自启启动可被区分（静默驻留托盘，不弹主界面）。
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
            winreg.SetValueEx(key, app_name, 0, winreg.REG_SZ, f'"{target}" --autostart')
            log.info("已设置开机自启: %s", target)
        else:
            try:
                winreg.DeleteValue(key, app_name)
                log.info("已取消开机自启")
            except FileNotFoundError:
                pass
    finally:
        winreg.CloseKey(key)


def is_autostart_enabled() -> bool:
    """读取 HKCU\\...\\Run 注册表项，判断开机自启是否已开启。

    无该项或读取失败时返回 False（降级为默认关闭）。
    """
    import winreg

    key_path = r"Software\Microsoft\Windows\CurrentVersion\Run"
    app_name = "PhoneTyper"
    try:
        key = winreg.OpenKey(winreg.HKEY_CURRENT_USER, key_path, 0, winreg.KEY_READ)
    except OSError as exc:
        log.warning("读取开机自启状态失败: %s", exc)
        return False
    try:
        winreg.QueryValueEx(key, app_name)
        return True
    except FileNotFoundError:
        return False
    except OSError as exc:
        log.warning("读取开机自启状态失败: %s", exc)
        return False
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
            target=get_or_create_settings_window, daemon=True).start()),
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


# ---- 单实例运行（命名 Mutex + 命名 Event 唤起） ----------------------
_single_instance_mutex = None
_show_main_event = None
_raise_flag = threading.Event()

_MUTEX_NAME = "Local\\PhoneTyper-SingleInstance-Mutex-v1"
_EVENT_NAME = "Local\\PhoneTyper-ShowMain-Event-v1"
_ERROR_ALREADY_EXISTS = 183


def _acquire_single_instance() -> bool:
    """惰性创建命名 Mutex；已存在实例返回 False，否则返回 True。异常降级为 True。"""
    global _single_instance_mutex
    try:
        kernel32 = ctypes.windll.kernel32
        kernel32.CreateMutexW.restype = ctypes.c_void_p
        kernel32.CreateMutexW.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_wchar_p]
        handle = kernel32.CreateMutexW(None, False, _MUTEX_NAME)
        if not handle:
            log.warning("单实例 Mutex 创建失败，降级不拦截")
            return True
        _single_instance_mutex = handle
        if kernel32.GetLastError() == _ERROR_ALREADY_EXISTS:
            return False
        return True
    except Exception as exc:
        log.warning("单实例检测异常，降级不拦截: %s", exc)
        return True


def _signal_existing_instance() -> bool:
    """通知已有实例唤起主界面（OpenEvent + SetEvent）。失败返回 False。"""
    try:
        kernel32 = ctypes.windll.kernel32
        kernel32.OpenEventW.restype = ctypes.c_void_p
        kernel32.OpenEventW.argtypes = [ctypes.c_uint, ctypes.c_int, ctypes.c_wchar_p]
        kernel32.SetEvent.restype = ctypes.c_int
        kernel32.SetEvent.argtypes = [ctypes.c_void_p]
        kernel32.CloseHandle.restype = ctypes.c_int
        kernel32.CloseHandle.argtypes = [ctypes.c_void_p]
        handle = kernel32.OpenEventW(0x0002, False, _EVENT_NAME)
        if not handle:
            log.warning("打开唤起事件失败（已有实例可能未建监听）")
            return False
        try:
            kernel32.SetEvent(handle)
        finally:
            kernel32.CloseHandle(handle)
        return True
    except Exception as exc:
        log.warning("通知已有实例唤起异常: %s", exc)
        return False


def _start_show_main_listener() -> None:
    """第一实例创建命名事件 + daemon 监听线程，收到唤起时置 _raise_flag 并弹主界面。"""
    global _show_main_event
    try:
        kernel32 = ctypes.windll.kernel32
        kernel32.CreateEventW.restype = ctypes.c_void_p
        kernel32.CreateEventW.argtypes = [ctypes.c_void_p, ctypes.c_int, ctypes.c_int, ctypes.c_wchar_p]
        kernel32.WaitForSingleObject.restype = ctypes.c_uint
        kernel32.WaitForSingleObject.argtypes = [ctypes.c_void_p, ctypes.c_uint]
        handle = kernel32.CreateEventW(None, False, False, _EVENT_NAME)
        if not handle:
            log.warning("唤起事件创建失败，单实例唤起能力降级不可用")
            return
        _show_main_event = handle
    except Exception as exc:
        log.warning("唤起监听初始化异常: %s", exc)
        return

    def _listen():
        try:
            kernel32 = ctypes.windll.kernel32
            while True:
                ret = kernel32.WaitForSingleObject(_show_main_event, 0xFFFFFFFF)
                if ret == 0:
                    _raise_flag.set()
                    threading.Thread(target=show_status_window, daemon=True).start()
                else:
                    break
        except Exception as exc:
            log.warning("唤起监听线程异常: %s", exc)

    threading.Thread(target=_listen, daemon=True).start()


def _enable_dpi_awareness() -> None:
    """声明进程级 DPI awareness，使 Tkinter 在高 DPI 屏清晰渲染。三级降级，零新依赖。"""
    try:
        ctypes.windll.shcore.SetProcessDpiAwareness(2)
        return
    except Exception:
        pass
    try:
        ctypes.windll.user32.SetProcessDPIAware()
    except Exception:
        pass


def run() -> None:
    """托盘模式入口：起服务 + 跑托盘（阻塞主线程）。"""
    _enable_dpi_awareness()
    setup_logging(console=False)
    log.info("Phone-Typer v%s 托盘模式启动", APP_VERSION)

    if not _acquire_single_instance():
        if "--autostart" in sys.argv:
            log.info("已有实例运行中，自启触发静默退出（不唤起）")
        else:
            if _signal_existing_instance():
                log.info("已通知已有实例唤起主界面，本次启动静默退出")
            else:
                log.info("已有实例运行但唤起失败，本次启动静默退出")
        logging.shutdown()
        os._exit(0)

    _start_show_main_listener()

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
        if "--autostart" in sys.argv:
            log.info("开机自启模式启动，静默驻留托盘（不弹主界面）")
        else:
            log.info("启动时已弹出主界面窗口")
            threading.Thread(target=show_status_window, daemon=True).start()
        icon.run()
    except Exception as exc:
        log.error("托盘启动失败: %s", exc)
        os._exit(1)
