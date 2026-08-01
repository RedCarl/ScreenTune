//! # 快捷键绑定
//!
//! 定义「全局快捷键 → 动作」的绑定关系，以字符串形式持久化
//! （如 `Ctrl+Alt+1`），保证 config.json 可读性好、版本兼容。

use serde::{Deserialize, Serialize};

/// 快捷键可触发的动作
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HotkeyAction {
    /// 恢复默认显示参数
    RestoreDefault,
    /// 应用指定配置方案
    ApplyProfile { profile_id: String },
    /// 显示主窗口
    ShowWindow,
}

impl HotkeyAction {
    /// 返回人类可读的动作描述键（供 UI 本地化展示）
    pub fn description_key(&self) -> &'static str {
        match self {
            HotkeyAction::RestoreDefault => "hotkey.action.restore_default",
            HotkeyAction::ApplyProfile { .. } => "hotkey.action.apply_profile",
            HotkeyAction::ShowWindow => "hotkey.action.show_window",
        }
    }
}

/// 一条快捷键绑定
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotkeyBinding {
    /// 绑定唯一标识（如 `restore_default`、`profile:rust`）
    pub id: String,
    /// 快捷键字符串（如 `Ctrl+Alt+1`），由 hotkey crate 解析
    pub spec: String,
    /// 绑定动作
    pub action: HotkeyAction,
}

impl HotkeyBinding {
    /// 构造一条绑定
    pub fn new(id: impl Into<String>, spec: impl Into<String>, action: HotkeyAction) -> Self {
        Self {
            id: id.into(),
            spec: spec.into(),
            action,
        }
    }
}
