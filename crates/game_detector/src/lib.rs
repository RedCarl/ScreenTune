//! # 游戏进程检测
//!
//! 周期枚举系统进程，与规则表（exe 名 → 配置方案）匹配：
//! - 检测到匹配进程 → `Entered`（应用对应方案）；
//! - 匹配进程全部退出 → `Exited`（恢复默认方案）。
//!
//! 规则表可扩展：未来可增加按窗口标题、进程路径等匹配维度。
//! 检测轮询本身很轻量（约 2 秒一次快照），空闲 CPU 占用接近 0。

use std::sync::RwLock;

use screen_tune_config::GameRule;
use tracing::debug;

/// 检测事件
#[derive(Debug, Clone, PartialEq)]
pub enum DetectorEvent {
    /// 检测到游戏进程启动，应应用方案
    Entered {
        /// 命中的规则 id
        rule_id: String,
        /// 应应用的方案 id
        profile_id: String,
    },
    /// 游戏进程已退出，应恢复默认
    Exited {
        /// 之前命中的规则 id
        rule_id: String,
    },
}

/// 进程列表提供者（便于注入 mock 进行测试）
pub trait ProcessLister: Send + Sync {
    /// 返回当前全部进程的可执行文件名（统一小写）。
    /// 枚举失败时应返回空列表（调用方视作「无游戏进程」）。
    fn running_exes(&self) -> Vec<String>;
}

/// 创建平台默认的进程列表提供者
pub fn default_lister() -> Box<dyn ProcessLister> {
    #[cfg(windows)]
    {
        Box::new(win32::ToolhelpLister)
    }
    #[cfg(not(windows))]
    {
        Box::new(MockLister::new(Vec::new()))
    }
}

/// 游戏检测器（状态机）
pub struct GameDetector {
    /// 进程枚举后端
    lister: Box<dyn ProcessLister>,
    /// 规则表（顺序即优先级：第一条命中生效）
    rules: RwLock<Vec<GameRule>>,
    /// 当前生效的规则（rule_id, profile_id）
    active: Option<(String, String)>,
}

impl GameDetector {
    /// 创建检测器
    pub fn new(lister: Box<dyn ProcessLister>, rules: Vec<GameRule>) -> Self {
        Self {
            lister,
            rules: RwLock::new(rules),
            active: None,
        }
    }

    /// 替换进程列表提供者（测试用）
    pub fn set_lister(&mut self, lister: Box<dyn ProcessLister>) {
        self.lister = lister;
    }

    /// 更新规则表（配置变更时调用）
    pub fn set_rules(&self, rules: Vec<GameRule>) {
        *self.rules.write().unwrap() = rules;
    }

    /// 当前命中的规则（(rule_id, profile_id)）
    pub fn active_rule(&self) -> Option<(String, String)> {
        self.active.clone()
    }

    /// 执行一次检测（周期调用）
    pub fn tick(&mut self) -> Option<DetectorEvent> {
        let exes = self.lister.running_exes();
        if exes.is_empty() {
            // 空列表（无进程 / 枚举失败）：视作无游戏进程，走正常状态机
            return self.advance(None);
        }

        let rules = self.rules.read().unwrap();
        let matched = rules
            .iter()
            .find(|r| exes.iter().any(|e| r.matches(e)))
            .cloned();
        drop(rules);
        self.advance(matched)
    }

    /// 状态机推进：根据（可能命中的规则）产生事件
    fn advance(&mut self, matched: Option<GameRule>) -> Option<DetectorEvent> {
        // 先取出旧的生效规则（避免借用冲突）
        let old_active = self.active.clone();
        match (old_active, matched) {
            // 无变化
            (Some((rid, _)), Some(rule)) if rid == rule.id => None,
            // 从无 → 有
            (None, Some(rule)) => {
                debug!("检测到游戏进程 {}（规则 {}）", rule.exe_name, rule.id);
                self.active = Some((rule.id.clone(), rule.profile_id.clone()));
                Some(DetectorEvent::Entered {
                    rule_id: rule.id,
                    profile_id: rule.profile_id,
                })
            }
            // 从一个规则 → 另一个规则（直接切换）
            (Some(old_rid), Some(rule)) => {
                debug!("游戏规则切换: {} → {}", old_rid.0, rule.id);
                self.active = Some((rule.id.clone(), rule.profile_id.clone()));
                Some(DetectorEvent::Entered {
                    rule_id: rule.id,
                    profile_id: rule.profile_id,
                })
            }
            // 从有 → 无
            (Some((rid, _)), None) => {
                debug!("游戏进程已退出（规则 {}），恢复默认", rid);
                self.active = None;
                Some(DetectorEvent::Exited { rule_id: rid })
            }
            // 无 → 无
            (None, None) => None,
        }
    }
}

/// Mock 进程列表（测试 / 非 Windows 开发）
#[derive(Debug, Default, Clone)]
pub struct MockLister {
    exes: Vec<String>,
}

impl MockLister {
    /// 以初始进程列表创建
    pub fn new(exes: Vec<String>) -> Self {
        Self {
            exes: exes.into_iter().map(|s| s.to_lowercase()).collect(),
        }
    }

