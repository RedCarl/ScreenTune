//! # 饱和度 Color Matrix（3×3 颜色矩阵）
//!
//! 标准饱和度矩阵：以 Rec.709 亮度系数（L = 0.2126R + 0.7152G + 0.0722B）
//! 计算灰度分量，再按饱和度系数向灰度或纯色方向插值。
//!
//! ```text
//! [R']   [ (1-s)·Lr + s,  (1-s)·Lg,       (1-s)·Lb      ] [R]
//! [G'] = [ (1-s)·Lr,      (1-s)·Lg + s,   (1-s)·Lb      ] [G]
//! [B']   [ (1-s)·Lr,      (1-s)·Lg,       (1-s)·Lb + s  ] [B]
//! ```
//!
//! **重要说明**：`SetDeviceGammaRamp` 的 LUT 是逐通道独立的（R 输出只取决于 R 输入），
//! 物理上无法直接执行跨通道的 3×3 矩阵运算。因此当前 Gamma 模拟路径使用
//! `lut::vibrance_curve` 的逐通道近似；本模块保留完整矩阵数学，用于：
//! 1. 单元测试验证近似算法的行为边界；
//! 2. 未来接入 GPU Shader（游戏内 Overlay）或 NVIDIA/AMD/Intel 官方 API 时，
//!    直接复用本矩阵，架构无需变动。

use screen_tune_common::consts::{SATURATION_MAX, SATURATION_MIN};

/// Rec.709 亮度系数
const LUM_R: f32 = 0.2126;
const LUM_G: f32 = 0.7152;
const LUM_B: f32 = 0.0722;

/// 生成 3×3 饱和度矩阵（行主序）
///
/// # 参数
/// - `saturation`: 饱和度，范围 [0, 2]，1.0 表示恒等
pub fn saturation_matrix(saturation: f32) -> [f32; 9] {
    let s = saturation.clamp(SATURATION_MIN / 100.0, SATURATION_MAX / 100.0);
    let i = 1.0 - s;
    [
        i * LUM_R + s,
        i * LUM_G,
        i * LUM_B,
        i * LUM_R,
        i * LUM_G + s,
        i * LUM_B,
        i * LUM_R,
        i * LUM_G,
        i * LUM_B + s,
    ]
}

/// 对一组 RGB（0..1）应用 3×3 矩阵
pub fn apply_matrix(rgb: [f32; 3], m: &[f32; 9]) -> [f32; 3] {
    [
        (m[0] * rgb[0] + m[1] * rgb[1] + m[2] * rgb[2]).clamp(0.0, 1.0),
        (m[3] * rgb[0] + m[4] * rgb[1] + m[5] * rgb[2]).clamp(0.0, 1.0),
        (m[6] * rgb[0] + m[7] * rgb[1] + m[8] * rgb[2]).clamp(0.0, 1.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use screen_tune_common::consts::SATURATION_DEFAULT;

    /// 饱和度 100% 时矩阵为恒等：任意颜色保持不变
    #[test]
    fn saturation_one_is_identity() {
        let m = saturation_matrix(SATURATION_DEFAULT / 100.0);
        for rgb in [
            [0.2, 0.5, 0.9],
            [0.0, 0.0, 0.0],
            [1.0, 1.0, 1.0],
            [0.61, 0.32, 0.71],
        ] {
            let out = apply_matrix(rgb, &m);
            for i in 0..3 {
                assert!((out[i] - rgb[i]).abs() < 1e-5, "期望恒等，得到 {out:?}");
            }
        }
    }

    /// 饱和度 0% 时输出纯灰度（三通道相等），且亮度守恒（近似）
    #[test]
    fn saturation_zero_is_gray() {
        let m = saturation_matrix(0.0);
        let out = apply_matrix([0.3, 0.6, 0.9], &m);
        assert!((out[0] - out[1]).abs() < 1e-5);
        assert!((out[1] - out[2]).abs() < 1e-5);
        // 灰度值应接近 Rec.709 亮度
        let lum = 0.2126 * 0.3 + 0.7152 * 0.6 + 0.0722 * 0.9;
        assert!((out[0] - lum).abs() < 1e-5);
    }

    /// 饱和度 200% 时颜色更纯：最大值提升、最小值压低
    #[test]
    fn saturation_boost_pushes_apart() {
        let m = saturation_matrix(2.0);
        let rgb = [0.3, 0.5, 0.7];
        let out = apply_matrix(rgb, &m);
        assert!(out[2] >= rgb[2], "最大值应提升: {out:?}");
        assert!(out[0] <= rgb[0], "最小值应压低: {out:?}");
        // 灰度输入保持不变（矩阵对中性色的数学性质）
        let gray = apply_matrix([0.5, 0.5, 0.5], &m);
        for v in gray {
            assert!((v - 0.5).abs() < 1e-5);
        }
    }
}
