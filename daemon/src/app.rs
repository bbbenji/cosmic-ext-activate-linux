// SPDX-License-Identifier: GPL-3.0-or-later

//! The headless libcosmic application that owns the watermark surfaces.

use std::collections::HashMap;

use activate_linux_config::{
    self as settings, APP_ID, Config, DAEMON_ID, SystemInfo, Update, classify, resolve_targets,
};
use cosmic::app::{Core, CosmicFlags, Settings, Task};
use cosmic::cctk::sctk::reexports::client::protocol::wl_output::WlOutput;
use cosmic::iced::event::wayland::{self, OutputEvent};
use cosmic::iced::event::{self, listen_with};
use cosmic::iced::platform_specific::shell::commands::layer_surface::{
    KeyboardInteractivity, set_anchor, set_layer, set_margin, set_show_on_lock,
};
use cosmic::iced::runtime::platform_specific::wayland::layer_surface::{
    IcedOutput, SctkLayerSurfaceSettings,
};
use cosmic::iced::{Font, Subscription, window};
use cosmic::surface::action::{LiveSettings, destroy_layer_shell, simple_layer_shell};
use cosmic::widget::autosize::autosize;
use cosmic::{Element, surface};

use crate::fonts::FontFamilies;
use crate::layout;
use crate::lines::{Lines, lines};
use crate::outputs::Outputs;
use crate::reconcile::{self, Slot};
use crate::view;

/// Start the watermark. Returns once the event loop exits; a second instance
/// hands over to the first through D-Bus and returns straight away.
pub fn run() -> cosmic::iced::Result {
    cosmic::app::run_single_instance::<Daemon>(
        Settings::default()
            .no_main_window(true)
            .exit_on_close(false),
        Flags,
    )
}

#[derive(Debug, Clone, Default)]
pub struct Flags;

impl CosmicFlags for Flags {
    type SubCommand = String;
    type Args = Vec<String>;
}

#[derive(Debug, Clone)]
pub enum Message {
    Config(Config),
    Output(OutputEvent, WlOutput),
    Opened(window::Id),
    Closed(window::Id),
}

pub struct Daemon {
    core: Core,
    config: Config,
    info: SystemInfo,
    lines: Option<Lines>,
    families: FontFamilies,
    font: Font,
    outputs: Outputs<WlOutput>,
    surfaces: HashMap<Slot, window::Id>,
}

impl cosmic::Application for Daemon {
    type Executor = cosmic::executor::Default;
    type Flags = Flags;
    type Message = Message;
    const APP_ID: &'static str = DAEMON_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Flags) -> (Self, Task<Message>) {
        let config = match settings::open() {
            Ok(handle) => settings::load(&handle),
            Err(why) => {
                tracing::warn!(%why, "cannot open config, using defaults");
                Config::default()
            }
        }
        .sanitized();

        let info = SystemInfo::detect();
        let mut families = FontFamilies::default();
        let font = families.font(&config);
        let mut daemon = Self {
            core,
            lines: lines(&config, &info),
            config,
            info,
            families,
            font,
            outputs: Outputs::new(),
            surfaces: HashMap::new(),
        };
        let task = daemon.reconcile();
        (daemon, task)
    }

    fn view(&self) -> Element<'_, Message> {
        // There is no main window; libcosmic still requires this method.
        cosmic::widget::Space::new().into()
    }

    fn view_window(&self, id: window::Id) -> Element<'_, Message> {
        // A layer surface created with `size: None` is only resized to its
        // content through libcosmic's autosize widget; without it the surface
        // stays at its 1×1 minimum.
        autosize(
            view::watermark(&self.config, self.lines.as_ref(), self.font),
            cosmic::widget::Id::new(format!("watermark-{id}")),
        )
        .limits(layout::size_limits())
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            self.core.watch_config::<Config>(APP_ID).map(|update| {
                for why in update.errors {
                    tracing::warn!(%why, "invalid config field, using its default");
                }
                Message::Config(update.config)
            }),
            listen_with(|event, _status, id| match event {
                event::Event::PlatformSpecific(event::PlatformSpecific::Wayland(
                    wayland::Event::Output(output_event, output),
                )) => Some(Message::Output(output_event, output)),
                event::Event::Window(window::Event::Opened { .. }) => Some(Message::Opened(id)),
                _ => None,
            }),
        ])
    }

    /// libcosmic routes a layer surface closed by the compositor (e.g. its
    /// output was unplugged) here, not through `window::Event::Closed`.
    fn on_close_requested(&self, id: window::Id) -> Option<Message> {
        Some(Message::Closed(id))
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Config(config) => self.apply_config(config.sanitized()),
            Message::Output(output_event, output) => {
                let changed = match output_event {
                    OutputEvent::Created(Some(info)) | OutputEvent::InfoUpdate(info) => info
                        .name
                        .is_some_and(|name| self.outputs.upsert(output, name)),
                    OutputEvent::Created(None) => false,
                    OutputEvent::Removed => self.outputs.remove(&output),
                };
                if changed {
                    self.reconcile()
                } else {
                    Task::none()
                }
            }
            // The lock flag can only be set once the surface exists.
            Message::Opened(id) => {
                if self.surfaces.values().any(|s| *s == id) {
                    set_show_on_lock(id, self.config.show_on_lockscreen)
                } else {
                    Task::none()
                }
            }
            // The compositor closes a surface when its output goes away. Our own
            // destroys land here too, but their ids are already forgotten.
            Message::Closed(id) => {
                let before = self.surfaces.len();
                self.surfaces.retain(|_, s| *s != id);
                if self.surfaces.len() == before {
                    Task::none()
                } else {
                    self.reconcile()
                }
            }
        }
    }
}

