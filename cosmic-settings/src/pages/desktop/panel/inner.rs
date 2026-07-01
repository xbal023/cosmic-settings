use cosmic::cctk::sctk::reexports::client::Proxy;
use cosmic::cctk::sctk::reexports::client::backend::ObjectId;
use cosmic::cctk::sctk::reexports::client::protocol::wl_output::WlOutput;
use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};
use cosmic::cosmic_theme::{Density, Roundness};
use cosmic::iced::{Alignment, Length};
use cosmic::widget::{button, container, dropdown, icon, row, settings, slider, space, space::horizontal as horizontal_space, text, toggler};
use cosmic::{Element, Task, surface, theme};

use cosmic::Apply;
use cosmic_config::ConfigSet;
use cosmic_panel_config::{
    AutoHide, CosmicPanelBackground, CosmicPanelConfig, CosmicPanelContainerConfig,
    CosmicPanelOuput, PanelAnchor, PanelSize,
};
use cosmic_settings_page::{self as page, Section};
use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, Default, PartialEq, Eq)]
pub enum ToplevelFilter {
    #[default]
    ActiveWorkspace,
    ConfiguredOutput,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, CosmicConfigEntry)]
#[version = 1]
pub struct AppListConfig {
    pub filter_top_levels: Option<ToplevelFilter>,
    pub favorites: Vec<String>,
    pub enable_drag_source: bool,
    #[serde(default = "default_magnification_enabled")]
    pub magnification_enabled: bool,
    #[serde(default = "default_magnification_scale")]
    pub magnification_scale: f32,
}

fn default_magnification_enabled() -> bool { true }
fn default_magnification_scale() -> f32 { 1.4 }

impl Default for AppListConfig {
    fn default() -> Self {
        Self {
            filter_top_levels: None,
            favorites: Vec::new(),
            enable_drag_source: true,
            magnification_enabled: default_magnification_enabled(),
            magnification_scale: default_magnification_scale(),
        }
    }
}

pub struct PageInner {
    pub(crate) config_helper: Option<cosmic_config::Config>,
    pub(crate) panel_config: Option<CosmicPanelConfig>,
    pub(crate) app_list_config_helper: Option<cosmic_config::Config>,
    pub(crate) app_list_config: Option<AppListConfig>,
    pub opacity: f32,
    pub opacity_changing: bool,
    pub size: PanelSize,
    pub outputs: Vec<String>,
    pub anchors: Vec<String>,
    pub backgrounds: Vec<String>,
    pub(crate) container_config: Option<CosmicPanelContainerConfig>,
    // TODO move these into panel config
    pub(crate) outputs_map: HashMap<ObjectId, (String, WlOutput)>,
    pub(crate) system_default: Option<CosmicPanelConfig>,
    pub(crate) system_container: Option<CosmicPanelContainerConfig>,
}

impl Default for PageInner {
    fn default() -> Self {
        Self {
            config_helper: Option::default(),
            panel_config: Option::default(),
            app_list_config_helper: Option::default(),
            app_list_config: Option::default(),
            opacity: 0.0,
            opacity_changing: false,
            size: PanelSize::M,
            outputs: vec![fl!("all-displays")],
            anchors: vec![
                Anchor(PanelAnchor::Left).to_string(),
                Anchor(PanelAnchor::Right).to_string(),
                Anchor(PanelAnchor::Top).to_string(),
                Anchor(PanelAnchor::Bottom).to_string(),
            ],
            backgrounds: vec![
                Appearance::Match.to_string(),
                Appearance::Light.to_string(),
                Appearance::Dark.to_string(),
            ],
            container_config: Option::default(),
            outputs_map: HashMap::default(),
            system_default: None,
            system_container: cosmic::cosmic_config::Config::system(
                cosmic_panel_config::NAME,
                CosmicPanelConfig::VERSION,
            )
            .map(
                |c| match CosmicPanelContainerConfig::load_from_config(&c, true) {
                    Ok(c) => c,
                    Err((errs, c)) => {
                        for err in errs.into_iter().filter(cosmic_config::Error::is_err) {
                            tracing::error!(?err, "Error when loading Panel container config.");
                        }
                        c
                    }
                },
            )
            .ok(),
        }
    }
}

