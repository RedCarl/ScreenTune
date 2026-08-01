//! # ScreenTune 界面 crate
//!
//! 分层：UI（本 crate）→ Service（各 crate 的 Manager / AppCore）→ Display → Win32。
//! 本 crate 绝不直接调用 Win32 API；所有平台能力都经由服务层 trait 访问。
//!
//! 组成：
//! - `app`      eframe 主应用（窗口布局、事件轮询、页面调度）
//! - `core`     服务组合层（AppCore：集成显示/方案/热键/托盘/自启/游戏检测）
//! - `theme`    Fluent 风格主题与中文字体加载
//! - `i18n`     中英双语字典
//! - `pages`    各导航页面

pub mod app;
pub mod core;
pub mod i18n;
pub mod pages;
pub mod theme;

pub use app::{create, native_options, ScreenTuneApp};
pub use core::{AppCore, AppEvent};
pub use i18n::Tr;
