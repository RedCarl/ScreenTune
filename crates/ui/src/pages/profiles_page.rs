//! # 配置方案页
//!
//! 方案列表 + 详情编辑：参数微调、应用、保存、新建、删除、JSON 导入导出。
//! 内置方案不可删除；编辑内置方案等同于覆盖同名文件。

use std::sync::Arc;

use egui::{Color32, RichText, TextEdit, Ui};
use screen_tune_common::consts::*;
use screen_tune_common::DisplayParams;
use screen_tune_config::Profile;

use crate::core::AppCore;
use crate::i18n::Tr;

/// 方案页状态
#[derive(Debug, Default)]
pub struct ProfilePageState {
    /// 当前选中的方案 id
    pub selected_id: String,
    /// 编辑缓冲区（选中方案的可编辑副本）
    pub edit_buf: Option<Profile>,
    /// 新建方案的计数（保证 id 唯一）
    new_counter: u32,
    /// 导入 JSON 输入框
    pub import_text: String,
    /// 待替换的编辑缓冲（新建/导入后延迟提交，避免借用冲突）
    pending_replace: Option<Profile>,
    /// 待清空的编辑缓冲（删除后延迟提交）
    pending_clear: bool,
}

/// 渲染方案页。返回 true 表示方案列表/参数发生变更（调用方重建托盘菜单）。
pub fn show(
    ui: &mut Ui,
    core: &Arc<AppCore>,
    tr: &Tr,
    state: &mut ProfilePageState,
    toast: &mut Option<(String, f32)>,
) -> bool {
    let mut profiles_changed = false;

    // 快照方案列表（避免跨锁借用）
    let profiles = core.profiles.read().unwrap().list().to_vec();
    if profiles.is_empty() {
        ui.label(tr.t("display.no_monitor"));
        return false;
    }

    // 选中有效性：默认选中第一个
    if state.selected_id.is_empty() || !profiles.iter().any(|p| p.id == state.selected_id) {
        state.selected_id = profiles[0].id.clone();
        state.edit_buf = Some(profiles[0].clone());
    }

    ui.horizontal(|ui| {
        // ---------------------------------------------------------
        // 左侧：方案列表
        // ---------------------------------------------------------
        egui::ScrollArea::vertical()
            .max_height(ui.available_height() - 8.0)
            .show(ui, |ui| {
                let current = core.current_profile();
                let applied_mark = tr.t("profiles.applied");
                for profile in &profiles {
                    let is_selected = profile.id == state.selected_id;
                    let text = if Some(&profile.id) == current.as_ref() {
                        format!("{}  ·  {} ✓", profile.name, applied_mark)
                    } else {
                        profile.name.clone()
                    };
                    let resp = ui.selectable_label(is_selected, text);
                    if resp.clicked() && !is_selected {
                        state.selected_id = profile.id.clone();
                        state.edit_buf = Some(profile.clone());
                    }
                    // 悬停提示：内置方案标识
                    if profile.builtin {
                        resp.on_hover_text(tr.t("profiles.builtin"));
                    }
                }
            });

        // ---------------------------------------------------------
        // 右侧：详情编辑
        // ---------------------------------------------------------
        let Some(buf) = state.edit_buf.as_mut() else {
            return false;
        };
        ui.separator();
        ui.add_space(4.0);
        ui.vertical(|ui| {
            ui.set_width(ui.available_width() - 24.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr.t("profiles.name")).strong());
                ui.add(TextEdit::singleline(&mut buf.name).hint_text(tr.t("profiles.enter_name")));
                if buf.builtin {
                    ui.label(
                        RichText::new(format!("[{}]", tr.t("profiles.builtin")))
                            .small()
                            .color(Color32::from_rgb(130, 190, 255)),
                    );
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr.t("profiles.desc")).strong());
                ui.add(
                    TextEdit::singleline(&mut buf.description)
                        .hint_text(tr.t("profiles.desc_placeholder")),
                );
            });

            ui.add_space(4.0);
            edit_params_sliders(ui, tr, &mut buf.params);
            ui.add_space(4.0);

            // 动作按钮行
            ui.horizontal(|ui| {
                if ui
                    .button(RichText::new(format!("▶ {}", tr.t("profiles.apply"))).strong())
                    .clicked()
                {
                    let id = buf.id.clone();
                    if core.apply_profile(&id, true).is_ok() {
                        *toast = Some((format!("{}: {}", tr.t("profiles.applied"), buf.name), 3.0));
                        profiles_changed = true;
                    }
                }
                if ui.button(tr.t("profiles.save")).clicked() {
                    if buf.name.trim().is_empty() {
                        *toast = Some((tr.t("profiles.enter_name"), 3.0));
                    } else {
                        let saved = buf.clone();
                        if core.profiles.write().unwrap().update(saved).is_ok() {
                            core.save_config();
                            core.rebuild_tray();
                            *toast = Some((tr.t("toast.saved"), 2.0));
                            profiles_changed = true;
                        }
                    }
                }
                // 新建
                if ui.button(tr.t("profiles.new")).clicked() {
                    state.new_counter += 1;
                    let id = format!("custom-{}", state.new_counter);
                    let p = Profile::new(&id, "新方案", DisplayParams::default());
                    if core.profiles.write().unwrap().create(p.clone()).is_ok() {
                        state.selected_id = id;
                        // 延迟替换编辑缓冲（当前处于借用中）
                        state.pending_replace = Some(p);
                        core.rebuild_tray();
                        profiles_changed = true;
                    }
                }
                // 删除（内置不可删除）
                if !buf.builtin && ui.button(tr.t("profiles.delete")).clicked() {
                    let id = buf.id.clone();
                    if core.profiles.write().unwrap().delete(&id).is_ok() {
                        core.rebuild_tray();
                        state.pending_clear = true;
                        profiles_changed = true;
                    }
                }
            });

            ui.add_space(8.0);
            ui.separator();

            // 导入 / 导出
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr.t("profiles.import")).strong());
                ui.add(
                    TextEdit::multiline(&mut state.import_text)
                        .desired_rows(3)
                        .hint_text(tr.t("profiles.import_hint"))
                        .desired_width(ui.available_width() - 120.0),
                );
                if ui.button(tr.t("profiles.import")).clicked() {
                    let text = state.import_text.clone();
                    match core.profiles.write().unwrap().import_json(&text) {
                        Ok(imported) => {
                            core.rebuild_tray();
                            state.selected_id = imported.id.clone();
                            // 延迟替换编辑缓冲（当前处于借用中）
                            state.pending_replace = Some(imported);
                            *toast = Some((tr.t("profiles.imported"), 3.0));
                            profiles_changed = true;
                        }
                        Err(e) => {
                            *toast = Some((format!("导入失败: {e:#}"), 4.0));
                        }
                    }
                }
            });

            // 导出到剪贴板
            if ui.button(tr.t("profiles.export")).clicked() {
                let id = buf.id.clone();
                if let Ok(json) = core.profiles.read().unwrap().export_json(&id) {
                    ui.ctx().copy_text(json);
                    *toast = Some((tr.t("profiles.exported"), 2.0));
                }
            }
        });
        // horizontal 闭包显式返回 bool（与 let-else 的 `return false` 一致）
        profiles_changed
    });

    // 延迟提交：替换 / 清空编辑缓冲（此时借用已结束）
    if let Some(p) = state.pending_replace.take() {
        state.selected_id = p.id.clone();
        state.edit_buf = Some(p);
    }
    if std::mem::take(&mut state.pending_clear) {
        state.edit_buf = None;
        state.selected_id = String::new();
    }

    profiles_changed
}