pub trait PanelPage {
    fn inner(&self) -> &PageInner;

    fn inner_mut(&mut self) -> &mut PageInner;

    fn autohide_label(&self) -> String;

    fn gap_label(&self) -> String;

    fn extend_label(&self) -> String;

    fn configure_applets_label(&self) -> String;

    fn applets_page_id(&self) -> &'static str;
}

pub(crate) fn behavior_and_position<
    P: page::Page<crate::pages::Message> + PanelPage,
    T: Fn(Message) -> crate::pages::Message + Copy + Send + Sync + 'static,
>(
    p: &P,
    msg_map: T,
) -> Section<crate::pages::Message> {
    crate::slab!(descriptions {
        autohide_label = p.autohide_label();
        position = fl!("panel-behavior-and-position", "position");
        display = fl!("panel-behavior-and-position", "display");
    });

    Section::default()
        .title(fl!("panel-behavior-and-position"))
        .descriptions(descriptions)
        .view::<P>(move |_binder, page, section| {
            let descriptions = &section.descriptions;
            let page = page.inner();
            let Some(panel_config) = page.panel_config.as_ref() else {
                return Element::from(text::body(fl!("unknown")));
            };
            settings::section()
                .title(&section.title)
                .add(
                    settings::item::builder(&descriptions[autohide_label])
                        .toggler(panel_config.autohide.is_some(), Message::AutoHidePanel),
                )
                .add(settings::item(
                    &descriptions[position],
                    dropdown::popup_dropdown(
                        page.anchors.as_slice(),
                        Some(panel_config.anchor as usize),
                        Message::PanelAnchor,
                        cosmic::iced::window::Id::RESERVED,
                        Message::Surface,
                        move |a| crate::app::Message::PageMessage(msg_map(a)),
                    ),
                ))
                .add(settings::item(
                    &descriptions[display],
                    dropdown::popup_dropdown(
                        page.outputs.as_slice(),
                        match &panel_config.output {
                            CosmicPanelOuput::All => Some(0),
                            CosmicPanelOuput::Active => None,
                            CosmicPanelOuput::Name(n) => page.outputs.iter().position(|o| o == n),
                        },
                        Message::Output,
                        cosmic::iced::window::Id::RESERVED,
                        Message::Surface,
                        move |a| crate::app::Message::PageMessage(msg_map(a)),
                    ),
                ))
                .apply(Element::from)
                .map(msg_map)
        })
}

pub(crate) fn style<
    P: page::Page<crate::pages::Message> + PanelPage,
    T: Fn(Message) -> crate::pages::Message + Copy + Send + Sync + 'static,
