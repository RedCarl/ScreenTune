//! # 应用级常量
//!
//! 集中管理 ScreenTune 的各类取值范围与默认值，避免魔法数字散落各处。

/// Gamma 取值范围下限（50 表示最暗）
pub const GAMMA_MIN: f32 = 50.0;
/// Gamma 取值范围上限（150 表示最亮）
pub const GAMMA_MAX: f32 = 150.0;
/// Gamma 默认值（100 表示无调整，曲线恒等）
pub const GAMMA_DEFAULT: f32 = 100.0;

/// 亮度取值范围下限（0 表示全黑）
pub const BRIGHTNESS_MIN: f32 = 0.0;
/// 亮度取值范围上限（100 表示原始亮度）
pub const BRIGHTNESS_MAX: f32 = 100.0;
/// 亮度默认值（100 表示无调整）
pub const BRIGHTNESS_DEFAULT: f32 = 100.0;

/// 对比度取值范围下限（50 表示全部压到中灰）
pub const CONTRAST_MIN: f32 = 50.0;
/// 对比度取值范围上限（150 表示硬切）
pub const CONTRAST_MAX: f32 = 150.0;
/// 对比度默认值（100 表示无调整）
pub const CONTRAST_DEFAULT: f32 = 100.0;

/// 饱和度取值范围下限（0 表示完全去饱和）
pub const SATURATION_MIN: f32 = 0.0;
/// 饱和度取值范围上限（200 表示强烈增强，对应 NVIDIA Digital Vibrance 风格）
pub const SATURATION_MAX: f32 = 200.0;
/// 饱和度默认值（100 表示无调整）
pub const SATURATION_DEFAULT: f32 = 100.0;

/// 色温取值范围下限（2500K，暖黄）
pub const TEMPERATURE_MIN_K: u32 = 2500;
/// 色温取值范围上限（10000K，冷蓝）
pub const TEMPERATURE_MAX_K: u32 = 10000;
/// 色温默认值（6500K，标准 D65 白点，即无调整）
pub const TEMPERATURE_DEFAULT_K: u32 = 6500;

/// 应用显示名称
pub const APP_NAME: &str = "ScreenTune";
/// 应用 Tagline
pub const APP_TAGLINE: &str = "Instant Display Control for Gamers";
/// 当前版本号（与 Cargo.toml 保持一致）
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
