//! # 显示服务管理器（服务层）
//!
//! UI 与热键/游戏检测等上层组件通过 `DisplayManager` 操作显示器：
//! - 维护每个显示器的基线（启动时的原始 LUT）与当前参数；
//! - 合并参数生成 LUT 并实时下发（Gamma 模拟路径）；
//! - 亮度优先 DDC/CI，不支持时回退 Gamma 模拟；
//! - 原始 LUT 持久化备份与退出恢复（含崩溃恢复）。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::RwLock;

use anyhow::{Context, Result};
use screen_tune_common::DisplayParams;
use screen_tune_common::MonitorInfo;
use tracing::{debug, info, trace, warn};

use crate::backend::{DisplayBackend, MonitorHandle, GAMMA_RAMP_LEN};
use crate::lut::build_ramp;
use crate::persist;

/// 单个显示器的运行状态
#[derive(Debug)]
pub struct MonitorState {
    /// 平台句柄
    pub handle: MonitorHandle,
    /// 显示器信息
    pub info: MonitorInfo,
    /// 当前生效参数
    pub params: DisplayParams,
    /// 基线（启动时原始 LUT，所有曲线基于它构建）
    baseline: [u16; GAMMA_RAMP_LEN],
    /// DDC 基线亮度（退出恢复用；None 表示不支持 DDC）
    ddc_baseline_brightness: Option<u32>,
}

impl MonitorState {
    /// 显示器 id
    pub fn id(&self) -> &str {
        &self.info.id
    }
}

/// 显示服务管理器
pub struct DisplayManager {
    backend: Box<dyn DisplayBackend>,
    /// 显示器状态表（monitor id → 状态）
    monitors: RwLock<HashMap<String, MonitorState>>,
    /// 原始 LUT 备份目录
    backup_dir: PathBuf,
}

