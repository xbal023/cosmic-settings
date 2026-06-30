// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: GPL-3.0-only

pub mod device_profiles;

use cosmic::{
    Apply, Element, Task,
    iced::{Alignment, Length, window},
    surface,
    widget::{self, settings, space::horizontal as horizontal_space},
};
use cosmic_config::{Config, ConfigGet, ConfigSet};
use cosmic_settings_page::{self as page, Section, section};
use cosmic_settings_sound_subscription as subscription;
use slotmap::SlotMap;

const AUDIO_CONFIG: &str = "com.system76.CosmicAudio";
const AMPLIFICATION_SINK: &str = "amplification_sink";
const AMPLIFICATION_SOURCE: &str = "amplification_source";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemSound {
    VolumeChange,
    MessageNew,
    ScreenCapture,
    PowerPlug,
    PowerUnplug,
    AlarmClock,
    TrashEmpty,
}

impl SystemSound {
    pub fn all() -> &'static [Self] {
        &[
            Self::VolumeChange,
            Self::MessageNew,
            Self::ScreenCapture,
            Self::PowerPlug,
            Self::PowerUnplug,
            Self::AlarmClock,
            Self::TrashEmpty,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::VolumeChange => "Volume Ubah",
            Self::MessageNew => "Notifikasi Pesan Baru",
            Self::ScreenCapture => "Tangkapan Layar / Screenshot",
            Self::PowerPlug => "Pengisi Daya Dicolok",
            Self::PowerUnplug => "Pengisi Daya Dicabut",
            Self::AlarmClock => "Baterai Lemah / Alarm",
            Self::TrashEmpty => "Tempat Sampah Dikosongkan",
        }
    }

    pub fn filename(&self) -> &'static str {
        match self {
            Self::VolumeChange => "audio-volume-change.oga",
            Self::MessageNew => "message-new-instant.oga",
            Self::ScreenCapture => "screen-capture.oga",
            Self::PowerPlug => "power-plug.oga",
            Self::PowerUnplug => "power-unplug.oga",
            Self::AlarmClock => "alarm-clock-elapsed.oga",
            Self::TrashEmpty => "trash-empty.oga",
        }
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    /// Reload the model
    Reload,
    /// Change the default output.
    SetDefaultSink(usize),
    /// Change the default input output.
    SetDefaultSource(usize),
    /// Set the profile of a sound device.
    SetProfile(u32, u32),
    /// Change the balance of the active sink.
    SetSinkBalance(u32),
    /// Request to change the default output volume.
    SetSinkVolume(u32),
    /// Request to change the input volume.
    SetSourceVolume(u32),
    /// Messages handled by the sound module in cosmic-settings-subscriptions
    Subscription(subscription::Message),
    /// Surface Action
    Surface(surface::Action),
    /// Toggle the mute status of the output.
    ToggleSinkMute,
    /// Toggle the mute status of the input output.
    ToggleSourceMute,
    /// Toggle amplification for sink
    ToggleOverAmplificationSink(bool),
    /// Toggle amplification for sink
    ToggleOverAmplificationSource(bool),
    TargetSoundSelected(usize),
    CustomSoundUploadPressed,
    CustomSoundFileSelected(Option<std::path::PathBuf>),
    CustomSoundUploadFinished,
}

impl From<Message> for crate::pages::Message {
    fn from(message: Message) -> Self {
        crate::pages::Message::Sound(message)
    }
}

impl From<Message> for crate::Message {
    fn from(message: Message) -> Self {
        crate::Message::PageMessage(message.into())
    }
}

impl From<subscription::Message> for Message {
    fn from(val: subscription::Message) -> Self {
        Message::Subscription(val)
    }
}

pub struct Page {
    entity: page::Entity,
    device_profiles: page::Entity,
    pub(self) model: subscription::Model,
    sound_config: Option<Config>,
    amplification_sink: bool,
    amplification_source: bool,
    selected_target_sound_idx: usize,
}

