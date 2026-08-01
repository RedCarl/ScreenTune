//! # 设置页
//!
//! 开机自启 / 关闭行为 / 语言 / 主题 / 游戏自动切换规则 / 日志级别 / 关于。

use std::sync::Arc;

use egui::{Color32, RichText, Ui};
use screen_tune_common::consts::{APP_TAGLINE, APP_VERSION};
use screen_tune_config::{GameRule, Language, ThemePref};

use crate::core::AppCore;
use crate::i18n::Tr;

/// 设置页状态
#[derive(Debug, Default)]
pub struct SettingsPageState {
    /// 游戏规则编辑缓冲
    pub rules: Vec<GameRule>,
    /// 缓冲是否已加载（进入页面时一次性加载）
    pub rules_loaded: bool,
}

/// 渲染设置页。返回 (主题是否变更, 语言是否变更)。
pub fn show(
    ui: &mut Ui,
    core: &Arc<AppCore>,
    tr: &Tr,
    state: &mut SettingsPageState,
    toast: &mut Option<(String, f32)>,
) -> (bool, bool) {
    let mut theme_changed = false;
    let mut lang_changed = false;

    // 首次进入加载规则缓冲
    if !state.rules_loaded {
        state.rules = core.config.read().unwrap().game_rules.clone();
        state.rules_loaded = true;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // ------------------------------------------------
            // 常规
            // ------------------------------------------------
            section(ui, tr, "settings.title");
            ui.horizontal(|ui| {
                let mut startup = core.startup_enabled();
                if ui
                    .checkbox(&mut startup, tr.t("settings.startup"))
                    .changed()
                {
                    core.toggle_startup();
                }
            });
            ui.horizontal(|ui| {
                let mut close_to_tray = core.config.read().unwrap().close_to_tray;
                if ui
                    .checkbox(&mut close_to_tray, tr.t("settings.close_to_tray"))
                    .changed()
                {
                    core.config.write().unwrap().close_to_tray = close_to_tray;
                    core.save_config();
                }
            });

            // 语言
            ui.horizontal(|ui| {
                ui.add_sized(
                    [140.0, 24.0],
                    egui::Label::new(RichText::new(tr.t("settings.language")).strong()),
                );
                let mut lang = core.config.read().unwrap().language;
                egui::ComboBox::from_id_salt("lang_select")
                    .selected_text(match lang {
                        Language::Zh => "中文（简体）",
                        Language::En => "English",
                    })
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(&mut lang, Language::Zh, "中文（简体）")
                            .clicked()
                        {
                            lang_changed = true;
                        }
                        if ui
                            .selectable_value(&mut lang, Language::En, "English")
                            .clicked()
                        {
                            lang_changed = true;
                        }
                    });
                if lang_changed {
                    core.config.write().unwrap().language = lang;
                    core.save_config();
                }
            });

            // 主题
            ui.horizontal(|ui| {
                ui.add_sized(
                    [140.0, 24.0],
                    egui::Label::new(RichText::new(tr.t("settings.theme")).strong()),
                );
                let mut theme = core.config.read().unwrap().theme;
                egui::ComboBox::from_id_salt("theme_select")
                    .selected_text(match theme {
                        ThemePref::Dark => tr.t("settings.dark"),
                        ThemePref::Light => tr.t("settings.light"),
                    })
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(&mut theme, ThemePref::Dark, tr.t("settings.dark"))
                            .clicked()
                        {
                            theme_changed = true;
                        }
                        if ui
                            .selectable_value(&mut theme, ThemePref::Light, tr.t("settings.light"))
                            .clicked()
                        {
                            theme_changed = true;
                        }
                    });
                if theme_changed {
                    core.config.write().unwrap().theme = theme;
                    core.save_config();
                }
            });

            // 日志级别
            ui.horizontal(|ui| {
                ui.add_sized(
                    [140.0, 24.0],
                    egui::Label::new(RichText::new(tr.t("settings.log_level")).strong()),
                );
                let mut level = core.config.read().unwrap().log_level.clone();
                egui::ComboBox::from_id_salt("log_level_select")
                    .selected_text(level.clone())
                    .show_ui(ui, |ui| {
                        for lv in ["trace", "debug", "info", "warn", "error"] {
                            ui.selectable_value(&mut level, lv.to_string(), lv);
                        }
                    });
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    core.config.write().unwrap().log_level = level;
                    core.save_config();
                    *toast = Some((tr.t("toast.saved"), 2.0));
                }
            });

            ui.add_space(10.0);
            ui.separator();

            // ------------------------------------------------
            // 游戏自动切换
            // ------------------------------------------------
            section(ui, tr, "settings.game_detection");
            ui.label(
                RichText::new(tr.t("settings.game_detection_tip"))
                    .small()
                    .color(Color32::from_rgb(150, 150, 150)),
            );

            ui.horizontal(|ui| {
                let mut enabled = core.config.read().unwrap().game_detection_enabled;
                if ui
                    .checkbox(&mut enabled, tr.t("settings.game_detection"))
                    .changed()
                {
                    core.config.write().unwrap().game_detection_enabled = enabled;
                    core.sync_detector_rules();
                    core.save_config();
                }
                ui.add_space(16.0);
                ui.label(RichText::new(tr.t("settings.game_poll")).strong());
                let mut interval = core.config.read().unwrap().game_poll_interval_secs as f32;
                if ui
                    .add(
                        egui::DragValue::new(&mut interval)
                            .range(1.0..=60.0)
                            .suffix(" s"),
                    )
                    .changed()
                {
                    core.config.write().unwrap().game_poll_interval_secs = interval as u64;
                    core.save_config();
                }
            });

            // 规则表
            ui.add_space(6.0);
            ui.label(RichText::new(tr.t("settings.game_rules")).strong());
            let profile_options: Vec<(String, String)> = core
                .profiles
                .read()
                .unwrap()
                .list()
                .iter()
                .map(|p| (p.id.clone(), p.name.clone()))
                .collect();

            let mut remove: Option<usize> = None;
            for (idx, rule) in state.rules.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut rule.enabled, "");
                    ui.add_sized(
                        [180.0, 24.0],
                        egui::TextEdit::singleline(&mut rule.exe_name)
                            .hint_text(tr.t("settings.rule_placeholder")),
                    );
                    // 方案选择
                    let selected = rule.profile_id.clone();
                    let show = profile_options
                        .iter()
                        .find(|(id, _)| *id == selected)
                        .map(|(_, n)| n.clone())
                        .unwrap_or_else(|| selected.clone());
                    egui::ComboBox::from_id_salt(format!("rule_profile_{idx}"))
                        .selected_text(show)
                        .width(140.0)
                        .show_ui(ui, |ui| {
                            for (id, name) in &profile_options {
                                if ui
                                    .selectable_value(
                                        &mut rule.profile_id,
                                        id.clone(),
                                        name.clone(),
                                    )
                                    .clicked()
                                {
                                    let _ = ui;
                                }
                            }
                        });
                    if ui
                        .button("✕")
                        .on_hover_text(tr.t("common.delete"))
                        .clicked()
                    {
                        remove = Some(idx);
                    }
                });
            }
            if let Some(idx) = remove {
                state.rules.remove(idx);
            }

            ui.horizontal(|ui| {
                if ui.button(tr.t("settings.add_rule")).clicked() {
                    let next_id = format!("rule:custom-{}", state.rules.len() + 1);
                    state.rules.push(GameRule::new(
                        next_id,
                        String::new(),
                        profile_options
                            .first()
                            .map(|(id, _)| id.clone())
                            .unwrap_or_default(),
                    ));
                }
                if ui
                    .button(RichText::new(tr.t("profiles.save")).strong())
                    .clicked()
                {
                    let rules = state.rules.clone();
                    let mut config = core.config.write().unwrap();
                    config.game_rules = rules;
                    drop(config);
                    core.sync_detector_rules();
                    core.save_config();
                    *toast = Some((tr.t("toast.rule_saved"), 2.0));
                }
            });

            ui.add_space(10.0);
            ui.separator();

            // ------------------------------------------------
            // 关于
            // ------------------------------------------------
            section(ui, tr, "settings.about");
            ui.label(
                RichText::new(APP_TAGLINE)
                    .italics()
                    .color(Color32::from_rgb(130, 190, 255)),
            );
            ui.label(format!("{} {APP_VERSION}", tr.t("app.version")));
            ui.label(
                RichText::new(format!(
                    "{}  ·  {}",
                    tr.t("settings.roadmap"),
                    tr.t("settings.roadmap_items")
                ))
                .small()
                .color(Color32::from_rgb(150, 150, 150)),
            );
            ui.label(
                RichText::new(tr.t("settings.no_telemetry"))
                    .small()
                    .color(Color32::from_rgb(120, 200, 150)),
            );
        });

    (theme_changed, lang_changed)
}

/// 小节标题
fn section(ui: &mut Ui, tr: &Tr, title_key: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(tr.t(title_key)).size(16.0).strong());
    ui.add_space(4.0);
}