/// 参数编辑滑块组（编辑缓冲，不实时应用）
fn edit_params_sliders(ui: &mut Ui, tr: &Tr, params: &mut DisplayParams) {
    let mut temp = params.temperature_k as f32;

    param_row(
        ui,
        tr,
        "display.gamma",
        &mut params.gamma,
        GAMMA_MIN..=GAMMA_MAX,
        "",
    );
    param_row(
        ui,
        tr,
        "display.brightness",
        &mut params.brightness,
        BRIGHTNESS_MIN..=BRIGHTNESS_MAX,
        "",
    );
    param_row(
        ui,
        tr,
        "display.contrast",
        &mut params.contrast,
        CONTRAST_MIN..=CONTRAST_MAX,
        "",
    );
    param_row(
        ui,
        tr,
        "display.saturation",
        &mut params.saturation,
        SATURATION_MIN..=SATURATION_MAX,
        "%",
    );
    param_row(
        ui,
        tr,
        "display.temperature",
        &mut temp,
        TEMPERATURE_MIN_K as f32..=TEMPERATURE_MAX_K as f32,
        "K",
    );
    params.temperature_k = temp.round() as u32;
}

/// 参数行（编辑缓冲用）
fn param_row(
    ui: &mut Ui,
    tr: &Tr,
    label_key: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
) {
    ui.horizontal(|ui| {
        ui.add_sized(
            [120.0, 20.0],
            egui::Label::new(RichText::new(tr.t(label_key))),
        );
        ui.add(
            egui::Slider::new(value, range)
                .suffix(suffix)
                .show_value(true),
        );
    });
}
