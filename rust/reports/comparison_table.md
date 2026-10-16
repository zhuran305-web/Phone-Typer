# Phone-Typer 桌面端 Python vs Rust 资源对比表

测量日期：2026-10-16 | 测量工具：simple/src/measure.py（psutil 7.2.2）| 口径：USS（memory_full_info().uss）

## 核心指标对比

| 指标 | Python 版（simple v1.1.0） | Rust 版（v2.0.0-rust） | 降幅 | 目标 | 达标 |
|------|--------------------------|----------------------|------|------|------|
| **cold-tray 私有内存 USS** | 25.54 MB | **1.78 MB** | **-93.0%** | ≤8 MB | ✅ |
| cold-tray 工作集 RSS | ~60 MB | 14.3 MB | -76.2% | — | ✅ |
| cold-tray 提交 VMS | ~65 MB | 2.72 MB | -95.8% | — | ✅ |
| cold-tray CPU 均值 | ~0.1% | 0.05% | — | <1% | ✅ |
| cold-tray CPU 峰值 | ~2.0% | 1.6% | — | — | ✅ |
| 进程数 | 1 | 1 | — | =1 | ✅ |
| 线程数 | ~8 | 16 | — | — | ✅ |
| 句柄数 | ~150 | 213 | — | — | ✅ |
| **包体（exe + 资源）** | 30.12 MB | **2.1 MB** | **-93.0%** | ≤30 MB | ✅ |
| 二进制单独 | 22.19 MB | 2.0 MB | -91.0% | — | ✅ |
| 启动时间 | ~2s | <1s（日志时间戳） | — | ≤3s | ✅ |
| /api/config 时延 | <50ms | 0ms | — | ≤200ms | ✅ |
| soak USS 增长（100s+WS注入） | 未测 | 0.15 MB | — | ≤3MB/1800s | ✅ |
| 日志磁盘上界 | ~1.5 MB | 6.8 KB | — | ≤1.5 MB | ✅ |

## 架构形态对比

| 维度 | Python 版 | Rust 版 |
|------|----------|---------|
| 分发形态 | 单 exe + `_internal/` 目录（PyInstaller onedir） | 单 exe + `web/` 目录（便携） |
| 依赖打包 | _internal 含 Python 运行时 + 全部库 | 无运行时，静态链接（LTO fat） |
| GUI 框架 | tkinter + customtkinter | 原生 Win32 API + GDI |
| 托盘 | pystray + PIL | tray-icon 0.21 |
| HTTP | 内置 http.server | tiny_http 0.12 |
| WebSocket | websockets 库 | tokio-tungstenite 0.26 |
| 异步运行时 | asyncio（单线程） | tokio（worker=2） |
| 日志 | logging + RotatingFileHandler | tracing + 自实现轮转 |
| 单实例 | win32event + mutex | windows crate Mutex/Event |
| 注册表自启 | winreg | winreg 0.55 |
| 二维码 | qrcode + PIL | qrcode 0.14 + GDI 直渲 |
| 剪贴板 | pyperclip | arboard 3 |
| 版本号单源 | server.py APP_VERSION | version.rs APP_VERSION |
| release 优化 | PyInstaller + UPX | opt-level=z + lto=fat + codegen-units=1 + strip=symbols |

## 结论

Rust 版全部量化目标达标，USS 从 25.54 MB 降至 1.78 MB（降幅 93%），包体从 30.12 MB 降至 2.1 MB（降幅 93%）。
