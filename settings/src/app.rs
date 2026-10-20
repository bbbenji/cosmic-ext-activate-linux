// SPDX-License-Identifier: GPL-3.0-or-later

//! The settings window.

use std::collections::HashMap;
use std::time::Duration;

use activate_linux_config::{
    self as settings, APP_ID, Config, Corner, FONT_SIZE_MAX, FONT_SIZE_MIN, FontStyle, MARGIN_MAX,
    Monitors, Rgba, cosmic_config,
};
use cosmic::app::context_drawer;
use cosmic::app::{Core, CosmicFlags, Task};
use cosmic::cctk::sctk::reexports::client::protocol::wl_output::WlOutput;
use cosmic::iced::Color;
use cosmic::iced::event::wayland::{self, OutputEvent};
use cosmic::iced::event::{self, listen_with};
use cosmic::iced::{Alignment, Length, Subscription, time};
use cosmic::prelude::*;
use cosmic::widget::color_picker::{ColorPickerModel, ColorPickerUpdate};
use cosmic::widget::{self, about::About, menu, settings as rows, toaster};

use crate::autostart::Autostart;
use crate::daemon;
use crate::fl;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!(
    "../../resources/icons/hicolor/256x256/apps/io.github.bbbenji.ActivateLinux.png"
);
const DAEMON_POLL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone, Default)]
pub struct Flags;

impl CosmicFlags for Flags {
    type SubCommand = String;
    type Args = Vec<String>;
}

#[derive(Debug, Clone)]
pub enum Message {
    Config(Config),
    CloseToast(widget::ToastId),
    LaunchUrl(String),
    ToggleAbout,
    // Watermark section
    PollDaemon,
    DaemonStatus(bool),
    StartDaemon,
    DaemonStarted(bool),
    Autostart(bool),
    // Text
    MainText(String),
    SecondaryText(String),
    TogglePlaceholders,
    // Font and colour
    FontFilter(String),
    FontFamily(usize),
    FontStyle(usize),
    FontSize(u16),
    ColorPicker(ColorPickerUpdate),
    Opacity(f32),
    TextShadow(bool),
    BackgroundPlate(bool),
    // Position
    Corner(usize),
    MarginX(u32),
    MarginY(u32),
    // Behaviour
    OverWindows(bool),
    ShowOnLockscreen(bool),
    CopyShortcut,
    OpenKeyboardSettings,
    // Displays
    Monitors(usize),
    Output(OutputEvent, WlOutput),
}

pub struct App {
    core: Core,
    about: About,
    handle: Option<cosmic_config::Config>,
    config: Config,
    toasts: widget::Toasts<Message>,
    autostart: Option<Autostart>,
    autostart_enabled: bool,
    daemon_running: bool,
    /// No accelerators yet, but `menu::items` borrows a map for the life of the view.
    // MenuAction is zero-sized; this is the template's key-binding map type.
    #[allow(clippy::zero_sized_map_values)]
    key_binds: HashMap<menu::KeyBind, MenuAction>,
    show_placeholders: bool,
    font_families: Vec<String>,
    font_filter: String,
    /// `font_families` filtered by `font_filter`; what the dropdown shows.
    font_choices: Vec<String>,
    color_picker: ColorPickerModel,
    /// Connected outputs by connector name, maintained from Wayland output events.
    outputs: Vec<(WlOutput, String)>,
    style_labels: Vec<String>,
    corner_labels: Vec<String>,
}