impl DisplayManager {
    /// 创建管理器并完成初始化：崩溃恢复 → 读取基线 → 持久化基线
    pub fn init(backend: Box<dyn DisplayBackend>, backup_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&backup_dir)
            .with_context(|| format!("创建备份目录失败: {}", backup_dir.display()))?;
        let manager = Self {
            backend,
            monitors: RwLock::new(HashMap::new()),
            backup_dir,
        };
        manager.refresh_monitors()?;
        Ok(manager)
    }

    /// （重新）枚举显示器：新增显示器走完整初始化，消失的显示器恢复基线
    pub fn refresh_monitors(&self) -> Result<()> {
        let enumerated = self.backend.list_monitors().context("枚举显示器失败")?;

        let mut monitors = self.monitors.write().unwrap();
        let mut alive: Vec<String> = Vec::new();

        for (handle, info) in enumerated {
            alive.push(info.id.clone());
            if monitors.contains_key(&info.id) {
                continue;
            }
            // 崩溃恢复：若存在残留备份，先写回原始 LUT（上次异常退出的残留）
            if persist::has_baseline(&self.backup_dir, &info.id) {
                if let Some(baseline) = persist::load_baseline(&self.backup_dir, &info.id) {
                    info!("发现异常退出残留，恢复显示器 {} 的原始 LUT", info.id);
                    let _ = self.backend.set_gamma_ramp(&handle, &baseline);
                }
                persist::remove_baseline(&self.backup_dir, &info.id);
            }

            // 读取当前 LUT 作为基线，并持久化
            let baseline = self
                .backend
                .get_gamma_ramp(&handle)
                .with_context(|| format!("读取显示器 {} 的 Gamma Ramp 失败", info.id))?;
            if let Err(e) = persist::save_baseline(&self.backup_dir, &info.id, &baseline) {
                warn!("保存 LUT 备份失败: {e:#}");
            }

            // 记录 DDC 基线亮度
            let ddc_baseline_brightness = self
                .backend
                .get_ddc_brightness(&handle)
                .ok()
                .flatten()
                .map(|(cur, _)| cur);

            debug!(
                "显示器 {}（{}）已就绪：DDC 亮度基线 {:?}",
                info.name, info.id, ddc_baseline_brightness
            );
            monitors.insert(
                info.id.clone(),
                MonitorState {
                    handle,
                    info,
                    params: DisplayParams::default(),
                    baseline,
                    ddc_baseline_brightness,
                },
            );
        }

        // 消失的显示器：恢复其基线（尽力而为）并移除
        let dead: Vec<String> = monitors
            .keys()
            .filter(|id| !alive.contains(id))
            .cloned()
            .collect();
        for id in dead {
            if let Some(state) = monitors.remove(&id) {
                debug!("显示器 {} 已断开，恢复原始 LUT", state.info.name);
                let _ = self.backend.set_gamma_ramp(&state.handle, &state.baseline);
                persist::remove_baseline(&self.backup_dir, &id);
            }
        }
        Ok(())
    }

    /// 当前全部显示器快照（(信息, 当前参数)）
    pub fn snapshot(&self) -> Vec<(MonitorInfo, DisplayParams)> {
        let monitors = self.monitors.read().unwrap();
        let mut v: Vec<_> = monitors
            .values()
            .map(|m| (m.info.clone(), m.params))
            .collect();
        v.sort_by(|a, b| a.0.id.cmp(&b.0.id));
        v
    }

    /// 单个显示器的当前参数
    pub fn params_of(&self, monitor_id: &str) -> Option<DisplayParams> {
        self.monitors
            .read()
            .unwrap()
            .get(monitor_id)
            .map(|m| m.params)
    }

    /// 向单个显示器下发一组参数（实时生效）
    pub fn set_params(&self, monitor_id: &str, params: DisplayParams) -> Result<()> {
        let params = params.clamped();
        let (handle, baseline, supports_ddc, ddc_base) = {
            let monitors = self.monitors.read().unwrap();
            let m = monitors
                .get(monitor_id)
                .with_context(|| format!("显示器不存在: {monitor_id}"))?;
            (
                m.handle.clone(),
                m.baseline,
                m.info.supports_ddc,
                m.ddc_baseline_brightness,
            )
        };

        // 亮度：DDC 显示器走硬件（VCP 0x10），LUT 不再叠加亮度因子；
        // 非 DDC 显示器走 Gamma 模拟。
        let lut_params = if supports_ddc {
            let mut p = params;
            p.brightness = screen_tune_common::consts::BRIGHTNESS_DEFAULT;
            p
        } else {
            params
        };

        let ramp = build_ramp(&baseline, &lut_params);
        self.backend
            .set_gamma_ramp(&handle, &ramp)
            .with_context(|| format!("写入 Gamma Ramp 失败: {monitor_id}"))?;

        if supports_ddc {
            let target = params.brightness.round().clamp(0.0, 100.0) as u32;
            // 与基线相同则无需写（避免不必要的 DDC 通信）
            if ddc_base.map(|b| b != target).unwrap_or(true) {
                self.backend
                    .set_ddc_brightness(&handle, target)
                    .with_context(|| format!("设置 DDC 亮度失败: {monitor_id}"))?;
            }
        }

        let mut monitors = self.monitors.write().unwrap();
        if let Some(m) = monitors.get_mut(monitor_id) {
            m.params = params;
        }
        trace!("显示器 {monitor_id} 参数已应用: {:?}", params);
        Ok(())
    }

    /// 向全部显示器同步下发同一组参数
    pub fn apply_to_all(&self, params: DisplayParams) -> Result<()> {
        let ids: Vec<String> = self
            .monitors
            .read()
            .unwrap()
            .values()
            .map(|m| m.id().to_string())
            .collect();
        for id in ids {
            self.set_params(&id, params)?;
        }
        Ok(())
    }

    /// 读取指定显示器的 DDC 亮度（(当前, 最大)）；不支持 DDC 时返回 Ok(None)
    pub fn ddc_brightness(&self, monitor_id: &str) -> Result<Option<(u32, u32)>> {
        let monitors = self.monitors.read().unwrap();
        let handle = monitors
            .get(monitor_id)
            .with_context(|| format!("显示器不存在: {monitor_id}"))?
            .handle
            .clone();
        drop(monitors);
        self.backend.get_ddc_brightness(&handle)
    }

    /// 恢复默认参数（全部显示器）
    pub fn restore_default(&self) -> Result<()> {
        self.apply_to_all(DisplayParams::default())
    }

    /// 恢复指定显示器的原始 LUT（DDC 亮度一并还原）
    pub fn restore_baseline(&self, monitor_id: &str) -> Result<()> {
        let (handle, baseline, ddc_base) = {
            let monitors = self.monitors.read().unwrap();
            let m = monitors
                .get(monitor_id)
                .with_context(|| format!("显示器不存在: {monitor_id}"))?;
            (m.handle.clone(), m.baseline, m.ddc_baseline_brightness)
        };
        self.backend
            .set_gamma_ramp(&handle, &baseline)
            .with_context(|| format!("恢复 Gamma Ramp 失败: {monitor_id}"))?;
        if let Some(base) = ddc_base {
            let _ = self.backend.set_ddc_brightness(&handle, base);
        }
        persist::remove_baseline(&self.backup_dir, monitor_id);
        let mut monitors = self.monitors.write().unwrap();
        if let Some(m) = monitors.get_mut(monitor_id) {
            m.params = DisplayParams::default();
        }
        Ok(())
    }

    /// 退出清理：全部显示器恢复原始 LUT 并删除备份
    pub fn shutdown(&self) {
        info!("正在恢复全部显示器的原始 LUT…");
        let ids: Vec<String> = self
            .monitors
            .read()
            .unwrap()
            .values()
            .map(|m| m.id().to_string())
            .collect();
        for id in ids {
            if let Err(e) = self.restore_baseline(&id) {
                warn!("退出恢复显示器 {id} 失败: {e:#}");
            }
        }
        info!("原始 LUT 已恢复");
    }
}