>(
    p: &P,
    msg_map: T,
) -> Section<crate::pages::Message> {
    crate::slab!(descriptions {
        gap_label = p.gap_label();
        extend_label = p.extend_label();
        appearance = fl!("panel-style", "appearance");
        background_opacity = fl!("panel-style", "background-opacity");
        size = fl!("panel-style", "size");
        magnification = String::from("Icon Magnification");
        magnification_scale = String::from("Max Scale");
    });

    Section::default()
        .title(fl!("panel-style"))
        .descriptions(descriptions)
        .view::<P>(move |_binder, page, section| {
            let descriptions = &section.descriptions;
            let inner = page.inner();
            let Some(panel_config) = inner.panel_config.as_ref() else {
                return Element::from(text::body(fl!("unknown")));
            };
            let mut section_builder = settings::section()
                .title(&section.title)
                .add(
                    settings::item::builder(&descriptions[gap_label])
                        .toggler(panel_config.anchor_gap, Message::AnchorGap),
                )
                .add(
                    settings::item::builder(&descriptions[extend_label])
                        .toggler(panel_config.expand_to_edges, Message::ExtendToEdge),
                )
                .add(settings::item(
                    &descriptions[appearance],
                    dropdown::popup_dropdown(
                        inner.backgrounds.as_slice(),
                        match panel_config.background {
                            CosmicPanelBackground::ThemeDefault => Some(0),
                            CosmicPanelBackground::Light => Some(1),
                            CosmicPanelBackground::Dark => Some(2),
                            CosmicPanelBackground::Color(_) => None,
                        },
                        Message::Appearance,
                        cosmic::iced::window::Id::RESERVED,
                        Message::Surface,
                        move |a| crate::app::Message::PageMessage(msg_map(a)),
                    ),
                ))
                .add(settings::item::builder(&descriptions[size]).flex_control({
                    // TODO custom discrete slider variant
                    row::with_children(vec![
                        text::body(fl!("small")).into(),
                        slider(
                            0..=4,
                            match inner.size {
                                PanelSize::XS => 0,
                                PanelSize::S => 1,
                                PanelSize::M => 2,
                                PanelSize::L => 3,
                                PanelSize::XL => 4,
                                PanelSize::Custom(_) => 2,
                            },
                            |v| {
                                if v == 0 {
                                    Message::PanelSize(PanelSize::XS)
                                } else if v == 1 {
                                    Message::PanelSize(PanelSize::S)
                                } else if v == 2 {
                                    Message::PanelSize(PanelSize::M)
                                } else if v == 3 {
                                    Message::PanelSize(PanelSize::L)
                                } else {
                                    Message::PanelSize(PanelSize::XL)
                                }
                            },
                        )
                        .on_release(Message::PanelSizeCommit)
                        .width(Length::Fill)
                        .apply(cosmic::widget::container)
                        .max_width(250)
                        .into(),
                        text::body(fl!("large")).into(),
                    ])
                    .align_y(Alignment::Center)
                    .spacing(8)
                    .width(Length::Fill)
                }))
                .add(
                    settings::item::builder(&descriptions[background_opacity]).flex_control({
                        row::with_capacity(2)
                            .align_y(Alignment::Center)
                            .spacing(8)
                            .width(Length::Fill)
                            .push(
                                text::body(fl!(
                                    "number",
                                    HashMap::from_iter(vec![(
                                        "number",
                                        (panel_config.opacity * 100.0) as i32
                                    )])
                                ))
                                .width(Length::Fixed(22.0))
                                .align_x(Alignment::Center),
                            )
                            .push(
                                slider(0..=100, (panel_config.opacity * 100.0) as i32, |v| {
                                    Message::OpacityRequest(v as f32 / 100.0)
                                })
                                .width(Length::Fill)
                                .apply(container)
                                .max_width(250),
                            )
                    }),
                );

            if let Some(app_list_config) = inner.app_list_config.as_ref() {
                section_builder = section_builder
                    .add(settings::item(
                        &descriptions[magnification],
                        toggler(app_list_config.magnification_enabled).on_toggle(Message::MagnificationEnabled),
                    ))
                    .add(settings::item::builder(&descriptions[magnification_scale]).flex_control({
                        let scale_pct = (app_list_config.magnification_scale * 100.0) as i32;
                        row::with_capacity(2)
                            .align_y(Alignment::Center)
                            .spacing(8)
                            .width(Length::Fill)
                            .push(
                                text::body(fl!(
                                    "number",
                                    HashMap::from_iter(vec![(
                                        "number",
                                        scale_pct
                                    )])
                                ))
                                .width(Length::Fixed(35.0))
                                .align_x(Alignment::Center),
                            )
                            .push(
                                slider(100..=200, scale_pct, |v| {
                                    Message::MagnificationScale(v as f32 / 100.0)
                                })
                                .width(Length::Fill)
                                .apply(container)
                                .max_width(250),
                            )
                    }));
            }

            section_builder
                .apply(Element::from)
                .map(msg_map)
        })
}

