//! # 系统托盘
//!
//! 封装 tray-icon：构建带菜单的托盘图标、动态重建菜单、轮询菜单事件。
//! 菜单项：
//! 打开 / 恢复默认 / 各配置方案 / 开机启动（勾选）/ 检查更新（预留）/ 退出
//!
//! 非 Windows 平台为 no-op（保证 macOS 开发环境可编译运行）。

use anyhow::Result;
use screen_tune_config::Profile;
#[cfg(windows)]
use tracing::{debug, warn};

/// 托盘菜单事件（业务语义）
#[derive(Debug, Clone, PartialEq)]
pub enum TrayCommand {
    /// 显示主窗口
    ShowWindow,
    /// 恢复默认显示参数
    RestoreDefault,
    /// 应用指定方案
    ApplyProfile(String),
    /// 切换开机自启
    ToggleStartup,
    /// 退出应用
    Quit,
}

/// 托盘管理器
pub struct TrayManager {
    #[cfg(windows)]
    tray: Option<tray_icon::TrayIcon>,
    /// 当前开机自启状态（用于勾选展示）
    startup_enabled: bool,
    /// 当前方案列表（用于菜单重建）
    profiles: Vec<Profile>,
}

impl TrayManager {
    /// 创建空托盘管理器（构建前必须先调用 `rebuild_menu`）
    pub fn new() -> Self {
        Self {
            #[cfg(windows)]
            tray: None,
            startup_enabled: false,
            profiles: Vec::new(),
        }
    }

    /// 构建托盘图标并填充菜单
    pub fn rebuild_menu(&mut self, profiles: &[Profile], startup_enabled: bool) -> Result<()> {
        self.profiles = profiles.to_vec();
        self.startup_enabled = startup_enabled;

        #[cfg(windows)]
        {
            let menu = build_menu(profiles, startup_enabled);
            match &self.tray {
                Some(tray) => {
                    // tray-icon 0.24：set_menu 返回 ()
                    tray.set_menu(Some(Box::new(menu)));
                }
                None => {
                    let icon = build_icon()?;
                    let tray = tray_icon::TrayIconBuilder::new()
                        .with_id("screen-tune-tray")
                        .with_tooltip("ScreenTune - Instant Display Control for Gamers")
                        .with_icon(icon)
                        .with_menu(Box::new(menu))
                        .build()
                        .map_err(|e| anyhow::anyhow!("创建托盘图标失败: {e}"))?;
                    debug!("系统托盘已创建");
                    self.tray = Some(tray);
                }
            }
        }

        #[cfg(not(windows))]
        {
            // 非 Windows 平台不创建托盘（no-op）
            let _ = (profiles, startup_enabled);
        }
        Ok(())
    }

    /// 轮询托盘菜单事件
    pub fn poll_events(&mut self) -> Vec<TrayCommand> {
        #[cfg(windows)]
        {
            let mut commands = Vec::new();
            while let Ok(event) = tray_icon::menu::MenuEvent::receiver().try_recv() {
                let id = event.id.0.as_str();
                match id {
                    "open" => commands.push(TrayCommand::ShowWindow),
                    "restore" => commands.push(TrayCommand::RestoreDefault),
                    "startup" => commands.push(TrayCommand::ToggleStartup),
                    "quit" => commands.push(TrayCommand::Quit),
                    other => {
                        if let Some(profile_id) = other.strip_prefix("profile:") {
                            commands.push(TrayCommand::ApplyProfile(profile_id.to_string()));
                        } else {
                            warn!("未知托盘菜单项: {other}");
                        }
                    }
                }
            }
            commands
        }

        #[cfg(not(windows))]
        {
            Vec::new()
        }
    }
}

impl Default for TrayManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 构建托盘菜单
#[cfg(windows)]
fn build_menu(profiles: &[Profile], startup_enabled: bool) -> tray_icon::menu::Menu {
    use tray_icon::menu::{
        CheckMenuItemBuilder, Menu, MenuItem, MenuItemBuilder, PredefinedMenuItem,
    };

    let menu = Menu::new();

    let open = MenuItemBuilder::new()
        .id("open".into())
        .text("打开 ScreenTune")
        .build();
    let restore = MenuItemBuilder::new()
        .id("restore".into())
        .text("恢复默认")
        .build();
    let _ = menu.append_items(&[&open, &restore, &PredefinedMenuItem::separator()]);

    // 方案菜单项：id 形如 profile:rust
    for profile in profiles {
        let item = MenuItemBuilder::new()
            .id(format!("profile:{}", profile.id).into())
            .text(format!("应用方案：{}", profile.name))
            .build();
        let _ = menu.append(&item);
    }
    let _ = menu.append_items(&[&PredefinedMenuItem::separator()]);

    // 开机自启（勾选当前状态）
    let startup = CheckMenuItemBuilder::new()
        .id("startup".into())
        .text("开机自动启动")
        .checked(startup_enabled)
        .build();
    // 检查更新（预留功能，置灰）
    let check_updates = MenuItem::new("检查更新（敬请期待）", false, None);

    let _ = menu.append_items(&[&startup, &check_updates, &PredefinedMenuItem::separator()]);

    let quit = MenuItemBuilder::new()
        .id("quit".into())
        .text("退出")
        .build();
    let _ = menu.append(&quit);
    menu
}

/// 构建托盘图标（32×32 RGBA，程序内绘制，无需外部资源文件）
#[cfg(windows)]
fn build_icon() -> Result<tray_icon::Icon> {
    const SIZE: usize = 32;
    let mut rgba = vec![0u8; SIZE * SIZE * 4];

    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = (y * SIZE + x) * 4;
            let fx = x as f32 / (SIZE - 1) as f32;
            let fy = y as f32 / (SIZE - 1) as f32;

            // 对角渐变背景（深蓝 → 深灰），模拟 Fluent 深色风格
            let t = (fx * 0.7 + fy * 0.3).clamp(0.0, 1.0);
            let base_r = (31.0 + 40.0 * t) as u8;
            let base_g = (31.0 + 48.0 * t) as u8;
            let base_b = (43.0 + 66.0 * t) as u8;

            // 中心绘制三根「亮度条」（R/G/B 通道意象），蓝绿色渐变
            let cx = fx * 24.0 - 4.0;
            let in_bar = (4.0..28.0).contains(&(fx * 32.0)) && (8.0..24.0).contains(&(fy * 32.0));
            if in_bar {
                let bar = ((fx * 32.0 - 4.0) / 8.0) as usize;
                let level = 0.35 + 0.5 * fy;
                let (r, g, b) = match bar {
                    0 => (
                        0.9 * level * 255.0,
                        0.55 * level * 255.0,
                        0.35 * level * 255.0,
                    ),
                    1 => (
                        0.6 * level * 255.0,
                        0.75 * level * 255.0,
                        0.6 * level * 255.0,
                    ),
                    _ => (
                        0.4 * level * 255.0,
                        0.55 * level * 255.0,
                        0.95 * level * 255.0,
                    ),
                };
                rgba[i] = r as u8;
                rgba[i + 1] = g as u8;
                rgba[i + 2] = b as u8;
            } else {
                rgba[i] = base_r;
                rgba[i + 1] = base_g;
                rgba[i + 2] = base_b;
            }
            rgba[i + 3] = 255;
            let _ = cx;
        }
    }

    tray_icon::Icon::from_rgba(rgba, SIZE as u32, SIZE as u32)
        .map_err(|e| anyhow::anyhow!("生成托盘图标失败: {e}"))
}
