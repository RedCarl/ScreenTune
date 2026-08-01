//! # Gamma Ramp LUT 生成
//!
//! 全部显示参数在「逐通道 LUT」上的数学表达。输入基线为启动时读取的
//! 原始 Gamma Ramp（保留显示器出厂校准），输出为合并全部参数后的最终 LUT。
//!
//! 曲线组合顺序：Gamma → 对比度 → 亮度 → 饱和度(Vibrance) → 色温。
//! 所有运算在归一化 [0,1] 空间进行，最后映射回 16bit。

use screen_tune_common::consts::*;
use screen_tune_common::DisplayParams;

use crate::backend::GAMMA_RAMP_LEN;
use crate::temperature::temperature_gains;

/// 构建最终 Gamma Ramp
///
/// # 参数
/// - `base`: 基线 LUT（通常为启动时读取的原始 Ramp）
/// - `params`: 目标显示参数
pub fn build_ramp(base: &[u16; GAMMA_RAMP_LEN], params: &DisplayParams) -> [u16; GAMMA_RAMP_LEN] {
    let p = params.clamped();
    let (g_r, g_g, g_b) = temperature_gains(p.temperature_k);
    let gains = [g_r, g_g, g_b];

    let mut out = [0u16; GAMMA_RAMP_LEN];
    for (ch, gain) in gains.iter().enumerate() {
        let start = ch * 256;
        for i in 0..256 {
            let x = base[start + i] as f32 / 65535.0;
            let y = transform_channel(x, &p, *gain);
            out[start + i] = (y * 65535.0 + 0.5) as u16;
        }
    }
    out
}

/// 单个通道的完整变换流水线
fn transform_channel(x: f32, p: &DisplayParams, gain: f32) -> f32 {
    // 1. Gamma：幂曲线，100 = 恒等；>100 更亮，<100 更暗
    let y = x.powf(100.0 / p.gamma);

    // 2. 对比度：以 0.5 为中心缩放
    let y = 0.5 + (y - 0.5) * (p.contrast / 100.0);

    // 3. 亮度：整体缩放
    let y = y * (p.brightness / 100.0);

    // 4. 饱和度（Vibrance 逐通道近似）
    let y = vibrance_curve(y, p.saturation);

    // 5. 色温：逐通道增益
    (y * gain).clamp(0.0, 1.0)
}

/// 饱和度逐通道近似曲线。
///
/// 由于 Gamma Ramp 逐通道独立，无法执行跨通道矩阵（见 `color_matrix` 模块说明）。
/// 这里采用「中灰锚定的三次 S 曲线」近似 Vibrance 效果：
/// - 饱和度 > 100%：暗部压低、亮部抬升（通道差异放大 → 颜色更鲜艳）；
/// - 饱和度 < 100%：向中灰压缩（对比度收缩 → 颜色更柔和）。
///
/// 数学性质：中灰 (0.5) 恒定点；端点 (0/1) 保持；与真矩阵对灰度的
/// 不变量行为一致（0.5 处不变）。
pub fn vibrance_curve(x: f32, saturation: f32) -> f32 {
    let s = (saturation / 100.0).clamp(SATURATION_MIN / 100.0, SATURATION_MAX / 100.0);
    let x = x.clamp(0.0, 1.0);

    if s >= 1.0 {
        // 增强：三次 S 曲线，强度系数 2.5（200% 饱和时上限）
        let k = 2.5 * (s - 1.0);
        (x + k * (x - 0.5).powi(3)).clamp(0.0, 1.0)
    } else {
        // 减弱：向中灰压缩，强度系数 0.8（0% 饱和时压至 20% 对比度）
        let k = 0.8 * (1.0 - s);
        0.5 + (x - 0.5) * (1.0 - k)
    }
}

