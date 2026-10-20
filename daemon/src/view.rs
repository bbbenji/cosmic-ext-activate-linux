// SPDX-License-Identifier: GPL-3.0-or-later

//! The watermark widget tree. Ignores the COSMIC theme on purpose: the user
//! picks font and colour.

use activate_linux_config::{Config, Rgba, points_to_pixels};
use cosmic::iced::widget::{container::Style as ContainerStyle, stack};
use cosmic::iced::{Alignment, Background, Border, Color, Font, Length, Padding};
use cosmic::widget::{Space, column, container, text};
use cosmic::{Element, theme};

use crate::lines::Lines;

const SHADOW: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.4);
const PLATE: Color = Color::from_rgba(0.0, 0.0, 0.0, 0.5);
const PLATE_RADIUS: f32 = 8.0;
const PLATE_PADDING: u16 = 16;

pub fn watermark<'a, M: 'a>(config: &Config, lines: Option<&Lines>, font: Font) -> Element<'a, M> {
    let Some(lines) = lines else {
        return Space::new()
            .width(Length::Fixed(1.0))
            .height(Length::Fixed(1.0))
            .into();
    };

    let color = to_color(config.font_color);
    let shadow = config.text_shadow;
    let align = if config.corner.is_left() {
        Alignment::Start
    } else {
        Alignment::End
    };

    let mut body = column::with_capacity(2).align_x(align);
    if !lines.main.is_empty() {
        body = body.push(label(
            &lines.main,
            points_to_pixels(config.font_size),
            font,
            color,
            shadow,
        ));
    }
    if !lines.secondary.is_empty() {
        body = body.push(label(
            &lines.secondary,
            points_to_pixels(config.secondary_font_size()),
            font,
            color,
            shadow,
        ));
    }

    let body = container(body);
    if config.background_plate {
        body.padding(PLATE_PADDING)
            .class(theme::Container::custom(|_| ContainerStyle {
                background: Some(Background::Color(PLATE)),
                border: Border {
                    radius: PLATE_RADIUS.into(),
                    ..Border::default()
                },
                ..ContainerStyle::default()
            }))
            .into()
    } else {
        body.into()
    }
}

/// One line of text. The shadow is a second copy offset by (1, 1) behind it;
/// iced has no blurred text shadow.
fn label<'a, M: 'a>(
    content: &str,
    size: f32,
    font: Font,
    color: Color,
    shadow: bool,
) -> Element<'a, M> {
    let fg = text(content.to_owned())
        .size(size)
        .font(font)
        .class(theme::Text::Color(color));
    if !shadow {
        return fg.into();
    }
    let bg = text(content.to_owned())
        .size(size)
        .font(font)
        .class(theme::Text::Color(SHADOW));
    stack![
        container(bg).padding(Padding {
            top: 1.0,
            left: 1.0,
            ..Padding::ZERO
        }),
        container(fg).padding(Padding {
            bottom: 1.0,
            right: 1.0,
            ..Padding::ZERO
        }),
    ]
    .into()
}

fn to_color(c: Rgba) -> Color {
    Color::from_rgba(c.r, c.g, c.b, c.a)
}
