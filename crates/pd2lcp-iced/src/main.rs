#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::LazyLock,
};

#[cfg(target_os = "linux")]
use std::time::{Duration, Instant};

use color_eyre::eyre::Result;
use iced::{
    Alignment, Border, Color, Element,
    Length::{self, Fill},
    Subscription, Task, Theme,
    futures::{SinkExt, Stream},
    keyboard, stream,
    widget::{
        Column, Space, button, checkbox, column as col, container, pick_list, progress_bar, row,
        scrollable, slider, svg::Handle, text,
    },
};

use libpd2lcp::{
    base_game::{install_d2, install_d2_lod, relocate_files},
    error::Error,
    event::{Event, EventNotify},
    filter::{
        Filter, FilterGroup, LOCAL_FILTER_GROUP, activate_filter, delete_filter, download_filter,
        get_filter_authors, get_filters,
    },
    launch::launch,
    pd2_updater::{install_pd2, update_available},
    settings::{GraphicsMode, Settings},
    state::State,
};

#[cfg(target_os = "linux")]
use gilrs::{Axis as GamepadAxis, Button as GamepadButton, EventType as GamepadEventType, Gilrs};

const ICON: &[u8] = include_bytes!("../../../icon.svg");

static EVENT_NOTIFY: LazyLock<EventNotify> = LazyLock::new(EventNotify::default);

mod palette {
    use iced::Color;

    pub const BACKGROUND: Color = Color::from_rgb(0.102, 0.102, 0.118); // #1a1a1e
    pub const SURFACE: Color = Color::from_rgb(0.141, 0.141, 0.161); // #242429
    pub const BORDER: Color = Color::from_rgb(0.220, 0.220, 0.247); // #38383f
    pub const TEXT: Color = Color::from_rgb(0.949, 0.949, 0.953); // #f2f2f3
    pub const TEXT_SECONDARY: Color = Color::from_rgb(0.604, 0.604, 0.635); // #9a9aa2
    pub const ACCENT: Color = Color::from_rgb(0.357, 0.557, 0.937); // #5b8def
    pub const ERROR: Color = Color::from_rgb(0.898, 0.282, 0.298); // #e5484d
    pub const ACTIVE: Color = Color::from_rgb(0.4196, 0.6196, 0.4706); // #6B9E78
    pub const DOWNLOADED: Color = Color::from_rgb(0.7882, 0.6353, 0.1529); // #C9A227
    pub const NEW_AVAILABLE: Color = Color::from_rgb(0.7882, 0.4863, 0.2941); // #C97C4B
}

mod space {
    pub const XS: f32 = 4.0;
    pub const SM: f32 = 8.0;
    pub const MD: f32 = 12.0;
    pub const LG: f32 = 16.0;
    pub const XL: f32 = 24.0;
}

fn lighten(c: Color, amount: f32) -> Color {
    Color {
        r: (c.r + amount).min(1.0),
        g: (c.g + amount).min(1.0),
        b: (c.b + amount).min(1.0),
        a: c.a,
    }
}

fn darken(c: Color, amount: f32) -> Color {
    Color {
        r: (c.r - amount).max(0.0),
        g: (c.g - amount).max(0.0),
        b: (c.b - amount).max(0.0),
        a: c.a,
    }
}

fn page_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(palette::BACKGROUND.into()),
        text_color: Some(palette::TEXT),
        ..Default::default()
    }
}

fn card_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(palette::SURFACE.into()),
        border: Border {
            color: palette::BORDER,
            width: 1.0,
            radius: 12.0.into(),
        },
        text_color: Some(palette::TEXT),
        ..Default::default()
    }
}

fn error_card_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(palette::SURFACE.into()),
        border: Border {
            color: palette::ERROR,
            width: 1.0,
            radius: 12.0.into(),
        },
        text_color: Some(palette::TEXT),
        ..Default::default()
    }
}

fn pick_list_style(_theme: &Theme, _status: pick_list::Status) -> pick_list::Style {
    pick_list::Style {
        text_color: Color::WHITE,
        placeholder_color: Color::WHITE,
        background: palette::SURFACE.into(),
        handle_color: Color::WHITE,
        border: Border {
            radius: 8.0.into(),
            color: palette::BORDER,
            width: 1.0,
        },
    }
}

fn checkbox_style(theme: &Theme, status: checkbox::Status) -> checkbox::Style {
    let base = palette::ACCENT;
    let background = match status {
        checkbox::Status::Active { is_checked } if is_checked => base,
        checkbox::Status::Active { is_checked: _ } => palette::SURFACE,
        checkbox::Status::Hovered { is_checked } if is_checked => lighten(base, 0.06),
        checkbox::Status::Hovered { is_checked: _ } => darken(palette::SURFACE, 0.03),
        checkbox::Status::Disabled { is_checked: _ } => Color { a: 0.32, ..base },
    };

    checkbox::Style {
        background: background.into(),
        border: Border {
            color: palette::BORDER,
            width: 1.0,
            radius: 5.0.into(),
        },
        ..checkbox::primary(theme, status)
    }
}

fn slider_style(_theme: &Theme, status: slider::Status) -> slider::Style {
    let base = palette::ACCENT;
    let background = match status {
        slider::Status::Hovered => lighten(base, 0.06),
        slider::Status::Dragged => darken(base, 0.08),
        slider::Status::Active => base,
    };

    slider::Style {
        rail: slider::Rail {
            backgrounds: (background.into(), palette::SURFACE.into()),
            width: 10.0,
            border: Border {
                color: palette::BORDER,
                width: 1.0,
                radius: 5.0.into(),
            },
        },
        handle: slider::Handle {
            shape: slider::HandleShape::Circle { radius: 8.0 },
            background: background.into(),
            border_width: 0.0,
            border_color: palette::BORDER,
        },
    }
}

