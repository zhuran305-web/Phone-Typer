# 文台（Wentai）

Phone-Typer 配套 Android 客户端：把手机当成电脑的无线键盘，打字实时同步到电脑光标处，并接收电脑端推送的剪贴板文本。

- 应用名：**文台**
- 包名：`com.phonetyper.wentai`
- 版本：`1.0.0`（versionCode 1）
- 最低支持：Android 8.0（minSdk 26），targetSdk 34

> 后台策略：**后台静默、不保活**。切到后台不显示通知、不创建前台服务、不做任何保活；后台期间连接可能被系统断开属预期，回到前台立即重连并对齐。后台期间不接收电脑端推送。

## 功能

- 扫码 / 相册 / 手动三种配对方式（`ptyper://connect` 配对码，缺 ws 端口自动经 `/api/config` 补全）
- 实时同步打字（输入法组合态抑制 + 150ms 防抖，全文上行为电脑端做 diff 键入）
- 整段发送（与实时同步模式互斥）
- 电脑 → 手机剪贴板接收、高亮提示与一键复制
- 前台断线自动重连（≥3 秒退避、网络恢复即连、PIN 错误熔断）
- 深色界面、简体中文、专属图标

## 目录结构

```text
wentai/
├─ wentai-icon-fg-1024.png   图标素材
├─ build.bat                 Windows 一键构建入口
├─ scripts/build.py          工具链检测 + 图标生成 + Gradle 出包 + 产物收集
├─ tools/gen_icons.py        由素材生成各密度图标
├─ gradle/                   Gradle Wrapper 配置与版本目录
├─ settings.gradle.kts / build.gradle.kts
├─ app/                      应用模块（Kotlin 源码与资源）
└─ dist/                     构建产物（APK 集中输出）
```

## 构建

### 依赖

- **JDK 17+**（设置 `JAVA_HOME`）
- **Android SDK**（设置 `ANDROID_HOME` 或 `ANDROID_SDK_ROOT`），需 `platforms;android-34` 与 `build-tools`
- **Gradle 8.7**（或补全 `gradle/wrapper/gradle-wrapper.jar` 后使用 Wrapper）
- 生成图标需要 **Python 3 + Pillow**（`pip install Pillow`）

### 一键构建

```bat
build.bat
```

脚本会：检测工具链 → 校验图标素材 → 生成各密度图标 → 调用 Gradle `assembleRelease` → 把 APK 收集到 `wentai/dist/wentai-1.0.0.apk`。

工具链缺失时脚本会**中止构建**并输出缺失清单与安装指引；图标素材缺失时会提示素材路径。所有产物只落在 `wentai/` 目录内。

### 手动构建

```bash
pip install Pillow
python tools/gen_icons.py
gradle copyReleaseApk      # 或 ./gradlew copyReleaseApk
```

## 安装使用

1. 电脑端运行 `PhoneTyper.exe`，右键托盘 →「显示主界面」，找到 **App 配对二维码**
2. 安装 `wentai/dist/wentai-1.0.0.apk`
3. 打开文台 →「配对设置」→「扫码配对」扫描该二维码（或手动填写主机地址、端口、PIN）
4. 手机与电脑连**同一 WiFi**；连接成功后即可实时同步打字

## 与电脑端协议

完全复用 Phone-Typer 现有协议，电脑端无需升级：

- 上行：`{"pin":..,"type":"sync"|"send","text":..}`
- 下行：`{"ok":true,"bs":..,"typed":..,"s":..}` / `{"ok":false,"err":"bad-pin"|"bad-json"}` / `{"type":"clip","text":..}`
- 补全/版本：`GET /api/config` → `{"ws_port":..,"version":..}`

## 版本说明

| 版本 | 说明 |
|------|------|
| 1.0.0 | 首个版本：配对、实时同步、整段发送、剪贴板接收、前台重连、后台静默 |