pub(crate) fn inner_glow<
    P: page::Page<crate::pages::Message> + PanelPage,
    T: Fn(Message) -> crate::pages::Message + Copy + Send + Sync + 'static,
>(
    _p: &P,
    msg_map: T,
) -> Section<crate::pages::Message> {
    Section::default()
        .title("Kustomisasi Inner Glow")
        .view::<P>(move |_binder, page, section| {
            let inner = page.inner();
            let Some(panel_config) = inner.panel_config.as_ref() else {
                return Element::from(text::body(fl!("unknown")));
            };
            
            let glow = panel_config.inner_glow.clone().unwrap_or_default();

            let section_builder = settings::section()
                .title(&section.title)
                .add(
                    settings::item::builder("Kecerahan Glow").flex_control({
                        row::with_capacity(2)
                            .align_y(Alignment::Center)
                            .spacing(8)
                            .width(Length::Fill)
                            .push(
                                text::body(format!("{}%", (glow.brightness * 100.0) as i32))
                                .width(Length::Fixed(45.0))
                                .align_x(Alignment::Center),
                            )
                            .push(
                                slider(0..=100, (glow.brightness * 100.0) as i32, |v| {
                                    Message::InnerGlowBrightness(v as f32 / 100.0)
                                })
                                .width(Length::Fill)
                                .apply(container)
                                .max_width(250),
                            )
                    })
                )
                .add(
                    settings::item::builder("Tingkat Inner Glow").flex_control({
                        row::with_capacity(2)
                            .align_y(Alignment::Center)
                            .spacing(8)
                            .width(Length::Fill)
                            .push(
                                text::body(format!("{}%", (glow.level * 100.0) as i32))
                                .width(Length::Fixed(45.0))
                                .align_x(Alignment::Center),
                            )
                            .push(
                                slider(0..=100, (glow.level * 100.0) as i32, |v| {
                                    Message::InnerGlowLevel(v as f32 / 100.0)
                                })
                                .width(Length::Fill)
                                .apply(container)
                                .max_width(250),
                            )
                    })
                )
                .add(
                    settings::item("Aktifkan Animasi Glow", toggler(glow.animation_enabled).on_toggle(Message::InnerGlowAnimationEnabled))
                )
                .add(
                    settings::item::builder("Waktu Animasi (ms)").flex_control({
                        row::with_capacity(2)
                            .align_y(Alignment::Center)
                            .spacing(8)
                            .width(Length::Fill)
                            .push(
                                text::body(format!("{} ms", glow.animation_time_ms))
                                .width(Length::Fixed(70.0))
                                .align_x(Alignment::Center),
                            )
                            .push(
                                slider(100..=10000, glow.animation_time_ms, |v| {
                                    Message::InnerGlowAnimationTime(v)
                                })
                                .width(Length::Fill)
                                .apply(container)
                                .max_width(250),
                            )
                    })
                )
                .add(
                    settings::item::builder("Warna Glow (Merah)").flex_control({
                        row::with_capacity(2)
                            .align_y(Alignment::Center)
                            .spacing(8)
                            .width(Length::Fill)
                            .push(
                                text::body(format!("{}%", (glow.color[0] * 100.0) as i32))
                                .width(Length::Fixed(45.0))
                                .align_x(Alignment::Center),
                            )
                            .push(
                                slider(0..=100, (glow.color[0] * 100.0) as i32, |v| {
                                    Message::InnerGlowColorR(v as f32 / 100.0)
                                })
                                .width(Length::Fill)
                                .apply(container)
                                .max_width(250),
                            )
                    })
                )
                .add(
                    settings::item::builder("Warna Glow (Hijau)").flex_control({
                        row::with_capacity(2)
                            .align_y(Alignment::Center)
                            .spacing(8)
                            .width(Length::Fill)
                            .push(
                                text::body(format!("{}%", (glow.color[1] * 100.0) as i32))
                                .width(Length::Fixed(45.0))
                                .align_x(Alignment::Center),
                            )
                            .push(
                                slider(0..=100, (glow.color[1] * 100.0) as i32, |v| {
                                    Message::InnerGlowColorG(v as f32 / 100.0)
                                })
                                .width(Length::Fill)
                                .apply(container)
                                .max_width(250),
                            )
                    })
                )
                .add(
                    settings::item::builder("Warna Glow (Biru)").flex_control({
                        row::with_capacity(2)
                            .align_y(Alignment::Center)
                            .spacing(8)
                            .width(Length::Fill)
                            .push(
                                text::body(format!("{}%", (glow.color[2] * 100.0) as i32))
                                .width(Length::Fixed(45.0))
                                .align_x(Alignment::Center),
                            )
                            .push(
                                slider(0..=100, (glow.color[2] * 100.0) as i32, |v| {
                                    Message::InnerGlowColorB(v as f32 / 100.0)
                                })
                                .width(Length::Fill)
                                .apply(container)
                                .max_width(250),
                            )
                    })
                );

            section_builder
                .apply(Element::from)
                .map(msg_map)
        })
}

