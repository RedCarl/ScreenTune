//! # 全局快捷键
//!
//! 封装 `global-hotkey`（tauri 生态）：
//! - 快捷键字符串（`Ctrl+Alt+1`）↔ 平台组合的解析与格式化；
//! - 批量注册 / 重新注册与冲突检测（注册失败即视为被其他程序占用）；
//! - 事件轮询，转换为应用动作。

use std::collections::HashMap;

use anyhow::{Context, Result};
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager};
use screen_tune_config::HotkeyAction;
use tracing::{debug, warn};

/// 快捷键修饰键
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modifier {
    /// Ctrl（macOS 为 Command 对应位）
    Ctrl,
    /// Alt
    Alt,
    /// Shift
    Shift,
    /// Win / Super / Meta
    Win,
}

impl Modifier {
    /// 转为 global-hotkey 修饰键位
    fn to_flags(self) -> Modifiers {
        match self {
            Modifier::Ctrl => Modifiers::CONTROL,
            Modifier::Alt => Modifiers::ALT,
            Modifier::Shift => Modifiers::SHIFT,
            Modifier::Win => Modifiers::META,
        }
    }

    /// 名称（快捷键字符串中的片段，如 `Ctrl`）
    fn name(self) -> &'static str {
        match self {
            Modifier::Ctrl => "Ctrl",
            Modifier::Alt => "Alt",
            Modifier::Shift => "Shift",
            Modifier::Win => "Win",
        }
    }
}

/// 解析后的快捷键
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotkeySpec {
    /// 修饰键（顺序无关，格式化时按 Ctrl/Alt/Shift/Win 排序）
    pub modifiers: Vec<Modifier>,
    /// 主键
    pub code: Code,
}

impl HotkeySpec {
    /// 解析快捷键字符串（如 `Ctrl+Alt+1`），支持大小写混写、多种分隔符与修饰键乱序。
    /// 除修饰键以外的部分必须是唯一的主键。
    pub fn parse(spec: &str) -> Result<Self> {
        let parts: Vec<&str> = spec.split(['+', '-', ',']).map(str::trim).collect();
        if parts.is_empty() {
            anyhow::bail!("快捷键为空");
        }
        let mut modifiers = Vec::new();
        let mut code_part: Option<&str> = None;
        for part in &parts {
            if let Some(m) = parse_modifier(part) {
                if modifiers.contains(&m) {
                    anyhow::bail!("重复的修饰键: {part}");
                }
                modifiers.push(m);
            } else if code_part.is_some() {
                anyhow::bail!("存在多个主键: {part}");
            } else {
                code_part = Some(part);
            }
        }
        let Some(code_part) = code_part else {
            anyhow::bail!("缺少主键");
        };
        let code = parse_code(code_part)?;
        Ok(Self { modifiers, code })
    }

    /// 格式化为规范字符串（如 `Ctrl+Alt+1`）
    pub fn format(&self) -> String {
        let mut parts: Vec<String> = [
            Modifier::Ctrl,
            Modifier::Alt,
            Modifier::Shift,
            Modifier::Win,
        ]
        .iter()
        .filter(|m| self.modifiers.contains(m))
        .map(|m| m.name().to_string())
        .collect();
        parts.push(format_code(self.code));
        parts.join("+")
    }

    /// 构建 global-hotkey 对象
    fn to_hotkey(&self) -> HotKey {
        let mut flags = Modifiers::empty();
        for m in &self.modifiers {
            flags |= m.to_flags();
        }
        HotKey::new(Some(flags), self.code)
    }
}

/// 解析修饰键名；不是修饰键时返回 None
fn parse_modifier(part: &str) -> Option<Modifier> {
    match part.to_lowercase().as_str() {
        "ctrl" | "control" => Some(Modifier::Ctrl),
        "alt" | "option" => Some(Modifier::Alt),
        "shift" => Some(Modifier::Shift),
        "win" | "meta" | "super" | "cmd" | "command" => Some(Modifier::Win),
        _ => None,
    }
}

