//! # 配置存储
//!
//! 负责定位标准目录并完成 config.json / profiles/*.json 的读写。
//! 目录约定（Windows 示例）：
//! - 配置目录：`%APPDATA%\ScreenTune\`
//!   - `config.json`        全局配置
//!   - `profiles\*.json`    配置方案
//! - 数据目录：`%LOCALAPPDATA%\ScreenTune\`
//!   - `logs\`              日志
//!   - `gamma_backup\`      原始 LUT 备份（崩溃恢复用）
//!
//! macOS（开发环境）下对应 `~/Library/Application Support/ScreenTune`。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use tracing::{debug, warn};

use crate::{AppConfig, Profile};

/// 配置存储：封装全部文件 IO
#[derive(Clone)]
pub struct ConfigStore {
    /// 配置目录（config.json 与 profiles/）
    config_dir: PathBuf,
    /// 数据目录（logs/ 与 gamma_backup/）
    data_dir: PathBuf,
}

impl ConfigStore {
    /// 定位标准目录并创建目录结构
    pub fn init() -> Result<Self> {
        let dirs = ProjectDirs::from("", "", "ScreenTune").context("无法定位系统标准目录")?;
        Self::init_in_full(
            dirs.config_dir().to_path_buf(),
            dirs.data_local_dir().to_path_buf(),
        )
    }

    /// 使用指定目录初始化（测试与隔离环境使用）
    pub fn init_in(config_dir: PathBuf) -> Result<Self> {
        Self::init_in_full(config_dir.clone(), config_dir)
    }

    /// 使用独立的配置/数据目录初始化
    pub fn init_in_full(config_dir: PathBuf, data_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(config_dir.join("profiles"))
            .with_context(|| format!("无法创建配置目录: {}", config_dir.display()))?;
        std::fs::create_dir_all(data_dir.join("logs"))
            .with_context(|| format!("无法创建数据目录: {}", data_dir.display()))?;
        std::fs::create_dir_all(data_dir.join("gamma_backup"))
            .with_context(|| format!("无法创建备份目录: {}", data_dir.display()))?;

        debug!("配置目录: {}", config_dir.display());
        debug!("数据目录: {}", data_dir.display());
        Ok(Self {
            config_dir,
            data_dir,
        })
    }

