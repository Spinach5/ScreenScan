//! 首选项窗口：六个标签页，所有修改即时生效。

use std::path::Path;
use std::path::PathBuf;

use egui::Color32;
use egui::RichText;
use egui::TextureHandle;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::WindowAttributes;
use winit::window::WindowId;

use crate::config::Config;
use crate::config::DEFAULT_FILENAME_FORMAT;
use crate::config::LogLevel;
use crate::config::TrayIconSetting;
use crate::font::FontChoice;
use crate::font::FontEntry;
use crate::graphics::RenderedWindow;
use crate::i18n::Language;
use crate::i18n::t;

#[derive(Debug)]
pub enum Effect {
    /// 仅持久化配置。
    Save,
    /// 后台扫描系统字体。
    ScanFonts,
    /// `None` 表示恢复默认字体。
    FontSelected(Option<FontChoice>),
    TrayIcon(TrayIconSetting),
    PickTrayIcon,
    PickConfigDir,
    Language(Language),
    Autostart(bool),
    LogLevel,
    OpenLogDir,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    General,
    Interface,
    Output,
    Language,
    Update,
    About,
}

impl Tab {
    const ALL: [Self; 6] = [
        Self::General,
        Self::Interface,
        Self::Output,
        Self::Language,
        Self::Update,
        Self::About,
    ];

    fn label(self, lang: Language) -> &'static str {
        match self {
            Self::General => t(lang, "tab.general"),
            Self::Interface => t(lang, "tab.interface"),
            Self::Output => t(lang, "tab.output"),
            Self::Language => t(lang, "tab.language"),
            Self::Update => t(lang, "tab.update"),
            Self::About => t(lang, "tab.about"),
        }
    }
}

struct Preview {
    path: PathBuf,
    image: Option<(TextureHandle, egui::Vec2)>,
}

pub struct SettingsWindow {
    rendered: RenderedWindow,
    tab: Tab,
    font_picker_open: bool,
    font_filter: String,
    language_picker_open: bool,
    preview: Option<Preview>,
    status: Option<String>,
}

impl SettingsWindow {
    pub fn create(event_loop: &ActiveEventLoop, lang: Language) -> anyhow::Result<Self> {
        let attributes = WindowAttributes::default()
            .with_title(t(lang, "settings.title"))
            .with_inner_size(LogicalSize::new(760.0, 560.0))
            .with_min_inner_size(LogicalSize::new(660.0, 460.0))
            .with_resizable(true);

        let rendered = unsafe { RenderedWindow::create(event_loop, attributes)? };

        if let Ok(image) = crate::tray::default_icon_image() {
            let (width, height) = image.dimensions();
            if let Ok(icon) = winit::window::Icon::from_rgba(image.into_raw(), width, height) {
                rendered.window().set_window_icon(Some(icon));
            }
        }

        rendered.window().set_visible(true);
        rendered.window().focus_window();
        rendered.window().request_redraw();

        Ok(Self {
            rendered,
            tab: Tab::General,
            font_picker_open: false,
            font_filter: String::new(),
            language_picker_open: false,
            preview: None,
            status: None,
        })
    }

    pub fn window_id(&self) -> WindowId {
        self.rendered.window_id()
    }

    pub fn request_redraw(&self) {
        self.rendered.window().request_redraw();
    }

    pub fn focus(&self) {
        self.rendered.window().focus_window();
    }

    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = Some(status.into());
    }

    pub fn invalidate_preview(&mut self) {
        self.preview = None;
    }

    pub fn set_language(&mut self, lang: Language) {
        self.rendered.window().set_title(t(lang, "settings.title"));
        self.request_redraw();
    }

    pub fn reinstall_fonts(&self) {
        crate::font::install(self.rendered.context());
        self.request_redraw();
    }

    /// 处理非 RedrawRequested 事件；返回 `true` 表示窗口应关闭。
    pub fn handle_window_event(&mut self, event: &WindowEvent) -> bool {
        use winit::event::WindowEvent::*;

        match event {
            CloseRequested | Destroyed => return true,
            Resized(size) => self.rendered.resize(*size),
            _ => {}
        }

        if self.rendered.on_window_event(event) {
            self.request_redraw();
        }
        false
    }

    pub fn redraw(
        &mut self,
        config: &mut Config,
        config_path: &str,
        fonts: &[FontEntry],
        fonts_scanning: bool,
    ) -> Vec<Effect> {
        let ctx = self.rendered.context().clone();
        update_preview(&mut self.preview, config, &ctx);

        let Self {
            rendered,
            tab,
            font_picker_open,
            font_filter,
            language_picker_open,
            preview,
            status,
        } = self;

        let mut effects = Vec::new();
        rendered.render(|ui| {
            settings_ui(
                ui,
                tab,
                font_picker_open,
                font_filter,
                language_picker_open,
                preview.as_ref(),
                status.as_deref(),
                config_path,
                config,
                fonts,
                fonts_scanning,
                &mut effects,
            );
        });
        effects
    }
}

