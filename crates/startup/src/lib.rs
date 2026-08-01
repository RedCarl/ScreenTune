//! # 开机自启
//!
//! Windows：写入 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`，
//! 键名 `ScreenTune`，值为当前可执行文件路径。
//! 写入 HKCU 无需管理员权限，且仅影响当前用户。
//!
//! 其他平台：内存 mock（no-op），保证跨平台编译与测试。

use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};

/// 开机自启后端抽象
pub trait StartupBackend: Send + Sync {
    /// 当前是否已启用开机自启
    fn is_enabled(&self) -> bool;
    /// 设置开机自启状态
    fn set_enabled(&self, enabled: bool) -> Result<()>;
}

/// 创建平台默认的自启后端
pub fn default_startup_backend() -> Box<dyn StartupBackend> {
    #[cfg(windows)]
    {
        Box::new(win32::Win32Startup::new())
    }
    #[cfg(not(windows))]
    {
        Box::new(MockStartup::default())
    }
}

/// 内存 mock：用于开发调试与单元测试（全平台可用）
#[derive(Debug, Default)]
pub struct MockStartup {
    enabled: AtomicBool,
}

impl StartupBackend for MockStartup {
    fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }

    fn set_enabled(&self, enabled: bool) -> Result<()> {
        self.enabled.store(enabled, Ordering::Relaxed);
        Ok(())
    }
}

/// Win32 实现（注册表）
#[cfg(windows)]
pub mod win32 {
    use super::*;
    use tracing::{debug, warn};
    use windows::core::PCWSTR;
    use windows::Win32::System::Registry::{
        RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
        HKEY_CURRENT_USER, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_SAM_FLAGS, REG_SZ, REG_VALUE_TYPE,
    };

    /// Run 注册表项路径
    const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
    /// 注册表值名
    const VALUE_NAME: &str = "ScreenTune";

    /// Win32 自启后端
    pub struct Win32Startup;

    impl Win32Startup {
        /// 构造后端
        pub fn new() -> Self {
            Self
        }
    }

    impl Default for Win32Startup {
        fn default() -> Self {
            Self::new()
        }
    }

    impl StartupBackend for Win32Startup {
        fn is_enabled(&self) -> bool {
            // 打开 Run 键
            let Some(key) = open_run_key(KEY_QUERY_VALUE) else {
                return false;
            };
            // 查询值大小（仅判断存在性）
            let name: Vec<u16> = VALUE_NAME
                .encode_utf16()
                .chain(std::iter::once(0))
                .collect();
            let mut data_len = 0u32;
            let status = unsafe {
                RegQueryValueExW(
                    key,
                    PCWSTR::from_raw(name.as_ptr()),
                    None,
                    None,
                    None,
                    Some(&mut data_len),
                )
            };
            let _ = unsafe { RegCloseKey(key) };
            status.0 == 0
        }

        fn set_enabled(&self, enabled: bool) -> Result<()> {
            if enabled {
                // 写入当前可执行文件路径（UTF-16 + 结尾 NUL）
                let exe = std::env::current_exe()
                    .map_err(|e| anyhow::anyhow!("获取可执行文件路径失败: {e}"))?;
                let mut data: Vec<u8> = exe
                    .to_string_lossy()
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .flat_map(u16::to_le_bytes)
                    .collect();

                let key = open_run_key(KEY_SET_VALUE)
                    .ok_or_else(|| anyhow::anyhow!("无法打开 Run 注册表键"))?;
                let name: Vec<u16> = VALUE_NAME
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                let status = unsafe {
                    RegSetValueExW(
                        key,
                        PCWSTR::from_raw(name.as_ptr()),
                        None,
                        REG_VALUE_TYPE::REG_SZ,
                        Some(&mut data),
                    )
                };
                let _ = unsafe { RegCloseKey(key) };
                if status.0 != 0 {
                    anyhow::bail!("写入注册表失败（错误码 {}）", status.0);
                }
                debug!("已启用开机自启: {}", exe.display());
            } else {
                let Some(key) = open_run_key(KEY_SET_VALUE) else {
                    return Ok(()); // 键不存在则无需删除
                };
                let name: Vec<u16> = VALUE_NAME
                    .encode_utf16()
                    .chain(std::iter::once(0))
                    .collect();
                let status = unsafe { RegDeleteValueW(key, PCWSTR::from_raw(name.as_ptr())) };
                let _ = unsafe { RegCloseKey(key) };
                if status.0 != 0 && status.0 != 2 {
                    // 错误码 2 = 值不存在（视为已关闭）
                    warn!("删除注册表值失败（错误码 {}）", status.0);
                }
                debug!("已禁用开机自启");
            }
            Ok(())
        }
    }

    /// 打开 Run 键（带权限），失败返回 None
    fn open_run_key(access: REG_SAM_FLAGS) -> Option<HKEY> {
        let path: Vec<u16> = RUN_KEY.encode_utf16().chain(std::iter::once(0)).collect();
        let mut key = HKEY::default();
        let status = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER,
                PCWSTR::from_raw(path.as_ptr()),
                0,
                access,
                &mut key,
            )
        };
        if status.0 != 0 {
            warn!("RegOpenKeyExW 失败（错误码 {}）", status.0);
            return None;
        }
        Some(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mock 后端开关行为正确（全平台可测）
    #[test]
    fn mock_startup_toggles() {
        let backend = MockStartup::default();
        assert!(!backend.is_enabled());
        backend.set_enabled(true).unwrap();
        assert!(backend.is_enabled());
        backend.set_enabled(false).unwrap();
        assert!(!backend.is_enabled());
    }
}
