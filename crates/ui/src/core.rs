//! # 服务组合层（AppCore）
//!
//! UI 与全部后台服务的唯一接口：整合显示、方案、快捷键、托盘、开机自启
//! 与游戏自动检测，屏蔽跨服务协作细节。UI 只依赖本结构公开方法。

use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use anyhow::{Context, Result};
use screen_tune_config::{AppConfig, ConfigStore, GameRule, HotkeyAction};
use screen_tune_display::{default_backend, DisplayManager};
use screen_tune_game_detector::{default_lister, DetectorEvent, GameDetector};
use screen_tune_hotkey::HotkeyManager;
use screen_tune_profile::ProfileManager;
use screen_tune_startup::StartupBackend;
use screen_tune_tray::{TrayCommand, TrayManager};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

/// UI 消费的应用事件
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    /// 请求显示主窗口
    ShowWindow,
    /// 请求退出应用
    Quit,
    /// 方案已应用（`Some(id)`；`None` 表示恢复默认）
    ProfileApplied { profile_id: Option<String> },
    /// 游戏检测应用了方案
    GameProfileApplied(String),
    /// 游戏退出，已恢复默认
    GameProfileExited,
    /// 开机自启状态变化
    StartupChanged(bool),
    /// 通用提示（toast 文案）
    Toast(String),
}

/// 服务组合层
pub struct AppCore {
    /// 配置存储
    pub store: ConfigStore,
    /// 全局配置（运行期可改）
    pub config: RwLock<AppConfig>,
    /// 显示服务
    pub display: DisplayManager,
    /// 方案服务
    pub profiles: RwLock<ProfileManager>,
    /// 全局快捷键（平台不可用时为 None）
    hotkeys: Mutex<Option<HotkeyManager>>,
    /// 系统托盘
    tray: Mutex<TrayManager>,
    /// 开机自启
    startup: Box<dyn StartupBackend>,
    /// 游戏检测状态机
    detector: Mutex<GameDetector>,
    /// 游戏检测后台任务
    _game_task: Mutex<Option<JoinHandle<()>>>,
    /// 最近一次热键注册的冲突列表
    pub hotkey_conflicts: RwLock<Vec<(String, String)>>,
    /// 当前生效方案（None = 手动自定义）
    current_profile: RwLock<Option<String>>,
    /// 事件发送端（游戏检测任务等后台 → UI）
    event_tx: Sender<AppEvent>,
    /// 事件接收端（UI 轮询；Receiver 非 Sync，用 Mutex 包裹以共享 AppCore）
    event_rx: Mutex<Receiver<AppEvent>>,
}

impl AppCore {
    /// 创建核心并完成数据初始化：
    /// 显示器基线 → 恢复显示器记忆参数 → 应用最后使用方案。
    pub fn init(store: ConfigStore, config: AppConfig) -> Result<Arc<Self>> {
        let display = DisplayManager::init(default_backend(), store.gamma_backup_dir())
            .context("初始化显示引擎失败")?;
        let profiles = ProfileManager::init(&store).context("加载配置方案失败")?;
        let detector = GameDetector::new(default_lister(), config.game_rules.clone());

        let (event_tx, event_rx) = channel();
        let core = Arc::new(Self {
            store,
            config: RwLock::new(config),
            display,
            profiles: RwLock::new(profiles),
            hotkeys: Mutex::new(None),
            tray: Mutex::new(TrayManager::new()),
            startup: screen_tune_startup::default_startup_backend(),
            detector: Mutex::new(detector),
            _game_task: Mutex::new(None),
            hotkey_conflicts: RwLock::new(Vec::new()),
            current_profile: RwLock::new(None),
            event_tx,
            event_rx: Mutex::new(event_rx),
        });

        // 恢复每个显示器记住的参数（热插拔后新显示器走默认参数）
        for (monitor_id, params) in core.config.read().unwrap().monitor_params.clone() {
            if let Err(e) = core.display.set_params(&monitor_id, params) {
                warn!("恢复显示器 {monitor_id} 参数失败: {e:#}");
            }
        }
        // 应用最后使用的方案
        let last = core.config.read().unwrap().last_profile_id.clone();
        if let Some(pid) = last {
            if core.profiles.read().unwrap().exists(&pid) {
                info!("启动时应用上次使用的方案: {pid}");
                if let Err(e) = core.apply_profile(&pid, false) {
                    warn!("应用方案 {pid} 失败: {e:#}");
                }
            }
        }
        Ok(core)
    }

