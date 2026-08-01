//! # 显示后端抽象
//!
//! `DisplayBackend` trait 是「服务层」与「平台 API」之间的唯一桥梁。
//! Windows 上由 `Win32Backend` 实现，其他平台 / 测试使用 `MockBackend`。
//! 所有 Win32 调用都被封装在 `win32.rs` 中，禁止越过本 trait 直接调用系统 API。

use anyhow::Result;
use screen_tune_common::MonitorInfo;

/// Gamma Ramp 原始缓冲区长度（3 通道 × 256 项）
pub const GAMMA_RAMP_LEN: usize = 1536;

/// 平台私有数据盒（支持 Any 下行转换与深拷贝）
pub(crate) trait HandleData: std::any::Any + Send + Sync {
    /// 深拷贝自身（MonitorHandle 需要 Clone，但 Box<dyn Any> 无法自动克隆）
    fn clone_box(&self) -> Box<dyn HandleData>;
    /// 下行转换为 Any（供后端取回具体类型；仅 Windows 后端使用）
    #[cfg_attr(not(windows), allow(dead_code))]
    fn as_any(&self) -> &dyn std::any::Any;
}

impl<T: std::any::Any + Send + Sync + Clone> HandleData for T {
    fn clone_box(&self) -> Box<dyn HandleData> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// 显示器平台句柄：包含枚举阶段获取的全部平台数据，
/// 由后端自己解释（win32 后端内部持有 HMONITOR 与设备名等）。
pub struct MonitorHandle {
    /// 与 `MonitorInfo::id` 一致：Windows 为 `\\.\DISPLAY1`，mock 为 `mock-N`
    pub id: String,
    /// 平台私有数据（后端内部使用；跨后端实现互不可见）
    pub(crate) data: Box<dyn HandleData>,
}

impl std::fmt::Debug for MonitorHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MonitorHandle")
            .field("id", &self.id)
            .finish()
    }
}

impl Clone for MonitorHandle {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            data: self.data.clone_box(),
        }
    }
}

impl MonitorHandle {
    /// 由后端构造句柄
    pub fn new<T: std::any::Any + Send + Sync + Clone>(id: String, data: T) -> Self {
        Self {
            id,
            data: Box::new(data),
        }
    }
}

/// 显示后端抽象
pub trait DisplayBackend: Send + Sync {
    /// 枚举当前全部显示器
    fn list_monitors(&self) -> Result<Vec<(MonitorHandle, MonitorInfo)>>;

    /// 读取指定显示器的当前 Gamma Ramp（1536 项，R/G/B 各 256）
    fn get_gamma_ramp(&self, handle: &MonitorHandle) -> Result<[u16; GAMMA_RAMP_LEN]>;

    /// 写入指定显示器的 Gamma Ramp
    fn set_gamma_ramp(&self, handle: &MonitorHandle, ramp: &[u16; GAMMA_RAMP_LEN]) -> Result<()>;

    /// 通过 DDC/CI 设置硬件亮度（0..100）。返回 `None` 表示该显示器不支持 DDC。
    fn set_ddc_brightness(&self, handle: &MonitorHandle, value: u32) -> Result<Option<()>>;

    /// 通过 DDC/CI 设置硬件对比度（0..100）
    fn set_ddc_contrast(&self, handle: &MonitorHandle, value: u32) -> Result<Option<()>>;

    /// 通过 DDC/CI 读取当前亮度（current, max）
    fn get_ddc_brightness(&self, handle: &MonitorHandle) -> Result<Option<(u32, u32)>>;

    /// 通过 DDC/CI 读取当前对比度（current, max）
    fn get_ddc_contrast(&self, handle: &MonitorHandle) -> Result<Option<(u32, u32)>>;
}
