use eframe::egui;
use eframe::egui::{Color32, Rgba, Rounding, Stroke, Vec2};
use iced::pure::widget::{button, radio, Column, Row};
use iced::pure::Element;
use iced::{Alignment, Background, Color};
use iced_aw::pure::card;

pub fn grid<'a, Message: 'static>(
    iter: Vec<impl Into<Element<'a, Message>>>,
    columns: usize,
    row_gap: u16,
    col_gap: u16,
) -> Column<'a, Message> {
    let mut col: Column<Message> = Column::new()
        .spacing(row_gap)
        .align_items(Alignment::Center);
    let mut row: Row<Message> = Row::new().spacing(col_gap).align_items(Alignment::Center);
    for (j, o) in iter.into_iter().enumerate() {
        if j % columns == 0 && j > 0 {
            col = col.push(row);
            row = Row::new().spacing(col_gap).align_items(Alignment::Center);
        }
        row = row.push(o);
    }
    col.push(row)
}

pub const ACTIVE_BLUE_OLD: Color = Color::from_rgb(
    0x72 as f32 / 255.0,
    0x89 as f32 / 255.0,
    0xDA as f32 / 255.0,
);
pub const FOREST_GREEN_OLD: Color = Color::from_rgb(
    0x22 as f32 / 255.0,
    0x8B as f32 / 255.0,
    0x22 as f32 / 255.0,
);
pub static CUSTOM_RED_OLD: Color = Color::from_rgb(
    0xC0 as f32 / 255.0,
    0x1C as f32 / 255.0,
    0x1C as f32 / 255.0,
);
pub const LIGHT_GRAY_OLD: Color = Color::from_rgb(
    0xC2 as f32 / 255.0,
    0xC2 as f32 / 255.0,
    0xC2 as f32 / 255.0,
);

const HOVERED_OLD: Color = Color::from_rgb(
    0x67 as f32 / 255.0,
    0x7B as f32 / 255.0,
    0xC4 as f32 / 255.0,
);

pub const BACKGROUND_OLD: Color = Color::from_rgb(
    0x2F as f32 / 255.0,
    0x31 as f32 / 255.0,
    0x36 as f32 / 255.0,
);

const BORDER_RADIUS_OLD: f32 = 35.0;

pub struct Submit;
impl button::StyleSheet for Submit {
    fn active(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(Color {
                a: 0.2,
                ..LIGHT_GRAY_OLD
            })),
            border_radius: BORDER_RADIUS_OLD,
            text_color: FOREST_GREEN_OLD,
            ..button::Style::default()
        }
    }

    fn hovered(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(Color {
                a: 0.4,
                ..LIGHT_GRAY_OLD
            })),
            ..self.active()
        }
    }

    fn pressed(&self) -> button::Style {
        button::Style {
            border_width: 3.0,
            border_color: Color::WHITE,
            ..self.hovered()
        }
    }
}

pub struct Cancel;
impl button::StyleSheet for Cancel {
    fn active(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(Color {
                a: 0.2,
                ..LIGHT_GRAY_OLD
            })),
            border_radius: BORDER_RADIUS_OLD,
            text_color: CUSTOM_RED_OLD,
            ..button::Style::default()
        }
    }

    fn hovered(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(Color {
                a: 0.4,
                ..LIGHT_GRAY_OLD
            })),
            ..self.active()
        }
    }

    fn pressed(&self) -> button::Style {
        button::Style {
            border_width: 3.0,
            border_color: Color::WHITE,
            ..self.hovered()
        }
    }
}

pub struct Select;
impl button::StyleSheet for Select {
    fn active(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(Color {
                a: 0.4,
                ..LIGHT_GRAY_OLD
            })),
            border_radius: BORDER_RADIUS_OLD,
            text_color: Color::BLACK,
            ..button::Style::default()
        }
    }

    fn hovered(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(HOVERED_OLD)),
            text_color: Color::WHITE,
            ..self.active()
        }
    }

    fn pressed(&self) -> button::Style {
        button::Style {
            border_width: 3.0,
            border_color: Color::WHITE,
            ..self.hovered()
        }
    }
}

pub struct Done;
impl button::StyleSheet for Done {
    fn active(&self) -> button::Style {
        button::Style {
            background: None,
            border_radius: BORDER_RADIUS_OLD,
            text_color: FOREST_GREEN_OLD,
            ..button::Style::default()
        }
    }

    fn hovered(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(Color {
                a: 0.1,
                ..LIGHT_GRAY_OLD
            })),
            ..self.active()
        }
    }

    fn pressed(&self) -> button::Style {
        button::Style {
            border_width: 3.0,
            border_color: Color::WHITE,
            ..self.hovered()
        }
    }
}

pub struct Transparent;
impl button::StyleSheet for Transparent {
    fn active(&self) -> button::Style {
        button::Style {
            background: None,
            border_radius: BORDER_RADIUS_OLD,
            text_color: Color::BLACK,
            ..button::Style::default()
        }
    }