impl Default for Page {
    fn default() -> Self {
        let mut model = subscription::Model::default();
        model.unplugged_text = fl!("sound-device-port-unplugged");
        model.hd_audio_text = fl!("sound-hd-audio");
        model.usb_audio_text = fl!("sound-usb-audio");
        Self {
            entity: page::Entity::default(),
            device_profiles: page::Entity::default(),
            model,
            sound_config: None,
            amplification_sink: false,
            amplification_source: false,
            selected_target_sound_idx: 0,
        }
    }
}

impl page::Page<crate::pages::Message> for Page {
    fn on_enter(&mut self) -> cosmic::Task<crate::pages::Message> {
        match Config::new(AUDIO_CONFIG, 1) {
            Ok(config) => {
                self.amplification_sink = config.get::<bool>(AMPLIFICATION_SINK).unwrap_or(true);
                self.amplification_source =
                    config.get::<bool>(AMPLIFICATION_SOURCE).unwrap_or(false);
                self.sound_config = Some(config);
            }
            Err(why) => {
                tracing::error!(?why, "Failed to load sound config");
                self.amplification_sink = true;
                self.amplification_source = false;
            }
        }
        Task::none()
    }

    fn content(
        &self,
        sections: &mut SlotMap<section::Entity, Section<crate::pages::Message>>,
    ) -> Option<page::Content> {
        Some(vec![
            sections.insert(output()),
            sections.insert(input()),
            sections.insert(custom_sound_section()),
            sections.insert(device_profiles()),
        ])
    }

    fn info(&self) -> page::Info {
        page::Info::new("sound", "preferences-sound-symbolic")
            .title(fl!("sound"))
            .description(fl!("xdg-entry-sound-comment"))
    }

    fn set_id(&mut self, entity: page::Entity) {
        self.entity = entity;
    }

    fn subscription(
        &self,
        _core: &cosmic::Core,
    ) -> cosmic::iced::Subscription<crate::pages::Message> {
        cosmic::iced::Subscription::run(subscription::watch)
            .map(|message| Message::Subscription(message).into())
    }

    fn on_leave(&mut self) -> Task<crate::pages::Message> {
        *self = Page {
            entity: self.entity,
            device_profiles: self.device_profiles,
            ..Page::default()
        };

        Task::none()
    }
}

impl page::AutoBind<crate::pages::Message> for Page {
    fn sub_pages(
        mut page: page::Insert<crate::pages::Message>,
    ) -> page::Insert<crate::pages::Message> {
        let id = page.sub_page_with_id::<device_profiles::Page>();
        let model = page.model.page_mut::<Page>().unwrap();
        model.device_profiles = id;
        page
    }
}

impl Page {
    pub fn update(&mut self, message: Message) -> Task<crate::app::Message> {
        match message {
            Message::Surface(a) => return cosmic::task::message(crate::app::Message::Surface(a)),

            Message::Subscription(message) => {
                return self
                    .model
                    .update(message)
                    .map(|message| Message::Subscription(message).into());
            }

            Message::SetSinkBalance(balance) => {
                return self
                    .model
                    .set_sink_balance(balance)
                    .map(|message| Message::Subscription(message).into());
            }

            Message::SetDefaultSink(pos) => {
                return self
                    .model
                    .set_default_sink(pos)
                    .map(|message| Message::Subscription(message).into());
            }

            Message::SetDefaultSource(pos) => {
                return self
                    .model
                    .set_default_source(pos)
                    .map(|message| Message::Subscription(message).into());
            }

            Message::ToggleSinkMute => self.model.toggle_sink_mute(),

            Message::ToggleSourceMute => self.model.toggle_source_mute(),

            Message::SetSinkVolume(volume) => {
                return self
                    .model
                    .set_sink_volume(volume)
                    .map(|message| Message::Subscription(message).into());
            }

            Message::SetSourceVolume(volume) => {
                return self
                    .model
                    .set_source_volume(volume)
                    .map(|message| Message::Subscription(message).into());
            }

            Message::ToggleOverAmplificationSink(enabled) => {
                self.amplification_sink = enabled;

                if let Some(config) = &self.sound_config
                    && let Err(why) = config.set(AMPLIFICATION_SINK, enabled)
                {
                    tracing::error!(?why, "Failed to save over amplification setting");
                }
            }

            Message::ToggleOverAmplificationSource(enabled) => {
                self.amplification_source = enabled;

                if let Some(config) = &self.sound_config
                    && let Err(why) = config.set(AMPLIFICATION_SOURCE, enabled)
                {
                    tracing::error!(?why, "Failed to save over amplification setting");
                }
            }

            Message::SetProfile(object_id, index) => {
                self.model.set_profile(object_id, index, true);
            }

            Message::Reload => {
                let mut model = subscription::Model::default();
                model.hd_audio_text = std::mem::take(&mut self.model.hd_audio_text);
                model.unplugged_text = std::mem::take(&mut self.model.unplugged_text);
                model.usb_audio_text = std::mem::take(&mut self.model.usb_audio_text);
                self.model = model;
            }

            Message::TargetSoundSelected(idx) => {
                self.selected_target_sound_idx = idx;
            }

            Message::CustomSoundUploadPressed => {
                return cosmic::Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Pilih Suara Kustom (Audio Apapun)")
                            .add_filter("Audio Files", &["oga", "ogg", "mp3", "wav", "flac", "m4a", "aac", "wma"])
                            .pick_file()
                            .await
                            .map(|handle| handle.path().to_path_buf())
                    },
                    |path_opt| Message::CustomSoundFileSelected(path_opt).into()
                );
            }