fn primary_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let base = palette::ACCENT;
    let background = match status {
        button::Status::Hovered => lighten(base, 0.06),
        button::Status::Pressed => darken(base, 0.08),
        button::Status::Disabled => Color { a: 0.35, ..base },
        button::Status::Active => base,
    };

    button::Style {
        background: Some(background.into()),
        text_color: Color::WHITE,
        border: Border {
            radius: 8.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn active_button_style_scrollable(_theme: &Theme, status: button::Status) -> button::Style {
    let base = palette::ACTIVE;
    let background = match status {
        button::Status::Hovered => lighten(base, 0.06),
        button::Status::Pressed => darken(base, 0.08),
        button::Status::Disabled => Color { a: 0.35, ..base },
        button::Status::Active => base,
    };

    button::Style {
        background: Some(background.into()),
        text_color: Color::BLACK,
        border: Border {
            radius: 0.0.into(),
            width: 0.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn downloaded_button_style_scrollable(_theme: &Theme, status: button::Status) -> button::Style {
    let base = palette::DOWNLOADED;
    let background = match status {
        button::Status::Hovered => lighten(base, 0.06),
        button::Status::Pressed => darken(base, 0.08),
        button::Status::Disabled => Color { a: 0.35, ..base },
        button::Status::Active => base,
    };

    button::Style {
        background: Some(background.into()),
        text_color: Color::BLACK,
        border: Border {
            radius: 0.0.into(),
            width: 0.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn updatable_button_style_scrollable(_theme: &Theme, status: button::Status) -> button::Style {
    let base = palette::NEW_AVAILABLE;
    let background = match status {
        button::Status::Hovered => lighten(base, 0.06),
        button::Status::Pressed => darken(base, 0.08),
        button::Status::Disabled => Color { a: 0.35, ..base },
        button::Status::Active => base,
    };

    button::Style {
        background: Some(background.into()),
        text_color: Color::BLACK,
        border: Border {
            radius: 0.0.into(),
            width: 0.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn primary_button_style_scrollable(_theme: &Theme, status: button::Status) -> button::Style {
    let base = palette::ACCENT;
    let background = match status {
        button::Status::Hovered => lighten(base, 0.06),
        button::Status::Pressed => darken(base, 0.08),
        button::Status::Disabled => Color { a: 0.35, ..base },
        button::Status::Active => base,
    };

    button::Style {
        background: Some(background.into()),
        text_color: Color::WHITE,
        border: Border {
            radius: 0.0.into(),
            width: 0.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn secondary_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let border_color = match status {
        button::Status::Hovered => palette::ACCENT,
        _ => palette::BORDER,
    };

    button::Style {
        background: Some(Color::TRANSPARENT.into()),
        text_color: palette::TEXT,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

fn secondary_button_style_scrollable(_theme: &Theme, status: button::Status) -> button::Style {
    let border_color = match status {
        button::Status::Hovered => palette::ACCENT,
        _ => palette::BORDER,
    };

    button::Style {
        background: Some(Color::TRANSPARENT.into()),
        text_color: palette::TEXT,
        border: Border {
            color: border_color,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn danger_button_style(_theme: &Theme, status: button::Status) -> button::Style {
    let border_color = match status {
        button::Status::Hovered => palette::ACCENT,
        _ => palette::BORDER,
    };

    button::Style {
        background: Some(palette::ERROR.into()),
        text_color: palette::TEXT,
        border: Border {
            color: border_color,
            width: 1.0,
            radius: 8.0.into(),
        },
        ..Default::default()
    }
}

fn progress_style(_theme: &Theme) -> progress_bar::Style {
    progress_bar::Style {
        background: palette::BACKGROUND.into(),
        bar: palette::ACCENT.into(),
        border: Border {
            color: palette::BORDER,
            width: 1.0,
            radius: 6.0.into(),
        },
    }
}

fn scrollable_style(theme: &Theme, status: scrollable::Status) -> scrollable::Style {
    scrollable::Style {
        container: container::Style {
            background: Some(palette::BACKGROUND.into()),
            border: Border {
                color: palette::BORDER,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        },
        vertical_rail: scrollable::Rail {
            background: Some(palette::BACKGROUND.into()),
            border: Border {
                color: palette::BORDER,
                width: 1.0,
                radius: 0.0.into(),
            },
            scroller: scrollable::Scroller {
                background: palette::SURFACE.into(),
                border: Border {
                    color: palette::BORDER,
                    width: 1.0,
                    radius: 0.0.into(),
                },
            },
        },
        ..scrollable::default(theme, status)
    }
}

fn settings_section<'a>(
    title: &'a str,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    col![
        text(title.to_uppercase())
            .size(12)
            .color(palette::TEXT_SECONDARY),
        Space::new().height(space::SM),
        content.into(),
    ]
    .spacing(space::XS)
    .into()
}

fn page<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(
        container(content)
            .padding(space::XL)
            .max_width(420.0)
            .max_height(600.0)
            .style(card_style),
    )
    .width(Fill)
    .height(Fill)
    .center(Fill)
    .style(page_style)
    .into()
}

fn step_indicator<'a>(current: u8, total: u8) -> Element<'a, Message> {
    text(format!("STEP {current} OF {total}"))
        .size(12)
        .color(palette::ACCENT)
        .into()
}

#[derive(Debug)]
struct Launcher {
    // State
    game_state: Option<State>,
    gui_state: GuiState,
    settings: Settings,
    new_settings: Settings,

    // Error mechanism
    error_prev_screen: Option<GuiState>,

    // Runtime values
    // Filter stuff
    selected_author: Option<FilterGroup>,
    selected_filter: Option<(Filter, String)>,
    filter_authors: Option<Vec<FilterGroup>>,
    filters: Option<Vec<(Filter, String)>>,

    // Strings
    launch_button_label: &'static str,
    reset_button_label: &'static str,
    init_screen_text: &'static str,

    // Progress
    downloading_filter: bool,
    installing_progress: Option<(u32, u32)>,

    launch_button_disabled: bool,
    allow_next: bool,
    disable_get_started: bool,

    disallow_filter_group_switching: bool,

    // Reset confirm
    confirm: bool,

    // Controller and keyboard navigation
    focus_index: usize,
    window_focused: bool,
}

impl Launcher {
    pub fn default_with(
        game_state: Option<State>,
        gui_state: GuiState,
        settings: Settings,
    ) -> Launcher {
        Launcher {
            game_state,
            gui_state,
            settings: settings.clone(),
            new_settings: settings,

            error_prev_screen: None,

            selected_author: None,
            selected_filter: None,
            filter_authors: None,
            filters: None,

            launch_button_label: "Launch",
            reset_button_label: "Reset Installation",
            init_screen_text: "PD2LCP is not set up",

            downloading_filter: false,
            installing_progress: None,

            launch_button_disabled: false,
            allow_next: false,
            disable_get_started: false,

            disallow_filter_group_switching: false,

            confirm: false,

            focus_index: 0,
            window_focused: true,
        }
    }
}

#[derive(Clone, Debug)]
enum GuiState {
    Init,
    InstallD2,
    InstallLOD,
    Main,
    Error(String),
    Settings,
    Filters,
}

#[derive(Clone, Debug)]
enum Message {
    // Events
    NotifyEvent(Event),

    // Task results
    Fallible(Result<(), String>),

    ActivateFilterTask(Result<Option<String>, String>),
    InstallerExit(Result<(), String>),
    InitGameState(Result<State, String>),
    LaunchTask(Result<(), String>),
    UpdateCheckTask(Result<bool, String>),
    UpdateTask(Result<(), String>),
    GetFiltersTask(Result<Vec<(Filter, String)>, String>),
    FetchAuthorsTask(Result<Vec<FilterGroup>, String>),
    RelocateFilesTask(Result<(), String>),

    // Settings
    GraphicsMode(GraphicsMode),
    SoundCheckbox(bool),
    BnetCheckbox(bool),
    ForceX11Checkbox(bool),
    UpdatesCheckbox(bool),
    RussianLanguageCheckbox(bool),
    ScaleSlider(f32),

    // Installers selected
    D2InstallerSelected(Option<PathBuf>),
    LODInstallerSelected(Option<PathBuf>),

    // Filters sceren
    FilterAuthorSelected(FilterGroup),
    FilterSelected((Filter, String)),

    // Buttons
    // Init screen
    InitButton,

    // Set up D2 & LOD screens
    FilePickerButtonD2,
    FilePickerButtonLOD,
    NextButton,

    // Main screen
    LaunchButton,
    SettingsButton,
    ExitButton,
    FilterButton,

    // Error screen
    ErrorReturnButton,

    // Settings screen
    ApplyButton,
    ResetButton,

    // Filter screen
    ActivateButton,
    ReturnButton,
    DownloadButton,
    RemoveButton,

    // Keyboard/gamepad navigation
    Navigate(Navigation),
    GamepadNavigate(Navigation),
    WindowFocusChanged(bool),
}

#[derive(Clone, Copy, Debug)]
enum Navigation {
    Previous,
    Next,
    Left,
    Right,
    Activate,
    Back,
}

fn navigation_item_count(state: &Launcher) -> usize {
    match state.gui_state {
        GuiState::Init | GuiState::Error(_) => 1,
        GuiState::InstallD2 | GuiState::InstallLOD => 1 + usize::from(state.allow_next),
        GuiState::Main => 4,
        GuiState::Settings => 10,
        GuiState::Filters => {
            4 + state.filter_authors.as_ref().map_or(0, Vec::len)
                + state.filters.as_ref().map_or(0, Vec::len)
        }
    }
}

fn navigation_message(state: &Launcher) -> Option<Message> {
    let index = state.focus_index;

    match state.gui_state {
        GuiState::Init => Some(Message::InitButton),
        GuiState::InstallD2 => match index {
            0 => Some(Message::FilePickerButtonD2),
            1 if state.allow_next => Some(Message::NextButton),
            _ => None,
        },
        GuiState::InstallLOD => match index {
            0 => Some(Message::FilePickerButtonLOD),
            1 if state.allow_next => Some(Message::NextButton),
            _ => None,
        },
        GuiState::Main => match index {
            0 => Some(Message::FilterButton),
            1 => Some(Message::SettingsButton),
            2 => Some(Message::ExitButton),
            3 => Some(Message::LaunchButton),
            _ => None,
        },
        GuiState::Error(_) => Some(Message::ErrorReturnButton),
        GuiState::Settings => match index {
            0 => Some(Message::ReturnButton),
            1 => Some(Message::GraphicsMode(
                match state.new_settings.graphics_mode {
                    GraphicsMode::DDRAW => GraphicsMode::_3DFX,
                    GraphicsMode::_3DFX => GraphicsMode::DDRAW,
                },
            )),
            2 => Some(Message::SoundCheckbox(!state.new_settings.sndbkg)),
            3 => Some(Message::BnetCheckbox(!state.new_settings.skiptobnet)),
            4 => Some(Message::UpdatesCheckbox(!state.new_settings.no_updates)),
            5 => Some(Message::ForceX11Checkbox(!state.new_settings.force_x11)),
            6 => Some(Message::RussianLanguageCheckbox(
                !state.new_settings.russian_language,
            )),
            7 => None,
            8 => Some(Message::ResetButton),
            9 if state.settings != state.new_settings => Some(Message::ApplyButton),
            _ => None,
        },
        GuiState::Filters => {
            if index == 0 {
                return Some(Message::ReturnButton);
            }

            let author_count = state.filter_authors.as_ref().map_or(0, Vec::len);
            if index <= author_count {
                return state
                    .filter_authors
                    .as_ref()
                    .and_then(|authors| authors.get(index - 1))
                    .cloned()
                    .map(Message::FilterAuthorSelected);
            }

            let filter_index = index - 1 - author_count;
            let filter_count = state.filters.as_ref().map_or(0, Vec::len);
            if filter_index < filter_count {
                return state
                    .filters
                    .as_ref()
                    .and_then(|filters| filters.get(filter_index))
                    .cloned()
                    .map(Message::FilterSelected);
            }

            match index - (1 + author_count + filter_count) {
                0 => Some(Message::ActivateButton),
                1 => Some(Message::DownloadButton),
                2 => Some(Message::RemoveButton),
                _ => None,
            }
        }
    }
}

fn navigation_target<'a>(
    content: impl Into<Element<'a, Message>>,
    focused: bool,
) -> Element<'a, Message> {
    let content = content.into();

    if focused {
        container(content)
            .style(|_| container::Style {
                background: Some(Color::TRANSPARENT.into()),
                border: Border {
                    color: palette::ACCENT,
                    width: 2.0,
                    radius: 9.0.into(),
                },
                ..Default::default()
            })
            .into()
    } else {
        content
    }
}

fn update(state: &mut Launcher, message: Message) -> Task<Message> {
    match message {
        Message::Navigate(navigation) => match navigation {
            Navigation::Previous | Navigation::Next => {
                let count = navigation_item_count(state).max(1);
                state.focus_index = match navigation {
                    Navigation::Previous => (state.focus_index + count - 1) % count,
                    _ => (state.focus_index + 1) % count,
                };
            }
            Navigation::Left | Navigation::Right => match state.gui_state {
                GuiState::Settings if state.focus_index == 1 => {
                    state.new_settings.graphics_mode = match state.new_settings.graphics_mode {
                        GraphicsMode::DDRAW => GraphicsMode::_3DFX,
                        GraphicsMode::_3DFX => GraphicsMode::DDRAW,
                    };
                }
                GuiState::Settings if (2..=6).contains(&state.focus_index) => {
                    let value = matches!(navigation, Navigation::Right);
                    match state.focus_index {
                        2 => state.new_settings.sndbkg = value,
                        3 => state.new_settings.skiptobnet = value,
                        4 => state.new_settings.no_updates = value,
                        5 => state.new_settings.force_x11 = value,
                        6 => state.new_settings.russian_language = value,
                        _ => (),
                    }
                }
                GuiState::Settings if state.focus_index == 7 => {
                    let delta = if matches!(navigation, Navigation::Right) {
                        0.05
                    } else {
                        -0.05
                    };
                    state.new_settings.scale_factor =
                        (state.new_settings.scale_factor + delta).clamp(0.75, 2.0);
                }
                _ => {
                    let count = navigation_item_count(state).max(1);
                    state.focus_index = if matches!(navigation, Navigation::Left) {
                        (state.focus_index + count - 1) % count
                    } else {
                        (state.focus_index + 1) % count
                    };
                }
            },
            Navigation::Activate => {
                if let Some(message) = navigation_message(state) {
                    return update(state, message);
                }
            }
            Navigation::Back => match state.gui_state {
                GuiState::Settings | GuiState::Filters => {
                    return update(state, Message::ReturnButton);
                }
                GuiState::Error(_) => return update(state, Message::ErrorReturnButton),
                GuiState::InstallLOD => {
                    state.gui_state = GuiState::InstallD2;
                    state.focus_index = 0;
                }
                GuiState::Init | GuiState::InstallD2 | GuiState::Main => (),
            },
        },
        Message::GamepadNavigate(navigation) => {
            if state.window_focused {
                return update(state, Message::Navigate(navigation));
            }
        }
        Message::WindowFocusChanged(focused) => state.window_focused = focused,
        Message::ResetButton => 'rst: {
            if !state.confirm {
                state.confirm = true;
                state.reset_button_label = "Confirm";

                break 'rst;
            }

            if let Some(ref g_state) = state.game_state {
                #[cfg(target_os = "linux")]
                let game_state = None;
                #[cfg(target_os = "windows")]
                let game_state = Some(g_state.clone());

                #[cfg(target_os = "linux")]
                let gui_state = GuiState::Init;
                #[cfg(target_os = "windows")]
                let gui_state = GuiState::InstallD2;

                let mut new_state =
                    Launcher::default_with(game_state, gui_state, state.settings.clone());

                // Now we just clean it up
                let base = g_state.base();

                let _ = handle_fallible(
                    &mut new_state,
                    cleanup(false, base).map_err(|e| e.to_string()),
                    |_, _| None,
                );

                *state = new_state;
            }
        }
        Message::Fallible(res) => {
            let _ = handle_fallible(state, res, |_, _| None);
        }
        Message::InstallerExit(res) => {
            state.allow_next = true;
            let _ = handle_fallible(state, res, |_, _| None);
        }
        Message::InitButton => {
            state.disable_get_started = true;

            return Task::perform(State::init(EVENT_NOTIFY.clone()), |game_state| {
                Message::InitGameState(game_state.map_err(|e| e.to_string()))
            });
        }
        Message::NextButton => {
            state.allow_next = false;
            match state.gui_state {
                GuiState::InstallD2 => {
                    state.gui_state = GuiState::InstallLOD;
                    state.focus_index = 0;
                }
                GuiState::InstallLOD => {
                    return Task::perform(relocate_files(state.game_state.clone()), |r| {
                        Message::RelocateFilesTask(r.map_err(|e| e.to_string()))
                    });
                }
                _ => (),
            }
        }
        Message::RelocateFilesTask(res) => match res {
            Ok(_) => {
                state.gui_state = GuiState::Main;
                state.focus_index = 0;

                if let Some(ref game_state) = state.game_state {
                    if let Err(e) = File::create(game_state.base().join("setup_finished")) {
                        display_error(state, e.to_string());
                    }
                } else {
                    display_error(state, Error::Pd2lcpNotInitialised.to_string());
                }
            }
            Err(e) => {
                state.gui_state = GuiState::InstallD2;

                display_error(state, e);
            }
        },
        Message::FilePickerButtonD2 => {
            return Task::perform(
                rfd::AsyncFileDialog::new()
                    .add_filter("exe", &["exe"])
                    .set_can_create_directories(false)
                    .set_title("Select D2 Installer")
                    .pick_file(),
                |p| Message::D2InstallerSelected(p.map(|p| p.path().to_path_buf())),
            );
        }
        Message::FilePickerButtonLOD => {
            return Task::perform(
                rfd::AsyncFileDialog::new()
                    .add_filter("exe", &["exe"])
                    .set_can_create_directories(false)
                    .set_title("Select D2 LOD Installer")
                    .pick_file(),
                |p| Message::LODInstallerSelected(p.map(|p| p.path().to_path_buf())),
            );
        }
        Message::D2InstallerSelected(p) => {
            if let Some(path) = p {
                return Task::perform(install_d2(state.game_state.clone(), path), |r| {
                    Message::InstallerExit(r.map_err(|r| r.to_string()))
                });
            }
        }
        Message::LODInstallerSelected(p) => {
            if let Some(path) = p {
                return Task::perform(install_d2_lod(state.game_state.clone(), path), |r| {
                    Message::InstallerExit(r.map_err(|r| r.to_string()))
                });
            }
        }
        Message::InitGameState(res) => {
            state.disable_get_started = false;
            state.init_screen_text = "PD2LCP is not set up";

            let _ = handle_fallible(state, res, |state, game_state| {
                state.game_state = Some(game_state);
                state.gui_state = GuiState::InstallD2;
                state.focus_index = 0;

                None
            });
        }
        Message::SettingsButton => {
            state.new_settings = state.settings.clone();
            state.confirm = false;
            state.reset_button_label = "Reset Installation";
            state.gui_state = GuiState::Settings;
            state.focus_index = 0;
        }
        Message::ExitButton => {
            return iced::exit();
        }
        Message::ReturnButton => {
            state.new_settings = state.settings.clone();
            state.gui_state = GuiState::Main;
            state.focus_index = 0;
        }
        Message::GraphicsMode(new) => state.new_settings.graphics_mode = new,
        Message::SoundCheckbox(b) => state.new_settings.sndbkg = b,
        Message::BnetCheckbox(b) => state.new_settings.skiptobnet = b,
        Message::UpdatesCheckbox(b) => state.new_settings.no_updates = b,
        Message::RussianLanguageCheckbox(b) => state.new_settings.russian_language = b,
        Message::ForceX11Checkbox(b) => state.new_settings.force_x11 = b,
        Message::ScaleSlider(scale) => state.new_settings.scale_factor = scale / 100.0,
        Message::ApplyButton => {
            state.settings = state.new_settings.clone();

            return Task::perform(
                State::serialise_settings(state.game_state.clone(), state.settings.clone()),
                |res| Message::Fallible(res.map_err(|e| e.to_string())),
            );
        }
        Message::ErrorReturnButton => {
            if let Some(prev) = state.error_prev_screen.take() {
                state.gui_state = prev;
                state.focus_index = 0;
            } else {
                display_error(state, "This shouldn't happen. Report it".to_string());
            }
        }
        Message::LaunchButton => {
            state.launch_button_disabled = true;

            if state.settings.no_updates {
                return Task::perform(
                    launch(state.game_state.clone(), state.settings.clone()),
                    |r| Message::LaunchTask(r.map_err(|e| e.to_string())),
                );
            } else {
                return Task::perform(
                    activate_filter(
                        state.game_state.clone(),
                        state.settings.active_filter.clone().map(|f| f.0),
                        state
                            .settings
                            .active_filter
                            .clone()
                            .map(|f| f.1)
                            .unwrap_or_default(),
                        EVENT_NOTIFY.clone(),
                    ),
                    |r| Message::ActivateFilterTask(r.map_err(|e| e.to_string())),
                )
                .chain(Task::perform(
                    update_available(state.game_state.clone()),
                    |r| Message::UpdateCheckTask(r.map_err(|e| e.to_string())),
                ));
            }
        }
        Message::ActivateFilterTask(res) => {
            if let Some(task) = handle_fallible(state, res, |state, maybe_sha| {
                if let Some(sha) = maybe_sha
                    && let Some((ref mut active_filter, ref mut active_sha)) =
                        state.settings.active_filter
                    && let Some(download_entry_sha) =
                        state.settings.downloaded_filters.get_mut(active_filter)
                {
                    *active_sha = sha.clone();
                    *download_entry_sha = sha;

                    return Some(Task::perform(
                        State::serialise_settings(state.game_state.clone(), state.settings.clone()),
                        |r| Message::Fallible(r.map_err(|e| e.to_string())),
                    ));
                }

                None
            }) {
                return task;
            }
        }
        Message::UpdateCheckTask(res) => {
            if let Some(task) = handle_fallible(state, res, |state, b| {
                if b {
                    state.launch_button_label = "Updating";

                    return Some(Task::perform(
                        install_pd2(state.game_state.clone(), EVENT_NOTIFY.clone()),
                        |r| Message::UpdateTask(r.map_err(|e| e.to_string())),
                    ));
                }

                Some(Task::perform(
                    launch(state.game_state.clone(), state.settings.clone()),
                    |r| Message::LaunchTask(r.map_err(|e| e.to_string())),
                ))
            }) {
                return task;
            }
        }
        Message::UpdateTask(res) => {
            if let Some(task) = handle_fallible(state, res, |state, _| {
                state.launch_button_label = "Launch";
                state.installing_progress = None;

                Some(Task::perform(
                    launch(state.game_state.clone(), state.settings.clone()),
                    |r| Message::LaunchTask(r.map_err(|e| e.to_string())),
                ))
            }) {
                return task;
            }
        }
        Message::LaunchTask(res) => {
            let _ = handle_fallible(state, res, |state, _| {
                state.launch_button_disabled = false;
                None
            });
        }
        Message::NotifyEvent(event) => match event {
            Event::UpdatingPD2 { done, total } => {
                state.installing_progress = Some((done, total));
            }
            Event::DoneUpdating => {
                state.installing_progress = None;
            }
            Event::Error(e) => display_error(state, e),
            Event::DownloadingWine => state.init_screen_text = "Downloading Wine...",
            Event::FinishedDownloadingWine => (),
            Event::InitPrefix => state.init_screen_text = "Setting up the prefix...",
            Event::FinishedInitPrefix => state.init_screen_text = "Done!",
            Event::UpdatingFilter => state.downloading_filter = true,
            Event::DoneUpdatingFilter => state.downloading_filter = false,
        },
        Message::FilterButton => {
            state.filters = None;
            state.selected_filter = None;
            state.selected_author = None;
            state.gui_state = GuiState::Filters;
            state.focus_index = 0;

            return Task::perform(get_filter_authors(), |r| {
                Message::FetchAuthorsTask(r.map_err(|e| e.to_string()))
            });
        }
        Message::FilterAuthorSelected(fg) => {
            state.selected_filter = None;
            state.selected_author = Some(fg.clone());
            state.disallow_filter_group_switching = true;

            return Task::perform(get_filters(state.game_state.clone(), fg), |r| {
                Message::GetFiltersTask(r.map_err(|e| e.to_string()))
            });
        }
        Message::GetFiltersTask(res) => {
            state.disallow_filter_group_switching = false;

            let _ = handle_fallible(state, res, |state, filters| {
                state.filters = Some(filters);
                None
            });
        }
        Message::FilterSelected(f) => {
            state.selected_filter = Some(f);
        }
        Message::FetchAuthorsTask(res) => {
            let _ = handle_fallible(state, res, |state, authors| {
                state.filter_authors = Some(authors);
                None
            });
        }
        Message::ActivateButton => {
            state.settings.active_filter = state.selected_filter.clone();

            return Task::perform(
                State::serialise_settings(state.game_state.clone(), state.settings.clone()),
                |r| Message::Fallible(r.map_err(|e| e.to_string())),
            );
        }
        Message::DownloadButton => {
            if let Some((ref sel_filter, ref sel_filter_sha)) = state.selected_filter {
                state
                    .settings
                    .downloaded_filters
                    .insert(sel_filter.clone(), sel_filter_sha.clone());

                return Task::perform(
                    download_filter(state.game_state.clone(), sel_filter.clone()),
                    |r| Message::Fallible(r.map_err(|e| e.to_string())),
                )
                .chain(Task::perform(
                    State::serialise_settings(state.game_state.clone(), state.settings.clone()),
                    |r| Message::Fallible(r.map_err(|e| e.to_string())),
                ));
            }
        }
        Message::RemoveButton => {
            if let Some((ref sel_filter, _)) = state.selected_filter {
                state.settings.downloaded_filters.remove(sel_filter);

                if let Some((af, _)) = state.settings.active_filter.as_ref()
                    && af == sel_filter
                {
                    state.settings.active_filter = None;
                }

                return Task::perform(
                    delete_filter(state.game_state.clone(), sel_filter.clone()),
                    |r| Message::Fallible(r.map_err(|e| e.to_string())),
                )
                .chain(Task::perform(
                    State::serialise_settings(state.game_state.clone(), state.settings.clone()),
                    |r| Message::Fallible(r.map_err(|e| e.to_string())),
                ))
                .chain(if sel_filter.grouping.name.as_str() == "Local" {
                    state.selected_filter = None;

                    Task::perform(
                        get_filters(state.game_state.clone(), LOCAL_FILTER_GROUP.clone()),
                        |r| Message::GetFiltersTask(r.map_err(|e| e.to_string())),
                    )
                } else {
                    Task::none()
                });
            }
        }
    }

    Task::none()
}

fn view(state: &Launcher) -> Element<'_, Message> {
    match state.gui_state {
        GuiState::Init => init_screen(state),
        GuiState::InstallD2 => install_d2_screen(state),
        GuiState::InstallLOD => install_d2_lod_screen(state),
        GuiState::Main => main_screen(state),
        GuiState::Settings => settings_screen(state),
        GuiState::Error(ref e) => error_screen(e, state.focus_index == 0),
        GuiState::Filters => filters_screen(state),
    }
}

fn init_screen(state: &Launcher) -> Element<'_, Message> {
    let content = col![
        text("Welcome").size(22).color(palette::TEXT),
        text(state.init_screen_text)
            .size(14)
            .color(palette::TEXT_SECONDARY),
        Space::new().height(space::MD),
        navigation_target(
            button(text("Get Started").center())
                .on_press_maybe(if !state.disable_get_started {
                    Some(Message::InitButton)
                } else {
                    None
                })
                .width(Fill)
                .padding(space::MD)
                .style(primary_button_style),
            state.focus_index == 0,
        )
    ]
    .spacing(space::SM)
    .width(Fill);

    page(content)
}

fn install_d2_screen(state: &Launcher) -> Element<'_, Message> {
    let content = col![
        step_indicator(1, 2),
        text("Install Diablo II").size(22).color(palette::TEXT),
        text("Select your Diablo II installer and install it")
            .size(14)
            .color(palette::TEXT_SECONDARY),
        Space::new().height(space::LG),
        navigation_target(
            button(text("Select D2 Installer").center())
                .on_press(Message::FilePickerButtonD2)
                .width(Fill)
                .padding(space::MD)
                .style(secondary_button_style),
            state.focus_index == 0,
        ),
        Space::new().height(space::SM),
        navigation_target(
            button(text("Next").center())
                .on_press_maybe(if state.allow_next {
                    Some(Message::NextButton)
                } else {
                    None
                })
                .width(Fill)
                .padding(space::MD)
                .style(primary_button_style),
            state.focus_index == 1 && state.allow_next,
        )
    ]
    .spacing(space::SM)
    .width(Fill);

    page(content)
}

fn install_d2_lod_screen(state: &Launcher) -> Element<'_, Message> {
    let content = col![
        step_indicator(2, 2),
        text("Install Lord of Destruction")
            .size(22)
            .color(palette::TEXT),
        text("Select your D2 LOD installer and install it")
            .size(14)
            .color(palette::TEXT_SECONDARY),
        Space::new().height(space::LG),
        navigation_target(
            button(text("Select D2 LOD Installer").center())
                .on_press(Message::FilePickerButtonLOD)
                .width(Fill)
                .padding(space::MD)
                .style(secondary_button_style),
            state.focus_index == 0,
        ),
        Space::new().height(space::SM),
        navigation_target(
            button(text("Next").center())
                .on_press_maybe(if state.allow_next {
                    Some(Message::NextButton)
                } else {
                    None
                })
                .width(Fill)
                .padding(space::MD)
                .style(primary_button_style),
            state.focus_index == 1 && state.allow_next,
        )
    ]
    .spacing(space::SM)
    .width(Fill);

    page(content)
}

fn main_screen(state: &Launcher) -> Element<'_, Message> {
    let header = row![
        iced::widget::svg(Handle::from_memory(ICON))
            .height(Length::Fixed(40.0))
            .width(Length::Fixed(40.0)),
        Space::new().width(space::LG),
        text("PD2LCP").size(20).color(palette::TEXT).width(Fill),
        navigation_target(
            button(text("Filters").size(13))
                .on_press(Message::FilterButton)
                .padding([10.0, 12.0])
                .style(secondary_button_style),
            state.focus_index == 0,
        ),
        Space::new().width(space::MD),
        navigation_target(
            button(text("Settings").size(13))
                .on_press(Message::SettingsButton)
                .padding([10.0, 12.0])
                .style(secondary_button_style),
            state.focus_index == 1,
        ),
        Space::new().width(space::MD),
        navigation_target(
            button(text("Exit").size(13))
                .on_press(Message::ExitButton)
                .padding([10.0, 12.0])
                .style(secondary_button_style),
            state.focus_index == 2,
        )
    ]
    .align_y(iced::Alignment::Center);

    let mut content = col![header, Space::new().height(space::LG)]
        .spacing(space::SM)
        .width(Fill);

    content = content.push(navigation_target(
        button(text(state.launch_button_label).center())
            .on_press_maybe(if !state.launch_button_disabled {
                Some(Message::LaunchButton)
            } else {
                None
            })
            .width(Fill)
            .padding(space::MD)
            .style(primary_button_style),
        state.focus_index == 3,
    ));

    if let Some((done, total)) = state.installing_progress {
        content = content.push(Space::new().height(space::MD));

        content = content.push(
            col![
                text(format!("Downloading - {done}/{total}"))
                    .size(13)
                    .color(palette::TEXT_SECONDARY),
                progress_bar(0f32..=total as f32, done as f32).style(progress_style),
            ]
            .spacing(space::XS)
            .width(Fill),
        );
    }

    if state.downloading_filter {
        content = content.push(Space::new().height(space::MD));

        content = content.push(
            text("Downloading filter")
                .size(13)
                .color(palette::TEXT_SECONDARY),
        );
    }

    page(content)
}

fn settings_screen(state: &Launcher) -> Element<'_, Message> {
    let renderers = [GraphicsMode::DDRAW, GraphicsMode::_3DFX];

    let content = col![
        row![
            text("Settings").size(20).color(palette::TEXT).width(Fill),
            navigation_target(
                button(text("Back").size(13))
                    .on_press(Message::ReturnButton)
                    .padding([10.0, 12.0])
                    .style(secondary_button_style),
                state.focus_index == 0,
            ),
        ]
        .align_y(iced::Alignment::Center),
        Space::new().height(space::XS),
        settings_section(
            "Graphics",
            row![
                text("Renderer"),
                Space::new().width(space::SM),
                navigation_target(
                    pick_list(
                        renderers,
                        Some(state.new_settings.graphics_mode),
                        Message::GraphicsMode
                    )
                    .style(pick_list_style),
                    state.focus_index == 1,
                )
            ]
            .align_y(Alignment::Center)
        ),
        Space::new().height(space::MD),
        settings_section(
            "Sound",
            navigation_target(
                checkbox(state.new_settings.sndbkg)
                    .label("Sound in background")
                    .style(checkbox_style)
                    .on_toggle(Message::SoundCheckbox),
                state.focus_index == 2,
            )
        ),
        Space::new().height(space::LG),
        settings_section(
            "Misc",
            col![
                navigation_target(
                    checkbox(state.new_settings.skiptobnet)
                        .label("Skip to battlenet")
                        .style(checkbox_style)
                        .on_toggle(Message::BnetCheckbox),
                    state.focus_index == 3,
                ),
                Space::new().height(space::XS),
                navigation_target(
                    checkbox(state.new_settings.no_updates)
                        .label("Disable updates")
                        .style(checkbox_style)
                        .on_toggle(Message::UpdatesCheckbox),
                    state.focus_index == 4,
                ),
                Space::new().height(space::XS),
                navigation_target(
                    checkbox(state.new_settings.force_x11)
                        .label("Force X11")
                        .style(checkbox_style)
                        .on_toggle(Message::ForceX11Checkbox),
                    state.focus_index == 5,
                ),
                Space::new().height(space::XS),
                navigation_target(
                    checkbox(state.new_settings.russian_language)
                        .label("Russian language")
                        .style(checkbox_style)
                        .on_toggle(Message::RussianLanguageCheckbox),
                    state.focus_index == 6,
                ),
            ]
        ),
        Space::new().height(space::LG),
        settings_section(
            "Launcher",
            col![
                row![
                    text("UI Scale"),
                    Space::new().width(space::MD),
                    text(format!("{:.0}%", state.new_settings.scale_factor * 100.0))
                ]
                .align_y(iced::Alignment::Center),
                Space::new().width(space::LG),
                navigation_target(
                    slider(
                        75.0..=200.0,
                        state.new_settings.scale_factor * 100.0,
                        Message::ScaleSlider
                    )
                    .style(slider_style),
                    state.focus_index == 7,
                )
            ]
        ),
        Space::new().height(space::XS),
        row![
            navigation_target(
                container(
                    button(state.reset_button_label)
                        .style(if state.confirm {
                            danger_button_style
                        } else {
                            secondary_button_style
                        })
                        .on_press(Message::ResetButton),
                )
                .align_left(Fill),
                state.focus_index == 8,
            ),
            navigation_target(
                container(
                    button(text("Apply").size(13))
                        .style(primary_button_style)
                        .on_press_maybe(if state.settings != state.new_settings {
                            Some(Message::ApplyButton)
                        } else {
                            None
                        }),
                )
                .align_right(Fill),
                state.focus_index == 9,
            )
        ]
    ]
    .spacing(space::SM)
    .width(Fill);

    page(content)
}

fn error_screen(message: &str, return_focused: bool) -> Element<'_, Message> {
    let content = col![
        text("Something went wrong").size(20).color(palette::ERROR),
        text(message).size(14).color(palette::TEXT_SECONDARY),
        Space::new().height(space::LG),
        navigation_target(
            button(text("Return").center())
                .on_press(Message::ErrorReturnButton)
                .width(Fill)
                .padding(space::MD)
                .style(primary_button_style),
            return_focused,
        ),
    ]
    .spacing(space::SM)
    .width(Fill);

    container(
        container(content)
            .padding(space::XL)
            .max_width(420.0)
            .style(error_card_style),
    )
    .width(Fill)
    .height(Fill)
    .center(Fill)
    .style(page_style)
    .into()
}

fn filters_screen(state: &Launcher) -> Element<'_, Message> {
    let header = row![
        text("Filters").size(20).color(palette::TEXT).width(Fill),
        navigation_target(
            button(text("Return").size(13))
                .on_press(Message::ReturnButton)
                .padding([10.0, 12.0])
                .style(secondary_button_style),
            state.focus_index == 0,
        )
    ];

    let mut selections = row![];

    let authors_selection = scrollable(
        Column::with_children(if let Some(ref authors) = state.filter_authors {
            authors
                .iter()
                .enumerate()
                .map(|(index, a)| {
                    navigation_target(
                        button(a.name.as_str())
                            .on_press_maybe(if state.disallow_filter_group_switching {
                                None
                            } else {
                                Some(Message::FilterAuthorSelected(a.clone()))
                            })
                            .width(Fill)
                            .style(if Some(a) == state.selected_author.as_ref() {
                                primary_button_style_scrollable
                            } else {
                                secondary_button_style_scrollable
                            }),
                        state.focus_index == index + 1,
                    )
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        })
        .spacing(space::XS),
    )
    .style(scrollable_style)
    .height(Fill)
    .width(Length::FillPortion(1));

    selections = selections.push(authors_selection);

    selections = selections.push(Space::new().width(space::XS));

    let filters_selection = scrollable(Column::with_children(
        if let Some(ref filters) = state.filters {
            filters
                .iter()
                .enumerate()
                .map(|(index, (filter, filter_sha))| {
                    let focus_index = 1 + state.filter_authors.as_ref().map_or(0, Vec::len) + index;
                    navigation_target(
                        button(filter.name.as_str())
                            .on_press_maybe(if state.disallow_filter_group_switching {
                                None
                            } else {
                                Some(Message::FilterSelected((
                                    filter.clone(),
                                    filter_sha.clone(),
                                )))
                            })
                            .width(Fill)
                            .style(
                                if let Some((sf, _)) = state.selected_filter.as_ref()
                                    && sf == filter
                                {
                                    primary_button_style_scrollable
                                } else if let Some((sf, _)) = state.settings.active_filter.as_ref()
                                    && sf == filter
                                {
                                    active_button_style_scrollable
                                } else if let Some(curr_sha) =
                                    state.settings.downloaded_filters.get(filter)
                                {
                                    // We check if it's downloaded at all
                                    // Now we separate if it needs updating or not
                                    if curr_sha != filter_sha {
                                        updatable_button_style_scrollable
                                    } else {
                                        downloaded_button_style_scrollable
                                    }
                                } else {
                                    secondary_button_style_scrollable
                                },
                            ),
                        focus_index == state.focus_index,
                    )
                })
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        },
    ))
    .style(scrollable_style)
    .height(Fill)
    .width(Length::FillPortion(1));

    selections = selections.push(filters_selection);

    let controls = row![
        navigation_target(
            button("Activate")
                .on_press_maybe(
                    if let Some((ref sel_filter, _)) = state.selected_filter
                        && state.settings.active_filter != state.selected_filter
                        && (state.settings.downloaded_filters.contains_key(sel_filter)
                            || state
                                .selected_filter
                                .as_ref()
                                .map(|(f, _)| f.grouping.name.as_str() == "Local")
                                .unwrap_or(false))
                    {
                        Some(Message::ActivateButton)
                    } else {
                        None
                    }
                )
                .style(primary_button_style),
            state.focus_index
                == 1 + state.filter_authors.as_ref().map_or(0, Vec::len)
                    + state.filters.as_ref().map_or(0, Vec::len)
        ),
        Space::new().width(space::XS),
        navigation_target(
            button("Download")
                .on_press_maybe(
                    if let Some((ref sel_filter, ref sel_filter_sha)) = state.selected_filter
                        && state
                            .selected_filter
                            .as_ref()
                            .map(|(f, _)| f.grouping.name.as_str() != "Local")
                            .unwrap_or(false)
                    {
                        // If it's out of date, we should expose Download again
                        if let Some(local_filter_sha) =
                            state.settings.downloaded_filters.get(sel_filter)
                            && local_filter_sha != sel_filter_sha
                        {
                            Some(Message::DownloadButton)
                        } else if !state.settings.downloaded_filters.contains_key(sel_filter) {
                            Some(Message::DownloadButton)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                )
                .style(primary_button_style),
            state.focus_index
                == 2 + state.filter_authors.as_ref().map_or(0, Vec::len)
                    + state.filters.as_ref().map_or(0, Vec::len)
        ),
        Space::new().width(space::XS),
        navigation_target(
            button("Remove")
                .on_press_maybe(
                    if let Some((ref sel_filter, _)) = state.selected_filter
                        && (state.settings.downloaded_filters.contains_key(sel_filter)
                            || state
                                .selected_filter
                                .as_ref()
                                .map(|(f, _)| f.grouping.name.as_str() == "Local")
                                .unwrap_or(false))
                    {
                        Some(Message::RemoveButton)
                    } else {
                        None
                    }
                )
                .style(primary_button_style),
            state.focus_index
                == 3 + state.filter_authors.as_ref().map_or(0, Vec::len)
                    + state.filters.as_ref().map_or(0, Vec::len)
        ),
    ];

    let content = col![
        header,
        Space::new().height(space::MD),
        selections,
        Space::new().height(space::MD),
        controls
    ];

    page(content)
}

fn worker() -> impl Stream<Item = Message> {
    stream::channel(100, async |mut output| {
        let notify = EVENT_NOTIFY.clone();

        loop {
            for event in notify.wait_event().await.expect("mutex poisoned, exiting") {
                output
                    .send(Message::NotifyEvent(event))
                    .await
                    .expect("event notify stream broken");
            }
        }
    })
}

fn keyboard_navigation() -> Subscription<Message> {
    keyboard::listen().filter_map(|event| match event {
        keyboard::Event::KeyPressed {
            key: keyboard::Key::Named(key),
            modifiers,
            repeat: false,
            ..
        } => match key {
            keyboard::key::Named::ArrowUp => Some(Message::Navigate(Navigation::Previous)),
            keyboard::key::Named::ArrowDown => Some(Message::Navigate(Navigation::Next)),
            keyboard::key::Named::ArrowLeft => Some(Message::Navigate(Navigation::Left)),
            keyboard::key::Named::ArrowRight => Some(Message::Navigate(Navigation::Right)),
            keyboard::key::Named::Enter | keyboard::key::Named::Space => {
                Some(Message::Navigate(Navigation::Activate))
            }
            keyboard::key::Named::Escape => Some(Message::Navigate(Navigation::Back)),
            keyboard::key::Named::Tab if modifiers.shift() => {
                Some(Message::Navigate(Navigation::Previous))
            }
            keyboard::key::Named::Tab => Some(Message::Navigate(Navigation::Next)),
            _ => None,
        },
        _ => None,
    })
}

#[cfg(target_os = "linux")]
fn gamepad_worker() -> impl Stream<Item = Message> {
    stream::channel(100, async |mut output| {
        let Ok(mut gilrs) = Gilrs::new() else {
            return;
        };

        let mut last_stick_navigation = Instant::now();

        loop {
            while let Some(event) = gilrs.next_event() {
                let navigation = match event.event {
                    GamepadEventType::ButtonPressed(GamepadButton::DPadUp, _)
                    | GamepadEventType::ButtonRepeated(GamepadButton::DPadUp, _) => {
                        Some(Navigation::Previous)
                    }
                    GamepadEventType::ButtonPressed(GamepadButton::DPadDown, _)
                    | GamepadEventType::ButtonRepeated(GamepadButton::DPadDown, _) => {
                        Some(Navigation::Next)
                    }
                    GamepadEventType::ButtonPressed(GamepadButton::DPadLeft, _)
                    | GamepadEventType::ButtonRepeated(GamepadButton::DPadLeft, _) => {
                        Some(Navigation::Left)
                    }
                    GamepadEventType::ButtonPressed(GamepadButton::DPadRight, _)
                    | GamepadEventType::ButtonRepeated(GamepadButton::DPadRight, _) => {
                        Some(Navigation::Right)
                    }
                    GamepadEventType::ButtonPressed(GamepadButton::South, _)
                    | GamepadEventType::ButtonPressed(GamepadButton::Start, _) => {
                        Some(Navigation::Activate)
                    }
                    GamepadEventType::ButtonPressed(GamepadButton::East, _)
                    | GamepadEventType::ButtonPressed(GamepadButton::Select, _) => {
                        Some(Navigation::Back)
                    }
                    GamepadEventType::AxisChanged(GamepadAxis::LeftStickY, value, _)
                        if value.abs() >= 0.65
                            && last_stick_navigation.elapsed() >= Duration::from_millis(180) =>
                    {
                        last_stick_navigation = Instant::now();
                        Some(if value > 0.0 {
                            Navigation::Next
                        } else {
                            Navigation::Previous
                        })
                    }
                    GamepadEventType::AxisChanged(GamepadAxis::LeftStickX, value, _)
                        if value.abs() >= 0.65
                            && last_stick_navigation.elapsed() >= Duration::from_millis(180) =>
                    {
                        last_stick_navigation = Instant::now();
                        Some(if value > 0.0 {
                            Navigation::Right
                        } else {
                            Navigation::Left
                        })
                    }
                    _ => None,
                };

                if let Some(navigation) = navigation
                    && output
                        .send(Message::GamepadNavigate(navigation))
                        .await
                        .is_err()
                {
                    return;
                }
            }

            tokio::time::sleep(Duration::from_millis(16)).await;
        }
    })
}

fn window_focus_events() -> Subscription<Message> {
    iced::window::events().filter_map(|(_, event)| match event {
        iced::window::Event::Focused => Some(Message::WindowFocusChanged(true)),
        iced::window::Event::Unfocused => Some(Message::WindowFocusChanged(false)),
        _ => None,
    })
}

#[cfg(not(target_os = "linux"))]
fn gamepad_worker() -> impl Stream<Item = Message> {
    stream::channel(1, async |mut _output| std::future::pending::<()>().await)
}

fn subscription(_: &Launcher) -> Subscription<Message> {
    Subscription::batch([
        Subscription::run(worker),
        Subscription::run(gamepad_worker),
        keyboard_navigation(),
        window_focus_events(),
    ])
}

fn main() -> Result<()> {
    // Panic hook so we report stuff that comes up before the GUI initialises
    let default_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |panic_info| {
        rfd::MessageDialog::new()
            .set_level(rfd::MessageLevel::Error)
            .set_title("Panic occurred")
            .set_description(panic_info.to_string())
            .show();

        default_hook(panic_info)
    }));

    let args = std::env::args().collect::<Vec<String>>();

    let fullscreen = args.iter().any(|arg| arg == "-gamemode") || cfg!(feature = "gamemode");

    let mut update_flag = false;

    if args.iter().any(|arg| arg == "-skiplauncher") {
        // Here we just check if we have to update or not, then launch or update
        let state_raw = State::init_raw().expect("Failed to init state");
        let settings = state_raw.deserialise_settings();

        let rt = tokio::runtime::Runtime::new().expect("Failed to initiate tokio runtime");

        if let Ok(true) = rt.block_on(update_available(Some(state_raw.clone())))
            && !settings.no_updates
        {
            // Update is available (if it errors we just launch, maybe they don't have internet connectivity)
            update_flag = true;
        } else {
            // Otherwise we just launch
            rt.block_on(launch(Some(state_raw), settings))
                .expect("Failed to launch pd2");

            std::process::exit(0);
        };
    }

    iced::application(
        move || {
            let state_raw = State::init_raw().expect("Failed to init state");
            let settings = state_raw.deserialise_settings();

            let base_path = state_raw.base();

            let setup_finished_path = base_path.join("setup_finished");

            let (game_state, gui_state) = if !setup_finished_path.exists() {
                if base_path.exists() {
                    cleanup(true, base_path).expect("Failed to clean up");
                }

                #[cfg(target_os = "linux")]
                let ret = (None, GuiState::Init);

                #[cfg(target_os = "windows")]
                let ret = (
                    Some(State::init_raw().expect("Failed to init state")),
                    GuiState::InstallD2,
                );

                ret
            } else {
                (Some(state_raw), GuiState::Main)
            };

            let app = Launcher::default_with(game_state.clone(), gui_state, settings);

            let task = if update_flag {
                Task::perform(install_pd2(game_state, EVENT_NOTIFY.clone()), |r| {
                    Message::UpdateTask(r.map_err(|e| e.to_string()))
                })
            } else {
                Task::none()
            };

            (app, task)
        },
        update,
        view,
    )
    .scale_factor(|state| state.settings.scale_factor)
    .window(iced::window::Settings {
        fullscreen,
        ..Default::default()
    })
    .subscription(subscription)
    .run()?;

    Ok(())
}

fn handle_fallible<T, F>(
    state: &mut Launcher,
    res: Result<T, String>,
    f: F,
) -> Option<Task<Message>>
where
    F: FnOnce(&mut Launcher, T) -> Option<Task<Message>>,
{
    match res {
        Ok(o) => return f(state, o),
        Err(e) => display_error(state, e),
    }

    None
}

fn display_error(state: &mut Launcher, error: String) {
    state.error_prev_screen = Some(state.gui_state.clone());
    state.gui_state = GuiState::Error(error);
    state.focus_index = 0;
}

#[cfg(target_os = "linux")]
fn cleanup(_: bool, path: &Path) -> Result<(), Error> {
    std::fs::remove_dir_all(path).map_err(|e| e.into())
}

#[cfg(target_os = "windows")]
fn cleanup(startup: bool, path: &Path) -> Result<(), Error> {
    use std::fs::read_dir;

    if startup {
        return Ok(());
    }

    for entry in read_dir(path)? {
        if let Ok(entry) = entry {
            let entry = entry.path();

            // Skip the launcher itself
            if let Some(name) = entry.file_name() {
                if Some(name) == std::env::current_exe()?.file_name() {
                    continue;
                }
            }

            if entry.is_dir() {
                std::fs::remove_dir_all(entry)?;
            } else {
                std::fs::remove_file(entry)?;
            }
        }
    }

    Ok(())
}