impl cosmic::Application for App {
    type Executor = cosmic::executor::Default;
    type Flags = Flags;
    type Message = Message;
    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Flags) -> (Self, Task<Message>) {
        let handle = settings::open()
            .inspect_err(|why| tracing::error!(%why, "cannot open config"))
            .ok();
        let config = handle.as_ref().map(settings::load).unwrap_or_default();
        let autostart = Autostart::user();

        let about = About::default()
            .name(fl!("app-title"))
            .icon(widget::icon::from_raster_bytes(APP_ICON))
            .version(env!("CARGO_PKG_VERSION"))
            .links([(fl!("repository"), REPOSITORY)])
            .license(env!("CARGO_PKG_LICENSE"));

        let color_picker = color_picker_for(config.font_color);
        let mut app = Self {
            core,
            about,
            handle,
            config,
            toasts: widget::Toasts::new(Message::CloseToast),
            autostart_enabled: autostart.as_ref().is_none_or(Autostart::is_enabled),
            autostart,
            daemon_running: false,
            key_binds: HashMap::new(),
            show_placeholders: false,
            font_families: Vec::new(),
            font_filter: String::new(),
            font_choices: Vec::new(),
            color_picker,
            outputs: Vec::new(),
            style_labels: vec![
                fl!("font-style-normal"),
                fl!("font-style-italic"),
                fl!("font-style-oblique"),
            ],
            corner_labels: vec![
                fl!("corner-bottom-right"),
                fl!("corner-bottom-left"),
                fl!("corner-top-right"),
                fl!("corner-top-left"),
            ],
        };
        app.font_families = crate::fonts::system_families();
        app.font_choices = crate::fonts::filter(&app.font_families, "", &app.config.font_family);
        let title = app.set_title();
        (app, Task::batch([title, poll_daemon()]))
    }

    fn header_start(&self) -> Vec<Element<'_, Message>> {
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            menu::root(fl!("view")).apply(Element::from),
            menu::items(
                &self.key_binds,
                vec![menu::Item::Button(fl!("about"), None, MenuAction::About)],
            ),
        )]);
        vec![menu_bar.into()]
    }

    fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<'_, Message>> {
        self.core.window.show_context.then(|| {
            context_drawer::about(
                &self.about,
                |url| Message::LaunchUrl(url.to_owned()),
                Message::ToggleAbout,
            )
        })
    }

    fn view(&self) -> Element<'_, Message> {
        let spacing = cosmic::theme::spacing();
        let page = widget::column::with_children(self.sections())
            .spacing(spacing.space_m)
            .padding(spacing.space_m)
            .max_width(800);

        let content = widget::scrollable(
            widget::container(page)
                .width(Length::Fill)
                .align_x(Alignment::Center),
        );
        toaster(&self.toasts, content)
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            self.core
                .watch_config::<Config>(APP_ID)
                .map(|update| Message::Config(update.config)),
            time::every(DAEMON_POLL).map(|_| Message::PollDaemon),
            listen_with(|event, _, _| match event {
                event::Event::PlatformSpecific(event::PlatformSpecific::Wayland(
                    wayland::Event::Output(e, output),
                )) => Some(Message::Output(e, output)),
                _ => None,
            }),
        ])
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Config(config) => {
                if config.font_color != self.config.font_color {
                    self.color_picker = color_picker_for(config.font_color);
                }
                self.font_choices = crate::fonts::filter(
                    &self.font_families,
                    &self.font_filter,
                    &config.font_family,
                );
                self.config = config;
            }
            Message::CloseToast(id) => self.toasts.remove(id),
            Message::LaunchUrl(url) => {
                if let Err(why) = open::that_detached(&url) {
                    tracing::error!(%why, url, "failed to open url");
                }
            }
            Message::ToggleAbout => {
                self.core.window.show_context = !self.core.window.show_context;
            }
            Message::PollDaemon => return poll_daemon(),
            Message::DaemonStatus(running) => self.daemon_running = running,
            Message::StartDaemon => {
                return cosmic::task::future(async {
                    let spawned = daemon::start().await;
                    // Give it a moment to claim its bus name before checking.
                    tokio::time::sleep(Duration::from_millis(750)).await;
                    Message::DaemonStarted(spawned && daemon::is_running().await)
                });
            }
            Message::DaemonStarted(running) => {
                self.daemon_running = running;
                if !running {
                    return self.toast(fl!("daemon-start-failed"));
                }
            }
            Message::Autostart(enabled) => {
                if let Some(autostart) = &self.autostart {
                    match autostart.set_enabled(enabled) {
                        Ok(()) => self.autostart_enabled = enabled,
                        Err(why) => {
                            return self.toast(fl!("error-write", reason = why.to_string()));
                        }
                    }
                }
            }
            message => return self.update_setting(message),
        }
        Task::none()
    }
}

