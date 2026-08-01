//! # Win32 显示后端
//!
//! 全部 Windows 官方 API 调用集中于此文件，严格封装在 `DisplayBackend` trait 之后，
//! 服务层与 UI 层绝不直接触碰 Win32。
//!
//! 用到的官方 API：
//! - `EnumDisplayMonitors` / `GetMonitorInfoW`：枚举显示器、获取设备名
//! - `CreateDCW` + `GetDeviceGammaRamp` / `SetDeviceGammaRamp`：读写 Gamma Ramp
//! - `GetPhysicalMonitorsFromHMONITOR` + `SetVCPFeature` 等：DDC/CI 亮度控制

#![cfg(windows)]

use std::ffi::c_void;
use std::mem::size_of;

use anyhow::{Context, Result};
use screen_tune_common::MonitorInfo;
use tracing::{debug, trace, warn};

use windows::core::PCWSTR;
use windows::Win32::Devices::Display::{
    DestroyPhysicalMonitor, GetNumberOfPhysicalMonitorsFromHMONITOR,
    GetPhysicalMonitorsFromHMONITOR, GetVCPFeatureAndVCPFeatureReply, SetVCPFeature,
    PHYSICAL_MONITOR,
};
use windows::Win32::Foundation::{BOOL, LPARAM, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateDCW, DeleteDC, EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORENUMPROC,
    MONITORINFOEXW, MONITORINFOF_PRIMARY,
};
use windows::Win32::UI::ColorSystem::{GetDeviceGammaRamp, SetDeviceGammaRamp};

use crate::backend::{DisplayBackend, MonitorHandle, GAMMA_RAMP_LEN};

/// DDC/CI VCP 亮度码
const MC_BRIGHTNESS: u8 = 0x10;
/// DDC/CI VCP 对比度码
const MC_CONTRAST: u8 = 0x12;

/// Win32 后端（无内部状态；每次操作实时获取句柄，天然免疫句柄失效问题）
pub struct Win32Backend;

impl Win32Backend {
    /// 构造 Win32 后端
    pub fn new() -> Self {
        Self
    }
}

impl Default for Win32Backend {
    fn default() -> Self {
        Self::new()
    }
}

/// Win32 显示器平台数据（跨线程传递用裸值存储句柄）
#[derive(Debug, Clone)]
pub(crate) struct Win32Data {
    /// HMONITOR 的原始值（跨线程安全地保存句柄）
    hmonitor_raw: usize,
    /// 显示器设备名（如 `\\.\DISPLAY1`），用于 CreateDCW
    device_name: String,
}

impl Win32Data {
    /// 还原 HMONITOR
    fn hmonitor(&self) -> HMONITOR {
        HMONITOR(self.hmonitor_raw as *mut c_void)
    }
}

impl DisplayBackend for Win32Backend {
    fn list_monitors(&self) -> Result<Vec<(MonitorHandle, MonitorInfo)>> {
        // -----------------------------------------------------------
        // 1. 枚举全部 HMONITOR
        // -----------------------------------------------------------
        let mut hmonitors: Vec<HMONITOR> = Vec::new();
        let result = unsafe {
            EnumDisplayMonitors(
                None,
                None,
                Some(enum_monitor_proc),
                &mut hmonitors as *mut Vec<HMONITOR> as usize as LPARAM,
            )
        };
        if !result.as_bool() {
            anyhow::bail!("EnumDisplayMonitors 失败");
        }
        trace!("检测到 {} 个显示器", hmonitors.len());

        // -----------------------------------------------------------
        // 2. 逐个提取信息
        // -----------------------------------------------------------
        let mut out = Vec::new();
        for (idx, hmonitor) in hmonitors.into_iter().enumerate() {
            let Some((device_name, is_primary, width, height)) = query_monitor_info(hmonitor)
            else {
                warn!("无法读取显示器 {} 的信息，跳过", idx);
                continue;
            };

            // 物理显示器描述（DDC 探测的副产品）；失败则用通用名
            let mut supports_ddc = false;
            let mut description = String::new();
            if let Ok(phys) = physical_monitors(hmonitor) {
                if let Some(first) = phys.first() {
                    // 探测亮度 VCP：能读到当前值说明支持 DDC/CI
                    let mut current = 0u32;
                    let mut max = 0u32;
                    let ok = unsafe {
                        GetVCPFeatureAndVCPFeatureReply(
                            first.hPhysicalMonitor,
                            MC_BRIGHTNESS,
                            None,
                            &mut current,
                            Some(&mut max),
                        )
                    };
                    supports_ddc = ok != 0;
                    description = wide_string(&first.szPhysicalMonitorDescription);
                }
                // 释放物理显示器句柄
                for pm in &phys {
                    let _ = unsafe { DestroyPhysicalMonitor(pm.hPhysicalMonitor) };
                }
            }

            let name = if description.trim().is_empty() {
                format!("显示器 {}", idx + 1)
            } else {
                description.trim().to_string()
            };

            debug!(
                "显示器 {}: 设备名 {} 主屏 {} DDC {} {}x{}",
                idx + 1,
                device_name,
                is_primary,
                supports_ddc,
                width,
                height
            );

            let info =
                MonitorInfo::new(&device_name, name, is_primary, supports_ddc, width, height);
            let handle = MonitorHandle::new(
                device_name.clone(),
                Box::new(Win32Data {
                    hmonitor_raw: hmonitor.0 as usize,
                    device_name,
                }),
            );
            out.push((handle, info));
        }
        Ok(out)
    }

