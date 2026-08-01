//! # Fluent 风格主题与字体
//!
//! 仿 Windows 11 / PowerToys 的 Fluent 深色主题：
//! - 深灰分层背景（#202020 窗口 / #2B2B2B 面板 / #0B0B0B 输入框）
//! - 强调色 #0078D4（Fluent Accent）
//! - 圆角控件（4px 常规 / 8px 窗口）
//! - 中文界面字体（系统字体加载，Windows 优先微软雅黑）

use egui::{Color32, CornerRadius, FontFamily, Stroke, Vec2, Visuals};
use screen_tune_config::ThemePref;
use tracing::debug;

/// Fluent Accent 色（深色主题）
pub const ACCENT: Color32 = Color32::from_rgb(0, 120, 212);
/// Fluent Accent 亮色（浅色主题）
pub const ACCENT_LIGHT: Color32 = Color32::from_rgb(0, 95, 184);
/// 深色窗口背景
pub const BG_WINDOW: Color32 = Color32::from_rgb(32, 32, 32);
/// 深色面板背景
pub const BG_PANEL: Color32 = Color32::from_rgb(43, 43, 43);
/// 深色输入框背景
pub const BG_INPUT: Color32 = Color32::from_rgb(11, 11, 11);

/// 应用主题到 egui 上下文
pub fn apply_theme(ctx: &egui::Context, pref: ThemePref) {
    match pref {
        ThemePref::Dark => {
            ctx.set_theme(egui::Theme::Dark);
            apply_dark_visuals(ctx);
        }
        ThemePref::Light => {
            ctx.set_theme(egui::Theme::Light);
            apply_light_visuals(ctx);
        }
    }
}

/// 深色主题视觉定制
fn apply_dark_visuals(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    // 基础分层背景
    visuals.panel_fill = BG_WINDOW;
    visuals.window_fill = BG_WINDOW;
    visuals.extreme_bg_color = BG_INPUT;
    visuals.faint_bg_color = Color32::from_rgb(24, 24, 24);

    // 选中 / 链接
    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke = Stroke::new(1.0, ACCENT);
    visuals.hyperlink_color = Color32::from_rgb(76, 194, 255);

    // 控件三态
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(58, 58, 58);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(58, 58, 58);
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, Color32::from_rgb(80, 80, 80));
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(230, 230, 230));
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(70, 70, 70);
    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(70, 70, 70);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(100, 100, 100));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::from_rgb(255, 255, 255));
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.active.weak_bg_fill = ACCENT;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, Color32::from_rgb(255, 255, 255));

    // 圆角（Fluent 风格）
    let radius = CornerRadius::same(4);
    visuals.widgets.inactive.corner_radius = radius;
    visuals.widgets.hovered.corner_radius = radius;
    visuals.widgets.active.corner_radius = radius;
    visuals.window_corner_radius = CornerRadius::same(8);

    ctx.set_visuals(visuals);
    apply_spacing(ctx, egui::Theme::Dark);
}

/// 浅色主题视觉定制（Fluent Light）
fn apply_light_visuals(ctx: &egui::Context) {
    let mut visuals = Visuals::light();

    visuals.panel_fill = Color32::from_rgb(243, 243, 243);
    visuals.window_fill = Color32::from_rgb(250, 250, 250);
    visuals.extreme_bg_color = Color32::WHITE;
    visuals.faint_bg_color = Color32::from_rgb(235, 235, 235);
    visuals.selection.bg_fill = ACCENT_LIGHT;
    visuals.hyperlink_color = Color32::from_rgb(0, 95, 184);

    let radius = CornerRadius::same(4);
    visuals.widgets.inactive.corner_radius = radius;
    visuals.widgets.hovered.corner_radius = radius;
    visuals.widgets.active.corner_radius = radius;
    visuals.window_corner_radius = CornerRadius::same(8);

    ctx.set_visuals(visuals);
    apply_spacing(ctx, egui::Theme::Light);
}

/// 间距统一（egui 0.35 按主题维护 style）
fn apply_spacing(ctx: &egui::Context, theme: egui::Theme) {
    let mut style = (*ctx.style_of(theme)).clone();
    style.spacing.item_spacing = Vec2::new(10.0, 8.0);
    style.spacing.button_padding = Vec2::new(14.0, 7.0);
    style.spacing.interact_size.y = 28.0;
    ctx.set_style_of(theme, style);
}

/// 加载系统中文字体（中文界面必需；失败时回退到英文界面）
pub fn install_cjk_font(ctx: &egui::Context) {
    let Some((name, data)) = find_system_cjk_font() else {
        debug!("未找到系统中文字体，中文界面将显示异常");
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        name.clone(),
        std::sync::Arc::new(egui::FontData::from_owned(data)),
    );
    // 置顶优先级：中文字体放在最前（拉丁字形与默认字体一致的部分由后续字体补充）
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        if let Some(list) = fonts.families.get_mut(&family) {
            list.insert(0, name.clone());
        }
    }
    ctx.set_fonts(fonts);
    debug!("已加载中文字体: {name}");
}

/// 常见系统中文字体候选（Windows / macOS / Linux）
fn find_system_cjk_font() -> Option<(String, Vec<u8>)> {
    const CANDIDATES: &[(&str, &str)] = &[
        // Windows：微软雅黑 / 黑体 / 宋体
        ("msyh", "C:/Windows/Fonts/msyh.ttc"),
        ("msyh", "C:/Windows/Fonts/msyh.ttf"),
        ("msyh", "C:/Windows/Fonts/msyhl.ttc"),
        ("simhei", "C:/Windows/Fonts/simhei.ttf"),
        ("simsun", "C:/Windows/Fonts/simsun.ttc"),
        // macOS：苹方
        ("pingfang", "/System/Library/Fonts/PingFang.ttc"),
        ("stheitisc", "/System/Library/Fonts/STHeiti Light.ttc"),
        // Linux：思源黑体
        (
            "notocjk",
            "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        ),
        ("wqy", "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc"),
    ];
    for (name, path) in CANDIDATES {
        if let Ok(data) = std::fs::read(path) {
            if data.len() > 1024 {
                return Some(((*name).to_string(), data));
            }
        }
    }
    None
}
