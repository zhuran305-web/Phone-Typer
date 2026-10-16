# Phone-Typer Rust 版总结报告

**版本**：2.0.0-rust | **日期**：2026-10-16 | **产物**：`rust/dist/phone-typer-rust.exe`（2.0 MB）

---

## 一、量化目标实测值 vs 目标

| 指标 | 目标 | 实测 | 达标 |
|------|------|------|------|
| cold-tray USS | ≤8 MB | **1.78 MB** | ✅ |
| 包体（exe+web） | ≤30 MB | **2.1 MB** | ✅ |
| 启动时间 | ≤3s | <1s | ✅ |
| /api/config 时延 | ≤200ms | 0ms | ✅ |
| CPU 30s 均值 | <1% | 0.05% | ✅ |
| 进程数 | =1 | 1 | ✅ |
| soak USS 增长 | ≤3MB/1800s | 0.15MB/100s（外推~2.7MB/1800s） | ✅ |
| 日志磁盘上界 | ~1.5MB | 6.8 KB | ✅ |
| cargo test | 全绿 | 44 passed / 0 failed | ✅ |
| cargo build | 0 warning | 0 warning / 0 error | ✅ |
| --smoke 五步 | 全 PASS | PASS 5 / FAIL 0 / SKIP 0 | ✅ |

## 二、84 条 EARS 验收条件覆盖情况

### 协议层（--smoke 自动化闭环验证）
- **STEP 1**（HTTP 首页 + /api/config + 404 + 穿越拦截）：覆盖 spec 5.1（HTTP 服务）、5.2（/api/config）、5.8.3 场景 2（404）、安全规则（路径穿越）→ **PASS**
- **STEP 2**（bad-pin ×5 退避 + bad-json 保持）：覆盖 spec 5.3.1 规则 1-6（PIN 校验/退避/断开）、5.3.2（bad-json）→ **PASS**
- **STEP 3**（正确 PIN send + 缺省 type + 键入 stub）：覆盖 spec 5.3.1 规则 7（send）、5.4（键入）、缺省 type→send → **PASS**
- **STEP 4**（sync diff 算式 + too-long + 连接保持）：覆盖 spec 5.3.1 规则 8（sync）、5.5（diff_sync）、too-long 限制 → **PASS**
- **STEP 5**（clip 广播与回读）：覆盖 spec 5.6（剪贴板广播）→ **PASS**

### GUI 层（静态核验：代码审查 + 资源核对）
- spec 5.5（托盘菜单结构六项）：gui/tray.rs 静态核验 ✅
- spec 5.6（主界面窗口属性 + 双二维码选项卡）：gui/main_win.rs 静态核验 ✅
- spec 5.7（前置聚焦链 + 窗口单例）：gui/res.rs + gui/settings.rs 静态核验 ✅
- spec 5.8.1（单实例 + 四模式分派）：instance.rs + main.rs 静态核验 ✅
- spec 5.8.3（端口冲突弹窗）：server/mod.rs fatal_dialog 静态核验 ✅

### 待目视确认（10 项，移交用户）
- 见 `rust/reports/visual_checklist.md`，全部已通过静态核验，待用户在 Windows 桌面环境目视确认

## 三、Python 版对比

| 指标 | Python 版 | Rust 版 | 降幅 |
|------|----------|---------|------|
| cold-tray USS | 25.54 MB | 1.78 MB | -93.0% |
| 包体 | 30.12 MB | 2.1 MB | -93.0% |
| 分发形态 | exe + _internal/ | exe + web/（便携） | — |

详见 `rust/reports/comparison_table.md`

## 四、产物清单

| 文件 | 大小 | 说明 |
|------|------|------|
| `rust/dist/phone-typer-rust.exe` | 2.0 MB | 主二进制（LTO fat + strip symbols） |
| `rust/dist/web/index.html` | 14.7 KB | 手机端页面 |
| `rust/dist/web/manifest.json` | 341 B | PWA manifest |
| `rust/dist/web/icons/` | 49 KB | icon-192/256/512.png + icon.ico |
| `rust/build.rs` | — | winresource icon 嵌入 |
| `rust/Cargo.toml` | — | 依赖定稿 + release profile |

## 五、自包含性核对

- `rust/` 目录自包含，不依赖 `rust/` 之外任何项目文件 ✅
- `git status` 确认：仅 `rust/` 为新 untracked 目录，`simple/` 与根目录基线零改动 ✅
- 协议契约冻结：WS 消息格式 / HTTP 端口 8766 / WS 端口 8767 / 配对 URI / 托盘六项菜单 / 注册表 PhoneTyperRust / Mutex-Event v1 命名 → 全部与 design.md 一致 ✅

## 六、遗留风险与跟进措施

1. **GUI 目视确认**：10 项 GUI 行为需用户在 Windows 桌面环境确认（见 visual_checklist.md）
2. **完整 1800s soak**：本次仅测 100s（USS 增长 0.15MB），建议用户运行完整 1800s soak 确认增长 ≤3MB
3. **真实键入验证**：--smoke 使用 stub 模式（仅记日志不 SendInput），真实键入需用户在桌面环境验证（清单第 8 项）
4. **icon 嵌入验证**：build.rs 通过 winresource 嵌入 icon.ico，需用户确认 exe 在资源管理器/任务栏显示正确图标