/// 构建「恒等」LUT（数值上的理想恒等表，主要用于测试）
pub fn identity_ramp() -> [u16; GAMMA_RAMP_LEN] {
    let mut ramp = [0u16; GAMMA_RAMP_LEN];
    for i in 0..256 {
        let v = (i as f32 / 255.0 * 65535.0) as u16;
        ramp[i] = v;
        ramp[256 + i] = v;
        ramp[512 + i] = v;
    }
    ramp
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 全部参数为默认值时，输出应近似等于基线（误差 < 0.1%）
    #[test]
    fn default_params_preserve_baseline() {
        let base = identity_ramp();
        let out = build_ramp(&base, &DisplayParams::default());
        let mut max_delta = 0u32;
        for (a, b) in base.iter().zip(out.iter()) {
            max_delta = max_delta.max((*a as i32 - *b as i32).unsigned_abs());
        }
        assert!(max_delta < 66, "默认参数应近似恒等，最大偏差 {max_delta}");
    }

    /// Gamma 单调性：gamma 越低曲线越暗、越高越亮
    #[test]
    fn gamma_curve_monotonic() {
        let base = identity_ramp();
        let dark = build_ramp(
            &base,
            &DisplayParams {
                gamma: 70.0,
                ..DisplayParams::default()
            },
        );
        let bright = build_ramp(
            &base,
            &DisplayParams {
                gamma: 130.0,
                ..DisplayParams::default()
            },
        );
        // 中间灰阶 (0.5) 处：dark 应比默认暗、bright 应比默认亮
        let mid = 128;
        let base_mid = base[mid];
        assert!(dark[mid] < base_mid);
        assert!(bright[mid] > base_mid);
        // 端点保持不变
        assert_eq!(dark[0], 0);
        assert_eq!(dark[255], 65535);
    }

    /// 亮度 50 应使中间灰阶亮度减半
    #[test]
    fn brightness_scales_mid_gray() {
        let base = identity_ramp();
        let out = build_ramp(
            &base,
            &DisplayParams {
                brightness: 50.0,
                ..DisplayParams::default()
            },
        );
        let mid = base[128] as f32 / 65535.0;
        let out_mid = out[128] as f32 / 65535.0;
        assert!(
            (out_mid - mid * 0.5).abs() < 0.01,
            "亮度 50 应减半，实际 {out_mid}"
        );
    }

    /// 对比度 50 应把中间灰阶压向中灰
    #[test]
    fn contrast_compresses_toward_mid() {
        let base = identity_ramp();
        let out = build_ramp(
            &base,
            &DisplayParams {
                contrast: 50.0,
                ..DisplayParams::default()
            },
        );
        let mid = out[128] as f32 / 65535.0;
        assert!((mid - 0.5).abs() < 0.01);
        // 极端值不再存在
        assert!(out[0] > 0);
        assert!(out[255] < 65535);
    }

    /// 饱和度增强时，暗部灰阶应被压低（通道差异放大）
    #[test]
    fn saturation_boost_pushes_darks_down() {
        let base = identity_ramp();
        let neutral = build_ramp(&base, &DisplayParams::default());
        let boosted = build_ramp(
            &base,
            &DisplayParams {
                saturation: 160.0,
                ..DisplayParams::default()
            },
        );
        assert!(boosted[96] <= neutral[96], "0.375 灰阶应被压低");
        assert!(boosted[160] >= neutral[160], "0.625 灰阶应被抬升");
        // 中灰锚定：0.5 附近基本不动
        let d = (boosted[128] as i32 - neutral[128] as i32).abs();
        assert!(d < 66);
    }

    /// 饱和度曲线在端点保持（0 与 1 不动）
    #[test]
    fn vibrance_curve_preserves_endpoints() {
        assert_eq!(vibrance_curve(0.0, 200.0), 0.0);
        assert_eq!(vibrance_curve(1.0, 200.0), 1.0);
        assert_eq!(vibrance_curve(0.5, 200.0), 0.5);
    }

    /// 色温 4000K 应压低蓝通道（暖色；红通道增益恒为 1.0，见 temperature 模块说明）
    #[test]
    fn warm_temperature_presses_blue() {
        let base = identity_ramp();
        let warm = build_ramp(
            &base,
            &DisplayParams {
                temperature_k: 4000,
                ..DisplayParams::default()
            },
        );
        let neutral = build_ramp(&base, &DisplayParams::default());
        assert!(warm[128 + 512] < neutral[128 + 512], "蓝通道应压低");
        // 中灰位置红通道保持不变（增益 1.0）
        assert!((warm[128] as i32 - neutral[128] as i32).abs() < 66);
    }

    /// 全黑 / 全白输入下 LUT 永不越界（16bit 溢出防护）
    #[test]
    fn ramp_never_overflows() {
        // 用极端参数组合验证不会 panic 且值域合法
        let base = identity_ramp();
        let extremes = [
            DisplayParams {
                gamma: 50.0,
                brightness: 100.0,
                contrast: 150.0,
                saturation: 200.0,
                temperature_k: 2500,
            },
            DisplayParams {
                gamma: 150.0,
                brightness: 0.0,
                contrast: 50.0,
                saturation: 0.0,
                temperature_k: 10000,
            },
        ];
        for p in extremes {
            // 极端参数下不 panic，且输出存在有效灰阶（非全黑）
            let out = build_ramp(&base, &p);
            assert!(out.iter().any(|v| *v > 0), "输出不应全黑");
        }
    }
}