impl App {
    fn set_title(&mut self) -> Task<Message> {
        // The header bar draws its own title; set_window_title only names the
        // window for the compositor (taskbar, alt-tab).
        self.set_header_title(fl!("app-title"));
        match self.core.main_window_id() {
            Some(id) => self.set_window_title(fl!("app-title"), id),
            None => Task::none(),
        }
    }

    fn toast(&mut self, message: String) -> Task<Message> {
        self.toasts
            .push(widget::Toast::new(message))
            .map(cosmic::Action::App)
    }

    /// Run a generated config setter. On failure, reload what's on disk so the
    /// UI shows the stored value, and show a toast.
    pub fn set(
        &mut self,
        write: impl FnOnce(&mut Config, &cosmic_config::Config) -> Result<bool, cosmic_config::Error>,
    ) -> Task<Message> {
        let error = match self.handle.as_ref() {
            None => fl!("error-no-config"),
            Some(handle) => match write(&mut self.config, handle) {
                Ok(_) => return Task::none(),
                Err(why) => {
                    tracing::error!(%why, "failed to write setting");
                    self.config = settings::load(handle);
                    fl!("error-write", reason = why.to_string())
                }
            },
        };
        self.toast(error)
    }

    /// Messages from the settings sections, each of which writes one config key.
    fn update_setting(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::MainText(text) => return self.set(|c, h| c.set_main_message(h, text)),
            Message::SecondaryText(text) => {
                return self.set(|c, h| c.set_secondary_message(h, text));
            }
            Message::TogglePlaceholders => self.show_placeholders = !self.show_placeholders,
            Message::FontFilter(query) => {
                self.font_choices =
                    crate::fonts::filter(&self.font_families, &query, &self.config.font_family);
                self.font_filter = query;
            }
            Message::FontFamily(i) => {
                if let Some(family) = self.font_choices.get(i).cloned() {
                    return self.set(|c, h| c.set_font_family(h, family));
                }
            }
            Message::FontStyle(i) => {
                if let Some(&style) = FontStyle::ALL.get(i) {
                    return self.set(|c, h| c.set_font_style(h, style));
                }
            }
            Message::FontSize(size) => return self.set(|c, h| c.set_font_size(h, size)),
            Message::ColorPicker(update) => {
                let applied = matches!(
                    update,
                    ColorPickerUpdate::AppliedColor | ColorPickerUpdate::ActionFinished
                );
                let task = self
                    .color_picker
                    .update::<Message>(update)
                    .map(cosmic::Action::App);
                if applied && let Some(color) = self.color_picker.get_applied_color() {
                    let rgba = Rgba::new(color.r, color.g, color.b, self.config.font_color.a);
                    return Task::batch([task, self.set(|c, h| c.set_font_color(h, rgba))]);
                }
                return task;
            }
            Message::Opacity(percent) => {
                let mut rgba = self.config.font_color;
                rgba.a = (percent / 100.0).clamp(0.0, 1.0);
                return self.set(|c, h| c.set_font_color(h, rgba));
            }
            Message::TextShadow(on) => return self.set(|c, h| c.set_text_shadow(h, on)),
            Message::BackgroundPlate(on) => return self.set(|c, h| c.set_background_plate(h, on)),
            Message::Corner(i) => {
                if let Some(&corner) = Corner::ALL.get(i) {
                    return self.set(|c, h| c.set_corner(h, corner));
                }
            }
            Message::MarginX(x) => return self.set(|c, h| c.set_margin_x(h, x.min(MARGIN_MAX))),
            Message::MarginY(y) => return self.set(|c, h| c.set_margin_y(h, y.min(MARGIN_MAX))),
            Message::OverWindows(on) => return self.set(|c, h| c.set_over_windows(h, on)),
            Message::ShowOnLockscreen(on) => {
                return self.set(|c, h| c.set_show_on_lockscreen(h, on));
            }
            Message::CopyShortcut => {
                return cosmic::iced::clipboard::write(format!(
                    "{} toggle-overlay",
                    daemon::DAEMON_BIN
                ));
            }
            Message::OpenKeyboardSettings => {
                if let Err(why) = std::process::Command::new("cosmic-settings")
                    .arg("keyboard")
                    .spawn()
                {
                    return self.toast(why.to_string());
                }
            }
            Message::Monitors(i) => {
                let names = self.output_names();
                let monitors = match i {
                    0 => Monitors::Automatic,
                    1 => Monitors::All,
                    i => match names.get(i - 2) {
                        Some(name) => Monitors::Output(name.clone()),
                        // The "(disconnected)" entry: keep the saved value.
                        None => return Task::none(),
                    },
                };
                return self.set(|c, h| c.set_monitors(h, monitors));
            }
            Message::Output(output_event, output) => match output_event {
                OutputEvent::Created(Some(info)) | OutputEvent::InfoUpdate(info) => {
                    if let Some(name) = info.name {
                        self.outputs.retain(|(o, _)| *o != output);
                        self.outputs.push((output, name));
                    }
                }
                OutputEvent::Created(None) => {}
                OutputEvent::Removed => self.outputs.retain(|(o, _)| *o != output),
            },
            _ => {}
        }
        Task::none()
    }

    fn output_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self.outputs.iter().map(|(_, n)| n.clone()).collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    fn sections(&self) -> Vec<Element<'_, Message>> {
        vec![
            self.watermark_section(),
            self.text_section(),
            self.font_section(),
            self.position_section(),
            self.behaviour_section(),
            self.displays_section(),
        ]
    }

    fn text_section(&self) -> Element<'_, Message> {
        let mut section = rows::section()
            .title(fl!("section-text"))
            .add(rows::item_row(vec![
                widget::text_input(fl!("main-message"), &self.config.main_message)
                    .label(fl!("main-message"))
                    .on_input(Message::MainText)
                    .into(),
            ]))
            .add(rows::item_row(vec![
                widget::text_input(fl!("secondary-message"), &self.config.secondary_message)
                    .label(fl!("secondary-message"))
                    .on_input(Message::SecondaryText)
                    .into(),
            ]))
            .add(rows::item(
                fl!("placeholders"),
                widget::button::icon(widget::icon::from_name(if self.show_placeholders {
                    "go-up-symbolic"
                } else {
                    "go-down-symbolic"
                }))
                .on_press(Message::TogglePlaceholders),
            ));
        if self.show_placeholders {
            for key in [
                fl!("placeholder-os"),
                fl!("placeholder-kernel"),
                fl!("placeholder-de"),
                fl!("placeholder-wayland-x11"),
                fl!("placeholder-wayland"),
                fl!("placeholder-x11"),
            ] {
                section = section.add(rows::item_row(vec![widget::text::caption(key).into()]));
            }
        }
        section.into()
    }

    fn font_section(&self) -> Element<'_, Message> {
        let family_index = index_of(&self.font_choices, &self.config.font_family);
        let style_index = index_of(&FontStyle::ALL, &self.config.font_style);
        let opacity = (self.config.font_color.a * 100.0).round();

        let mut section = rows::section()
            .title(fl!("section-font"))
            .add(rows::item_row(vec![
                widget::search_input(fl!("font-filter"), &self.font_filter)
                    .on_input(Message::FontFilter)
                    .into(),
            ]))
            .add(rows::item(
                fl!("font-family"),
                widget::dropdown(&self.font_choices, family_index, Message::FontFamily),
            ))
            .add(rows::item(
                fl!("font-style"),
                widget::dropdown(&self.style_labels, style_index, Message::FontStyle),
            ))
            .add(rows::item(
                fl!("font-size"),
                widget::spin_button(
                    self.config.font_size.to_string(),
                    fl!("font-size"),
                    self.config.font_size,
                    1,
                    FONT_SIZE_MIN,
                    FONT_SIZE_MAX,
                    Message::FontSize,
                ),
            ))
            .add(rows::item(
                fl!("font-color"),
                self.color_picker
                    .picker_button(Message::ColorPicker, None)
                    .width(48)
                    .height(32),
            ));
        if self.color_picker.get_is_active() {
            section = section.add(
                self.color_picker
                    .builder(Message::ColorPicker)
                    .reset_label(fl!("color-reset"))
                    .save_label(fl!("color-save"))
                    .cancel_label(fl!("color-cancel"))
                    .build(fl!("color-recent"), fl!("color-copy"), fl!("color-copied")),
            );
        }
        section
            .add(rows::item(
                fl!("opacity"),
                widget::row::with_capacity(2)
                    .push(widget::slider(0.0..=100.0, opacity, Message::Opacity).width(200))
                    .push(widget::text::body(format!("{opacity:.0} %")))
                    .spacing(cosmic::theme::spacing().space_s)
                    .align_y(Alignment::Center),
            ))
            .add(rows::item(
                fl!("text-shadow"),
                widget::toggler(self.config.text_shadow).on_toggle(Message::TextShadow),
            ))
            .add(rows::item(
                fl!("background-plate"),
                widget::toggler(self.config.background_plate).on_toggle(Message::BackgroundPlate),
            ))
            .into()
    }

    fn position_section(&self) -> Element<'_, Message> {
        let corner_index = index_of(&Corner::ALL, &self.config.corner);
        rows::section()
            .title(fl!("section-position"))
            .add(rows::item(
                fl!("corner"),
                widget::dropdown(&self.corner_labels, corner_index, Message::Corner),
            ))
            .add(rows::item(
                fl!("margin-x"),
                widget::spin_button(
                    self.config.margin_x.to_string(),
                    fl!("margin-x"),
                    self.config.margin_x,
                    5,
                    0,
                    MARGIN_MAX,
                    Message::MarginX,
                ),
            ))
            .add(rows::item(
                fl!("margin-y"),
                widget::spin_button(
                    self.config.margin_y.to_string(),
                    fl!("margin-y"),
                    self.config.margin_y,
                    5,
                    0,
                    MARGIN_MAX,
                    Message::MarginY,
                ),
            ))
            .into()
    }

    fn behaviour_section(&self) -> Element<'_, Message> {
        let command = format!("{} toggle-overlay", daemon::DAEMON_BIN);
        rows::section()
            .title(fl!("section-behaviour"))
            .add(
                rows::item::builder(fl!("over-windows"))
                    .description(fl!("over-windows-description"))
                    .toggler(self.config.over_windows, Message::OverWindows),
            )
            .add(
                rows::item::builder(fl!("lock-screen"))
                    .toggler(self.config.show_on_lockscreen, Message::ShowOnLockscreen),
            )
            // Read top to bottom: what the command is, then where to bind it.
            .add(rows::item_row(vec![
                widget::column::with_capacity(2)
                    .push(widget::text::body(fl!("shortcut")))
                    .push(widget::text::monotext(command))
                    .width(Length::Fill)
                    .into(),
                widget::button::standard(fl!("shortcut-copy"))
                    .on_press(Message::CopyShortcut)
                    .into(),
            ]))
            // Like the row above: the text takes the slack and wraps, the
            // button keeps its natural width.
            .add(rows::item_row(vec![
                widget::text::body(fl!("shortcut-description"))
                    .width(Length::Fill)
                    .into(),
                widget::button::standard(fl!("shortcut-open"))
                    .on_press(Message::OpenKeyboardSettings)
                    .into(),
            ]))
            .into()
    }

    fn displays_section(&self) -> Element<'_, Message> {
        let (labels, selected) = monitor_options(&self.output_names(), &self.config.monitors);
        rows::section()
            .title(fl!("section-displays"))
            .add(rows::item(
                fl!("displays"),
                widget::dropdown(labels, Some(selected), Message::Monitors),
            ))
            .into()
    }

    fn watermark_section(&self) -> Element<'_, Message> {
        let status = if self.daemon_running {
            fl!("daemon-running")
        } else {
            fl!("daemon-stopped")
        };
        let status_control = widget::row::with_capacity(2)
            .push(widget::text::body(status))
            .push_maybe((!self.daemon_running).then(|| {
                widget::button::suggested(fl!("daemon-start")).on_press(Message::StartDaemon)
            }))
            .spacing(cosmic::theme::spacing().space_s)
            .align_y(Alignment::Center);

        let autostart = widget::toggler(self.autostart_enabled);
        let autostart = if self.autostart.is_some() {
            autostart.on_toggle(Message::Autostart)
        } else {
            autostart
        };

        rows::section()
            .title(fl!("section-watermark"))
            .add(rows::item(fl!("daemon-status"), status_control))
            .add(rows::item(fl!("start-at-login"), autostart))
            .into()
    }
}

