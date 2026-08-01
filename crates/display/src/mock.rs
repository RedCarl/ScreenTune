//! # Mock 显示后端
//!
//! 全平台可用的内存后端：模拟 2 台显示器（一台支持 DDC、一台不支持），
//! 用于 macOS 开发调试与无显示器环境的单元测试。
//! 所有写操作记录在内存中，可通过 `state()` 断言验证。

use std::collections::HashMap;
use std::sync::RwLock;

use anyhow::Result;
use screen_tune_common::MonitorInfo;

use crate::backend::{DisplayBackend, MonitorHandle, GAMMA_RAMP_LEN};
use crate::lut::identity_ramp;

/// Mock 显示器数量
const MOCK_MONITOR_COUNT: usize = 2;

/// Mock 后端内部状态（内存「显示器」）
#[derive(Debug, Default)]
struct MockState {
    /// monitor id → 当前 Gamma Ramp
    ramps: HashMap<String, [u16; GAMMA_RAMP_LEN]>,
    /// monitor id → (亮度, 对比度)（DDC 值）
    ddc: HashMap<String, (u32, u32)>,
}

/// Mock 后端
pub struct MockBackend {
    state: RwLock<MockState>,
}

impl MockBackend {
    /// 构造 Mock 后端
    pub fn new() -> Self {
        let mut ramps = HashMap::new();
        let mut ddc = HashMap::new();
        for i in 0..MOCK_MONITOR_COUNT {
            ramps.insert(format!("mock-{i}"), identity_ramp());
            ddc.insert(format!("mock-{i}"), (80, 50));
        }
        Self {
            state: RwLock::new(MockState { ramps, ddc }),
        }
    }

    /// 读取内部状态快照（供测试断言）
    pub fn snapshot(&self) -> (Vec<[u16; GAMMA_RAMP_LEN]>, Vec<(u32, u32)>) {
        let s = self.state.read().unwrap();
        let ramps = (0..MOCK_MONITOR_COUNT)
            .map(|i| s.ramps[&format!("mock-{i}")])
            .collect();
        let ddc = (0..MOCK_MONITOR_COUNT)
            .map(|i| s.ddc[&format!("mock-{i}")])
            .collect();
        (ramps, ddc)
    }
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

/// Mock 显示器平台数据（仅用于标识，无实际内容）
#[derive(Debug, Clone)]
struct MockData;

impl DisplayBackend for MockBackend {
    fn list_monitors(&self) -> Result<Vec<(MonitorHandle, MonitorInfo)>> {
        let mut out = Vec::new();
        for i in 0..MOCK_MONITOR_COUNT {
            let id = format!("mock-{i}");
            // mock-0 支持 DDC；mock-1 不支持（用于测试回退路径）
            let supports_ddc = i == 0;
            let info = MonitorInfo::new(
                &id,
                format!("Mock Monitor {}", i + 1),
                i == 0,
                supports_ddc,
                1920 + i as u32 * 640,
                1080,
            );
            let handle = MonitorHandle::new(id, MockData);
            out.push((handle, info));
        }
        Ok(out)
    }

    fn get_gamma_ramp(&self, handle: &MonitorHandle) -> Result<[u16; GAMMA_RAMP_LEN]> {
        let s = self.state.read().unwrap();
        Ok(*s.ramps.get(&handle.id).unwrap_or(&identity_ramp()))
    }

    fn set_gamma_ramp(&self, handle: &MonitorHandle, ramp: &[u16; GAMMA_RAMP_LEN]) -> Result<()> {
        let mut s = self.state.write().unwrap();
        s.ramps.insert(handle.id.clone(), *ramp);
        Ok(())
    }

    fn set_ddc_brightness(&self, handle: &MonitorHandle, value: u32) -> Result<Option<()>> {
        let mut s = self.state.write().unwrap();
        if let Some(entry) = s.ddc.get_mut(&handle.id) {
            entry.0 = value.clamp(0, 100);
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }

    fn set_ddc_contrast(&self, handle: &MonitorHandle, value: u32) -> Result<Option<()>> {
        let mut s = self.state.write().unwrap();
        if let Some(entry) = s.ddc.get_mut(&handle.id) {
            entry.1 = value.clamp(0, 100);
            Ok(Some(()))
        } else {
            Ok(None)
        }
    }

    fn get_ddc_brightness(&self, handle: &MonitorHandle) -> Result<Option<(u32, u32)>> {
        let s = self.state.read().unwrap();
        Ok(s.ddc.get(&handle.id).map(|(cur, _)| (*cur, 100)))
    }

    fn get_ddc_contrast(&self, handle: &MonitorHandle) -> Result<Option<(u32, u32)>> {
        let s = self.state.read().unwrap();
        Ok(s.ddc.get(&handle.id).map(|(_, cur)| (*cur, 100)))
    }
}
