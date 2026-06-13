use crate::{
    embed::NeovideManager,
    persistence::{AppConfig, AppDataPaths, RecentFiles, SessionState},
    tabs::TabManager,
    theme::ThemeStore,
    ui::{
        launcher::LauncherView, settings_panel::SettingsPanel, statusbar::StatusBar,
        theme_panel::ThemePanel, titlebar::TitleBar, toast::ToastStack,
    },
};

pub struct NeoNoteApp {
    paths: AppDataPaths,
    config: AppConfig,
    session: SessionState,
    recent_files: RecentFiles,
    themes: ThemeStore,
    tabs: TabManager,
    neovide: NeovideManager,
    titlebar: TitleBar,
    statusbar: StatusBar,
    launcher: LauncherView,
    theme_panel: ThemePanel,
    settings_panel: SettingsPanel,
    toasts: ToastStack,
}

impl NeoNoteApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let paths = AppDataPaths::new();
        let config = AppConfig::load_or_default(&paths);
        let session = SessionState::load_or_default(&paths);
        let recent_files = RecentFiles::load_or_default(&paths);
        let themes = ThemeStore::load(&paths, config.active_theme.as_deref());

        if let Some(theme) = themes.active_theme() {
            cc.egui_ctx.set_visuals(theme.to_visuals());
        }

        Self {
            paths,
            config,
            session,
            recent_files,
            themes,
            tabs: TabManager::default(),
            neovide: NeovideManager::default(),
            titlebar: TitleBar,
            statusbar: StatusBar::default(),
            launcher: LauncherView,
            theme_panel: ThemePanel::default(),
            settings_panel: SettingsPanel::default(),
            toasts: ToastStack::default(),
        }
    }

    fn render_editor_shell(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("titlebar")
            .exact_height(36.0)
            .show(ctx, |ui| {
                self.titlebar.ui(ui, self.tabs.active_title());
            });

        egui::TopBottomPanel::top("tabs")
            .exact_height(34.0)
            .show(ctx, |ui| {
                self.tabs.ui(ui);
            });

        egui::TopBottomPanel::bottom("statusbar")
            .exact_height(26.0)
            .show(ctx, |ui| {
                self.statusbar.ui(ui, self.tabs.active_title());
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let rect = ui.available_rect_before_wrap();
            self.neovide.sync_editor_rect(rect);
            ui.centered_and_justified(|ui| {
                if self.tabs.is_empty() {
                    self.launcher.ui(ui, &self.recent_files, &self.themes);
                } else {
                    ui.label("Neovide embedding panel");
                }
            });
        });
    }
}

impl eframe::App for NeoNoteApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.render_editor_shell(ctx);
        self.theme_panel.ui(ctx, &mut self.themes);
        self.settings_panel.ui(ctx, &mut self.config);
        self.toasts.ui(ctx);

        let _ = (&self.paths, &self.session);
    }
}