    /// 配置目录路径
    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }

    /// 数据目录路径
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// 日志目录路径（由 app crate 创建）
    pub fn logs_dir(&self) -> PathBuf {
        self.data_dir.join("logs")
    }

    /// 原始 LUT 备份目录路径
    pub fn gamma_backup_dir(&self) -> PathBuf {
        self.data_dir.join("gamma_backup")
    }

    // ---------------------------------------------------------------
    // config.json
    // ---------------------------------------------------------------

    /// 加载全局配置；文件不存在或损坏时回退到默认值（损坏时保留坏文件为 .bak）
    pub fn load_config(&self) -> AppConfig {
        let path = self.config_dir.join("config.json");
        match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<AppConfig>(&text) {
                Ok(config) => {
                    debug!("已加载配置: {}", path.display());
                    config
                }
                Err(e) => {
                    warn!("配置文件损坏（{}），使用默认配置: {}", e, path.display());
                    let _ = std::fs::rename(&path, path.with_extension("json.bak"));
                    AppConfig::default()
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                debug!("配置文件不存在，使用默认配置: {}", path.display());
                AppConfig::default()
            }
            Err(e) => {
                warn!("读取配置失败（{}），使用默认配置", e);
                AppConfig::default()
            }
        }
    }

    /// 保存全局配置（原子写入：先写临时文件再重命名）
    pub fn save_config(&self, config: &AppConfig) -> Result<()> {
        let path = self.config_dir.join("config.json");
        let tmp = self.config_dir.join("config.json.tmp");
        let text = serde_json::to_string_pretty(config).context("序列化配置失败")?;
        std::fs::write(&tmp, text)
            .with_context(|| format!("写入临时配置失败: {}", tmp.display()))?;
        std::fs::rename(&tmp, &path)
            .with_context(|| format!("替换配置文件失败: {}", path.display()))?;
        debug!("配置已保存: {}", path.display());
        Ok(())
    }

    // ---------------------------------------------------------------
    // profiles/*.json
    // ---------------------------------------------------------------

    /// profiles 目录
    fn profiles_dir(&self) -> PathBuf {
        self.config_dir.join("profiles")
    }

    /// 列出目录中全部方案（按名称排序）
    pub fn list_profiles(&self) -> Vec<Profile> {
        let mut profiles = Vec::new();
        let dir = self.profiles_dir();
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(e) => {
                warn!("读取方案目录失败（{}）: {}", e, dir.display());
                return profiles;
            }
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            match std::fs::read_to_string(&path) {
                Ok(text) => match serde_json::from_str::<Profile>(&text) {
                    Ok(profile) => profiles.push(profile),
                    Err(e) => warn!("跳过损坏的方案文件 {}: {}", path.display(), e),
                },
                Err(e) => warn!("读取方案失败 {}: {}", path.display(), e),
            }
        }
        profiles.sort_by(|a, b| a.name.cmp(&b.name));
        profiles
    }

    /// 保存一个方案（原子写入）
    pub fn save_profile(&self, profile: &Profile) -> Result<()> {
        let path = self.profiles_dir().join(format!("{}.json", profile.id));
        let tmp = self.profiles_dir().join(format!("{}.json.tmp", profile.id));
        let text = serde_json::to_string_pretty(profile).context("序列化方案失败")?;
        std::fs::write(&tmp, text)
            .with_context(|| format!("写入临时方案失败: {}", tmp.display()))?;
        std::fs::rename(&tmp, &path)
            .with_context(|| format!("替换方案文件失败: {}", path.display()))?;
        Ok(())
    }

    /// 删除一个方案（内置方案调用前须先确认）
    pub fn delete_profile(&self, id: &str) -> Result<()> {
        let path = self.profiles_dir().join(format!("{}.json", id));
        if path.exists() {
            std::fs::remove_file(&path)
                .with_context(|| format!("删除方案文件失败: {}", path.display()))?;
        }
        Ok(())
    }

    /// 导入方案 JSON 文本；返回解析后的方案（调用方负责保存）
    pub fn import_profile_json(&self, json: &str) -> Result<Profile> {
        let profile: Profile = serde_json::from_str(json).context("方案 JSON 格式错误")?;
        if profile.id.is_empty()
            || profile
                .id
                .contains(['/', '\\', ':', '*', '?', '"', '<', '>', '|'])
        {
            anyhow::bail!("方案 id 包含非法字符: {:?}", profile.id);
        }
        Ok(profile)
    }

    /// 导出方案为格式化 JSON 文本
    pub fn export_profile_json(&self, profile: &Profile) -> Result<String> {
        serde_json::to_string_pretty(profile).context("序列化方案失败")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 使用临时目录验证方案导入导出与非法 id 校验
    #[test]
    fn import_export_and_validation() {
        let store = ConfigStore {
            config_dir: std::env::temp_dir().join("screen-tune-test-config"),
            data_dir: std::env::temp_dir().join("screen-tune-test-data"),
        };
        let profile = Profile::new("test", "测试", screen_tune_common::DisplayParams::default());
        let json = store.export_profile_json(&profile).unwrap();
        let back = store.import_profile_json(&json).unwrap();
        assert_eq!(profile, back);

        // 非法 id 必须被拒绝
        let bad = r#"{"id":"a/b","name":"x","description":"","params":{"gamma":100.0,"brightness":100.0,"contrast":100.0,"saturation":100.0,"temperature_k":6500},"hotkey":null,"builtin":false}"#;
        assert!(store.import_profile_json(bad).is_err());
    }
}