fn update_preview(preview: &mut Option<Preview>, config: &Config, ctx: &egui::Context) {
    let Some(path) = config.tray_icon.custom_path().map(Path::to_path_buf) else {
        *preview = None;
        return;
    };
    if preview.as_ref().is_some_and(|preview| preview.path == path) {
        return;
    }

    match image::open(&path) {
        Ok(image) => {
            let image = image.to_rgba8();
            let (width, height) = image.dimensions();
            let color_image = egui::ColorImage::from_rgba_unmultiplied(
                [width as usize, height as usize],
                image.as_raw(),
            );
            let scale = (96.0 / width.max(height) as f32).min(1.0);
            let size = egui::vec2(width as f32 * scale, height as f32 * scale);
            let texture = ctx.load_texture(
                "tray_icon_preview",
                color_image,
                egui::TextureOptions::LINEAR,
            );
            *preview = Some(Preview {
                path,
                image: Some((texture, size)),
            });
        }
        Err(e) => {
            log::warn!("预览托盘图标失败 {}: {e}", path.display());
            *preview = Some(Preview { path, image: None });
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn settings_ui(
    ui: &mut egui::Ui,
    tab: &mut Tab,
    font_picker_open: &mut bool,
    font_filter: &mut String,
    language_picker_open: &mut bool,
    preview: Option<&Preview>,
    status: Option<&str>,
    config_path: &str,
    config: &mut Config,
    fonts: &[FontEntry],
    fonts_scanning: bool,
    effects: &mut Vec<Effect>,
) {
    let lang = config.language;
    let background = ui.visuals().panel_fill;
    ui.painter().rect_filled(ui.max_rect(), 0.0, background);

    egui::Frame::new()
        .fill(background)
        .inner_margin(egui::Margin::same(16))
        .show(ui, |ui| {
            ui.set_min_size(ui.available_size());
            ui.heading(t(lang, "settings.title"));
            ui.add_space(8.0);

            ui.horizontal_wrapped(|ui| {
                for candidate in Tab::ALL {
                    if ui
                        .selectable_label(*tab == candidate, candidate.label(lang))
                        .clicked()
                    {
                        *tab = candidate;
                    }
                }
            });
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(10.0);

            let content_height = (ui.available_height() - 34.0).max(120.0);
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .max_height(content_height)
                .show(ui, |ui| match *tab {
                    Tab::General => general_tab(ui, config, config_path, effects),
                    Tab::Interface => interface_tab(
                        ui,
                        config,
                        fonts,
                        fonts_scanning,
                        font_picker_open,
                        preview,
                        effects,
                    ),
                    Tab::Output => output_tab(ui, config, effects),
                    Tab::Language => language_tab(ui, config, language_picker_open),
                    Tab::Update => update_tab(ui, lang),
                    Tab::About => about_tab(ui, lang),
                });

            ui.add_space(4.0);
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(t(lang, "common.apply_immediately"))
                        .small()
                        .weak(),
                );
                if let Some(status) = status {
                    ui.separator();
                    ui.label(RichText::new(status).small());
                }
            });
        });

    if *font_picker_open {
        font_picker_window(
            ui.ctx(),
            font_picker_open,
            font_filter,
            config,
            fonts,
            fonts_scanning,
            effects,
        );
    }
    if *language_picker_open {
        language_picker_window(ui.ctx(), language_picker_open, config, effects);
    }
}

fn general_tab(
    ui: &mut egui::Ui,
    config: &mut Config,
    config_path: &str,
    effects: &mut Vec<Effect>,
) {
    let lang = config.language;
    egui::Grid::new("settings_general")
        .num_columns(2)
        .spacing([24.0, 16.0])
        .show(ui, |ui| {
            ui.label(t(lang, "general.autostart"));
            if toggle_switch(ui, &mut config.autostart).changed() {
                effects.push(Effect::Autostart(config.autostart));
            }
            ui.end_row();

            ui.label(t(lang, "general.backup"));
            if toggle_switch(ui, &mut config.auto_backup).changed() {
                effects.push(Effect::Save);
            }
            ui.end_row();

            ui.label(t(lang, "general.config_path"));
            ui.vertical(|ui| {
                ui.label(RichText::new(config_path).monospace());
                ui.horizontal(|ui| {
                    if ui.button(t(lang, "general.browse")).clicked() {
                        effects.push(Effect::PickConfigDir);
                    }
                    ui.label(
                        RichText::new(t(lang, "general.migrate_hint"))
                            .small()
                            .weak(),
                    );
                });
            });
            ui.end_row();
        });

    ui.add_space(10.0);
    ui.label(RichText::new(t(lang, "general.backup_hint")).small().weak());
    ui.add_space(4.0);
    ui.label(
        RichText::new(t(lang, "general.autostart_hint"))
            .small()
            .weak(),
    );
}