/// 解析主键
fn parse_code(s: &str) -> Result<Code> {
    let s = s.to_uppercase();
    let s = s.as_str();
    // 数字键（0-9）
    if s.len() == 1 && s.as_bytes()[0].is_ascii_digit() {
        return Ok(num_key(s.as_bytes()[0] - b'0'));
    }
    // 字母键（A-Z）
    if s.len() == 1 && s.as_bytes()[0].is_ascii_alphabetic() {
        return Ok(letter_key(s.as_bytes()[0] as char));
    }
    // 功能键（F1-F24）
    if s.starts_with('F') && s.len() > 1 {
        if let Ok(n) = s[1..].parse::<u8>() {
            if (1..=24).contains(&n) {
                return Ok(f_key(n));
            }
        }
    }
    anyhow::bail!("不支持的主键: {s}")
}

/// 数字键 0-9 → Code（keyboard_types 命名为 Digit0..Digit9）
fn num_key(d: u8) -> Code {
    match d {
        0 => Code::Digit0,
        1 => Code::Digit1,
        2 => Code::Digit2,
        3 => Code::Digit3,
        4 => Code::Digit4,
        5 => Code::Digit5,
        6 => Code::Digit6,
        7 => Code::Digit7,
        8 => Code::Digit8,
        _ => Code::Digit9,
    }
}

/// 字母 A-Z → Code
fn letter_key(c: char) -> Code {
    match c {
        'A' => Code::KeyA,
        'B' => Code::KeyB,
        'C' => Code::KeyC,
        'D' => Code::KeyD,
        'E' => Code::KeyE,
        'F' => Code::KeyF,
        'G' => Code::KeyG,
        'H' => Code::KeyH,
        'I' => Code::KeyI,
        'J' => Code::KeyJ,
        'K' => Code::KeyK,
        'L' => Code::KeyL,
        'M' => Code::KeyM,
        'N' => Code::KeyN,
        'O' => Code::KeyO,
        'P' => Code::KeyP,
        'Q' => Code::KeyQ,
        'R' => Code::KeyR,
        'S' => Code::KeyS,
        'T' => Code::KeyT,
        'U' => Code::KeyU,
        'V' => Code::KeyV,
        'W' => Code::KeyW,
        'X' => Code::KeyX,
        'Y' => Code::KeyY,
        _ => Code::KeyZ,
    }
}

/// 功能键 F1-F24 → Code
fn f_key(n: u8) -> Code {
    match n {
        1 => Code::F1,
        2 => Code::F2,
        3 => Code::F3,
        4 => Code::F4,
        5 => Code::F5,
        6 => Code::F6,
        7 => Code::F7,
        8 => Code::F8,
        9 => Code::F9,
        10 => Code::F10,
        11 => Code::F11,
        12 => Code::F12,
        13 => Code::F13,
        14 => Code::F14,
        15 => Code::F15,
        16 => Code::F16,
        17 => Code::F17,
        18 => Code::F18,
        19 => Code::F19,
        20 => Code::F20,
        21 => Code::F21,
        22 => Code::F22,
        23 => Code::F23,
        _ => Code::F24,
    }
}

/// Code → 名称（与 parse 互补）
fn format_code(code: Code) -> String {
    let name = format!("{code:?}");
    // keyboard_types 枚举名称形如 KeyA / Digit5 / F1，去掉前缀转主键
    if let Some(rest) = name.strip_prefix("Key") {
        return rest.to_string();
    }
    if let Some(rest) = name.strip_prefix("Digit") {
        return rest.to_string();
    }
    name
}

// ---------------------------------------------------------------
// 管理器
// ---------------------------------------------------------------

/// 一次注册结果：成功列表 + 冲突列表（被其他程序占用的绑定）
#[derive(Debug, Default)]
pub struct RegisterOutcome {
    /// 成功注册的绑定 id
    pub ok: Vec<String>,
    /// 注册失败（冲突）的绑定 id 与原因
    pub conflicts: Vec<(String, String)>,
}

/// 全局快捷键管理器
pub struct HotkeyManager {
    manager: GlobalHotKeyManager,
    /// 平台热键 id → 已注册的 HotKey 对象（注销时必须用原对象）
    active: HashMap<u32, HotKey>,
    /// 平台热键 id → 绑定 id
    binding_of: HashMap<u32, String>,
    /// 当前生效的绑定（供展示与重注册）
    bindings: Vec<HotkeyBinding>,
}