    /// 平台绑定初始化：注册热键、构建托盘、启动游戏检测任务。
    /// 必须在 eframe 事件循环运行后（App::new 内）调用。
    pub fn init_platform_bindings(self: &Arc<Self>, rt: &tokio::runtime::Handle) {
        self.ensure_hotkey_manager();
        self.rebuild_hotkeys();
        self.rebuild_tray();
        self.spawn_game_detector(rt);
    }

    /// 把当前选中（第一台）显示器的参数同步到全部显示器，并脱离方案状态
    pub fn apply_current_to_all(&self) -> Result<()> {
        let Some(first_id) = self.display.snapshot().first().map(|(i, _)| i.id.clone()) else {
            return Ok(());
        };
        let params = self
            .display
            .params_of(&first_id)
            .with_context(|| format!("显示器不存在: {first_id}"))?;
        self.display.apply_to_all(params)?;
        {
            let mut config = self.config.write().unwrap();
            for (info, _) in self.display.snapshot() {
                config.monitor_params.insert(info.id, params.clamped());
            }
            config.last_profile_id = None;
            self.persist_config(&config);
        }
        *self.current_profile.write().unwrap() = None;
        Ok(())
    }

    // ---------------------------------------------------------------
    // 事件
    // ---------------------------------------------------------------

    /// 向 UI 发送事件
    pub fn send(&self, event: AppEvent) -> Result<()> {
        self.event_tx.send(event).context("事件通道已关闭")
    }

    /// UI 每帧轮询：热键事件 + 托盘事件 + 后台事件
    pub fn poll_external(&self) -> Vec<AppEvent> {
        let mut events = Vec::new();

        // 全局快捷键
        if let Some(manager) = self.hotkeys.lock().unwrap().as_mut() {
            for action in manager.poll_events() {
                events.extend(self.dispatch_hotkey(action));
            }
        }

        // 托盘菜单
        for cmd in self.tray.lock().unwrap().poll_events() {
            events.extend(self.dispatch_tray(cmd));
        }

        // 后台事件（游戏检测等）
        while let Ok(event) = self.event_rx.lock().unwrap().try_recv() {
            events.push(event);
        }
        events
    }

    /// 热键动作 → 事件
    fn dispatch_hotkey(&self, action: HotkeyAction) -> Vec<AppEvent> {
        match action {
            HotkeyAction::RestoreDefault => {
                let _ = self.restore_default();
                vec![AppEvent::ProfileApplied {
                    profile_id: Some("default".into()),
                }]
            }
            HotkeyAction::ApplyProfile { profile_id } => {
                let _ = self.apply_profile(&profile_id, true);
                vec![AppEvent::ProfileApplied {
                    profile_id: Some(profile_id),
                }]
            }
            HotkeyAction::ShowWindow => vec![AppEvent::ShowWindow],
        }
    }

    /// 托盘命令 → 事件
    fn dispatch_tray(&self, cmd: TrayCommand) -> Vec<AppEvent> {
        match cmd {
            TrayCommand::ShowWindow => vec![AppEvent::ShowWindow],
            TrayCommand::RestoreDefault => {
                let _ = self.restore_default();
                vec![AppEvent::ProfileApplied {
                    profile_id: Some("default".into()),
                }]
            }
            TrayCommand::ApplyProfile(profile_id) => {
                let _ = self.apply_profile(&profile_id, true);
                vec![AppEvent::ProfileApplied {
                    profile_id: Some(profile_id),
                }]
            }
            TrayCommand::ToggleStartup => {
                self.toggle_startup();
                Vec::new()
            }
            TrayCommand::Quit => vec![AppEvent::Quit],
        }
    }

    // ---------------------------------------------------------------
    // 显示与方案
    // ---------------------------------------------------------------