    fn get_gamma_ramp(&self, handle: &MonitorHandle) -> Result<[u16; GAMMA_RAMP_LEN]> {
        let data = handle
            .data
            .as_any()
            .downcast_ref::<Win32Data>()
            .context("句柄类型不匹配")?;
        let mut ramp = [0u16; GAMMA_RAMP_LEN];
        with_device_dc(&data.device_name, |hdc| {
            let ok = unsafe { GetDeviceGammaRamp(hdc, ramp.as_mut_ptr() as *mut c_void) };
            if !ok.as_bool() {
                anyhow::bail!("GetDeviceGammaRamp 失败: {}", data.device_name);
            }
            Ok(())
        })?;
        Ok(ramp)
    }

    fn set_gamma_ramp(&self, handle: &MonitorHandle, ramp: &[u16; GAMMA_RAMP_LEN]) -> Result<()> {
        let data = handle
            .data
            .as_any()
            .downcast_ref::<Win32Data>()
            .context("句柄类型不匹配")?;
        with_device_dc(&data.device_name, |hdc| {
            let ok = unsafe { SetDeviceGammaRamp(hdc, ramp.as_ptr() as *const c_void) };
            if !ok.as_bool() {
                anyhow::bail!("SetDeviceGammaRamp 失败: {}", data.device_name);
            }
            Ok(())
        })
    }

    fn set_ddc_brightness(&self, handle: &MonitorHandle, value: u32) -> Result<Option<()>> {
        let data = handle
            .data
            .as_any()
            .downcast_ref::<Win32Data>()
            .context("句柄类型不匹配")?;
        with_first_physical(data.hmonitor(), |pm| {
            let ok = unsafe { SetVCPFeature(pm.hPhysicalMonitor, MC_BRIGHTNESS, value) };
            if ok != 0 {
                Ok(())
            } else {
                anyhow::bail!("SetVCPFeature(亮度) 失败");
            }
        })
    }

    fn set_ddc_contrast(&self, handle: &MonitorHandle, value: u32) -> Result<Option<()>> {
        let data = handle
            .data
            .as_any()
            .downcast_ref::<Win32Data>()
            .context("句柄类型不匹配")?;
        with_first_physical(data.hmonitor(), |pm| {
            let ok = unsafe { SetVCPFeature(pm.hPhysicalMonitor, MC_CONTRAST, value) };
            if ok != 0 {
                Ok(())
            } else {
                anyhow::bail!("SetVCPFeature(对比度) 失败");
            }
        })
    }

    fn get_ddc_brightness(&self, handle: &MonitorHandle) -> Result<Option<(u32, u32)>> {
        let data = handle
            .data
            .as_any()
            .downcast_ref::<Win32Data>()
            .context("句柄类型不匹配")?;
        with_first_physical(data.hmonitor(), |pm| {
            let mut current = 0u32;
            let mut max = 0u32;
            let ok = unsafe {
                GetVCPFeatureAndVCPFeatureReply(
                    pm.hPhysicalMonitor,
                    MC_BRIGHTNESS,
                    None,
                    &mut current,
                    Some(&mut max),
                )
            };
            if ok != 0 {
                Ok((current, max))
            } else {
                anyhow::bail!("GetVCPFeatureAndVCPFeatureReply(亮度) 失败");
            }
        })
    }