fn interface_tab(
    ui: &mut egui::Ui,
    config: &mut Config,
    fonts: &[FontEntry],
    fonts_scanning: bool,
    font_picker_open: &mut bool,
    preview: Option<&Preview>,
    effects: &mut Vec<Effect>,
) {
    let lang = config.language;

    ui.label(RichText::new(t(lang, "interface.font")).strong());
    let current_font = config
        .font
        .as_ref()
        .map(|font| font.family.as_str())
        .unwrap_or(t(lang, "interface.font_default"));
    if ui
        .add_sized(
            [320.0, 30.0],
            egui::Button::new(format!("{current_font}  ▾")),
        )
        .clicked()
    {
        *font_picker_open = true;
        if fonts.is_empty() && !fonts_scanning {
            effects.push(Effect::ScanFonts);
        }
    }
    ui.label(RichText::new(t(lang, "interface.font_hint")).small().weak());
    ui.add_space(18.0);

    ui.label(RichText::new(t(lang, "interface.tray_icon")).strong());
    ui.add_space(4.0);
    let custom = matches!(config.tray_icon, TrayIconSetting::Custom(_));
    if ui
        .radio(!custom, t(lang, "interface.tray_default"))
        .clicked()
        && custom
    {
        effects.push(Effect::TrayIcon(TrayIconSetting::Default));
    }
    ui.horizontal(|ui| {
        if ui.radio(custom, t(lang, "interface.tray_custom")).clicked() && !custom {
            effects.push(Effect::PickTrayIcon);
        }
        if custom && ui.button(t(lang, "interface.tray_replace")).clicked() {
            effects.push(Effect::PickTrayIcon);
        }
    });

    if let Some(preview) = preview {
        match &preview.image {
            Some((texture, size)) => {
                let (rect, _) = ui.allocate_exact_size(*size, egui::Sense::hover());
                let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                ui.painter().image(texture.id(), rect, uv, Color32::WHITE);
            }
            None => {
                ui.label(
                    RichText::new(t(lang, "interface.tray_preview_failed"))
                        .small()
                        .color(Color32::from_rgb(220, 90, 90)),
                );
            }
        }
        ui.label(
            RichText::new(preview.path.display().to_string())
                .small()
                .weak(),
        );
    }
    ui.label(RichText::new(t(lang, "interface.tray_hint")).small().weak());
}

fn output_tab(ui: &mut egui::Ui, config: &mut Config, effects: &mut Vec<Effect>) {
    let lang = config.language;

    ui.horizontal(|ui| {
        ui.label(t(lang, "output.log_level"));
        let mut level = config.log_level;
        egui::ComboBox::from_id_salt("settings_log_level")
            .selected_text(level.label(lang))
            .show_ui(ui, |ui| {
                for candidate in LogLevel::ALL {
                    ui.selectable_value(&mut level, candidate, candidate.label(lang));
                }
            });
        if level != config.log_level {
            config.log_level = level;
            effects.push(Effect::LogLevel);
        }
    });
    ui.label(
        RichText::new(t(lang, "output.log_level_hint"))
            .small()
            .weak(),
    );

    ui.add_space(10.0);
    ui.horizontal(|ui| {
        ui.label(t(lang, "output.log_file"));
        ui.label(
            RichText::new(crate::util::path::display_path(&crate::logging::log_dir())).monospace(),
        );
        if ui.button(t(lang, "output.open_log_folder")).clicked() {
            effects.push(Effect::OpenLogDir);
        }
    });
    ui.label(RichText::new(t(lang, "output.log_hint")).small().weak());

    ui.add_space(18.0);
    ui.label(RichText::new(t(lang, "output.filename")).strong());
    let response = ui.add(
        egui::TextEdit::singleline(&mut config.filename_format)
            .desired_width(f32::INFINITY)
            .hint_text(DEFAULT_FILENAME_FORMAT),
    );
    if response.changed() {
        effects.push(Effect::Save);
    }
    ui.label(
        RichText::new(t(lang, "output.filename_hint"))
            .small()
            .weak(),
    );

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new(t(lang, "output.preview")).small().weak());
        ui.label(RichText::new(config.preview_filename()).monospace());
    });

    ui.add_space(8.0);
    if ui.button(t(lang, "output.restore")).clicked() {
        config.filename_format = DEFAULT_FILENAME_FORMAT.to_owned();
        effects.push(Effect::Save);
    }
}

