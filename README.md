# Phone-Typer

> 手机打字 → 电脑自动键入。把手机当成电脑的无线键盘。

在手机上打字，内容实时输出到电脑当前光标处（Word、VS Code、浏览器、聊天框等），如同真键盘在打字。支持实时同步（随打随出）和整段发送两种模式，还支持电脑→手机反向推送剪贴板。

## 功能

- **手机→电脑**：手机打字实时同步到电脑光标处（支持中间编辑、删除回删）
- **电脑→手机**：电脑复制文本，托盘「发送剪贴板到手机」推送到手机端接收
- **双码配对**：网页版入口二维码 + App 配对二维码（`ptyper://` 协议）
- **系统托盘**：后台运行，无控制台窗口，托盘菜单操作
- **设置页**：托盘右键改 PIN，热生效无需重启
- **PWA**：手机浏览器「添加到桌面」如原生 App

## 下载使用

1. 下载 `PhoneTyper.exe`（见 [Releases](../../releases)）
2. 双击运行（首次如弹防火墙，点「允许访问」）
3. 手机与电脑连**同一 WiFi**
4. 右键托盘图标 →「显示主界面」→ 手机扫二维码，或浏览器访问 `http://电脑IP:8766`
5. 输入 PIN（默认 `1234`），打字即同步到电脑光标处

> **注意**：exe 可能被杀软误报，加白名单即可。向任务管理器等管理员窗口键入需以管理员身份运行 exe。

## 开发

### 从源码运行

```bash
pip install -r requirements.txt
python server.py            # 控制台模式（排障）
```

### 打包 exe

```bash
pip install pyinstaller
pyinstaller PhoneTyper.spec --noconfirm
# 产出 dist/PhoneTyper.exe（约 15MB）
```

### 配置

`config.json`（exe 旁，升级不丢）：

```json
{
  "http_port": 8766,
  "ws_port": 8767,
  "pin": "1234",
  "restore_clipboard": true,
  "paste_delay_ms": 60,
  "auto_enter": false
}
```

改端口/PIN 后重启 exe 生效；PIN 也可在托盘「设置」里改（热生效）。

## 技术栈

- **电脑端**：Python + websockets + pynput + pyperclip + pystray + Pillow + tkinter
- **手机端**：浏览器网页（PWA）
- **协议**：HTTP 8766 发页面 + WebSocket 8767 收发数据

## 限制

- 手机与电脑需同一 WiFi 局域网（路由器「AP 隔离」需关闭）
- 无法向管理员权限窗口键入（Windows UIPI 限制），需以管理员身份运行 exe
- 同步期间电脑光标需保持在输入位置

## License

[MIT](LICENSE)