/// 一条可注册的绑定（绑定 id + 快捷键 + 动作）
#[derive(Debug, Clone)]
pub struct HotkeyBinding {
    /// 绑定唯一标识
    pub id: String,
    /// 快捷键规范字符串
    pub spec: String,
    /// 触发动作
    pub action: HotkeyAction,
}

impl HotkeyManager {
    /// 创建管理器（失败通常意味着平台事件循环尚未就绪）
    pub fn new() -> Result<Self> {
        let manager = GlobalHotKeyManager::new().context("初始化全局快捷键管理器失败")?;
        Ok(Self {
            manager,
            active: HashMap::new(),
            binding_of: HashMap::new(),
            bindings: Vec::new(),
        })
    }

    /// 以配置表中的绑定重建全部注册（先注销旧的，再注册新的）。
    /// 返回成功与冲突列表；冲突绑定不会被激活。
    pub fn rebuild(&mut self, bindings: Vec<HotkeyBinding>) -> RegisterOutcome {
        // 1. 注销全部现有注册（必须使用原 HotKey 对象）
        for (_, hotkey) in std::mem::take(&mut self.active) {
            if let Err(e) = self.manager.unregister(hotkey) {
                warn!("注销快捷键失败: {e}");
            }
        }
        self.binding_of.clear();
        self.bindings.clear();

        // 2. 注册新绑定
        let mut outcome = RegisterOutcome::default();
        for binding in &bindings {
            match HotkeySpec::parse(&binding.spec) {
                Err(e) => {
                    warn!("快捷键解析失败（{}）: {}: {}", binding.spec, binding.id, e);
                    outcome
                        .conflicts
                        .push((binding.id.clone(), format!("解析失败: {e}")));
                }
                Ok(spec) => {
                    let hotkey = spec.to_hotkey();
                    match self.manager.register(hotkey) {
                        Ok(()) => {
                            let platform_id = hotkey.id();
                            self.active.insert(platform_id, hotkey);
                            self.binding_of.insert(platform_id, binding.id.clone());
                            outcome.ok.push(binding.id.clone());
                            debug!("已注册全局快捷键 {} → {}", binding.spec, binding.id);
                        }
                        Err(e) => {
                            warn!(
                                "快捷键注册失败（可能被占用）: {} {}: {}",
                                binding.spec, binding.id, e
                            );
                            outcome
                                .conflicts
                                .push((binding.id.clone(), format!("已被占用: {e}")));
                        }
                    }
                }
            }
        }
        self.bindings = bindings;
        outcome
    }

    /// 轮询全局快捷键事件，返回触发的动作
    pub fn poll_events(&mut self) -> Vec<HotkeyAction> {
        let mut actions = Vec::new();
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if event.state() != global_hotkey::HotKeyState::Pressed {
                continue;
            }
            if let Some(binding_id) = self.binding_of.get(&event.id()) {
                if let Some(binding) = self.bindings.iter().find(|b| b.id == *binding_id) {
                    debug!("触发全局快捷键: {}（{}）", binding.spec, binding.id);
                    actions.push(binding.action.clone());
                }
            }
        }
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 解析与格式化往返一致
    #[test]
    fn parse_format_roundtrip() {
        for spec in [
            "Ctrl+Alt+1",
            "ctrl-alt-1",
            "Shift+Win+F5",
            "Alt+2",
            "Ctrl+Shift+F12",
        ] {
            let parsed = HotkeySpec::parse(spec).unwrap();
            let formatted = parsed.format();
            let reparsed = HotkeySpec::parse(&formatted).unwrap();
            assert_eq!(parsed, reparsed, "往返解析不一致: {spec} → {formatted}");
        }
    }

    /// 规范格式输出
    #[test]
    fn canonical_format() {
        assert_eq!(
            HotkeySpec::parse("1+alt+ctrl").unwrap().format(),
            "Ctrl+Alt+1"
        );
        assert_eq!(
            HotkeySpec::parse("F12+shift").unwrap().format(),
            "Shift+F12"
        );
    }

    /// 非法输入必须报错
    #[test]
    fn invalid_specs_rejected() {
        for bad in [
            "",
            "Ctrl+",
            "+",
            "Ctrl+Alt+Nothing",
            "Foo+Bar",
            "Ctrl+Ctrl+1",
        ] {
            assert!(HotkeySpec::parse(bad).is_err(), "应拒绝: {bad:?}");
        }
    }
}