fn language_tab(ui: &mut egui::Ui, config: &mut Config, language_picker_open: &mut bool) {
    let lang = config.language;
    ui.label(t(lang, "language.current"));
    ui.horizontal(|ui| {
        ui.label(RichText::new(config.language.native_name()).strong());
        if ui.button(t(lang, "language.picker")).clicked() {
            *language_picker_open = true;
        }
    });
    ui.label(RichText::new(t(lang, "language.hint")).small().weak());
}

fn update_tab(ui: &mut egui::Ui, lang: Language) {
    ui.add_space(32.0);
    ui.vertical_centered(|ui| {
        ui.heading(t(lang, "update.placeholder"));
        ui.add_space(8.0);
        ui.label(t(lang, "update.detail"));
    });
}

fn about_tab(ui: &mut egui::Ui, lang: Language) {
    ui.add_space(24.0);
    ui.vertical_centered(|ui| {
        ui.heading("ScreenScan");
        ui.add_space(8.0);
        ui.label(format!(
            "{} {}",
            t(lang, "about.version"),
            env!("CARGO_PKG_VERSION")
        ));
        ui.add_space(6.0);
        ui.label(t(lang, "about.placeholder"));
        ui.add_space(6.0);
        ui.label(RichText::new(t(lang, "about.copyright")).weak());
    });
}

fn font_picker_window(
    ctx: &egui::Context,
    open: &mut bool,
    filter: &mut String,
    config: &Config,
    fonts: &[FontEntry],
    fonts_scanning: bool,
    effects: &mut Vec<Effect>,
) {
    let lang = config.language;
    let mut is_open = true;
    egui::Window::new(t(lang, "interface.font_picker"))
        .open(&mut is_open)
        .collapsible(false)
        .resizable(true)
        .default_width(380.0)
        .default_height(420.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.add(
                egui::TextEdit::singleline(filter)
                    .hint_text(t(lang, "interface.font_search"))
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(6.0);

            if fonts.is_empty() {
                ui.label(if fonts_scanning {
                    t(lang, "interface.font_scanning")
                } else {
                    t(lang, "interface.font_empty")
                });
                return;
            }

            let needle = filter.trim().to_lowercase();
            let mut chosen = None;
            egui::ScrollArea::vertical()
                .max_height(330.0)
                .show(ui, |ui| {
                    if ui
                        .selectable_label(config.font.is_none(), t(lang, "interface.font_default"))
                        .clicked()
                    {
                        chosen = Some(None);
                    }
                    for entry in fonts.iter().filter(|entry| {
                        needle.is_empty() || entry.family.to_lowercase().contains(&needle)
                    }) {
                        let selected = config.font.as_ref().is_some_and(|font| {
                            font.path == entry.path && font.index == entry.index
                        });
                        if ui
                            .selectable_label(selected, entry.family.as_str())
                            .clicked()
                        {
                            chosen = Some(Some(entry.choice()));
                        }
                    }
                });

            if let Some(choice) = chosen {
                effects.push(Effect::FontSelected(choice));
                *open = false;
            }
        });
    if !is_open {
        *open = false;
    }
}

fn language_picker_window(
    ctx: &egui::Context,
    open: &mut bool,
    config: &Config,
    effects: &mut Vec<Effect>,
) {
    let lang = config.language;
    let mut is_open = true;
    let mut chosen = None;
    egui::Window::new(t(lang, "language.picker"))
        .open(&mut is_open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            for candidate in Language::ALL {
                if ui
                    .selectable_label(config.language == candidate, candidate.native_name())
                    .clicked()
                {
                    chosen = Some(candidate);
                }
            }
        });
    if let Some(candidate) = chosen {
        effects.push(Effect::Language(candidate));
        *open = false;
    }
    if !is_open {
        *open = false;
    }
}

fn toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let size = egui::vec2(42.0, 22.0);
    let (rect, mut response) = ui.allocate_exact_size(size, egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let animation = ui.ctx().animate_bool(response.id, *on);
        let radius = rect.height() / 2.0;
        let background = if *on {
            Color32::from_rgb(48, 130, 246)
        } else {
            ui.visuals().widgets.inactive.bg_fill
        };
        ui.painter().rect_filled(rect, radius, background);
        let knob_radius = radius - 3.0;
        let center_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), animation);
        ui.painter().circle_filled(
            egui::pos2(center_x, rect.center().y),
            knob_radius,
            Color32::WHITE,
        );
    }
    response
}
