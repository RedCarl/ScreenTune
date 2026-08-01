//! # 配置方案管理
//!
//! 提供方案的业务层操作：加载、增删改查、导入导出，
//! 以及内置示例方案（默认 / Rust / CS2 / PUBG / 办公 / 夜间）。
//!
//! 本 crate 不直接操作显示器；「应用方案」由 app 层的 AppCore 编排
//! （方案 → DisplayManager 下发）。

use anyhow::{Context, Result};
use screen_tune_common::DisplayParams;
use screen_tune_config::{ConfigStore, Profile};
use tracing::debug;

/// 内置方案 id
pub mod builtin {
    /// 默认方案 id
    pub const DEFAULT: &str = "default";
    /// Rust 游戏方案 id
    pub const RUST: &str = "rust";
    /// CS2 游戏方案 id
    pub const CS2: &str = "cs2";
    /// PUBG 游戏方案 id
    pub const PUBG: &str = "pubg";
    /// 办公方案 id
    pub const OFFICE: &str = "office";
    /// 夜间方案 id
    pub const NIGHT: &str = "night";
}

/// 方案管理器
pub struct ProfileManager {
    store: ConfigStore,
    /// 全部方案（含内置；内置方案仅存在于内存，不落盘）
    profiles: Vec<Profile>,
}

impl ProfileManager {
    /// 创建管理器并加载全部方案（目录中已有的 + 内置方案）
    pub fn init(store: &ConfigStore) -> Result<Self> {
        let mut profiles = store.list_profiles();
        let builtin = builtin_profiles();
        for b in builtin {
            // 用户同名方案优先（允许用户自定义内置方案的参数）
            if !profiles.iter().any(|p| p.id == b.id) {
                profiles.push(b);
            }
        }
        profiles.sort_by(|a, b| a.name.cmp(&b.name));
        debug!("已加载 {} 个配置方案", profiles.len());
        Ok(Self {
            store: store.clone(),
            profiles,
        })
    }

    /// 全部方案列表（按名称排序）
    pub fn list(&self) -> &[Profile] {
        &self.profiles
    }

