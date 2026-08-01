//! # ScreenTune 显示引擎
//!
//! 分层：`DisplayManager`（服务层）→ `DisplayBackend` trait（适配层）→ Win32 API。
//! UI 只允许通过 `DisplayManager` 的公开方法操作显示器，绝不直接触碰 Win32。
//!
//! 核心能力：
//! - 基于 `SetDeviceGammaRamp` 的多显示器 Gamma 调整（自己计算 LUT）；
//! - 亮度 / 对比度优先 DDC/CI（`SetVCPFeature`），不支持时回退 Gamma 模拟；
//! - 饱和度：Gamma 模拟（逐通道 Vibrance 曲线近似）+ ColorMatrix 数学层
//!   （为未来 GPU Shader / 厂商 API 预留替换路径）；
//! - 色温：CIE 黑体轨迹近似（Neil Bartlett 算法）；
//! - 原始 LUT 备份与退出恢复（含崩溃恢复）。

pub mod backend;
pub mod color_matrix;
pub mod lut;
pub mod manager;
pub mod mock;
pub mod persist;
pub mod temperature;

#[cfg(windows)]
pub mod win32;

pub use backend::{DisplayBackend, MonitorHandle};
pub use manager::DisplayManager;

/// 创建默认后端：Windows 使用真实 Win32 后端，其他平台使用 Mock 后端
/// （保证 macOS 开发环境与无显示器 CI 环境下程序可运行、可测试）。
pub fn default_backend() -> Box<dyn DisplayBackend> {
    #[cfg(windows)]
    {
        Box::new(win32::Win32Backend::new())
    }
    #[cfg(not(windows))]
    {
        Box::new(mock::MockBackend::new())
    }
}
