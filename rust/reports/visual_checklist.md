# Phone-Typer Rust 版 GUI 目视确认清单

本开发环境无法可靠运行 Windows GUI/托盘交互。以下各项已通过静态核验（代码审查 + 资源核对），移交用户目视确认。

**被测产物**：`rust/dist/phone-typer-rust.exe`（2.0 MB）+ `rust/dist/web/`（69 KB）

**运行方式**：双击 `phone-typer-rust.exe`，或命令行 `phone-typer-rust.exe --autostart`（静默驻留）

---

## 清单（10 项）

| # | 确认项 | 静态核验状态 | 目视确认 |
|---|--------|------------|---------|
| 1 | 托盘图标正常显示，tooltip 为 "Phone-Typer v2.0.0-rust" | ✅ gui/tray.rs: TrayIcon::new().with_tooltip("Phone-Typer v{version}")；icon.ico 存在（23 KB） | ☐ |
| 2 | 托盘右键六项菜单文字与顺序正确，双击图标打开主界面 | ✅ gui/tray.rs: 菜单顺序=显示主界面/复制访问地址/发送剪贴板到手机/设置/重启服务/退出；TrayIconEvent::DoubleClick → WM_APP+1 | ☐ |
| 3 | 主界面布局：状态行、只读地址框、双二维码选项卡、HTTP·WS·PIN 信息行、两个按钮、底部说明 | ✅ gui/main_win.rs: SysTabControl32 双 tab + GDI 内存位图直渲 + STATIC/Edit/Button 控件 | ☐ |
| 4 | 二维码可扫：网页码在手机浏览器打开页面；App 码唤起手机 App 并配对成功 | ✅ qr.rs: qrcode 0.14 生成矩阵 + GDI 直渲；配对 URI ptyper://connect?ip=&http=&ws=&pin= | ☐ |
| 5 | 设置窗口：PIN 修改后不重启程序，新 PIN 立即对 WS 消息生效；自启开关生效（Autoruns 核对 HKCU Run PhoneTyperRust） | ✅ gui/settings.rs: on_save → save_config → hub.reset_pin；autostart.rs: winreg HKCU Run "PhoneTyperRust" | ☐ |
| 6 | 重复双击二进制：唤起已有实例主界面，不出现第二实例/第二进程 | ✅ instance.rs: Mutex "Local\PhoneTyperRust-SingleInstance-Mutex-v1" + Event 唤起 + WM_APP_RAISE | ☐ |
| 7 | 开机自启重启后静默驻留托盘、不弹窗 | ✅ main.rs: --autostart → run_gui(show_main=false)；autostart.rs: HKCU Run 注册 | ☐ |
| 8 | 手机实际键入验证：向记事本等焦点窗口键入（含中文、换行、长文本），sync 中间编辑回删正确 | ✅ keys.rs: SendInput + diff.rs: diff_sync 算法；--smoke STEP 4 验证 diff 0/3/0→1/1/0→0/0/0 | ☐ |
| 9 | 端口占用时启动：错误弹窗出现，提示修改 config.json | ✅ server/mod.rs: fatal_dialog MessageBoxW；ws.rs: bind 失败致命退出+弹窗 | ☐ |
| 10 | 托盘"退出"后托盘图标消失、进程结束、无残留进程 | ✅ gui/mod.rs: Exit 菜单项 → break 消息泵 → 进程退出；instance.rs: Mutex 释放 | ☐ |

---

## 确认方式

请在 Windows 桌面环境逐项确认，将 ☐ 改为 ☑。全部确认通过后，Rust 版即可交付。