    fn get_ddc_contrast(&self, handle: &MonitorHandle) -> Result<Option<(u32, u32)>> {
        let data = handle
            .data
            .as_any()
            .downcast_ref::<Win32Data>()
            .context("句柄类型不匹配")?;
        with_first_physical(data.hmonitor(), |pm| {
            let mut current = 0u32;
            let mut max = 0u32;
            let ok = unsafe {
                GetVCPFeatureAndVCPFeatureReply(
                    pm.hPhysicalMonitor,
                    MC_CONTRAST,
                    None,
                    &mut current,
                    Some(&mut max),
                )
            };
            if ok != 0 {
                Ok((current, max))
            } else {
                anyhow::bail!("GetVCPFeatureAndVCPFeatureReply(对比度) 失败");
            }
        })
    }
}

// ---------------------------------------------------------------
// 内部辅助函数
// ---------------------------------------------------------------

/// EnumDisplayMonitors 回调：收集 HMONITOR 到传入的 Vec
unsafe extern "system" fn enum_monitor_proc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _lprc_clip: *mut RECT,
    dwdata: LPARAM,
) -> BOOL {
    // 通过 LPARAM 传回 Vec 指针（调用方生命周期内有效）
    let monitors = dwdata.0 as *mut Vec<HMONITOR>;
    if !monitors.is_null() {
        (*monitors).push(hmonitor);
    }
    BOOL(1)
}

/// 查询单个显示器的（设备名、是否主屏、宽、高）
fn query_monitor_info(hmonitor: HMONITOR) -> Option<(String, bool, u32, u32)> {
    let mut mi = MONITORINFOEXW {
        monitorInfo: windows::Win32::Graphics::Gdi::MONITORINFO {
            cbSize: size_of::<MONITORINFOEXW>() as u32,
            rcMonitor: RECT::default(),
            rcWork: RECT::default(),
            dwFlags: 0,
        },
        szDevice: [0u16; 32],
    };
    let ok = unsafe {
        GetMonitorInfoW(
            hmonitor,
            &mut mi as *mut MONITORINFOEXW as *mut windows::Win32::Graphics::Gdi::MONITORINFO,
        )
    };
    if !ok.as_bool() {
        return None;
    }
    let device_name = wide_string(&mi.szDevice);
    if device_name.is_empty() {
        return None;
    }
    let rc = mi.monitorInfo.rcMonitor;
    let is_primary = mi.monitorInfo.dwFlags & MONITORINFOF_PRIMARY != 0;
    let width = (rc.right - rc.left).max(0) as u32;
    let height = (rc.bottom - rc.top).max(0) as u32;
    Some((device_name, is_primary, width, height))
}

/// 在设备 DC 生命周期内执行闭包（自动创建与释放）
fn with_device_dc<T>(device_name: &str, f: impl FnOnce(HDC) -> Result<T>) -> Result<T> {
    let device_wide: Vec<u16> = device_name
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect();
    let hdc = unsafe {
        CreateDCW(
            PCWSTR::null(),
            PCWSTR::from_raw(device_wide.as_ptr()),
            PCWSTR::null(),
            None,
        )
    };
    if hdc.0.is_null() {
        anyhow::bail!("CreateDCW 失败: {device_name}");
    }
    let result = f(hdc);
    unsafe {
        DeleteDC(hdc);
    }
    result
}

/// 获取显示器的物理显示器列表（调用方负责销毁）
fn physical_monitors(hmonitor: HMONITOR) -> Result<Vec<PHYSICAL_MONITOR>> {
    let mut count = 0u32;
    unsafe {
        GetNumberOfPhysicalMonitorsFromHMONITOR(hmonitor, &mut count)
            .context("GetNumberOfPhysicalMonitorsFromHMONITOR 失败")?;
    }
    if count == 0 {
        return Ok(Vec::new());
    }
    let mut vec = vec![PHYSICAL_MONITOR::default(); count as usize];
    unsafe {
        GetPhysicalMonitorsFromHMONITOR(hmonitor, &mut vec)
            .context("GetPhysicalMonitorsFromHMONITOR 失败")?;
    }
    Ok(vec)
}

/// 对显示器的第一个物理显示器执行闭包（自动获取并销毁句柄）
fn with_first_physical<T>(
    hmonitor: HMONITOR,
    f: impl FnOnce(&PHYSICAL_MONITOR) -> Result<T>,
) -> Result<Option<T>> {
    let monitors = physical_monitors(hmonitor)?;
    let Some(first) = monitors.first() else {
        return Ok(None);
    };
    let result = f(first);
    for pm in &monitors {
        let _ = unsafe { DestroyPhysicalMonitor(pm.hPhysicalMonitor) };
    }
    result.map(Some)
}

/// 宽字符缓冲区 → String（截断于第一个 NUL）
fn wide_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len])
}
