//! # 显示器信息
//!
//! 描述系统中一块物理/逻辑显示器。由 display crate 的后端枚举产生，
//! 供 UI 与配置（按显示器记住参数）使用。

use serde::{Deserialize, Serialize};

/// 显示器信息（不含平台句柄，可安全跨线程传递与持久化）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonitorInfo {
    /// 稳定标识（Windows 为设备名如 `\\.\DISPLAY1`；mock 为 `mock-N`）
    pub id: String,
    /// 用户可读名称（Windows 优先使用物理显示器描述，回退为「显示器 N」）
    pub name: String,
    /// 是否为主显示器
    pub is_primary: bool,
    /// 是否支持 DDC/CI（亮度/对比度可直接写硬件；否则走 Gamma 模拟）
    pub supports_ddc: bool,
    /// 显示器尺寸（宽 × 高，逻辑像素），用于 UI 展示
    pub width: u32,
    pub height: u32,
}

impl MonitorInfo {
    /// 构造一个新的显示器信息（供各后端实现使用）
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        is_primary: bool,
        supports_ddc: bool,
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            is_primary,
            supports_ddc,
            width,
            height,
        }
    }
}
