//! # 国际化
//!
//! 轻量级双语字典（简体中文 / English），按 key 查找。
//! 全部界面文案集中于此，新增文案只需在两个字典各加一条。
//! 为 Roadmap 中的完整国际化预留了结构（扩展其他语言只需加字典）。

use screen_tune_config::Language;

/// 翻译句柄
#[derive(Debug, Clone)]
pub struct Tr {
    lang: Language,
}

impl Tr {
    /// 按语言创建翻译句柄
    pub fn new(lang: Language) -> Self {
        Self { lang }
    }

    /// 当前语言
    pub fn lang(&self) -> Language {
        self.lang
    }

    /// 翻译：按 key 查字典；未命中时原样返回 key
    pub fn t(&self, key: &str) -> String {
        let table = match self.lang {
            Language::Zh => ZH,
            Language::En => EN,
        };
        table
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, v)| v.to_string())
            .unwrap_or_else(|| key.to_string())
    }
}

/// 简体中文字典
const ZH: &[(&str, &str)] = &[
    // 应用
    ("app.title", "ScreenTune"),
    ("app.tagline", "Instant Display Control for Gamers"),
    ("app.version", "版本"),
    // 导航
    ("nav.display", "显示调节"),
    ("nav.profiles", "配置方案"),
    ("nav.hotkeys", "全局快捷键"),
    ("nav.settings", "设置"),
    ("nav.monitors", "显示器"),
    // 顶部栏
    ("header.monitor", "显示器"),
    ("header.apply_all", "同步到全部显示器"),
    ("header.current_profile", "当前方案"),
    ("header.custom", "自定义"),
    // 显示页
    ("display.gamma", "Gamma"),
    ("display.gamma_tip", "范围 50~150，默认 100。实时 Gamma 曲线调节。"),
    ("display.brightness", "亮度"),
    ("display.brightness_tip", "范围 0~100。支持 DDC/CI 时直控硬件，否则通过 Gamma 模拟。"),
    ("display.contrast", "对比度"),
    ("display.contrast_tip", "范围 50~150，默认 100。"),
    ("display.saturation", "饱和度"),
    ("display.saturation_tip", "范围 0~200%，默认 100%。模拟 NVIDIA Digital Vibrance。"),
    ("display.temperature", "色温"),
    ("display.temperature_tip", "范围 2500K~10000K，默认 6500K（D65）。"),
    ("display.restore", "恢复默认"),
    ("display.ddc", "DDC/CI 硬件直控"),
    ("display.gamma_mode", "Gamma 模拟"),
    ("display.no_monitor", "未检测到显示器"),
    ("display.reset_monitor", "恢复此显示器原始画面"),
    ("display.reset_monitor_tip", "将该显示器恢复到本程序启动前的原始 LUT 与亮度"),
    // 显示器页
    ("monitor.name", "名称"),
    ("monitor.resolution", "分辨率"),
    ("monitor.primary", "主显示器"),
    ("monitor.ddc", "DDC/CI"),
    ("monitor.brightness_now", "当前亮度"),
    ("monitor.refresh", "重新检测显示器"),
    ("monitor.refresh_tip", "显示器热插拔后点击刷新"),
    // 方案页
    ("profiles.title", "配置方案"),
    ("profiles.apply", "应用方案"),
    ("profiles.new", "新建方案"),
    ("profiles.delete", "删除"),
    ("profiles.import", "导入 JSON"),
    ("profiles.export", "导出 JSON"),
    ("profiles.name", "名称"),
    ("profiles.desc", "描述"),
    ("profiles.hotkey", "快捷键"),
    ("profiles.none", "无"),
    ("profiles.builtin", "内置"),
    ("profiles.applied", "已应用"),
    ("profiles.delete_confirm", "确定删除方案"),
    ("profiles.import_hint", "将方案 JSON 粘贴到此处，点击导入。"),
    ("profiles.exported", "已导出到剪贴板"),
    ("profiles.imported", "方案已导入"),
    ("profiles.enter_name", "请输入方案名称"),
    ("profiles.save", "保存"),
    ("profiles.cancel", "取消"),
    ("profiles.editing", "正在编辑"),
    ("profiles.desc_placeholder", "方案描述（可选）"),
    // 快捷键页
    ("hotkeys.title", "全局快捷键"),
    ("hotkeys.tip", "按下要设置的组合键（如 Ctrl+Alt+1）。系统级生效，游戏内也可用。"),
    ("hotkeys.edit", "修改"),
    ("hotkeys.capture_hint", "请按下新的组合键…（Esc 取消）"),
    ("hotkeys.conflict", "冲突"),
    ("hotkeys.action", "动作"),
    ("hotkeys.spec", "快捷键"),
    ("hotkeys.no_support", "当前平台不支持全局快捷键"),
    ("hotkeys.updated", "快捷键已更新"),
    ("hotkey.action.restore_default", "恢复默认"),
    ("hotkey.action.apply_profile", "应用方案"),
    ("hotkey.action.show_window", "显示主窗口"),
    // 设置页
    ("settings.title", "设置"),
    ("settings.startup", "开机自动启动"),
    ("settings.close_to_tray", "关闭窗口时最小化到托盘"),
    ("settings.language", "语言 / Language"),
    ("settings.theme", "主题"),
    ("settings.dark", "深色（Fluent Dark）"),
    ("settings.light", "浅色（Fluent Light）"),
    ("settings.game_detection", "游戏自动切换配置"),
    ("settings.game_detection_tip", "游戏启动时自动应用对应方案，退出后恢复默认。"),
    ("settings.game_poll", "检测间隔（秒）"),
    ("settings.game_rules", "游戏规则"),
    ("settings.rule_exe", "进程名"),
    ("settings.rule_profile", "应用方案"),
    ("settings.rule_enabled", "启用"),
    ("settings.add_rule", "添加规则"),
    ("settings.rule_placeholder", "如 rust.exe"),
    ("settings.log_level", "日志级别"),
    ("settings.about", "关于"),
    ("settings.roadmap", "Roadmap"),
    ("settings.roadmap_items", "HDR · ICC Profile · Night Light · AMD/Intel Color · NVIDIA API · Auto HDR · 命令行模式 · 插件系统 · Lua 脚本 · 自动更新 · 国际化"),
    ("settings.no_telemetry", "无遥测 · 无广告 · 无需联网 · 开源"),
    // 通用
    ("common.close", "关闭"),
    ("common.add", "添加"),
    ("common.delete", "删除"),
    ("common.off", "关闭"),
    ("common.on", "开启"),
    // 提示
    ("toast.hotkey_conflict", "以下快捷键被其他程序占用"),
    ("toast.game_profile", "游戏方案已应用："),
    ("toast.game_exited", "游戏退出，已恢复默认"),
    ("toast.restored", "已恢复默认参数"),
    ("toast.saved", "已保存"),
    ("toast.rule_saved", "游戏规则已保存"),
];

