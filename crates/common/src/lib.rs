//! # ScreenTune 公共基础库
//!
//! 本 crate 承载所有子 crate 共享的纯数据类型与常量，
//! 不包含任何平台相关代码，保证全平台可编译、可测试。

pub mod consts;
pub mod display_params;
pub mod monitor;

pub use display_params::DisplayParams;
pub use monitor::MonitorInfo;
