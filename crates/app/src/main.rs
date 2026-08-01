//! # ScreenTune 主程序入口
//!
//! 组装全部服务并启动 GUI：
//! 1. 定位标准目录、加载配置；
//! 2. 初始化 tracing 日志（logs/ 文件 + 终端双输出）；
//! 3. 初始化服务组合层（显示引擎 / 方案 / 基线恢复）；
//! 4. 启动 tokio 运行时（游戏检测任务）与 eframe 事件循环；
//! 5. 退出时恢复全部显示器的原始 LUT 并保存配置。

use std::sync::Arc;

use anyhow::{Context, Result};
use screen_tune_config::ConfigStore;
use screen_tune_ui::AppCore;
use tracing::{info, warn};
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    // -----------------------------------------------------------
    // 1. 配置存储与加载
    // -----------------------------------------------------------
    let store = ConfigStore::init().context("初始化配置目录失败")?;
    let config = store.load_config();

    // -----------------------------------------------------------
    // 2. 日志：文件（logs/）+ 终端双输出
    // -----------------------------------------------------------
    let _log_guard = init_tracing(&store, &config.log_level);

    info!(
        "ScreenTune {} 启动（配置目录: {}）",
        screen_tune_common::consts::APP_VERSION,
        store.config_dir().display()
    );

    // -----------------------------------------------------------
    // 3. 服务组合层（显示引擎基线 / 方案 / 恢复上次参数）
    // -----------------------------------------------------------
    let core = AppCore::init(store, config).context("初始化核心服务失败")?;

    // -----------------------------------------------------------
    // 4. tokio 运行时 + eframe 事件循环
    //    游戏检测任务依赖运行时；eframe 在 block_on 内运行以保证
    //    `tokio::runtime::Handle::current()` 可用。
    // -----------------------------------------------------------
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .build()
        .context("创建 tokio 运行时失败")?;

    let core_for_eframe = Arc::clone(&core);
    let result = rt.block_on(async move {
        let options = screen_tune_ui::native_options(&core_for_eframe);
        let app_creator = screen_tune_ui::create(core_for_eframe);
        eframe::run_native("ScreenTune", options, app_creator)
    });

    if let Err(e) = result {
        warn!("GUI 运行结束（可能为异常）: {e:#}");
    }

    // -----------------------------------------------------------
    // 5. 退出清理：恢复全部显示器原始 LUT + 保存配置
    // -----------------------------------------------------------
    info!("应用退出，开始清理…");
    core.display.shutdown();
    core.save_config();
    info!("清理完成，再见 👋");
    Ok(())
}

/// 初始化 tracing：滚动文件（logs/screen-tune.log）+ 终端双输出
fn init_tracing(
    store: &ConfigStore,
    default_level: &str,
) -> tracing_appender::non_blocking::WorkerGuard {
    let log_dir = store.logs_dir();
    if let Err(e) = std::fs::create_dir_all(&log_dir) {
        warn!("创建日志目录失败（{}）: {}", e, log_dir.display());
    }
    let file_appender = tracing_appender::rolling::daily(&log_dir, "screen-tune.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    // 环境变量 RUST_LOG 优先，否则使用配置中的日志级别
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(file_writer)
                .with_ansi(false),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(std::io::stderr)
                .with_ansi(true),
        )
        .with(filter)
        .init();
    guard
}
