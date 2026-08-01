//! # ScreenTune 配置 crate
//!
//! 职责：
//! 1. 定义全局配置（`AppConfig`）与配置方案（`Profile`）的数据结构；
//! 2. 提供 `ConfigStore` 完成 config.json / profiles/*.json 的读写、
//!    方案的导入导出与校验。
//!
//! 本 crate 只做「数据 + 文件 IO」，不包含任何显示 / 热键业务逻辑。

pub mod app_config;
pub mod game_rule;
pub mod hotkey;
pub mod profile;
pub mod store;

pub use app_config::{AppConfig, Language, ThemePref, WindowConfig};
pub use game_rule::GameRule;
pub use hotkey::{HotkeyAction, HotkeyBinding};
pub use profile::Profile;
pub use store::ConfigStore;
