//! # 主应用（eframe App）
//!
//! 负责窗口生命周期、事件轮询与整体布局：
//! - 顶部：应用名 + 显示器选择 + 全部同步
//! - 左侧：Fluent 风格导航
//! - 中央：各导航页面
//! - 关闭窗口默认最小化到托盘（可配置）

use std::sync::Arc;

use egui::containers::{CentralPanel, Panel};
use egui::{Color32, RichText, Vec2};
use screen_tune_common::consts::{APP_TAGLINE, APP_VERSION};
use screen_tune_common::DisplayParams;

use crate::core::{AppCore, AppEvent};
use crate::i18n::Tr;
use crate::pages::hotkeys_page;
use crate::pages::{HotkeyEditState, MonitorsPageState, ProfilePageState, SettingsPageState};
use crate::theme;

/// 导航页面
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    /// 显示调节
    Display,
    /// 配置方案
    Profiles,
    /// 全局快捷键
    Hotkeys,
    /// 设置
    Settings,
    /// 显示器
    Monitors,
}

/// eframe 主应用
pub struct ScreenTuneApp {
    /// 服务组合层
    core: Arc<AppCore>,
    /// 翻译句柄
    tr: Tr,
    /// 当前导航页
    page: Page,
    /// 当前选中的显示器 id
    selected_monitor: String,
    /// 选中显示器的编辑参数（拖动实时应用）
    draft: DisplayParams,
    /// 方案页状态
    profile_state: ProfilePageState,
    /// 设置页状态
    settings_state: SettingsPageState,
    /// 显示器页状态
    monitors_state: MonitorsPageState,
    /// 快捷键录制状态
    hotkey_edit: Option<HotkeyEditState>,
    /// 右上角 toast（文本, 剩余秒）
    toast: Option<(String, f32)>,
    /// 关闭请求是否已处理（防止同一次请求重复处理）
    close_handled: bool,
}

/// eframe 应用构造器类型
pub type AppCreator = Box<
    dyn FnOnce(
        &eframe::CreationContext,
    ) -> Result<Box<dyn eframe::App>, Box<dyn std::error::Error + Send + Sync>>,
>;

/// 创建 eframe 应用（事件循环就绪后调用）
pub fn create(core: Arc<AppCore>) -> AppCreator {
    Box::new(move |cc| {
        // 事件循环已就绪：初始化平台绑定（热键 / 托盘 / 游戏检测）
        let rt = tokio::runtime::Handle::current();
        core.init_platform_bindings(&rt);

        // 字体与主题
        theme::install_cjk_font(&cc.egui_ctx);
        theme::apply_theme(&cc.egui_ctx, core.config.read().unwrap().theme);

        let mut app = ScreenTuneApp::new(core);
        app.sync_draft();
        Ok(Box::new(app))
    })
}

/// 构造 NativeOptions（窗口几何来自配置）
pub fn native_options(core: &Arc<AppCore>) -> eframe::NativeOptions {
    let window = core.config.read().unwrap().window;
    let viewport = egui::ViewportBuilder::default()
        .with_title("ScreenTune")
        .with_app_id("ScreenTune")
        .with_inner_size(Vec2::new(window.width, window.height))
        .with_min_inner_size(Vec2::new(880.0, 560.0))
        .with_icon(Arc::new(window_icon()));
    let viewport = if let Some((x, y)) = window.position {
        viewport.with_position(egui::pos2(x, y))
    } else {
        viewport
    };
    eframe::NativeOptions {
        viewport,
        centered: window.position.is_none(),
        ..Default::default()
    }
}

impl ScreenTuneApp {
    /// 构造应用（核心已初始化）
    pub fn new(core: Arc<AppCore>) -> Self {
        let tr = Tr::new(core.config.read().unwrap().language);
        Self {
            core,
            tr,
            page: Page::Display,
            selected_monitor: String::new(),
            draft: DisplayParams::default(),
            profile_state: ProfilePageState::default(),
            settings_state: SettingsPageState::default(),
            monitors_state: MonitorsPageState::default(),
            hotkey_edit: None,
            toast: None,
            close_handled: false,
        }
    }

    /// 把选中显示器的当前参数同步到编辑草稿
    fn sync_draft(&mut self) {
        if let Some(params) = self.core.display.params_of(&self.selected_monitor) {
            self.draft = params;
        }
    }

    /// 确保选中显示器有效（热插拔后回退到第一台）
    fn ensure_selected_monitor(&mut self) {
        let snapshot = self.core.display.snapshot();
        if snapshot.is_empty() {
            self.selected_monitor.clear();
            return;
        }
        if !snapshot.iter().any(|(i, _)| i.id == self.selected_monitor) {
            self.selected_monitor = snapshot[0].0.id.clone();
            self.sync_draft();
        }
    }

    /// 保存窗口几何到配置
    fn save_window_geometry(&self, ctx: &egui::Context) {
        if let Some(rect) = ctx.input(|i| i.viewport().outer_rect) {
            let mut config = self.core.config.write().unwrap();
            config.window.position = Some((rect.min.x, rect.min.y));
            config.window.width = rect.width();
            config.window.height = rect.height();
            self.core.persist_config(&config);
        }
    }