    /// 当前生效方案 id（None = 自定义）
    pub fn current_profile(&self) -> Option<String> {
        self.current_profile.read().unwrap().clone()
    }

    /// 应用方案到全部显示器。
    /// `persist` 为 true 时记录为「最后使用方案」（用户手动应用/热键应用）；
    /// 游戏自动切换传 false。
    pub fn apply_profile(&self, id: &str, persist: bool) -> Result<()> {
        let profile = self
            .profiles
            .read()
            .unwrap()
            .get(id)
            .cloned()
            .with_context(|| format!("方案不存在: {id}"))?;

        self.display.apply_to_all(profile.params)?;

        // 更新记住的显示器参数（方案 = 全局统一）
        {
            let mut config = self.config.write().unwrap();
            for (info, _) in self.display.snapshot() {
                config.monitor_params.insert(info.id, profile.params);
            }
            if persist {
                config.last_profile_id = Some(id.to_string());
            }
            self.persist_config(&config);
        }
        *self.current_profile.write().unwrap() = Some(id.to_string());
        info!("已应用方案: {}（{}）", profile.name, id);
        Ok(())
    }

    /// 恢复默认参数（全部显示器）
    pub fn restore_default(&self) -> Result<()> {
        self.display.restore_default()?;
        {
            let mut config = self.config.write().unwrap();
            config.monitor_params.clear();
            config.last_profile_id = Some("default".to_string());
            self.persist_config(&config);
        }
        *self.current_profile.write().unwrap() = Some("default".to_string());
        info!("已恢复默认显示参数");
        Ok(())
    }

    /// 实时更新单个显示器的参数（滑块拖动中调用，实时生效不落盘）
    pub fn update_monitor_params(
        &self,
        monitor_id: &str,
        params: screen_tune_common::DisplayParams,
    ) -> Result<()> {
        self.display.set_params(monitor_id, params)?;
        {
            let mut config = self.config.write().unwrap();
            config
                .monitor_params
                .insert(monitor_id.to_string(), params.clamped());
            // 手动调整视为脱离方案
            config.last_profile_id = None;
        }
        *self.current_profile.write().unwrap() = None;
        Ok(())
    }

    /// 恢复单个显示器的原始画面（LUT + DDC 亮度）
    pub fn restore_monitor_baseline(&self, monitor_id: &str) -> Result<()> {
        self.display.restore_baseline(monitor_id)?;
        {
            let mut config = self.config.write().unwrap();
            config.monitor_params.remove(monitor_id);
            self.persist_config(&config);
        }
        info!("已恢复显示器 {monitor_id} 的原始画面");
        Ok(())
    }

    /// 把显示器参数变化持久化（滑块结束拖动时调用，避免高频写盘）
    pub fn persist_config(&self, config: &AppConfig) {
        if let Err(e) = self.store.save_config(config) {
            error!("保存配置失败: {e:#}");
        }
    }

    /// 持久化当前配置（供 UI 在配置修改后调用）
    pub fn save_config(&self) {
        let config = self.config.read().unwrap().clone();
        self.persist_config(&config);
    }

    /// 刷新显示器列表（热插拔后调用）
    pub fn refresh_monitors(&self) -> Result<()> {
        self.display.refresh_monitors()
    }

    // ---------------------------------------------------------------
    // 快捷键
    // ---------------------------------------------------------------

    /// 依据当前配置重建全部全局快捷键注册
    pub fn rebuild_hotkeys(&self) {
        let mut conflicts = Vec::new();
        let bindings: Vec<screen_tune_hotkey::HotkeyBinding> = self
            .config
            .read()
            .unwrap()
            .hotkeys
            .iter()
            .map(|b| screen_tune_hotkey::HotkeyBinding {
                id: b.id.clone(),
                spec: b.spec.clone(),
                action: b.action.clone(),
            })
            .collect();

        if self.hotkeys.lock().unwrap().is_none() {
            // 平台不可用：全部记为冲突
            for b in &bindings {
                conflicts.push((b.id.clone(), "平台不支持".to_string()));
            }
        } else if let Some(manager) = self.hotkeys.lock().unwrap().as_mut() {
            let outcome = manager.rebuild(bindings);
            conflicts = outcome.conflicts;
        }

        *self.hotkey_conflicts.write().unwrap() = conflicts.clone();
        if !conflicts.is_empty() {
            warn!("以下快捷键注册失败: {conflicts:?}");
            let _ = self.send(AppEvent::Toast(format!(
                "{}: {:?}",
                "快捷键冲突",
                conflicts.iter().map(|(_, r)| r).collect::<Vec<_>>()
            )));
        }
    }