    /// 按 id 查找方案
    pub fn get(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    /// 是否存在指定 id 的方案
    pub fn exists(&self, id: &str) -> bool {
        self.profiles.iter().any(|p| p.id == id)
    }

    /// 新增方案（id 冲突时报错）
    pub fn create(&mut self, profile: Profile) -> Result<()> {
        if self.exists(&profile.id) {
            anyhow::bail!("方案 id 已存在: {}", profile.id);
        }
        if profile.builtin {
            anyhow::bail!("内置方案不可重复创建");
        }
        self.store.save_profile(&profile)?;
        self.profiles.push(profile);
        self.profiles.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(())
    }

    /// 更新方案（不存在则创建；内置方案仅更新内存中的参数并落盘同名文件）
    pub fn update(&mut self, profile: Profile) -> Result<()> {
        if let Some(existing) = self.profiles.iter().find(|p| p.id == profile.id) {
            if existing.builtin {
                // 内置方案：允许修改参数与名称，但标记位保持内置
                let mut p = profile;
                p.builtin = true;
                self.store.save_profile(&p)?;
                if let Some(slot) = self.profiles.iter_mut().find(|x| x.id == p.id) {
                    *slot = p;
                }
                return Ok(());
            }
        }
        self.store.save_profile(&profile)?;
        if let Some(slot) = self.profiles.iter_mut().find(|p| p.id == profile.id) {
            *slot = profile;
        } else {
            self.profiles.push(profile);
        }
        self.profiles.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(())
    }

    /// 删除方案（内置方案拒绝删除）
    pub fn delete(&mut self, id: &str) -> Result<()> {
        let Some(profile) = self.profiles.iter().find(|p| p.id == id) else {
            anyhow::bail!("方案不存在: {id}");
        };
        if profile.builtin {
            anyhow::bail!("内置方案不可删除");
        }
        self.store.delete_profile(id)?;
        self.profiles.retain(|p| p.id != id);
        Ok(())
    }

    /// 导入方案 JSON；id 冲突时自动追加后缀。返回导入后的方案。
    pub fn import_json(&mut self, json: &str) -> Result<Profile> {
        let mut profile = self.store.import_profile_json(json)?;
        if self.exists(&profile.id) {
            let base = profile.id.clone();
            let mut n = 2;
            while self.exists(&profile.id) {
                profile.id = format!("{base}-{n}");
                n += 1;
            }
        }
        self.store.save_profile(&profile)?;
        self.profiles.push(profile.clone());
        self.profiles.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(profile)
    }

    /// 导出方案 JSON
    pub fn export_json(&self, id: &str) -> Result<String> {
        let profile = self.get(id).with_context(|| format!("方案不存在: {id}"))?;
        self.store.export_profile_json(profile)
    }
}

// ---------------------------------------------------------------
// 内置示例方案
// ---------------------------------------------------------------

/// 内置示例方案集合（含参数建议值，可自由修改）
pub fn builtin_profiles() -> Vec<Profile> {
    fn make(id: &str, name: &str, desc: &str, params: DisplayParams) -> Profile {
        Profile {
            id: id.to_string(),
            name: name.to_string(),
            description: desc.to_string(),
            params,
            hotkey: None,
            builtin: true,
        }
    }

    let p = |g: f32, b: f32, c: f32, s: f32, k: u32| DisplayParams {
        gamma: g,
        brightness: b,
        contrast: c,
        saturation: s,
        temperature_k: k,
    };

    vec![
        make(
            builtin::DEFAULT,
            "默认",
            "无任何调整的原始显示参数",
            p(100.0, 100.0, 100.0, 100.0, 6500),
        ),
        make(
            builtin::RUST,
            "Rust",
            "提升可见度，利于远距离观察",
            p(110.0, 90.0, 105.0, 120.0, 6000),
        ),
        make(
            builtin::CS2,
            "CS2",
            "增强对比，暗处细节更清晰",
            p(105.0, 95.0, 110.0, 115.0, 6200),
        ),
        make(
            builtin::PUBG,
            "PUBG",
            "鲜艳画面，快速发现敌人",
            p(115.0, 85.0, 115.0, 130.0, 5800),
        ),
        make(
            builtin::OFFICE,
            "办公",
            "柔和护眼的日常参数",
            p(100.0, 85.0, 100.0, 95.0, 6500),
        ),
        make(
            builtin::NIGHT,
            "夜间",
            "低亮度暖色调，夜间使用",
            p(90.0, 55.0, 95.0, 90.0, 4000),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造一个隔离的临时管理器（不触碰真实配置目录）
    fn temp_manager() -> ProfileManager {
        let dir =
            std::env::temp_dir().join(format!("screen-tune-profile-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let store = ConfigStore::init_in(dir.clone()).unwrap();
        ProfileManager::init(&store).unwrap()
    }

    /// 内置方案必须完整：默认 / Rust / CS2 / PUBG / 办公 / 夜间
    #[test]
    fn builtin_profiles_are_complete() {
        let profiles = builtin_profiles();
        let ids: Vec<_> = profiles.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                builtin::DEFAULT,
                builtin::RUST,
                builtin::CS2,
                builtin::PUBG,
                builtin::OFFICE,
                builtin::NIGHT
            ]
        );
        // 默认方案必须是无调整
        let def = profiles.iter().find(|p| p.id == builtin::DEFAULT).unwrap();
        assert!(def.params.is_default());
        // 游戏方案的参数必须实际有效（不等于默认）
        let rust = profiles.iter().find(|p| p.id == builtin::RUST).unwrap();
        assert!(!rust.params.is_default());
    }

    /// 方案 CRUD：创建 → 查询 → 更新 → 删除
    #[test]
    fn profile_crud() {
        let mut m = temp_manager();
        let p = Profile::new("my-profile", "我的方案", DisplayParams::default());
        m.create(p.clone()).unwrap();
        assert!(m.exists("my-profile"));
        // 重复创建必须失败
        assert!(m.create(p.clone()).is_err());

        let mut updated = p;
        updated.params.gamma = 120.0;
        m.update(updated.clone()).unwrap();
        assert_eq!(m.get("my-profile").unwrap().params.gamma, 120.0);

        m.delete("my-profile").unwrap();
        assert!(!m.exists("my-profile"));
        // 内置方案不可删除
        assert!(m.delete(builtin::RUST).is_err());
    }

    /// 导入 JSON：正常导入 + id 冲突自动改名
    #[test]
    fn import_handles_conflict() {
        let mut m = temp_manager();
        let json = m.export_json(builtin::RUST).unwrap();
        // 导入与内置同 id → 自动加后缀
        let imported = m.import_json(&json).unwrap();
        assert!(imported.id.starts_with("rust"));
        assert_ne!(imported.id, builtin::RUST);
        assert!(m.exists(&imported.id));
        // 非法 JSON 必须报错
        assert!(m.import_json("{not json}").is_err());
    }
}