    /// 处理窗口关闭请求（关闭 → 最小化到托盘）
    fn handle_window_events(&mut self, ctx: &egui::Context) {
        let close_requested = ctx.input(|i| i.viewport().close_requested());
        if !close_requested || self.close_handled {
            return;
        }
        self.close_handled = true;
        self.save_window_geometry(ctx);
        let close_to_tray = self.core.config.read().unwrap().close_to_tray;
        if close_to_tray {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
            // 允许下一次关闭请求再次进入（窗口再次打开后可再次隐藏）
            self.close_handled = false;
        }
    }

    /// 轮询并处理全部外部事件（热键 / 托盘 / 后台）
    fn handle_events(&mut self, ctx: &egui::Context) {
        for event in self.core.poll_external() {
            match event {
                AppEvent::ShowWindow => {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                }
                AppEvent::Quit => {
                    self.save_window_geometry(ctx);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                AppEvent::ProfileApplied { profile_id } => {
                    self.sync_draft();
                    if profile_id.as_deref() == Some("default") {
                        self.toast = Some((self.tr.t("toast.restored"), 2.0));
                    }
                }
                AppEvent::GameProfileApplied(id) => {
                    self.sync_draft();
                    let name = self
                        .core
                        .profiles
                        .read()
                        .unwrap()
                        .get(&id)
                        .map(|p| p.name.clone())
                        .unwrap_or(id);
                    self.toast = Some((format!("{} {name}", self.tr.t("toast.game_profile")), 3.0));
                }
                AppEvent::GameProfileExited => {
                    self.sync_draft();
                    self.toast = Some((self.tr.t("toast.game_exited"), 2.0));
                }
                AppEvent::StartupChanged(_) => {}
                AppEvent::Toast(msg) => self.toast = Some((msg, 4.0)),
            }
        }
    }

    /// 顶部栏：应用名 + 显示器选择 + 全部同步
    fn header_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("ScreenTune").size(17.0).strong());
            ui.separator();
            ui.label(RichText::new(self.tr.t("header.monitor")).strong());

            // 显示器选择
            let snapshot = self.core.display.snapshot();
            let selected_name = snapshot
                .iter()
                .find(|(i, _)| i.id == self.selected_monitor)
                .map(|(i, _)| i.name.clone())
                .unwrap_or_default();
            egui::ComboBox::from_id_salt("monitor_select")
                .selected_text(selected_name)
                .width(220.0)
                .show_ui(ui, |ui| {
                    for (info, _) in &snapshot {
                        let label = if info.is_primary {
                            format!("{} ★", info.name)
                        } else {
                            info.name.clone()
                        };
                        if ui
                            .selectable_value(&mut self.selected_monitor, info.id.clone(), label)
                            .changed()
                        {
                            self.sync_draft();
                        }
                    }
                });

            // 全部同步
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(self.tr.t("header.apply_all")).clicked() {
                    let _ = self.core.apply_current_to_all();
                }
            });
        });
    }

    /// 左侧导航（Fluent 风格：选中项半透明强调底 + 白字）
    fn nav_ui(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        let items = [
            (Page::Display, "nav.display"),
            (Page::Profiles, "nav.profiles"),
            (Page::Hotkeys, "nav.hotkeys"),
            (Page::Settings, "nav.settings"),
            (Page::Monitors, "nav.monitors"),
        ];
        for (page, key) in items {
            let selected = self.page == page;
            let bg = if selected {
                Color32::from_rgba_unmultiplied(0, 120, 212, 70)
            } else {
                Color32::TRANSPARENT
            };
            let frame = egui::Frame::new()
                .fill(bg)
                .corner_radius(egui::CornerRadius::same(6))
                .inner_margin(egui::Margin::symmetric(12, 7));
            frame.show(ui, |ui| {
                let resp = ui.add(
                    egui::Label::new(RichText::new(self.tr.t(key)).size(14.0).color(if selected {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(210, 210, 210)
                    }))
                    .sense(egui::Sense::click()),
                );
                if resp.clicked() {
                    self.page = page;
                    // 重新进入设置页时重新加载规则缓冲
                    self.settings_state.rules_loaded = false;
                }
            });
            ui.add_space(3.0);
        }

        // 底部版本信息
        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            ui.add_space(6.0);
            ui.label(
                RichText::new(format!("{} {}", self.tr.t("app.version"), APP_VERSION))
                    .small()
                    .color(Color32::from_rgb(140, 140, 140)),
            );
            ui.label(
                RichText::new(APP_TAGLINE)
                    .small()
                    .color(Color32::from_rgb(140, 140, 140)),
            );
        });
    }

    /// 右上角 toast
    fn toast_ui(&mut self, ctx: &egui::Context) {
        if let Some((text, _)) = &self.toast {
            egui::Area::new(egui::Id::new("toast_area"))
                .anchor(egui::Align2::RIGHT_TOP, [-18.0, 18.0])
                .show(ctx, |ui| {
                    egui::Frame::popup(ui.style())
                        .corner_radius(egui::CornerRadius::same(6))
                        .show(ui, |ui| {
                            ui.label(RichText::new(text).size(13.0));
                        });
                });
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
    }
}

