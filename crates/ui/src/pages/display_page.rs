//! # 显示调节页
//!
//! 当前选中显示器的全部参数调节：Gamma / 亮度 / 对比度 / 饱和度 / 色温。
//! 滑块拖动实时生效（不落盘），松手后持久化；页面内可一键恢复默认。

use std::sync::Arc;

use egui::{Color32, RichText, Ui};
use screen_tune_common::consts::*;
use screen_tune_common::DisplayParams;

use crate::core::AppCore;
use crate::i18n::Tr;

/// 渲染显示调节页
pub fn show(
    ui: &mut Ui,
    core: &Arc<AppCore>,
    tr: &Tr,
    monitor_id: &str,
    draft: &mut DisplayParams,
) {
    // 显示器信息（可能已断开）
    let snapshot = core.display.snapshot();
    let Some((info, _)) = snapshot.into_iter().find(|(i, _)| i.id == monitor_id) else {
        ui.centered_and_justified(|ui| {
            ui.label(RichText::new(tr.t("display.no_monitor")).size(16.0));
        });
        return;
    };

    // 当前方案标签
    let current_label = match core.current_profile() {
        Some(id) => {
            let name = core
                .profiles
                .read()
                .unwrap()
                .get(&id)
                .map(|p| p.name.clone())
                .unwrap_or(id);
            format!("{}: {}", tr.t("header.current_profile"), name)
        }
        None => tr.t("header.custom"),
    };
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("{} · {}×{}", info.name, info.width, info.height)).size(15.0),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(current_label).color(Color32::from_rgb(130, 190, 255)));
        });
    });
    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // 亮度模式提示
            let brightness_mode = if info.supports_ddc {
                tr.t("display.ddc")
            } else {
                tr.t("display.gamma_mode")
            };

            // 五个参数滑块；返回 Response 由调用处统一判定变化/拖拽结束
            let mut temp = draft.temperature_k as f32;
            let responses = [
                slider_row(
                    ui,
                    tr,
                    "display.gamma",
                    &mut draft.gamma,
                    GAMMA_MIN..=GAMMA_MAX,
                    "",
                    "display.gamma_tip",
                ),
                slider_row(
                    ui,
                    tr,
                    "display.brightness",
                    &mut draft.brightness,
                    BRIGHTNESS_MIN..=BRIGHTNESS_MAX,
                    "",
                    "display.brightness_tip",
                )
                .on_hover_text(brightness_mode),
                slider_row(
                    ui,
                    tr,
                    "display.contrast",
                    &mut draft.contrast,
                    CONTRAST_MIN..=CONTRAST_MAX,
                    "",
                    "display.contrast_tip",
                ),
                slider_row(
                    ui,
                    tr,
                    "display.saturation",
                    &mut draft.saturation,
                    SATURATION_MIN..=SATURATION_MAX,
                    "%",
                    "display.saturation_tip",
                ),
                slider_row(
                    ui,
                    tr,
                    "display.temperature",
                    &mut temp,
                    TEMPERATURE_MIN_K as f32..=TEMPERATURE_MAX_K as f32,
                    "K",
                    "display.temperature_tip",
                ),
            ];
            draft.temperature_k = temp.round() as u32;

            // 实时应用（不落盘）
            if responses.iter().any(|r| r.changed()) {
                let _ = core.update_monitor_params(monitor_id, *draft);
            }
            // 松手后持久化
            if responses.iter().any(|r| r.drag_stopped()) {
                core.save_config();
            }

            ui.add_space(12.0);
            ui.separator();
            ui.horizontal(|ui| {
                // 恢复默认（全部显示器）
                if ui
                    .button(RichText::new(tr.t("display.restore")).strong())
                    .clicked()
                {
                    let _ = core.restore_default();
                    *draft = DisplayParams::default();
                }
                // 恢复此显示器的原始画面
                if ui
                    .button(tr.t("display.reset_monitor"))
                    .on_hover_text(tr.t("display.reset_monitor_tip"))
                    .clicked()
                {
                    let _ = core.restore_monitor_baseline(monitor_id);
                    *draft = DisplayParams::default();
                }
            });
        });
}

/// 单行滑块：标签 + 滑块 + 数值 + 悬停提示，返回交互 Response
fn slider_row(
    ui: &mut Ui,
    tr: &Tr,
    label_key: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
    tip_key: &str,
) -> egui::Response {
    ui.horizontal(|ui| {
        ui.add_sized(
            [120.0, 20.0],
            egui::Label::new(RichText::new(tr.t(label_key)).strong()),
        );
        ui.add(
            egui::Slider::new(value, range)
                .suffix(suffix)
                .show_value(true)
                .trailing_fill(true),
        )
    })
    .inner
    .on_hover_text(tr.t(tip_key))
}