    fn hovered(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(Color {
                a: 0.1,
                ..LIGHT_GRAY_OLD
            })),
            ..self.active()
        }
    }

    fn pressed(&self) -> button::Style {
        button::Style {
            border_width: 3.0,
            border_color: Color::WHITE,
            ..self.hovered()
        }
    }
}

pub struct TransparentCancel;
impl button::StyleSheet for TransparentCancel {
    fn active(&self) -> button::Style {
        button::Style {
            background: None,
            border_radius: BORDER_RADIUS_OLD,
            text_color: CUSTOM_RED_OLD,
            ..button::Style::default()
        }
    }

    fn hovered(&self) -> button::Style {
        button::Style {
            background: Some(Background::Color(Color {
                a: 0.1,
                ..LIGHT_GRAY_OLD
            })),
            ..self.active()
        }
    }

    fn pressed(&self) -> button::Style {
        button::Style {
            border_width: 3.0,
            border_color: Color::WHITE,
            ..self.hovered()
        }
    }
}

pub struct Radio;
impl radio::StyleSheet for Radio {
    fn active(&self) -> radio::Style {
        radio::Style {
            background: Background::Color(Color::WHITE),
            dot_color: ACTIVE_BLUE_OLD,
            border_width: 3.0,
            border_color: LIGHT_GRAY_OLD,
            text_color: Some(Color::BLACK),
        }
    }

    fn hovered(&self) -> radio::Style {
        radio::Style {
            border_color: ACTIVE_BLUE_OLD,
            ..self.active()
        }
    }
}

pub struct Status;
impl card::StyleSheet for Status {
    fn active(&self) -> card::Style {
        card::Style {
            border_radius: 7.5,
            head_background: Background::Color(ACTIVE_BLUE_OLD),
            head_text_color: Color::WHITE,
            body_background: Background::Color(Color::WHITE),
            body_text_color: Color::BLACK,
            close_color: Color::WHITE,
            ..card::Style::default()
        }
    }
}

pub struct Success;
impl card::StyleSheet for Success {
    fn active(&self) -> card::Style {
        card::Style {
            border_radius: 7.5,
            head_background: Background::Color(FOREST_GREEN_OLD),
            head_text_color: Color::WHITE,
            body_background: Background::Color(Color::WHITE),
            body_text_color: Color::BLACK,
            close_color: Color::WHITE,
            ..card::Style::default()
        }
    }
}

pub struct Error;
impl card::StyleSheet for Error {
    fn active(&self) -> card::Style {
        card::Style {
            border_radius: 7.5,
            head_background: Background::Color(CUSTOM_RED_OLD),
            head_text_color: Color::WHITE,
            body_background: Background::Color(Color::WHITE),
            body_text_color: Color::BLACK,
            close_color: Color::WHITE,
            ..card::Style::default()
        }
    }
}

pub const LIGHT_GRAY: Rgba = Rgba::from_rgb(
    0xC2 as f32 / 255.0,
    0xC2 as f32 / 255.0,
    0xC2 as f32 / 255.0,
);

pub const HOVERED: Rgba = Rgba::from_rgb(
    0x67 as f32 / 255.0,
    0x7B as f32 / 255.0,
    0xC4 as f32 / 255.0,
);

pub const BORDER_RADIUS: f32 = 15.0;

pub enum Style {
    LauncherControls,
    LauncherSelection,
}

pub fn style_ui(ui: &mut egui::Ui, style: Style) {
    match style {
        Style::LauncherControls => {
            ui.spacing_mut().item_spacing = Vec2::splat(4.0);
            ui.spacing_mut().button_padding = Vec2::splat(4.0);
            ui.visuals_mut().button_frame = false;
            ui.visuals_mut().widgets.active.rounding = Rounding::same(10.0);
            ui.visuals_mut().widgets.hovered.rounding = Rounding::same(10.0);
            ui.visuals_mut().widgets.inactive.rounding = Rounding::same(10.0);
            ui.visuals_mut().widgets.noninteractive.rounding = Rounding::same(10.0);
        }
        Style::LauncherSelection => {
            ui.spacing_mut().item_spacing = Vec2::splat(5.0);
            ui.spacing_mut().button_padding = Vec2::new(10.0, 4.0);
            ui.visuals_mut().widgets.inactive.rounding = Rounding::same(BORDER_RADIUS);
            ui.visuals_mut().widgets.inactive.bg_fill = LIGHT_GRAY.multiply(0.4).into();
            ui.visuals_mut().widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::BLACK);
            ui.visuals_mut().widgets.hovered.rounding = Rounding::same(BORDER_RADIUS);
            ui.visuals_mut().widgets.hovered.bg_fill = HOVERED.into();
            ui.visuals_mut().widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::BLACK);
            ui.visuals_mut().widgets.active.rounding = Rounding::same(BORDER_RADIUS);
            ui.visuals_mut().widgets.active.bg_fill = HOVERED.into();
            ui.visuals_mut().widgets.active.bg_stroke = Stroke::new(1.5, Color32::WHITE);
            ui.visuals_mut().widgets.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
            ui.visuals_mut().widgets.noninteractive.rounding = Rounding::same(BORDER_RADIUS);
        }
    }
}
