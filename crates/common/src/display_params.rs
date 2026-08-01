//! # 显示器显示参数
//!
//! 描述一个显示器（或一套配置方案）的全部可调参数：
//! Gamma / 亮度 / 对比度 / 饱和度 / 色温。
//! 该类型同时用于「当前显示器状态」与「配置方案」，二者共享同一份数据。

use serde::{Deserialize, Serialize};

use crate::consts::*;

/// 一组完整的显示参数（全部为「出厂无调整」时取默认值）
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DisplayParams {
    /// Gamma 曲线强度，范围 [50, 150]，默认 100（恒等曲线）
    pub gamma: f32,
    /// 亮度，范围 [0, 100]，默认 100（优先 DDC/CI，不支持时 Gamma 模拟）
    pub brightness: f32,
    /// 对比度，范围 [50, 150]，默认 100
    pub contrast: f32,
    /// 饱和度，范围 [0, 200]，默认 100（模拟 NVIDIA Digital Vibrance）
    pub saturation: f32,
    /// 色温（开尔文），范围 [2500, 10000]，默认 6500（D65，无偏色）
    pub temperature_k: u32,
}

impl Default for DisplayParams {
    fn default() -> Self {
        Self {
            gamma: GAMMA_DEFAULT,
            brightness: BRIGHTNESS_DEFAULT,
            contrast: CONTRAST_DEFAULT,
            saturation: SATURATION_DEFAULT,
            temperature_k: TEMPERATURE_DEFAULT_K,
        }
    }
}

impl DisplayParams {
    /// 将所有字段钳制到合法范围（配置 / UI 输入不可信时调用）
    pub fn clamped(mut self) -> Self {
        self.gamma = self.gamma.clamp(GAMMA_MIN, GAMMA_MAX);
        self.brightness = self.brightness.clamp(BRIGHTNESS_MIN, BRIGHTNESS_MAX);
        self.contrast = self.contrast.clamp(CONTRAST_MIN, CONTRAST_MAX);
        self.saturation = self.saturation.clamp(SATURATION_MIN, SATURATION_MAX);
        self.temperature_k = self
            .temperature_k
            .clamp(TEMPERATURE_MIN_K, TEMPERATURE_MAX_K);
        self
    }

    /// 是否全部为默认值（等价于「无任何调整」）
    pub fn is_default(&self) -> bool {
        const EPS: f32 = 1e-3;
        (self.gamma - GAMMA_DEFAULT).abs() < EPS
            && (self.brightness - BRIGHTNESS_DEFAULT).abs() < EPS
            && (self.contrast - CONTRAST_DEFAULT).abs() < EPS
            && (self.saturation - SATURATION_DEFAULT).abs() < EPS
            && self.temperature_k == TEMPERATURE_DEFAULT_K
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 默认参数必须全部在合法范围内，且判定为「无调整」
    #[test]
    fn default_params_are_valid_and_neutral() {
        let p = DisplayParams::default();
        assert!(p.is_default());
        assert_eq!(p, p.clamped());
    }

    /// 越界值经 clamped 后必须回到合法范围
    #[test]
    fn clamped_restores_bounds() {
        let p = DisplayParams {
            gamma: 999.0,
            brightness: -10.0,
            contrast: 0.0,
            saturation: 500.0,
            temperature_k: 1,
        };
        let c = p.clamped();
        assert_eq!(c.gamma, GAMMA_MAX);
        assert_eq!(c.brightness, BRIGHTNESS_MIN);
        assert_eq!(c.contrast, CONTRAST_MIN);
        assert_eq!(c.saturation, SATURATION_MAX);
        assert_eq!(c.temperature_k, TEMPERATURE_MIN_K);
    }

    /// serde 序列化往返必须无损
    #[test]
    fn serde_roundtrip() {
        let p = DisplayParams {
            gamma: 115.5,
            brightness: 82.0,
            contrast: 108.0,
            saturation: 125.0,
            temperature_k: 5800,
        };
        let json = serde_json::to_string(&p).unwrap();
        let back: DisplayParams = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
    }
}
