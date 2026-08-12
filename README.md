# ScreenTune

[![CI](https://github.com/RedCarl/ScreenTune/actions/workflows/ci.yml/badge.svg)](https://github.com/RedCarl/ScreenTune/actions/workflows/ci.yml)
[![Release](https://github.com/RedCarl/ScreenTune/actions/workflows/release.yml/badge.svg)](https://github.com/RedCarl/ScreenTune/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Latest Release](https://img.shields.io/github/v/release/RedCarl/ScreenTune)](https://github.com/RedCarl/ScreenTune/releases/latest)

> **Instant Display Control for Gamers**

一款专注于 Windows 平台的专业显示增强工具：通过快捷键一键调整屏幕 **Gamma（灰度）、亮度、对比度、颜色饱和度、色温**，适用于 FPS 游戏玩家、设计师与日常办公用户。

| 特性 | 说明 |
| --- | --- |
| 🪶 原生 Windows | Rust 编写，极低资源占用（启动 <200ms、空闲 CPU ≈ 0%、内存 <50MB） |
| 🔒 隐私友好 | 无需联网、无广告、无遥测 |
| 🖥️ 多显示器 | 每台显示器独立调节，可一键全部同步，记住每台配置 |
| 🎮 游戏自动切换 | Rust / CS2 / PUBG 等游戏启动自动套用方案，退出恢复默认（规则可扩展） |
| ⌨️ 全局快捷键 | 系统级生效，游戏内也可用；支持自定义与冲突检测 |
| 🗂️ 配置方案 | 内置 默认 / Rust / CS2 / PUBG / 办公 / 夜间，支持 JSON 导入导出 |
| 🧪 稳定性 | 原始 LUT 备份 + 退出恢复 + 崩溃恢复，显示器永远能还原出厂画面 |

## 截图

> 📸 截图占位 —— 待发布后补充实际运行截图。

```
┌──────────────────────────────────────────────────────────────┐
│ ScreenTune │ 显示器：Dell U2720Q ★      [同步到全部显示器]      │
├────────────┼──────────────────────────────────────────────────┤
│ ▸ 显示调节  │  Gamma      ──●──────────────  115  %            │
│   配置方案  │  亮度       ────●────────────  82   DDC/CI 直控   │
│   全局快捷键 │  对比度     ──●──────────────  108  %            │
│   设置     │  饱和度     ────●────────────  125  %            │
│   显示器   │  色温       ───●─────────────  5800 K            │
│            │                                        [恢复默认]  │
├────────────┴──────────────────────────────────────────────────┤
│ v0.1.1 · MIT · 无遥测 · 无广告 · 无需联网                       │
└──────────────────────────────────────────────────────────────┘
```

## 安装

### 方式一：GitHub Release（推荐）

前往 [Releases](https://github.com/RedCarl/ScreenTune/releases) 下载最新版：

- `ScreenTune.exe` — 单文件可执行程序，双击即用，无需安装
- `ScreenTune-<版本>.zip` — 压缩包
- 下载后建议用附带的 `.sha256` 校验文件完整性

> 系统要求：Windows 10/11 64 位，.NET 无需安装（纯原生程序）。

### 方式二：从源码构建（macOS / Linux 开发环境）

```bash
# 质量门禁（fmt + clippy + test）
scripts/build.sh

# 仅快速检查
scripts/build.sh --fast
```

Windows 可执行程序由 **GitHub Actions 自动构建**（见下文 CI 说明），无需本地 Windows。

## 功能说明

### 1. Gamma（50 ~ 150，默认 100）

基于 `SetDeviceGammaRamp` 自己计算 LUT（256 点 × 3 通道），实时生效、毫秒级响应。

**关键保障：**

- 启动时保存每台显示器的**原始 LUT**（含出厂校准），所有曲线都基于它叠加；
- 退出 / 崩溃后自动恢复原始画面；
- 多显示器各自维护独立 LUT。

### 2. 亮度（0 ~ 100）

- 显示器支持 **DDC/CI** 时优先直控硬件（VCP 0x10），对画质零影响；
- 不支持时自动回退 **Gamma 模拟**。

### 3. 对比度（50 ~ 150）

以中灰（0.5）为中心缩放，实时生效。

### 4. 饱和度（0 ~ 200%，默认 100%）

模拟 **NVIDIA Digital Vibrance** 风格：

- **当前实现**：Gamma Ramp 上的逐通道 Vibrance 曲线（中灰锚定的三次 S 曲线），兼容 AMD / Intel / NVIDIA 全部显卡，无厂商依赖；
- **Color Matrix 数学层**：完整 3×3 饱和度矩阵（Rec.709）已实现并通过单元测试，为未来接入 **GPU Shader** 或 NVIDIA 官方 API 预留了无缝替换路径（见 `crates/display/src/color_matrix.rs`）。

> 说明：`SetDeviceGammaRamp` 的 LUT 逐通道独立，物理上无法执行跨通道矩阵运算，故 Gamma 模拟路径使用上述近似；显卡厂商 API 路径可直接复用矩阵层。

### 5. 色温（2500K ~ 10000K）

基于 **Neil Bartlett 黑体辐射近似算法**（f.lux 同款），以 6500K（D65）为中性基准，6500K 时零偏色。

### 6. 显示器管理

- 每台显示器独立调节、记住配置（`config.json` 中按显示器 id 保存）；
- 「同步到全部显示器」一键统一；
- 支持热插拔（手动刷新）。

### 7. 配置方案

内置方案：**默认 / Rust / CS2 / PUBG / 办公 / 夜间**（参数可改）。

每个方案保存 Gamma、亮度、对比度、饱和度、色温、快捷键；支持 **JSON 导入 / 导出**（导出到剪贴板，导入从文本粘贴）。

### 8. 全局快捷键

默认绑定（可修改，冲突自动检测并提示）：

| 快捷键 | 动作 |
| --- | --- |
| `Ctrl+Alt+1` | 恢复默认 |
| `Ctrl+Alt+2` | 应用 Rust 方案 |
| `Ctrl+Alt+3` | 应用 CS2 方案 |
| `Ctrl+Alt+4` | 应用 PUBG 方案 |

### 9. 系统托盘

关闭窗口默认最小化到托盘（可配置）。托盘菜单：

打开 / 恢复默认 / 各配置方案 / 开机自动启动（勾选）/ 检查更新（预留）/ 退出

### 10. 开机自启

写入 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`，无需管理员权限。

### 11. 游戏自动切换

轮询进程列表（默认 2 秒一次，极低开销），命中规则自动应用方案，进程退出自动恢复默认。规则表存于 `config.json`，新增游戏只需添加一行规则（如 `minecraft.exe → 办公`）。

### 12. 配置

`config.json`（`%APPDATA%\ScreenTune\`）保存：窗口大小 / 位置、语言、主题、快捷键、游戏规则、每台显示器参数、最后使用方案、日志级别等，全部自动保存。

## 开发

### 项目结构

```text
screen-tune/
├── Cargo.toml                  # Workspace 根清单（统一依赖版本）
├── crates/
│   ├── app/                    # 主程序入口：组装全部服务、日志、退出清理
│   ├── ui/                     # egui/eframe 界面：Fluent 深色主题、中英双语、页面
│   ├── display/                # 显示引擎：LUT 数学、DDC/CI、多显示器、基线备份
│   ├── profile/                # 配置方案管理（内置方案、CRUD、导入导出）
│   ├── hotkey/                 # 全局快捷键（global-hotkey 封装、冲突检测）
│   ├── tray/                   # 系统托盘（tray-icon 封装）
│   ├── startup/                # 开机自启（注册表）
│   ├── game_detector/          # 游戏进程检测与自动切换状态机
│   ├── config/                 # 配置/方案数据结构与持久化
│   └── common/                 # 公共类型与常量
├── scripts/
│   ├── build.sh                # 本地质量门禁
│   └── gen_icon.py             # 占位图标生成（纯标准库）
├── assets/                     # 图标资源（占位，可自行替换）
└── .github/workflows/
    ├── ci.yml                  # fmt / clippy / test / Windows 构建
    └── release.yml             # tag 触发 → 自动 Release
```

### 架构分层

严格分层，UI 永不直接触碰 Win32：

```text
UI (egui)  →  Service (AppCore / DisplayManager)  →  Backend Trait  →  Win32 API
                ↑
        hotkey / tray / startup / game_detector（各自 Manager + Trait）
```

所有平台能力均有 **Trait 抽象**（`DisplayBackend`、`StartupBackend`、`ProcessLister`），Windows 上为真实实现，macOS 与 CI 环境为 Mock 实现——**macOS 上可完整运行与测试全部逻辑**。

### 技术栈

| 领域 | 选择 |
| --- | --- |
| 语言 | Rust Stable |
| GUI | egui + eframe（Fluent 风格自绘主题） |
| Windows API | windows crate（官方绑定） |
| 热键 / 托盘 | global-hotkey / tray-icon（tauri 生态，活跃维护） |
| 序列化 | serde / serde_json |
| 日志 | tracing + tracing-subscriber + tracing-appender |
| 错误处理 | anyhow（全局无 panic 路径） |
| 异步 | tokio（仅游戏检测轻量任务） |
| 目录 | directories |

## 构建

### 本地（macOS / Linux）

```bash
cargo check --workspace        # 快速检查（含全部单元测试编译）
cargo test --workspace         # 单元测试
```

> macOS 上 Win32 后端不参与编译，运行 GUI 时使用 Mock 显示器后端，可完整调试界面与业务逻辑。

### Windows（GitHub Actions 自动构建）

推送到 GitHub 后，CI 自动完成：

```text
Push → fmt --check → clippy (-D warnings) → cargo test → Windows release build
     → ScreenTune.exe + ScreenTune.zip + SHA256 → Artifact
```

创建发布版本：

```bash
git tag v1.0.0
git push origin v1.0.0
```

即自动构建并在 [Releases](https://github.com/RedCarl/ScreenTune/releases) 发布。

## 贡献指南

1. Fork 本仓库并创建功能分支；
2. 提交前确保通过本地质量门禁：`scripts/build.sh`；
3. 提交信息使用 Conventional Commits 风格（`feat:` / `fix:` / `docs:` 等）；
4. 发起 Pull Request，CI 全绿后即可合并；
5. 新增界面文案请同时更新 `crates/ui/src/i18n.rs` 的中英文典（两表 key 必须一致，有测试守护）。

## Roadmap

- [ ] **HDR** 支持（Windows HDR API）
- [ ] **ICC Profile** 管理与加载
- [ ] **Night Light** 定时色温（时间驱动）
- [ ] **AMD / Intel Color** 官方 API 接入
- [ ] **NVIDIA API**（官方开放后接入，饱和度走 Color Matrix 真实路径）
- [ ] **Auto HDR**
- [ ] **命令行模式**（`screen-tune --profile rust`）
- [ ] **插件系统**
- [ ] **Lua 脚本**
- [ ] **自动更新**
- [ ] 国际化更多语言（当前中 / 英）

## License

[MIT](LICENSE)

Copyright (c) 2026 ScreenTune Authors