            Message::CustomSoundFileSelected(Some(source_path)) => {
                let target_sound = SystemSound::all()[self.selected_target_sound_idx];
                let target_filename = target_sound.filename();

                let dest_dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "~".to_string()))
                    .join(".local/share/sounds/freedesktop/stereo");

                if let Err(e) = std::fs::create_dir_all(&dest_dir) {
                    tracing::error!("Gagal membuat direktori tempat suara kustom: {}", e);
                    return Task::none();
                }

                let dest_path = dest_dir.join(target_filename);
                
                return cosmic::Task::perform(
                    async move {
                        let output = tokio::process::Command::new("ffmpeg")
                            .arg("-y") // Overwrite output files without asking
                            .arg("-i")
                            .arg(&source_path)
                            .arg("-c:a")
                            .arg("libvorbis")
                            .arg(&dest_path)
                            .output()
                            .await;
                        
                        match output {
                            Ok(out) if out.status.success() => {
                                tracing::info!("Berhasil menyimpan & mengonversi suara kustom di: {:?}", dest_path);
                            }
                            Ok(out) => {
                                let err = String::from_utf8_lossy(&out.stderr);
                                tracing::error!("Gagal mengonversi file (ffmpeg error): {}", err);
                            }
                            Err(e) => {
                                tracing::error!("Gagal menjalankan ffmpeg. Apakah ffmpeg sudah terinstal? Error: {}", e);
                            }
                        }
                    },
                    |_| Message::CustomSoundUploadFinished.into()
                );
            }

            Message::CustomSoundFileSelected(None) => {}

            Message::CustomSoundUploadFinished => {}
        }

        Task::none()
    }
}