/// English dictionary
const EN: &[(&str, &str)] = &[
    ("app.title", "ScreenTune"),
    ("app.tagline", "Instant Display Control for Gamers"),
    ("app.version", "Version"),
    ("nav.display", "Display"),
    ("nav.profiles", "Profiles"),
    ("nav.hotkeys", "Hotkeys"),
    ("nav.settings", "Settings"),
    ("nav.monitors", "Monitors"),
    ("header.monitor", "Monitor"),
    ("header.apply_all", "Apply to all monitors"),
    ("header.current_profile", "Current profile"),
    ("header.custom", "Custom"),
    ("display.gamma", "Gamma"),
    ("display.gamma_tip", "50~150, default 100. Real-time gamma curve."),
    ("display.brightness", "Brightness"),
    ("display.brightness_tip", "0~100. DDC/CI hardware control when supported, otherwise gamma simulation."),
    ("display.contrast", "Contrast"),
    ("display.contrast_tip", "50~150, default 100."),
    ("display.saturation", "Saturation"),
    ("display.saturation_tip", "0~200%, default 100%. NVIDIA Digital Vibrance style simulation."),
    ("display.temperature", "Color Temperature"),
    ("display.temperature_tip", "2500K~10000K, default 6500K (D65)."),
    ("display.restore", "Restore defaults"),
    ("display.ddc", "DDC/CI hardware control"),
    ("display.gamma_mode", "Gamma simulation"),
    ("display.no_monitor", "No monitor detected"),
    ("display.reset_monitor", "Restore this monitor"),
    ("display.reset_monitor_tip", "Restore this monitor to the original LUT and brightness at app start"),
    ("monitor.name", "Name"),
    ("monitor.resolution", "Resolution"),
    ("monitor.primary", "Primary"),
    ("monitor.ddc", "DDC/CI"),
    ("monitor.brightness_now", "Brightness"),
    ("monitor.refresh", "Re-scan monitors"),
    ("monitor.refresh_tip", "Refresh after monitor hot-plug"),
    ("profiles.title", "Profiles"),
    ("profiles.apply", "Apply"),
    ("profiles.new", "New profile"),
    ("profiles.delete", "Delete"),
    ("profiles.import", "Import JSON"),
    ("profiles.export", "Export JSON"),
    ("profiles.name", "Name"),
    ("profiles.desc", "Description"),
    ("profiles.hotkey", "Hotkey"),
    ("profiles.none", "None"),
    ("profiles.builtin", "Built-in"),
    ("profiles.applied", "Applied"),
    ("profiles.delete_confirm", "Delete profile"),
    ("profiles.import_hint", "Paste profile JSON here, then click Import."),
    ("profiles.exported", "Exported to clipboard"),
    ("profiles.imported", "Profile imported"),
    ("profiles.enter_name", "Enter a profile name"),
    ("profiles.save", "Save"),
    ("profiles.cancel", "Cancel"),
    ("profiles.editing", "Editing"),
    ("profiles.desc_placeholder", "Description (optional)"),
    ("hotkeys.title", "Global Hotkeys"),
    ("hotkeys.tip", "Press the combination you want (e.g. Ctrl+Alt+1). Works system-wide, even in games."),
    ("hotkeys.edit", "Edit"),
    ("hotkeys.capture_hint", "Press a new combination... (Esc to cancel)"),
    ("hotkeys.conflict", "Conflict"),
    ("hotkeys.action", "Action"),
    ("hotkeys.spec", "Hotkey"),
    ("hotkeys.no_support", "Global hotkeys are not supported on this platform"),
    ("hotkeys.updated", "Hotkeys updated"),
    ("hotkey.action.restore_default", "Restore defaults"),
    ("hotkey.action.apply_profile", "Apply profile"),
    ("hotkey.action.show_window", "Show main window"),
    ("settings.title", "Settings"),
    ("settings.startup", "Launch at startup"),
    ("settings.close_to_tray", "Minimize to tray on close"),
    ("settings.language", "Language"),
    ("settings.theme", "Theme"),
    ("settings.dark", "Dark (Fluent Dark)"),
    ("settings.light", "Light (Fluent Light)"),
    ("settings.game_detection", "Game Auto-Switch"),
    ("settings.game_detection_tip", "Apply the matching profile when a game starts; restore defaults on exit."),
    ("settings.game_poll", "Poll interval (s)"),
    ("settings.game_rules", "Game rules"),
    ("settings.rule_exe", "Process"),
    ("settings.rule_profile", "Profile"),
    ("settings.rule_enabled", "Enabled"),
    ("settings.add_rule", "Add rule"),
    ("settings.rule_placeholder", "e.g. rust.exe"),
    ("settings.log_level", "Log level"),
    ("settings.about", "About"),
    ("settings.roadmap", "Roadmap"),
    ("settings.roadmap_items", "HDR · ICC Profile · Night Light · AMD/Intel Color · NVIDIA API · Auto HDR · CLI mode · Plugin system · Lua scripts · Auto update · i18n"),
    ("settings.no_telemetry", "No telemetry · No ads · Fully offline · Open source"),
    ("common.close", "Close"),
    ("common.add", "Add"),
    ("common.delete", "Delete"),
    ("common.off", "Off"),
    ("common.on", "On"),
    ("toast.hotkey_conflict", "Hotkey conflicts with another program"),
    ("toast.game_profile", "Game profile applied: "),
    ("toast.game_exited", "Game exited, defaults restored"),
    ("toast.restored", "Defaults restored"),
    ("toast.saved", "Saved"),
    ("toast.rule_saved", "Game rules saved"),
];

#[cfg(test)]
mod tests {
    use super::*;

    /// 中英文典 key 集合必须完全一致
    #[test]
    fn dictionaries_are_parallel() {
        let zh_keys: Vec<_> = ZH.iter().map(|(k, _)| *k).collect();
        let en_keys: Vec<_> = EN.iter().map(|(k, _)| *k).collect();
        assert_eq!(zh_keys, en_keys, "中英文典 key 不一致");
    }

    /// 未知 key 原样返回
    #[test]
    fn unknown_key_passthrough() {
        let tr = Tr::new(Language::Zh);
        assert_eq!(tr.t("no.such.key"), "no.such.key");
    }
}