pub(crate) fn configuration<P: page::Page<crate::pages::Message> + PanelPage>(
    p: &P,
) -> Section<crate::pages::Message> {
    crate::slab!(descriptions {
        applets_label = p.configure_applets_label();
    });

    Section::default()
        .title(fl!("panel-applets"))
        .descriptions(descriptions)
        .view::<P>(move |binder, page, section| {
            let mut settings = settings::section().title(&section.title);
            let descriptions = &section.descriptions;
            settings = if let Some((panel_applets_entity, _panel_applets_info)) = binder
                .info
                .iter()
                .find(|(_, v)| v.id == page.applets_page_id())
            {
                settings.add(crate::widget::go_next_item(
                    &descriptions[applets_label],
                    crate::pages::Message::Page(panel_applets_entity),
                ))
            } else {
                settings
            };

            Element::from(settings)
        })
}

#[allow(clippy::module_name_repetitions)]
pub(crate) fn add_panel<
    P: page::Page<crate::pages::Message> + PanelPage,
    T: Fn(Message) -> crate::pages::Message + Copy + 'static,
>(
    msg_map: T,
) -> Section<crate::pages::Message> {
    crate::slab!(descriptions {
        reset_to_default = fl!("reset-to-default");
    });

    Section::default()
        .title(fl!("panel-missing"))
        .descriptions(descriptions)
        .view::<P>(move |_binder, _page, section| {
            let descriptions = &section.descriptions;
            button::standard(&descriptions[reset_to_default])
                .on_press(Message::FullReset)
                .apply(Element::from)
                .map(msg_map)
        })
}

#[allow(clippy::too_many_lines)]
pub fn reset_button<
    P: page::Page<crate::pages::Message> + PanelPage,
    T: Fn(Message) -> crate::pages::Message + Copy + 'static,
>(
    msg_map: T,
) -> Section<crate::pages::Message> {
    crate::slab!(descriptions {
        reset_to_default = fl!("reset-to-default");
    });

    Section::default()
        .descriptions(descriptions)
        .view::<P>(move |_binder, page, section| {
            let descriptions = &section.descriptions;
            let inner = page.inner();
            if inner.system_default == inner.panel_config {
                Element::from(space())
            } else {
                button::standard(&descriptions[reset_to_default])
                    .on_press(Message::ResetPanel)
                    .into()
            }
            .map(msg_map)
        })
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Anchor(PanelAnchor);

impl std::fmt::Display for Anchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self.0 {
                PanelAnchor::Top => fl!("panel-top"),
                PanelAnchor::Bottom => fl!("panel-bottom"),
                PanelAnchor::Left => fl!("panel-left"),
                PanelAnchor::Right => fl!("panel-right"),
            }
        )
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Appearance {
    Match,
    Light,
    Dark,
}