fn input() -> Section<crate::pages::Message> {
    crate::slab!(descriptions {
        volume = fl!("sound-input", "volume");
        device = fl!("sound-input", "device");
        _level = fl!("sound-input", "level");
        amplification = fl!("amplification");
        amplification_desc = fl!("amplification", "desc");
    });

    Section::default()
        .title(fl!("sound-input"))
        .descriptions(descriptions)
        .view::<Page>(move |_binder, page, section| {
            if page.model.sources().is_empty() {
                return widget::space().into();
            }

            let slider = if page.amplification_source {
                widget::slider(0..=150, page.model.source_volume, |change| {
                    Message::SetSourceVolume(change).into()
                })
                .breakpoints(&[100])
            } else {
                widget::slider(0..=100, page.model.source_volume, |change| {
                    Message::SetSourceVolume(change).into()
                })
            }
            .width(Length::Fill)
            .apply(widget::container)
            .max_width(250.);

            let volume_control = widget::row::with_capacity(4)
                .align_y(Alignment::Center)
                .push(
                    widget::button::icon(widget::icon::from_name(if page.model.source_mute {
                        "microphone-sensitivity-muted-symbolic"
                    } else {
                        "audio-input-microphone-symbolic"
                    }))
                    .on_press(Message::ToggleSourceMute.into()),
                )
                .push(
                    widget::text::body(&page.model.source_volume_text)
                        .width(Length::Fixed(22.0))
                        .align_x(Alignment::Center),
                )
                .push(horizontal_space().width(8.))
                .push(slider);
            let devices = widget::dropdown::popup_dropdown(
                page.model.sources(),
                Some(page.model.active_source().unwrap_or(0)),
                Message::SetDefaultSource,
                window::Id::RESERVED,
                Message::Surface,
                crate::Message::from,
            )
            .apply(Element::from)
            .map(crate::pages::Message::from);

            let mut controls = settings::section()
                .title(&section.title)
                .add(
                    settings::item::builder(&*section.descriptions[volume])
                        .flex_control(volume_control)
                        .align_items(Alignment::Center),
                )
                .add(settings::item(&*section.descriptions[device], devices));

            controls = controls.add(
                settings::item::builder(&*section.descriptions[amplification])
                    .description(&*section.descriptions[amplification_desc])
                    .control(
                        widget::toggler(page.amplification_source)
                            .on_toggle(|t| Message::ToggleOverAmplificationSource(t).into()),
                    ),
            );

            Element::from(controls)
        })
}

fn output() -> Section<crate::pages::Message> {
    crate::slab!(descriptions {
        volume = fl!("sound-output", "volume");
        device = fl!("sound-output", "device");
        _level = fl!("sound-output", "level");
        balance = fl!("sound-output", "balance");
        left = fl!("sound-output", "left");
        right = fl!("sound-output", "right");
        amplification = fl!("amplification");
        amplification_desc = fl!("amplification", "desc");
    });

    Section::default()
        .title(fl!("sound-output"))
        .descriptions(descriptions)
        .view::<Page>(move |_binder, page, section| {
            let slider = if page.amplification_sink {
                widget::slider(0..=150, page.model.sink_volume, |change| {
                    Message::SetSinkVolume(change).into()
                })
                .breakpoints(&[100])
            } else {
                widget::slider(0..=100, page.model.sink_volume, |change| {
                    Message::SetSinkVolume(change).into()
                })
            }
            .width(Length::Fill)
            .apply(widget::container)
            .max_width(250.);

            let volume_control = widget::row::with_capacity(4)
                .align_y(Alignment::Center)
                .push(
                    widget::button::icon(if page.model.sink_mute {
                        widget::icon::from_name("audio-volume-muted-symbolic")
                    } else {
                        widget::icon::from_name("audio-volume-high-symbolic")
                    })
                    .on_press(Message::ToggleSinkMute.into()),
                )
                .push(
                    widget::text::body(&page.model.sink_volume_text)
                        .width(Length::Fixed(22.0))
                        .align_x(Alignment::Center),
                )
                .push(horizontal_space().width(8.))
                .push(slider);

            let devices = widget::dropdown::popup_dropdown(
                page.model.sinks(),
                Some(page.model.active_sink().unwrap_or(0)),
                Message::SetDefaultSink,
                window::Id::RESERVED,
                Message::Surface,
                crate::Message::from,
            )
            .apply(Element::from)
            .map(crate::pages::Message::from);

            let mut controls = settings::section()
                .title(&section.title)
                .add(
                    settings::item::builder(&*section.descriptions[volume])
                        .flex_control(volume_control)
                        .align_items(Alignment::Center),
                )
                .add(settings::item(&*section.descriptions[device], devices))
                .add(settings::item(
                    &*section.descriptions[balance],
                    widget::row::with_capacity(5)
                        .align_y(Alignment::Center)
                        .push(
                            widget::column::with_capacity(2)
                                .align_x(Alignment::Center)
                                .push(
                                    widget::text::body(&*section.descriptions[left])
                                        .align_x(Alignment::Center),
                                )
                                .push(horizontal_space().width(22.)),
                        )
                        .push(horizontal_space().width(8.))
                        .push(
                            widget::slider(
                                0..=200,
                                (page.model.sink_balance.unwrap_or(1.0).max(0.) * 100.).round()
                                    as u32,
                                |change| Message::SetSinkBalance(change).into(),
                            )
                            .breakpoints(&[100]),
                        )
                        .push(horizontal_space().width(8.))
                        .push(
                            widget::column::with_capacity(2)
                                .align_x(Alignment::Center)
                                .push(
                                    widget::text::body(&*section.descriptions[right])
                                        .align_x(Alignment::Center),
                                )
                                .push(horizontal_space().width(22.0)),
                        ),
                ));

            controls = controls.add(
                settings::item::builder(&*section.descriptions[amplification])
                    .description(&*section.descriptions[amplification_desc])
                    .control(
                        widget::toggler(page.amplification_sink)
                            .on_toggle(|t| Message::ToggleOverAmplificationSink(t).into()),
                    ),
            );

            Element::from(controls)
        })
}