    /// 全局快捷键是否可用（平台支持且初始化成功）
    pub fn hotkeys_available(&self) -> bool {
        self.hotkeys.lock().unwrap().as_ref().is_some()
    }

    /// 平台事件循环就绪后创建快捷键管理器（失败则禁用快捷键）
    pub fn ensure_hotkey_manager(&self) {
        if self.hotkeys.lock().unwrap().is_some() {
            return;
        }
        match HotkeyManager::new() {
            Ok(manager) => {
                *self.hotkeys.lock().unwrap() = Some(manager);
            }
            Err(e) => {
                warn!("初始化全局快捷键失败，快捷键功能不可用: {e:#}");
            }
        }
    }

    // ---------------------------------------------------------------
    // 托盘
    // ---------------------------------------------------------------

    /// 重建托盘菜单（方案列表 / 自启状态变化时调用）
    pub fn rebuild_tray(&self) {
        let profiles: Vec<screen_tune_config::Profile> =
            self.profiles.read().unwrap().list().to_vec();
        let startup_enabled = self.config.read().unwrap().startup_enabled;
        if let Err(e) = self
            .tray
            .lock()
            .unwrap()
            .rebuild_menu(&profiles, startup_enabled)
        {
            warn!("更新托盘失败: {e:#}");
        }
    }

    // ---------------------------------------------------------------
    // 开机自启
    // ---------------------------------------------------------------

    /// 当前自启状态
    pub fn startup_enabled(&self) -> bool {
        self.config.read().unwrap().startup_enabled
    }

    /// 切换开机自启
    pub fn toggle_startup(&self) {
        let next = !self.startup_enabled();
        match self.startup.set_enabled(next) {
            Ok(()) => {
                self.config.write().unwrap().startup_enabled = next;
                self.save_config();
                self.rebuild_tray();
                let _ = self.send(AppEvent::StartupChanged(next));
                info!("开机自启: {}", if next { "开启" } else { "关闭" });
            }
            Err(e) => {
                warn!("切换开机自启失败: {e:#}");
                let _ = self.send(AppEvent::Toast(format!("开机自启设置失败: {e:#}")));
            }
        }
    }

    // ---------------------------------------------------------------
    // 游戏自动切换
    // ---------------------------------------------------------------

    /// 同步游戏检测规则（配置变更后调用）
    pub fn sync_detector_rules(&self) {
        let rules: Vec<GameRule> = self.config.read().unwrap().game_rules.clone();
        self.detector.lock().unwrap().set_rules(rules);
    }

    /// 启动游戏检测后台任务
    fn spawn_game_detector(self: &Arc<Self>, rt: &tokio::runtime::Handle) {
        let interval_secs = self.config.read().unwrap().game_poll_interval_secs.max(1);
        let core = Arc::clone(self);
        let handle = rt.spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));
            // 首 tick 立即执行
            ticker.tick().await;
            loop {
                ticker.tick().await;
                let event = core.detector.lock().unwrap().tick();
                match event {
                    Some(DetectorEvent::Entered { profile_id, .. }) => {
                        if let Err(e) = core.apply_profile(&profile_id, false) {
                            warn!("游戏自动应用方案失败: {e:#}");
                        }
                        let _ = core.send(AppEvent::GameProfileApplied(profile_id));
                    }
                    Some(DetectorEvent::Exited { .. }) => {
                        if let Err(e) = core.restore_default() {
                            warn!("游戏退出恢复默认失败: {e:#}");
                        }
                        let _ = core.send(AppEvent::GameProfileExited);
                    }
                    None => {}
                }
            }
        });
        *self._game_task.lock().unwrap() = Some(handle);
        info!("游戏自动切换检测已启动（间隔 {interval_secs}s）");
    }
}