    /// 更新进程列表（模拟游戏启动 / 退出）
    pub fn set_processes(&mut self, exes: Vec<String>) {
        self.exes = exes.into_iter().map(|s| s.to_lowercase()).collect();
    }
}

impl ProcessLister for MockLister {
    fn running_exes(&self) -> Vec<String> {
        self.exes.clone()
    }
}

/// Win32 实现（Toolhelp 快照）
#[cfg(windows)]
pub mod win32 {
    use super::*;
    use std::mem::size_of;
    use tracing::warn;
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    /// 基于 Toolhelp 快照的进程列表
    pub struct ToolhelpLister;

    impl ProcessLister for ToolhelpLister {
        fn running_exes(&self) -> Vec<String> {
            let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
            let Ok(snapshot) = snapshot else {
                warn!("CreateToolhelp32Snapshot 失败");
                return Vec::new();
            };
            if snapshot.0.is_null() {
                warn!("CreateToolhelp32Snapshot 返回无效句柄");
                return Vec::new();
            }

            let mut out = Vec::new();
            let mut entry = PROCESSENTRY32W {
                dwSize: size_of::<PROCESSENTRY32W>() as u32,
                ..Default::default()
            };

            // 首进程
            if unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok() {
                collect(&mut out, &entry);
                // 后续进程
                loop {
                    let next = unsafe { Process32NextW(snapshot, &mut entry) };
                    if !next.as_bool() {
                        break;
                    }
                    collect(&mut out, &entry);
                }
            }
            let _ = unsafe { CloseHandle(snapshot) };
            out
        }
    }

    /// 提取 exe 名（小写）
    fn collect(out: &mut Vec<String>, entry: &PROCESSENTRY32W) {
        let name = wide_string(&entry.szExeFile);
        if !name.is_empty() {
            out.push(name.to_lowercase());
        }
    }

    /// 宽字符缓冲区 → String
    fn wide_string(buf: &[u16]) -> String {
        let len = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detector(rules: Vec<GameRule>) -> GameDetector {
        GameDetector::new(Box::new(MockLister::new(Vec::new())), rules)
    }

    /// 进程启动 → Entered；进程退出 → Exited；期间无重复事件
    #[test]
    fn detect_enter_and_exit() {
        let mut detector = detector(vec![GameRule::new("r1", "rust.exe", "rust")]);
        assert_eq!(detector.tick(), None, "无进程时无事件");

        // 游戏启动
        detector.set_lister(Box::new(MockLister::new(vec![
            "notepad.exe".into(),
            "rust.exe".into(),
        ])));
        let ev = detector.tick().unwrap();
        assert_eq!(
            ev,
            DetectorEvent::Entered {
                rule_id: "r1".into(),
                profile_id: "rust".into()
            }
        );
        // 重复 tick 无事件
        assert_eq!(detector.tick(), None);

        // 游戏退出
        detector.set_lister(Box::new(MockLister::new(vec!["notepad.exe".into()])));
        let ev = detector.tick().unwrap();
        assert_eq!(
            ev,
            DetectorEvent::Exited {
                rule_id: "r1".into()
            }
        );
        assert_eq!(detector.tick(), None);
    }

    /// 多规则时按规则表顺序匹配；规则可热更新
    #[test]
    fn rule_priority_and_update() {
        let rules = vec![
            GameRule::new("r1", "rust.exe", "rust"),
            GameRule::new("r2", "cs2.exe", "cs2"),
        ];
        let mut detector = detector(rules);

        // 同时运行两个游戏 → 命中第一条
        detector.set_lister(Box::new(MockLister::new(vec![
            "cs2.exe".into(),
            "rust.exe".into(),
        ])));
        let ev = detector.tick().unwrap();
        assert_eq!(
            ev,
            DetectorEvent::Entered {
                rule_id: "r1".into(),
                profile_id: "rust".into()
            }
        );

        // 热更新规则：把 cs2 提到前面 → 发生切换
        detector.set_rules(vec![
            GameRule::new("r2", "cs2.exe", "cs2"),
            GameRule::new("r1", "rust.exe", "rust"),
        ]);
        let ev = detector.tick().unwrap();
        assert_eq!(
            ev,
            DetectorEvent::Entered {
                rule_id: "r2".into(),
                profile_id: "cs2".into()
            }
        );

        // 全部退出 → Exited
        detector.set_lister(Box::new(MockLister::new(Vec::new())));
        let ev = detector.tick().unwrap();
        assert_eq!(
            ev,
            DetectorEvent::Exited {
                rule_id: "r2".into()
            }
        );
    }

    /// 规则匹配不区分大小写（PUBG.exe）
    #[test]
    fn case_insensitive_matching() {
        let mut detector = detector(vec![GameRule::new("pubg", "PUBG.exe", "pubg")]);
        detector.set_lister(Box::new(MockLister::new(vec!["PUBG.exe".into()])));
        let ev = detector.tick().unwrap();
        assert_eq!(
            ev,
            DetectorEvent::Entered {
                rule_id: "pubg".into(),
                profile_id: "pubg".into()
            }
        );
    }
}
