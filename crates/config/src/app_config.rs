//! # 全局配置
//!
//! 对应 `config.json`。所有字段均为可选语义（使用 `#[serde(default)]`），
//! 保证旧版本配置文件缺失字段时依然可以正常加载。

use screen_tune_common::DisplayParams;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::{GameRule, HotkeyBinding};

/// 界面语言
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// 简体中文
    #[default]
    Zh,
    /// English
    En,
}

/// 主题偏好（当前仅深色为主打，预留浅色）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemePref {
    /// 深色（Fluent Dark，默认）
    #[default]
    Dark,
    /// 浅色（Fluent Light）
    Light,
}

/// 主窗口几何信息（启动时恢复窗口位置与大小）
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct WindowConfig {
    /// 窗口宽（逻辑像素）
    pub width: f32,
    /// 窗口高（逻辑像素）
    pub height: f32,
    /// 窗口位置（逻辑像素，屏幕坐标系；None 表示由系统决定）
    pub position: Option<(f32, f32)>,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            width: 1020.0,
            height: 680.0,
            position: None,
        }
    }
}

/// 全局配置根结构
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// 窗口几何
    pub window: WindowConfig,
    /// 界面语言
    pub language: Language,
    /// 主题
    pub theme: ThemePref,
    /// 关闭窗口时最小化到托盘（而非退出）
    pub close_to_tray: bool,
    /// 开机自启是否启用
    pub startup_enabled: bool,
    /// 最后使用的配置方案 id（启动时自动应用；None 表示恢复默认参数）
    pub last_profile_id: Option<String>,
    /// 全局快捷键绑定表
    pub hotkeys: Vec<HotkeyBinding>,
    /// 游戏自动切换规则表
    pub game_rules: Vec<GameRule>,
    /// 游戏自动切换总开关
    pub game_detection_enabled: bool,
    /// 游戏进程轮询间隔（秒）
    pub game_poll_interval_secs: u64,
    /// 按显示器记住的参数（显示器 id → 参数）
    pub monitor_params: HashMap<String, DisplayParams>,
    /// 日志级别（`trace` / `debug` / `info` / `warn` / `error`）
    pub log_level: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window: WindowConfig::default(),
            language: Language::Zh,
            theme: ThemePref::Dark,
            close_to_tray: true,
            startup_enabled: false,
            last_profile_id: None,
            hotkeys: default_hotkeys(),
            game_rules: default_game_rules(),
            game_detection_enabled: true,
            game_poll_interval_secs: 2,
            monitor_params: HashMap::new(),
            log_level: "info".to_string(),
        }
    }
}

/// 默认快捷键：Ctrl+Alt+1..4 恢复默认 / Rust / CS2 / PUBG
pub fn default_hotkeys() -> Vec<HotkeyBinding> {
    vec![
        HotkeyBinding::new(
            "restore_default",
            "Ctrl+Alt+1",
            crate::HotkeyAction::RestoreDefault,
        ),
        HotkeyBinding::new(
            "profile:rust",
            "Ctrl+Alt+2",
            crate::HotkeyAction::ApplyProfile {
                profile_id: "rust".into(),
            },
        ),
        HotkeyBinding::new(
            "profile:cs2",
            "Ctrl+Alt+3",
            crate::HotkeyAction::ApplyProfile {
                profile_id: "cs2".into(),
            },
        ),
        HotkeyBinding::new(
            "profile:pubg",
            "Ctrl+Alt+4",
            crate::HotkeyAction::ApplyProfile {
                profile_id: "pubg".into(),
            },
        ),
    ]
}

/// 默认游戏自动切换规则
pub fn default_game_rules() -> Vec<GameRule> {
    vec![
        GameRule::new("rule:rust", "rust.exe", "rust"),
        GameRule::new("rule:cs2", "cs2.exe", "cs2"),
        GameRule::new("rule:pubg", "PUBG.exe", "pubg"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 默认配置满足全部约束：快捷键表非空、轮询间隔合理、日志级别合法
    #[test]
    fn default_config_is_valid() {
        let c = AppConfig::default();
        assert_eq!(c.hotkeys.len(), 4);
        assert_eq!(c.game_rules.len(), 3);
        assert!(c.game_poll_interval_secs >= 1);
        assert!(["trace", "debug", "info", "warn", "error"].contains(&c.log_level.as_str()));
    }

    /// 缺失字段的旧版 config.json 必须能成功反序列化
    #[test]
    fn serde_missing_fields_tolerated() {
        let partial = r#"{"window":{"width":800.0,"height":600.0,"position":null}}"#;
        let c: AppConfig = serde_json::from_str(partial).unwrap();
        assert!(c.close_to_tray);
        assert_eq!(c.hotkeys.len(), 4);
    }
}