impl std::fmt::Display for Appearance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Appearance::Match => fl!("panel-appearance", "match"),
                Appearance::Light => fl!("panel-appearance", "light"),
                Appearance::Dark => fl!("panel-appearance", "dark"),
            }
        )
    }
}

impl TryFrom<CosmicPanelBackground> for Appearance {
    type Error = ();
    fn try_from(value: CosmicPanelBackground) -> Result<Self, Self::Error> {
        match value {
            CosmicPanelBackground::ThemeDefault => Ok(Appearance::Match),
            CosmicPanelBackground::Light => Ok(Appearance::Light),
            CosmicPanelBackground::Dark => Ok(Appearance::Dark),
            _ => Err(()),
        }
    }
}

impl From<Appearance> for CosmicPanelBackground {
    fn from(appearance: Appearance) -> Self {
        match appearance {
            Appearance::Match => CosmicPanelBackground::ThemeDefault,
            Appearance::Light => CosmicPanelBackground::Light,
            Appearance::Dark => CosmicPanelBackground::Dark,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Message {
    // panel messages
    AutoHidePanel(bool),
    PanelAnchor(usize),
    Output(usize),
    AnchorGap(bool),
    PanelSize(PanelSize),
    PanelSizeCommit,
    Appearance(usize),
    ExtendToEdge(bool),
    MagnificationEnabled(bool),
    MagnificationScale(f32),
    OpacityRequest(f32),
    OpacityApply,
    InnerGlowBrightness(f32),
    InnerGlowLevel(f32),
    InnerGlowAnimationEnabled(bool),
    InnerGlowAnimationTime(u32),
    InnerGlowColorR(f32),
    InnerGlowColorG(f32),
    InnerGlowColorB(f32),
    OutputAdded(String, WlOutput),
    OutputRemoved(WlOutput),
    PanelConfig(Box<CosmicPanelConfig>),
    ResetPanel,
    FullReset,
    Surface(surface::Action),
}

impl PageInner {
    pub(crate) fn update_defaults(&mut self) {
        let theme = cosmic::theme::system_preference();
        let theme = theme.cosmic();

        let Some(default) = self.system_default.as_mut() else {
            return;
        };

        let radius = theme.corner_radii;
        let roundness: Roundness = radius.into();

        if default.anchor_gap {
            let radii = theme.corner_radii.radius_xl[0] as u32;
            default.border_radius = radii;
        } else if matches!(roundness, Roundness::Round) && !default.expand_to_edges {
            default.border_radius = 12;
        } else {
            default.border_radius = 0;
        }

        let spacing = theme.spacing;
        let density = Density::from(spacing);
        default.spacing = match density {
            Density::Compact => 0,
            Density::Standard => 0,
            Density::Spacious => 4,
        };

        if self.panel_config.as_ref().is_some_and(|c| c.name == "Dock") {
            default.padding = match roundness {
                Roundness::Round => 4,
                Roundness::SlightlyRound => 4,
                Roundness::Square => 0,
            };
        }
    }

    #[allow(clippy::too_many_lines)]
    pub fn update(&mut self, message: Message) -> Task<Message> {
        let Some(helper) = self.config_helper.as_ref() else {
            return Task::none();
        };

        match &message {
            Message::ResetPanel => {
                if let Some((default, config)) = self
                    .system_default
                    .as_mut()
                    .zip(self.config_helper.as_ref())
                {
                    let theme = cosmic::theme::system_preference();
                    let theme = theme.cosmic();
                    let radius = theme.corner_radii;
                    let roundness: Roundness = radius.into();

                    if default.anchor_gap {
                        let radii = theme.corner_radii.radius_xl[0] as u32;
                        default.border_radius = radii;
                    } else if matches!(roundness, Roundness::Round) && !default.expand_to_edges {
                        default.border_radius = 12;
                    } else {
                        default.border_radius = 0;
                    }

                    if let Err(err) = default.write_entry(config) {
                        tracing::error!(?err, "Error resetting panel config.");
                    }
                    self.size.clone_from(&default.size);
                    self.system_default = Some(default.clone());
                    self.panel_config.clone_from(&self.system_default);
                } else {
                    tracing::error!("Panel config default is missing.");
                }
            }
            Message::FullReset => {
                if let Some(container) = self.system_container.as_ref()
                    && let Err(err) = container.write_entries()
                {
                    tracing::error!(?err, "Error fully resetting the panel config.");
                }
                // update the padding and spacing based on appearance
                let theme = cosmic::theme::system_preference();
                let theme = theme.cosmic();

                let radius = theme.corner_radii;
                let roundness: Roundness = radius.into();
                crate::pages::desktop::appearance::Page::update_panel_radii(roundness);

                let spacing = theme.spacing;
                let density = Density::from(spacing);
                crate::pages::desktop::appearance::Page::update_panel_spacing(density);

                let radius = theme.corner_radii;
                let roundness: Roundness = radius.into();
                crate::pages::desktop::appearance::Page::update_dock_padding(roundness);
            }
            _ => {}
        };

        let Some(panel_config) = self.panel_config.as_mut() else {
            return Task::none();
        };

        match message {
            Message::AutoHidePanel(enabled) => {
                if enabled {
                    _ = panel_config.set_exclusive_zone(helper, false);
                    _ = panel_config.set_autohide(
                        helper,
                        Some(AutoHide {
                            wait_time: 1000,
                            transition_time: 200,
                            handle_size: 4,
                            unhide_delay: 200,
                        }),
                    );
                } else {
                    _ = panel_config.set_exclusive_zone(helper, true);
                    _ = panel_config.set_autohide(helper, None);
                }
            }
            Message::PanelAnchor(i) => {
                if let Some(anchor) = [
                    PanelAnchor::Left,
                    PanelAnchor::Right,
                    PanelAnchor::Top,
                    PanelAnchor::Bottom,
                ]
                .iter()
                .find(|a| Anchor(**a).to_string() == self.anchors[i])
                {
                    _ = panel_config.set_anchor(helper, *anchor);
                }
            }
            Message::Output(i) => {
                if i == 0 {
                    _ = panel_config.set_output(helper, CosmicPanelOuput::All);
                } else {
                    _ = panel_config
                        .set_output(helper, CosmicPanelOuput::Name(self.outputs[i].clone()));
                }
            }
            Message::AnchorGap(enabled) => {
                _ = panel_config.set_anchor_gap(helper, enabled);

                if enabled {
                    _ = panel_config.set_margin(helper, 4);
                } else {
                    _ = panel_config.set_margin(helper, 0);
                }
                let theme = cosmic::theme::system_preference();
                let theme = theme.cosmic();
                let radius = theme.corner_radii.radius_xl[0] as u32;
                let new_radius = if enabled {
                    radius
                } else if !panel_config.expand_to_edges {
                    radius.min(12)
                } else {
                    0
                };
                _ = panel_config.set_border_radius(helper, new_radius).unwrap();
            }
            Message::PanelSize(size) => {
                self.size = size;
            }
            Message::PanelSizeCommit => {
                _ = panel_config.set_size(helper, self.size.clone());
                // Reset any size overrides the user might have set
                _ = panel_config.set_size_center(helper, None);
                _ = panel_config.set_size_wings(helper, None);
            }
            Message::Appearance(a) => {
                if let Some(b) = [Appearance::Match, Appearance::Light, Appearance::Dark]
                    .iter()
                    .find(|b| b.to_string() == self.backgrounds[a])
                {
                    _ = panel_config.set_background(helper, (*b).into());
                }
            }
            Message::ExtendToEdge(enabled) => {
                _ = panel_config.set_expand_to_edges(helper, enabled);

                let theme = cosmic::theme::system_preference();
                let theme = theme.cosmic();
                let radius = theme.corner_radii.radius_xl[0] as u32;
                let new_radius = if panel_config.anchor_gap {
                    radius
                } else if !enabled {
                    radius.min(12)
                } else {
                    0
                };
                _ = panel_config.set_border_radius(helper, new_radius).unwrap();
            }
            Message::MagnificationEnabled(enabled) => {
                if let Some(config) = self.app_list_config.as_mut() {
                    config.magnification_enabled = enabled;
                    if let Some(app_helper) = self.app_list_config_helper.as_ref() {
                        let _ = config.write_entry(app_helper);
                    }
                }
            }
            Message::MagnificationScale(scale) => {
                if let Some(config) = self.app_list_config.as_mut() {
                    config.magnification_scale = scale;
                    if let Some(app_helper) = self.app_list_config_helper.as_ref() {
                        let _ = config.write_entry(app_helper);
                    }
                }
            }
            Message::OpacityRequest(opacity) => {
                panel_config.opacity = opacity;

                if self.opacity_changing {
                    return Task::none();
                }

                self.opacity_changing = true;
                return cosmic::Task::future(async move {
                    tokio::time::sleep(Duration::from_millis(125)).await;
                    Message::OpacityApply
                });
            }

            Message::OpacityApply => {
                self.opacity_changing = false;
                _ = helper.set("opacity", panel_config.opacity);
            }
            
            Message::InnerGlowBrightness(val) => {
                let mut glow = panel_config.inner_glow.clone().unwrap_or_default();
                glow.brightness = val;
                panel_config.inner_glow = Some(glow);
                let _ = panel_config.write_entry(helper);
            }
            Message::InnerGlowLevel(val) => {
                let mut glow = panel_config.inner_glow.clone().unwrap_or_default();
                glow.level = val;
                panel_config.inner_glow = Some(glow);
                let _ = panel_config.write_entry(helper);
            }
            Message::InnerGlowAnimationEnabled(val) => {
                let mut glow = panel_config.inner_glow.clone().unwrap_or_default();
                glow.animation_enabled = val;
                panel_config.inner_glow = Some(glow);
                let _ = panel_config.write_entry(helper);
            }
            Message::InnerGlowAnimationTime(val) => {
                let mut glow = panel_config.inner_glow.clone().unwrap_or_default();
                glow.animation_time_ms = val;
                panel_config.inner_glow = Some(glow);
                let _ = panel_config.write_entry(helper);
            }
            Message::InnerGlowColorR(val) => {
                let mut glow = panel_config.inner_glow.clone().unwrap_or_default();
                glow.color[0] = val;
                panel_config.inner_glow = Some(glow);
                let _ = panel_config.write_entry(helper);
            }
            Message::InnerGlowColorG(val) => {
                let mut glow = panel_config.inner_glow.clone().unwrap_or_default();
                glow.color[1] = val;
                panel_config.inner_glow = Some(glow);
                let _ = panel_config.write_entry(helper);
            }
            Message::InnerGlowColorB(val) => {
                let mut glow = panel_config.inner_glow.clone().unwrap_or_default();
                glow.color[2] = val;
                panel_config.inner_glow = Some(glow);
                let _ = panel_config.write_entry(helper);
            }

            Message::OutputAdded(name, output) => {
                self.outputs.push(name.clone());
                self.outputs_map.insert(output.id(), (name, output));
                return Task::none();
            }
            Message::OutputRemoved(output) => {
                if let Some((name, _)) = self.outputs_map.remove(&output.id())
                    && let Some(pos) = self.outputs.iter().position(|o| o == &name)
                {
                    self.outputs.remove(pos);
                }
            }
            Message::PanelConfig(c) => {
                self.size = c.size.clone();
                self.panel_config = Some(*c);
                return Task::none();
            }
            Message::ResetPanel | Message::FullReset => {}
            Message::Surface(_) => {
                unimplemented!()
            }
        }

        Task::none()
    }
}
