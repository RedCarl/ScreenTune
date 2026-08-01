//! # 配置方案（Profile）
//!
//! 一套完整的显示参数快照 + 元信息，保存于 profiles/*.json。

use screen_tune_common::DisplayParams;
use serde::{Deserialize, Serialize};

/// 一个配置方案
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    /// 方案唯一标识（文件名即 `{id}.json`，需为合法文件名字符）
    pub id: String,
    /// 方案显示名称
    pub name: String,
    /// 方案描述（可选）
    #[serde(default)]
    pub description: String,
    /// 方案对应的显示参数
    pub params: DisplayParams,
    /// 可选：方案专属快捷键（字符串，如 `Ctrl+Alt+2`）；为空表示无
    #[serde(default)]
    pub hotkey: Option<String>,
    /// 是否为内置方案（内置方案不可删除，可修改参数）
    #[serde(default)]
    pub builtin: bool,
}

impl Profile {
    /// 构造一个新方案
    pub fn new(id: impl Into<String>, name: impl Into<String>, params: DisplayParams) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            params,
            hotkey: None,
            builtin: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 方案序列化往返无损
    #[test]
    fn profile_serde_roundtrip() {
        let p = Profile::new("test", "测试方案", DisplayParams::default());
        let json = serde_json::to_string_pretty(&p).unwrap();
        let back: Profile = serde_json::from_str(&json).unwrap();
        assert_eq!(p, back);
        // 新字段（builtin）缺失时也能反序列化（向后兼容）
        let old_json = r#"{"id":"a","name":"A","description":"","params":{"gamma":100.0,"brightness":100.0,"contrast":100.0,"saturation":100.0,"temperature_k":6500},"hotkey":null}"#;
        let parsed: Profile = serde_json::from_str(old_json).unwrap();
        assert!(!parsed.builtin);
    }
}
