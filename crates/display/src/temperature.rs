//! # 色温 → RGB 增益
//!
//! 采用 Neil Bartlett 提出的黑体辐射颜色近似算法（Kelvin → RGB），
//! 该算法在 1000K~40000K 范围内视觉精度良好，广泛用于 f.lux 等工具。
//!
//! 输出为相对增益（R/G/B 各 0~1 倍），以 6500K（D65 白点）为基准归一化，
//! 因此色温设为 6500K 时增益恒为 (1.0, 1.0, 1.0)，即完全无偏色。

/// 基准色温（开尔文），增益在此处归一化为 1
pub const REFERENCE_TEMPERATURE_K: u32 = 6500;

/// 计算给定色温的 RGB 增益
///
/// # 参数
/// - `kelvin`: 色温，范围建议 [2500, 10000]
///
/// # 返回
/// (r, g, b) 三个通道的增益系数，均 > 0
pub fn temperature_gains(kelvin: u32) -> (f32, f32, f32) {
    let k = kelvin.clamp(1000, 40000);
    // 温度以 100 为单位缩放，便于公式计算
    let t = (k as f32) / 100.0;

    // 红通道
    let r = if t <= 66.0 {
        255.0
    } else {
        329.6987 * (t - 60.0).powf(-0.133_205)
    };

    // 绿通道
    let g = if t <= 66.0 {
        99.4708 * t.ln() - 161.1196
    } else {
        288.1222 * (t - 60.0).powf(-0.075_515)
    };

    // 蓝通道
    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.5177 * (t - 10.0).ln() - 305.0448
    };

    // 钳制到合法范围
    let rgb = [
        r.clamp(0.0, 255.0),
        g.clamp(0.0, 255.0),
        b.clamp(0.0, 255.0),
    ];

    // 以 6500K 的颜色为基准归一化 → 6500K 时增益恰为 (1,1,1)
    let ref_rgb = reference_rgb();
    (
        rgb[0] / ref_rgb[0],
        rgb[1] / ref_rgb[1],
        rgb[2] / ref_rgb[2],
    )
}

/// 6500K 时算法的原始 RGB 值（归一化基准）
///
/// 注意：6500K 对应 t=65 ≤ 66，绿通道必须使用 t≤66 分支的公式。
fn reference_rgb() -> [f32; 3] {
    let t = (REFERENCE_TEMPERATURE_K as f32) / 100.0; // = 65.0
    let r: f32 = 255.0; // t ≤ 66 时红通道恒为 255
    let g = 99.4708 * t.ln() - 161.1196;
    let b = 138.5177 * (t - 10.0).ln() - 305.0448;
    [
        r.clamp(0.0, 255.0),
        g.clamp(0.0, 255.0),
        b.clamp(0.0, 255.0),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 6500K 必须精确返回 (1,1,1)
    #[test]
    fn reference_temperature_is_neutral() {
        let (r, g, b) = temperature_gains(6500);
        assert!((r - 1.0).abs() < 1e-6);
        assert!((g - 1.0).abs() < 1e-6);
        assert!((b - 1.0).abs() < 1e-6);
    }

    /// 低温（暖色）应偏红：B 通道增益显著低于 R（蓝被压低 → 感知偏暖）。
    /// 注：t≤66 时红通道原始值为 255（算法特性），归一化后恒为 1.0，
    /// 偏暖由绿/蓝通道下降实现。
    #[test]
    fn low_temperature_is_warm() {
        let (r, _, b) = temperature_gains(2500);
        assert!(r > b, "暖色应 R>B，实际 r={r} b={b}");
        assert!(b < 1.0, "暖色应压低蓝通道，实际 b={b}");
    }

    /// 高温（冷色）应偏蓝：B 增益 > R 增益
    #[test]
    fn high_temperature_is_cool() {
        let (r, _, b) = temperature_gains(10000);
        assert!(b > r, "冷色应 B>R，实际 r={r} b={b}");
        assert!(b > 1.0, "冷色应提升蓝色，实际 b={b}");
    }

    /// 增益必须全为正数
    #[test]
    fn gains_are_positive() {
        for k in [2500u32, 4000, 6500, 8000, 10000] {
            let (r, g, b) = temperature_gains(k);
            assert!(r > 0.0 && g > 0.0 && b > 0.0, "色温 {k}K 增益非正");
        }
    }
}