fn color_picker_for(color: Rgba) -> ColorPickerModel {
    let opaque = Color::from_rgb(color.r, color.g, color.b);
    ColorPickerModel::new(
        fl!("color-hex"),
        fl!("color-rgb"),
        Some(Color::WHITE),
        Some(opaque),
    )
}

fn index_of<T: PartialEq>(all: &[T], value: &T) -> Option<usize> {
    all.iter().position(|v| v == value)
}

/// Labels for the display dropdown and the selected index:
/// `[Automatic, All displays, <outputs…>, <saved output if disconnected>]`.
fn monitor_options(outputs: &[String], current: &Monitors) -> (Vec<String>, usize) {
    let mut labels = vec![fl!("displays-automatic"), fl!("displays-all")];
    labels.extend(outputs.iter().cloned());
    let selected = match current {
        Monitors::Automatic => 0,
        Monitors::All => 1,
        Monitors::Output(name) => {
            if let Some(i) = outputs.iter().position(|o| o == name) {
                i + 2
            } else {
                labels.push(fl!("display-disconnected", name = name.clone()));
                labels.len() - 1
            }
        }
    };
    (labels, selected)
}

fn poll_daemon() -> Task<Message> {
    cosmic::task::future(async { Message::DaemonStatus(daemon::is_running().await) })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MenuAction {
    About,
}

impl menu::action::MenuAction for MenuAction {
    type Message = Message;

    fn message(&self) -> Message {
        Message::ToggleAbout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn monitor_options_for_connected_outputs() {
        let (labels, selected) = monitor_options(
            &names(&["DP-1", "HDMI-A-1"]),
            &Monitors::Output("HDMI-A-1".into()),
        );
        assert_eq!(labels[2..], names(&["DP-1", "HDMI-A-1"]));
        assert_eq!(selected, 3);
    }

    #[test]
    fn disconnected_output_stays_selected() {
        let (labels, selected) =
            monitor_options(&names(&["DP-1"]), &Monitors::Output("DP-9".into()));
        assert_eq!(labels.len(), 4);
        assert!(labels[3].contains("DP-9"));
        assert_eq!(selected, 3);
    }

    #[test]
    fn automatic_and_all() {
        assert_eq!(monitor_options(&[], &Monitors::Automatic).1, 0);
        assert_eq!(monitor_options(&[], &Monitors::All).1, 1);
    }
}
