//! # 原始 LUT 备份
//!
//! 启动时把每个显示器的原始 Gamma Ramp 持久化到 `gamma_backup/`，
//! 退出时恢复原始 LUT 并删除备份。
//! 若程序异常退出（崩溃 / 杀进程），下次启动会发现残留备份，
//! 先写回原始 LUT 再读取基线——保证显示器永远能恢复出厂状态。
//!
//! 文件格式（自定义二进制，零依赖）：
//! ```text
//! 4B  magic   "STGR"
//! 4B  version u32 LE (=1)
//! 2B  id_len  u16 LE
//! N   monitor id UTF-8
//! 3072B      ramp 1536 × u16 LE
//! ```

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use tracing::{debug, warn};

use crate::backend::GAMMA_RAMP_LEN;

/// 备份文件魔数
const MAGIC: &[u8; 4] = b"STGR";
/// 当前格式版本
const VERSION: u32 = 1;

/// 写入（覆盖）一个显示器的基线备份
pub fn save_baseline(
    backup_dir: &Path,
    monitor_id: &str,
    ramp: &[u16; GAMMA_RAMP_LEN],
) -> Result<()> {
    let path = backup_path(backup_dir, monitor_id);
    let mut buf = Vec::with_capacity(4 + 4 + 2 + monitor_id.len() + GAMMA_RAMP_LEN * 2);
    buf.extend_from_slice(MAGIC);
    buf.extend_from_slice(&VERSION.to_le_bytes());
    let id_bytes = monitor_id.as_bytes();
    buf.extend_from_slice(&(id_bytes.len() as u16).to_le_bytes());
    buf.extend_from_slice(id_bytes);
    for v in ramp {
        buf.extend_from_slice(&v.to_le_bytes());
    }
    let tmp = path.with_extension("bin.tmp");
    std::fs::write(&tmp, &buf).with_context(|| format!("写入 LUT 备份失败: {}", tmp.display()))?;
    std::fs::rename(&tmp, &path)
        .with_context(|| format!("替换 LUT 备份失败: {}", path.display()))?;
    Ok(())
}

/// 读取一个显示器的基线备份；不存在或损坏时返回 None
pub fn load_baseline(backup_dir: &Path, monitor_id: &str) -> Option<[u16; GAMMA_RAMP_LEN]> {
    let path = backup_path(backup_dir, monitor_id);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(_) => return None,
    };
    match decode(&bytes, monitor_id) {
        Ok(ramp) => Some(ramp),
        Err(e) => {
            warn!("LUT 备份损坏，忽略: {}: {}", path.display(), e);
            None
        }
    }
}

/// 删除一个显示器的基线备份
pub fn remove_baseline(backup_dir: &Path, monitor_id: &str) {
    let path = backup_path(backup_dir, monitor_id);
    match std::fs::remove_file(&path) {
        Ok(()) => debug!("已删除 LUT 备份: {}", path.display()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => warn!("删除 LUT 备份失败（{}）: {}", e, path.display()),
    }
}

/// 是否存在残留备份（用于判断上次是否异常退出）
pub fn has_baseline(backup_dir: &Path, monitor_id: &str) -> bool {
    backup_path(backup_dir, monitor_id).exists()
}

/// 备份文件路径
fn backup_path(backup_dir: &Path, monitor_id: &str) -> PathBuf {
    // monitor id（如 \\.\DISPLAY1）含特殊字符，用十六进制编码做文件名
    let mut hex = String::with_capacity(monitor_id.len() * 2);
    for b in monitor_id.as_bytes() {
        hex.push_str(&format!("{:02x}", b));
    }
    backup_dir.join(format!("{hex}.bin"))
}

/// 解码备份文件
fn decode(bytes: &[u8], expected_id: &str) -> Result<[u16; GAMMA_RAMP_LEN]> {
    if bytes.len() < 4 + 4 + 2 {
        anyhow::bail!("文件过短");
    }
    if &bytes[0..4] != MAGIC {
        anyhow::bail!("魔数不匹配");
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if version != VERSION {
        anyhow::bail!("不支持的文件版本: {version}");
    }
    let id_len = u16::from_le_bytes(bytes[8..10].try_into().unwrap()) as usize;
    if bytes.len() < 10 + id_len + GAMMA_RAMP_LEN * 2 {
        anyhow::bail!("文件长度不足");
    }
    let id = std::str::from_utf8(&bytes[10..10 + id_len]).context("id 编码无效")?;
    if id != expected_id {
        anyhow::bail!("备份属于其他显示器: {id}");
    }
    let mut ramp = [0u16; GAMMA_RAMP_LEN];
    let data = &bytes[10 + id_len..];
    for (i, chunk) in data.chunks_exact(2).enumerate() {
        ramp[i] = u16::from_le_bytes([chunk[0], chunk[1]]);
    }
    Ok(ramp)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 备份往返无损；错误显示器 id 必须拒绝
    #[test]
    fn baseline_roundtrip() {
        let dir = std::env::temp_dir().join("screen-tune-persist-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let mut ramp = [0u16; GAMMA_RAMP_LEN];
        for (i, v) in ramp.iter_mut().enumerate() {
            *v = (i * 37) as u16 % 65535;
        }
        save_baseline(&dir, r"\\.\DISPLAY1", &ramp).unwrap();
        assert!(has_baseline(&dir, r"\\.\DISPLAY1"));
        let back = load_baseline(&dir, r"\\.\DISPLAY1").unwrap();
        assert_eq!(ramp, back);

        // 用错误 id 读取应返回 None（不匹配）
        assert!(load_baseline(&dir, r"\\.\DISPLAY2").is_none());

        // 覆盖写再读
        ramp[0] = 12345;
        save_baseline(&dir, r"\\.\DISPLAY1", &ramp).unwrap();
        let back2 = load_baseline(&dir, r"\\.\DISPLAY1").unwrap();
        assert_eq!(ramp, back2);

        remove_baseline(&dir, r"\\.\DISPLAY1");
        assert!(!has_baseline(&dir, r"\\.\DISPLAY1"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// 损坏文件应被识别并返回 None
    #[test]
    fn corrupt_file_returns_none() {
        let dir = std::env::temp_dir().join("screen-tune-persist-test-2");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("41414141.bin");
        std::fs::write(&path, b"garbage").unwrap();
        assert!(load_baseline(&dir, "AAAA").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
