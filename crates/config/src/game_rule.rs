//! # 游戏自动切换规则
//!
//! 一条规则表示：当指定进程（如 `rust.exe`）运行时，自动应用某配置方案；
//! 进程退出后恢复「默认」方案。规则表保存在 config.json 中，便于后续扩展
//! （例如将来支持按窗口标题 / 进程路径匹配）。

use serde::{Deserialize, Serialize};

/// 一条游戏进程 → 配置方案 的自动切换规则
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameRule {
    /// 规则唯一标识
    pub id: String,
    /// 进程可执行文件名（不区分大小写，如 `rust.exe`、`cs2.exe`、`PUBG.exe`）
    pub exe_name: String,
    /// 命中所应用的配置方案 id
    pub profile_id: String,
    /// 是否启用
    pub enabled: bool,
}

impl GameRule {
    /// 构造一条规则
    pub fn new(
        id: impl Into<String>,
        exe_name: impl Into<String>,
        profile_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            exe_name: exe_name.into(),
            profile_id: profile_id.into(),
            enabled: true,
        }
    }

    /// 判断给定进程名是否匹配本规则（不区分大小写）
    pub fn matches(&self, exe_name: &str) -> bool {
        self.enabled && self.exe_name.to_lowercase() == exe_name.to_lowercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 规则匹配不区分大小写
    #[test]
    fn rule_matches_case_insensitively() {
        let rule = GameRule::new("rust", "Rust.exe", "rust");
        assert!(rule.matches("rust.exe"));
        assert!(rule.matches("RUST.EXE"));
        assert!(!rule.matches("rust2.exe"));
        assert!(!rule.matches("cs2.exe"));
    }

    /// 禁用的规则不参与匹配
    #[test]
    fn disabled_rule_never_matches() {
        let mut rule = GameRule::new("cs2", "cs2.exe", "cs2");
        rule.enabled = false;
        assert!(!rule.matches("cs2.exe"));
    }
}