impl Daemon {
    fn apply_config(&mut self, config: Config) -> Task<Message> {
        let tier = classify(&self.config, &config);
        if tier == Update::None {
            return Task::none();
        }

        self.config = config;
        self.lines = lines(&self.config, &self.info);
        self.font = self.families.font(&self.config);

        match tier {
            // view_window runs again after every update, and the surfaces
            // autosize to the new content.
            Update::None | Update::Content => Task::none(),
            Update::Surface => self.update_surfaces(),
            Update::Rebuild => self.reconcile(),
        }
    }

    fn update_surfaces(&self) -> Task<Message> {
        let anchor = layout::anchor(self.config.corner);
        let margin = layout::margin(&self.config);
        let layer = layout::layer(&self.config);
        let lock = self.config.show_on_lockscreen;

        Task::batch(self.surfaces.values().flat_map(|&id| {
            [
                set_anchor(id, anchor),
                set_margin(id, margin.top, margin.right, margin.bottom, margin.left),
                set_layer(id, layer),
                set_show_on_lock(id, lock),
            ]
        }))
    }

    /// Create and destroy surfaces so they match the monitor preference.
    fn reconcile(&mut self) -> Task<Message> {
        let wanted = reconcile::wanted(&resolve_targets(
            &self.config.monitors,
            &self.outputs.names(),
        ));
        let existing: Vec<Slot> = self.surfaces.keys().cloned().collect();
        let plan = reconcile::plan(&wanted, &existing);

        let mut tasks = Vec::new();
        for slot in plan.destroy {
            if let Some(id) = self.surfaces.remove(&slot) {
                tasks.push(surface::surface_task(destroy_layer_shell(id)));
            }
        }
        for slot in plan.create {
            let output = match &slot {
                Slot::Automatic => IcedOutput::Active,
                Slot::Output(name) => match self.outputs.key_for(name) {
                    Some(output) => IcedOutput::Output(output.clone()),
                    None => continue,
                },
            };
            let id = window::Id::unique();
            self.surfaces.insert(slot, id);
            tasks.push(self.create_surface(id, output));
        }
        Task::batch(tasks)
    }

    fn create_surface(&self, id: window::Id, output: IcedOutput) -> Task<Message> {
        let layer = layout::layer(&self.config);
        let anchor = layout::anchor(self.config.corner);
        let margin = layout::margin(&self.config);

        surface::surface_task(simple_layer_shell(
            // Blur is off: nothing but the text may change what is behind it.
            || LiveSettings {
                blur: Some(false),
                ..LiveSettings::default()
            },
            move || SctkLayerSurfaceSettings {
                id,
                layer,
                anchor,
                margin,
                output: output.clone(),
                keyboard_interactivity: KeyboardInteractivity::None,
                // An empty input region lets every click through.
                input_zone: Some(Vec::new()),
                namespace: layout::NAMESPACE.into(),
                size: None,
                // 0 respects the panel's exclusive zone instead of overlapping it.
                exclusive_zone: 0,
                size_limits: layout::size_limits(),
            },
            None::<fn() -> Element<'static, cosmic::Action<Message>>>,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmic::Application;

    fn daemon_with(surfaces: HashMap<Slot, window::Id>) -> Daemon {
        let config = Config::default();
        let info = SystemInfo::from_sources(None, None, None, None);
        let mut families = FontFamilies::default();
        Daemon {
            core: Core::default(),
            lines: lines(&config, &info),
            font: families.font(&config),
            config,
            info,
            families,
            outputs: Outputs::new(),
            surfaces,
        }
    }

    #[test]
    fn compositor_closed_surface_is_reported_as_closed() {
        let id = window::Id::unique();
        let daemon = daemon_with(HashMap::from([(Slot::Automatic, id)]));
        assert!(matches!(
            daemon.on_close_requested(id),
            Some(Message::Closed(closed)) if closed == id
        ));
    }

    #[test]
    fn closed_automatic_surface_is_replaced() {
        let id = window::Id::unique();
        let mut daemon = daemon_with(HashMap::from([(Slot::Automatic, id)]));
        let message = daemon.on_close_requested(id).expect("close is reported");
        let _ = daemon.update(message);
        let replacement = daemon.surfaces.get(&Slot::Automatic).copied();
        assert!(replacement.is_some_and(|new| new != id));
    }
}
