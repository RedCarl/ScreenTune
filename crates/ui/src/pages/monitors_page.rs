//! # 显示器管理页
//!
//! 列出全部显示器：名称 / 分辨率 / 主屏 / DDC 支持 / 当前硬件亮度，
//! 支持重新检测（热插拔）与逐台恢复原始画面。

use std::collections::HashMap;
use std::sync::Arc;

use egui::{Color32, RichText, Ui};

use crate::core::AppCore;
use crate::i18n::Tr;

/// 显示器页状态（DDC 亮度缓存，避免每帧读硬件）
#[derive(Debug, Default)]
pub struct MonitorsPageState {
    /// monitor id → 最近一次读取的 DDC 亮度 (当前, 最大)
    brightness_cache: HashMap<String, Option<(u32, u32)>>,
}

/// 渲染显示器管理页
pub fn show(ui: &mut Ui, core: &Arc<AppCore>, tr: &Tr, state: &mut MonitorsPageState) {
    ui.horizontal(|ui| {
        if ui
            .button(RichText::new(format!("⟳ {}", tr.t("monitor.refresh"))))
            .on_hover_text(tr.t("monitor.refresh_tip"))
            .clicked()
        {
            let _ = core.refresh_monitors();
            state.brightness_cache.clear();
            for (info, _) in core.display.snapshot() {
                let id = info.id.clone();
                state
                    .brightness_cache
                    .insert(id.clone(), core.display.ddc_brightness(&id).ok().flatten());
            }
        }
        // 全部同步：以第一台显示器的当前参数为准，同步到全部显示器
        if ui.button(tr.t("header.apply_all")).clicked() {
            let first_id = core.display.snapshot().first().map(|(i, _)| i.id.clone());
            if let Some(first_id) = first_id {
                if let Some(params) = core.display.params_of(&first_id) {
                    let _ = core.display.apply_to_all(params);
                }
            }
        }
    });
    ui.add_space(8.0);

    let snapshot = core.display.snapshot();
    if snapshot.is_empty() {
        ui.label(tr.t("display.no_monitor"));
        return;
    }

    egui::Grid::new("monitors_grid")
        .num_columns(5)
        .spacing([16.0, 10.0])
        .striped(true)
        .show(ui, |ui| {
            // 表头
            for header in [
                tr.t("monitor.name"),
                tr.t("monitor.resolution"),
                tr.t("monitor.primary"),
                tr.t("monitor.ddc"),
                tr.t("monitor.brightness_now"),
            ] {
                ui.label(RichText::new(header).strong());
            }
            ui.end_row();

            // 数据行
            for (info, _) in &snapshot {
                ui.label(&info.name);
                ui.label(format!("{} × {}", info.width, info.height));
                ui.label(if info.is_primary { "✓" } else { "" });
                ui.label(
                    RichText::new(if info.supports_ddc {
                        "DDC/CI".to_string()
                    } else {
                        tr.t("display.gamma_mode")
                    })
                    .color(if info.supports_ddc {
                        Color32::from_rgb(80, 200, 120)
                    } else {
                        Color32::from_rgb(200, 160, 80)
                    }),
                );
                // 亮度列：缓存命中显示，否则读一次
                let brightness = state
                    .brightness_cache
                    .get(&info.id)
                    .cloned()
                    .unwrap_or_else(|| {
                        let v = core.display.ddc_brightness(&info.id).ok().flatten();
                        state.brightness_cache.insert(info.id.clone(), v);
                        v
                    });
                let _ = match brightness {
                    Some((cur, max)) if max > 0 => ui.label(format!("{} / {}", cur, max)),
                    _ => ui.label("—"),
                };
                ui.end_row();
            }
        });

    ui.add_space(8.0);
    // 每台显示器的操作
    for (info, _) in &snapshot {
        ui.horizontal(|ui| {
            ui.label(format!("{}: ", info.name));
            if ui.button(tr.t("display.reset_monitor")).clicked() {
                let _ = core.restore_monitor_baseline(&info.id);
            }
        });
    }
}
