//! # 导航页面
//!
//! 各页面均以「纯函数 + 显式状态」风格编写：页面状态由调用方持有
//! （ScreenTuneApp 或页面专用 state），服务访问一律经 `AppCore`。

pub mod display_page;
pub mod hotkeys_page;
pub mod monitors_page;
pub mod profiles_page;
pub mod settings_page;

pub use hotkeys_page::HotkeyEditState;
pub use monitors_page::MonitorsPageState;
pub use profiles_page::ProfilePageState;
pub use settings_page::SettingsPageState;
