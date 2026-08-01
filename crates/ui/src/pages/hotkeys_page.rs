//! # 全局快捷键页
//!
//! 展示全部绑定（动作 + 快捷键），支持「点击修改 → 按下新组合键」录制，
//! 并展示注册冲突（被其他程序占用的绑定）。

use std::sync::Arc;

use egui::{Color32, RichText, Ui};
use screen_tune_config::HotkeyAction;

use crate::core::AppCore;
use crate::i18n::Tr;

/// 快捷键录制状态
#[derive(Debug, Clone)]
pub struct HotkeyEditState {
    /// 正在编辑的绑定在 config.hotkeys 中的下标
    pub index: usize,
}

/// 渲染快捷键页
pub fn show(ui: &mut Ui, core: &Arc<AppCore>, tr: &Tr, edit: &mut Option<HotkeyEditState>) {
    // 快捷键管理器是否可用
    let available = core.hotkeys_available();
    if !available {
        ui.label(RichText::new(tr.t("hotkeys.no_support")).color(Color32::from_rgb(200, 160, 80)));
    }

    ui.label(
        RichText::new(tr.t("hotkeys.tip"))
            .small()
            .color(Color32::from_rgb(150, 150, 150)),
    );
    ui.add_space(8.0);

    // 冲突列表
    let conflicts = core.hotkey_conflicts.read().unwrap().clone();
    if !conflicts.is_empty() {
        ui.label(
            RichText::new(format!("⚠ {}", tr.t("toast.hotkey_conflict")))
                .color(Color32::from_rgb(230, 120, 90)),
        );
        for (id, reason) in &conflicts {
            ui.label(
                RichText::new(format!("  · {id}: {reason}"))
                    .small()
                    .color(Color32::from_rgb(230, 120, 90)),
            );
        }
        ui.add_space(6.0);
    }

    // 绑定列表
    let bindings = core.config.read().unwrap().hotkeys.clone();
    for (index, binding) in bindings.iter().enumerate() {
        let action_text = action_label(core, tr, &binding.action);
        let in_conflict = conflicts.iter().any(|(id, _)| id == &binding.id);
        let is_editing = edit.as_ref().map(|e| e.index == index).unwrap_or(false);

        ui.horizontal(|ui| {
            ui.add_sized([220.0, 24.0], egui::Label::new(action_text));
            if is_editing {
                // 录制中
                ui.label(
                    RichText::new(format!("… {}", tr.t("hotkeys.capture_hint")))
                        .color(Color32::from_rgb(255, 190, 90)),
                );
            } else {
                let spec_color = if in_conflict {
                    Color32::from_rgb(230, 120, 90)
                } else {
                    Color32::from_rgb(80, 200, 120)
                };
                ui.label(RichText::new(&binding.spec).monospace().color(spec_color));
                if ui.button(tr.t("hotkeys.edit")).clicked() {
                    *edit = Some(HotkeyEditState { index });
                }
            }
        });
    }
}

/// 动作 → 显示文本
fn action_label(core: &Arc<AppCore>, tr: &Tr, action: &HotkeyAction) -> String {
    match action {
        HotkeyAction::RestoreDefault => tr.t("hotkey.action.restore_default"),
        HotkeyAction::ShowWindow => tr.t("hotkey.action.show_window"),
        HotkeyAction::ApplyProfile { profile_id } => {
            let name = core
                .profiles
                .read()
                .unwrap()
                .get(profile_id)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| profile_id.clone());
            format!("{}: {name}", tr.t("hotkey.action.apply_profile"))
        }
    }
}

/// 处理按键捕获（录制状态下由主 update 每帧调用）。
/// 返回 true 表示录制已结束（成功或取消）。
pub fn capture_key(
    ctx: &egui::Context,
    core: &Arc<AppCore>,
    tr: &Tr,
    edit: &mut Option<HotkeyEditState>,
    toast: &mut Option<(String, f32)>,
) -> bool {
    let Some(state) = edit.clone() else {
        return false;
    };

    let events: Vec<egui::Event> = ctx.input(|i| i.events.clone());
    let mut finished = false;

    for event in events {
        let egui::Event::Key {
            key,
            pressed,
            modifiers,
            ..
        } = event
        else {
            continue;
        };
        if !pressed {
            continue;
        }
        // Esc 取消录制
        if key == egui::Key::Escape {
            *edit = None;
            finished = true;
            break;
        }
        // 忽略修饰键本身（Ctrl / Alt / Shift 单独按下不算完成）
        if is_modifier_key(key) {
            continue;
        }
        // 主键
        let Some(main) = main_key_name(key) else {
            continue; // 不支持的键（方向键等），继续等待
        };
        if !(modifiers.ctrl || modifiers.alt || modifiers.shift) {
            continue; // 无修饰键的全局热键过于危险，拒绝
        }

        // 组装规范字符串（Win 键系统级占用过多，暂不支持录制）
        let mut parts: Vec<&str> = Vec::new();
        if modifiers.ctrl {
            parts.push("Ctrl");
        }
        if modifiers.alt {
            parts.push("Alt");
        }
        if modifiers.shift {
            parts.push("Shift");
        }
        parts.push(&main);
        let spec = parts.join("+");

        // 用解析器校验
        if screen_tune_hotkey::HotkeySpec::parse(&spec).is_err() {
            *toast = Some((format!("{}: {spec}", tr.t("hotkeys.conflict")), 3.0));
            *edit = None;
            finished = true;
            break;
        }

        // 写入配置并重新注册
        {
            let mut config = core.config.write().unwrap();
            if let Some(binding) = config.hotkeys.get_mut(state.index) {
                binding.spec = spec.clone();
            }
        }
        core.save_config();
        core.rebuild_hotkeys();
        *toast = Some((format!("{}  {spec}", tr.t("hotkeys.updated")), 2.0));
        *edit = None;
        finished = true;
        break;
    }
    finished
}

/// 是否修饰键本身（egui 0.35 使用物理键名）
fn is_modifier_key(key: egui::Key) -> bool {
    matches!(
        key,
        egui::Key::ControlLeft
            | egui::Key::ControlRight
            | egui::Key::AltLeft
            | egui::Key::AltRight
            | egui::Key::ShiftLeft
            | egui::Key::ShiftRight
            | egui::Key::SuperLeft
            | egui::Key::SuperRight
    )
}

/// 主键 → 规范名（字母 / 数字 / F 键）；不支持的键返回 None
/// （egui 0.35 的 Key 枚举不支持区间 pattern，使用名称字符串判断）
fn main_key_name(key: egui::Key) -> Option<String> {
    let name = format!("{key:?}");
    // 字母键 A-Z
    if name.len() == 1 && name.as_bytes()[0].is_ascii_alphabetic() {
        return Some(name);
    }
    // 数字键 Num0..Num9
    if let Some(digit) = name.strip_prefix("Num") {
        if digit.len() == 1 && digit.as_bytes()[0].is_ascii_digit() {
            return Some(digit.to_string());
        }
    }
    // 功能键 F1..F24
    if name.starts_with('F') && name.len() > 1 {
        if let Ok(n) = name[1..].parse::<u8>() {
            if (1..=24).contains(&n) {
                return Some(name);
            }
        }
    }
    None
}