/// A section for opening the device profiles sub-page.
fn device_profiles() -> Section<crate::pages::Message> {
    crate::slab!(descriptions {
        button_txt = fl!("sound-device-profiles");
    });

    Section::default()
        .descriptions(descriptions)
        .view::<Page>(move |_binder, page, section| {
            let descriptions = &section.descriptions;
            let button = widget::row::with_children(vec![
                horizontal_space().into(),
                widget::icon::from_name("go-next-symbolic").size(16).into(),
            ]);

            let device_profiles = settings::item::builder(&*descriptions[button_txt])
                .control(button)
                .spacing(16)
                .apply(widget::container)
                .width(Length::Fill)
                .class(cosmic::theme::Container::List)
                .apply(widget::button::custom)
                .width(Length::Fill)
                .class(cosmic::theme::Button::Transparent)
                .on_press(crate::pages::Message::Page(page.device_profiles))
                .width(Length::Fill);

            settings::section().add(device_profiles).into()
        })
}

fn custom_sound_section() -> Section<crate::pages::Message> {
    Section::default()
        .title("Kustomisasi Efek Suara")
        .view::<Page>(move |_binder, page, _section| {
            let sound_options: Vec<String> = SystemSound::all()
                .iter()
                .map(|s| s.label().to_string())
                .collect();

            let sound_dropdown = widget::dropdown::popup_dropdown(
                sound_options,
                Some(page.selected_target_sound_idx),
                |index| Message::TargetSoundSelected(index),
                window::Id::RESERVED,
                Message::Surface,
                crate::Message::from,
            )
            .apply(Element::from)
            .map(crate::pages::Message::from);

            let upload_button = widget::button::text("Unggah Suara (Audio Apapun)")
                .on_press(Message::CustomSoundUploadPressed)
                .apply(Element::from)
                .map(crate::pages::Message::from);

            let controls = settings::section()
                .add(settings::item("Suara Yang Ingin Diganti", sound_dropdown))
                .add(settings::item("Berkas OGG (Lokal)", upload_button));

            Element::from(controls)
        })
}

// fn alerts() -> Section<crate::pages::Message> {
//     let mut descriptions = Slab::new();
//     let volume = descriptions.insert(fl!("sound-alerts", "volume"));
//     let sound = descriptions.insert(fl!("sound-alerts", "sound"));

//     Section::default()
//         .title(fl!("sound-alerts"))
//         .descriptions(descriptions)
//         .view::<Page>(move |_binder, _page, section| {
//             settings::section().title(&section.title)
//                 .add(settings::item(&section.descriptions[volume], text::body("TODO")))
//                 .add(settings::item(&section.descriptions[sound], text::body("TODO")))
//                 .into()
//         })
// }

// fn applications() -> Section<crate::pages::Message> {
//     let mut descriptions = Slab::new();

//     let applications = descriptions.insert(fl!("sound-applications", "desc"));

//     Section::default()
//         .title(fl!("sound-applications"))
//         .descriptions(descriptions)
//         .view::<Page>(move |_binder, _page, section| {
//             settings::section().title(&section.title)
//                 .add(settings::item(
//                     &*section.descriptions[applications],
//                     text::body("TODO"),
//                 ))
//                 .into()
//         })
// }