impl eframe::App for ScreenTuneApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();

        // 1. 窗口事件（关闭 → 托盘）
        self.handle_window_events(&ctx);

        // 2. 快捷键录制捕获（优先于页面交互）
        if self.hotkey_edit.is_some() {
            hotkeys_page::capture_key(
                &ctx,
                &self.core,
                &self.tr,
                &mut self.hotkey_edit,
                &mut self.toast,
            );
        }

        // 3. 外部事件
        self.handle_events(&ctx);

        // 4. 布局
        Panel::top("header")
            .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(14, 8)))
            .show(ui, |ui| self.header_ui(ui));

        Panel::left("nav")
            .exact_size(190.0)
            .resizable(false)
            .frame(egui::Frame::new().inner_margin(egui::Margin::symmetric(8, 0)))
            .show(ui, |ui| self.nav_ui(ui));

        CentralPanel::default()
            .frame(egui::Frame::new().inner_margin(egui::Margin::same(16)))
            .show(ui, |ui| match self.page {
                Page::Display => {
                    self.ensure_selected_monitor();
                    if self.selected_monitor.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.label(self.tr.t("display.no_monitor"));
                        });
                    } else {
                        crate::pages::display_page::show(
                            ui,
                            &self.core,
                            &self.tr,
                            &self.selected_monitor,
                            &mut self.draft,
                        );
                    }
                }
                Page::Profiles => {
                    crate::pages::profiles_page::show(
                        ui,
                        &self.core,
                        &self.tr,
                        &mut self.profile_state,
                        &mut self.toast,
                    );
                }
                Page::Hotkeys => {
                    crate::pages::hotkeys_page::show(
                        ui,
                        &self.core,
                        &self.tr,
                        &mut self.hotkey_edit,
                    );
                }
                Page::Settings => {
                    let (theme_changed, lang_changed) = crate::pages::settings_page::show(
                        ui,
                        &self.core,
                        &self.tr,
                        &mut self.settings_state,
                        &mut self.toast,
                    );
                    if theme_changed {
                        let theme = self.core.config.read().unwrap().theme;
                        theme::apply_theme(&ctx, theme);
                    }
                    if lang_changed {
                        let lang = self.core.config.read().unwrap().language;
                        self.tr = Tr::new(lang);
                    }
                }
                Page::Monitors => {
                    crate::pages::monitors_page::show(
                        ui,
                        &self.core,
                        &self.tr,
                        &mut self.monitors_state,
                    );
                }
            });

        // 5. toast 计时
        if let Some((_, remain)) = &mut self.toast {
            *remain -= ctx.input(|i| i.stable_dt);
            if *remain <= 0.0 {
                self.toast = None;
            }
        }
        self.toast_ui(&ctx);
    }
}

/// 生成窗口图标（64×64 RGBA，程序内绘制：深色渐变 + 三条通道色条）
fn window_icon() -> egui::viewport::IconData {
    const SIZE: u32 = 64;
    let mut rgba = vec![0u8; (SIZE * SIZE * 4) as usize];

    for y in 0..SIZE {
        for x in 0..SIZE {
            let i = ((y * SIZE + x) * 4) as usize;
            let fx = x as f32 / (SIZE - 1) as f32;
            let fy = y as f32 / (SIZE - 1) as f32;

            // 对角渐变背景（Fluent 深色）
            let t = (fx * 0.7 + fy * 0.3).clamp(0.0, 1.0);
            let mut r = (31.0 + 40.0 * t) as u8;
            let mut g = (31.0 + 48.0 * t) as u8;
            let mut b = (43.0 + 66.0 * t) as u8;

            // 中心三条「亮度条」（R/G/B 意象）
            let in_bar_x = (8.0..56.0).contains(&(fx * SIZE as f32));
            let in_bar_y = (16.0..48.0).contains(&(fy * SIZE as f32));
            if in_bar_x && in_bar_y {
                let bar = (((fx * SIZE as f32) - 8.0) / 16.0) as usize;
                let level = 0.3 + 0.55 * fy;
                match bar {
                    0 => {
                        r = (0.9 * level * 255.0) as u8;
                        g = (0.55 * level * 255.0) as u8;
                        b = (0.35 * level * 255.0) as u8;
                    }
                    1 => {
                        r = (0.6 * level * 255.0) as u8;
                        g = (0.75 * level * 255.0) as u8;
                        b = (0.6 * level * 255.0) as u8;
                    }
                    _ => {
                        r = (0.4 * level * 255.0) as u8;
                        g = (0.55 * level * 255.0) as u8;
                        b = (0.95 * level * 255.0) as u8;
                    }
                }
            }
            rgba[i] = r;
            rgba[i + 1] = g;
            rgba[i + 2] = b;
            rgba[i + 3] = 255;
        }
    }

    egui::viewport::IconData {
        rgba,
        width: SIZE,
        height: SIZE,
    }
}
