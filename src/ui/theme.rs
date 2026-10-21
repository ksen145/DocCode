//! Визуальная тема приложения: палитра и стили iced-виджетов.
//!
//! Тема не зависит от встроенной палитры iced: цвета задаются здесь, чтобы
//! интерфейс выглядел одинаково предсказуемо в светлом и тёмном режимах.
//! Все стили — чистые функции от [`Palette`], поэтому их удобно передавать
//! в замыкания `.style(...)`.

use iced::widget::{
    button, checkbox as checkbox_widget, container, progress_bar, text_editor, text_input,
};
use iced::{border, Background, Color, Font, Shadow, Vector};

/// Шрифт для заголовков.
pub const SEMIBOLD: Font = Font {
    weight: iced::font::Weight::Semibold,
    ..Font::DEFAULT
};

/// Шрифт для крупных акцентов.
pub const BOLD: Font = Font {
    weight: iced::font::Weight::Bold,
    ..Font::DEFAULT
};

/// Цвета интерфейса.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub canvas: Color,
    pub panel: Color,
    pub panel_alt: Color,
    pub input_bg: Color,
    pub hover: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub accent_hover: Color,
    pub accent_soft: Color,
    pub on_accent: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
    pub shadow: Color,
}

impl Palette {
    /// Палитра для выбранной темы.
    pub fn new(dark: bool) -> Self {
        if dark {
            Self {
                canvas: rgb(0x08, 0x0d, 0x19),
                panel: rgb(0x0f, 0x17, 0x2a),
                panel_alt: rgb(0x13, 0x1d, 0x33),
                input_bg: rgb(0x0b, 0x12, 0x22),
                hover: rgb(0x1b, 0x27, 0x40),
                border: rgb(0x24, 0x33, 0x50),
                text: rgb(0xe8, 0xed, 0xf7),
                muted: rgb(0x94, 0xa3, 0xb8),
                accent: rgb(0x93, 0x9f, 0xff),
                accent_hover: rgb(0xb4, 0xbc, 0xff),
                accent_soft: rgb(0x1e, 0x25, 0x4d),
                on_accent: rgb(0x0b, 0x10, 0x20),
                success: rgb(0x34, 0xd3, 0x99),
                warning: rgb(0xfb, 0xbf, 0x24),
                danger: rgb(0xf8, 0x71, 0x71),
                shadow: rgba(0, 0, 0, 0x55),
            }
        } else {
            Self {
                canvas: rgb(0xf3, 0xf5, 0xfa),
                panel: rgb(0xff, 0xff, 0xff),
                panel_alt: rgb(0xf8, 0xfa, 0xfc),
                input_bg: rgb(0xfb, 0xfc, 0xfe),
                hover: rgb(0xf1, 0xf5, 0xf9),
                border: rgb(0xe2, 0xe8, 0xf0),
                text: rgb(0x0f, 0x17, 0x2a),
                muted: rgb(0x64, 0x74, 0x8b),
                accent: rgb(0x4f, 0x46, 0xe5),
                accent_hover: rgb(0x43, 0x38, 0xca),
                accent_soft: rgb(0xee, 0xf2, 0xff),
                on_accent: rgb(0xff, 0xff, 0xff),
                success: rgb(0x05, 0x96, 0x69),
                warning: rgb(0xd9, 0x77, 0x06),
                danger: rgb(0xdc, 0x26, 0x26),
                shadow: rgba(15, 23, 42, 0x12),
            }
        }
    }
}

/// Фон приложения.
pub fn canvas(p: Palette) -> container::Style {
    container::Style {
        text_color: Some(p.text),
        background: Some(Background::Color(p.canvas)),
        ..container::Style::default()
    }
}

/// Обычная карточка контента.
pub fn card(p: Palette) -> container::Style {
    container::Style {
        text_color: Some(p.text),
        background: Some(Background::Color(p.panel)),
        border: border::rounded(14).color(p.border).width(1),
        shadow: Shadow {
            color: p.shadow,
            offset: Vector::new(0.0, 5.0),
            blur_radius: 18.0,
        },
    }
}

/// Карточка-акцент (герой, рекомендации).
pub fn hero(p: Palette) -> container::Style {
    container::Style {
        text_color: Some(p.text),
        background: Some(Background::Color(p.accent_soft)),
        border: border::rounded(16).color(p.accent).width(1),
        shadow: Shadow::default(),
    }
}

/// Логотип приложения.
pub fn logo(p: Palette) -> container::Style {
    container::Style {
        text_color: Some(p.on_accent),
        background: Some(Background::Color(p.accent)),
        border: border::rounded(10).color(p.accent).width(1),
        shadow: Shadow::default(),
    }
}

/// Боковая панель.
pub fn sidebar(p: Palette) -> container::Style {
    container::Style {
        text_color: Some(p.text),
        background: Some(Background::Color(p.panel)),
        border: border::rounded(0).color(p.border).width(1),
        shadow: Shadow::default(),
    }
}

/// Внутренняя панель без тени.
pub fn inset(p: Palette) -> container::Style {
    container::Style {
        text_color: Some(p.text),
        background: Some(Background::Color(p.panel_alt)),
        border: border::rounded(10).color(p.border).width(1),
        shadow: Shadow::default(),
    }
}

/// Шаг прогресса.
pub fn step(p: Palette, active: bool, done: bool) -> container::Style {
    let border_color = if active {
        p.accent
    } else if done {
        p.success
    } else {
        p.border
    };
    container::Style {
        text_color: Some(if active {
            p.accent
        } else if done {
            p.success
        } else {
            p.muted
        }),
        background: Some(Background::Color(if active {
            p.accent_soft
        } else {
            p.panel_alt
        })),
        border: border::rounded(10).color(border_color).width(1),
        shadow: Shadow::default(),
    }
}

/// Точка-индикатор.
pub fn dot(_p: Palette, color: Color) -> container::Style {
    container::Style {
        text_color: None,
        background: Some(Background::Color(color)),
        border: border::rounded(99).color(color).width(0),
        shadow: Shadow::default(),
    }
}

/// Маленький бейдж/пилюля.
pub fn badge(p: Palette) -> container::Style {
    container::Style {
        text_color: Some(p.accent),
        background: Some(Background::Color(p.accent_soft)),
        border: border::rounded(999).color(p.accent_soft).width(1),
        shadow: Shadow::default(),
    }
}

/// Поле ввода.
pub fn input(p: Palette, status: text_input::Status) -> text_input::Style {
    let (border_color, background) = match status {
        text_input::Status::Focused => (p.accent, p.panel),
        text_input::Status::Hovered => (p.muted, p.input_bg),
        _ => (p.border, p.input_bg),
    };
    text_input::Style {
        background: Background::Color(background),
        border: border::rounded(10).color(border_color).width(1),
        icon: p.muted,
        placeholder: p.muted,
        value: p.text,
        selection: p.accent_soft,
    }
}

/// Редактор YAML.
pub fn editor(p: Palette, status: text_editor::Status) -> text_editor::Style {
    let border_color = match status {
        text_editor::Status::Focused => p.accent,
        text_editor::Status::Hovered => p.muted,
        _ => p.border,
    };
    text_editor::Style {
        background: Background::Color(p.input_bg),
        border: border::rounded(10).color(border_color).width(1),
        icon: p.muted,
        placeholder: p.muted,
        value: p.text,
        selection: p.accent_soft,
    }
}

/// Чекбокс.
pub fn checkbox(p: Palette, status: checkbox_widget::Status) -> checkbox_widget::Style {
    let checked = match status {
        checkbox_widget::Status::Active { is_checked }
        | checkbox_widget::Status::Hovered { is_checked }
        | checkbox_widget::Status::Disabled { is_checked } => is_checked,
    };
    checkbox_widget::Style {
        background: Background::Color(if checked { p.accent } else { p.input_bg }),
        icon_color: p.on_accent,
        border: border::rounded(6)
            .color(if checked { p.accent } else { p.border })
            .width(1),
        text_color: Some(p.text),
    }
}

/// Прогресс-бар.
pub fn progress(p: Palette) -> progress_bar::Style {
    progress_bar::Style {
        background: Background::Color(p.panel_alt),
        bar: Background::Color(p.accent),
        border: border::rounded(6).color(p.border).width(1),
    }
}

/// Основная кнопка.
pub fn primary_button(p: Palette, status: button::Status) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Hovered | button::Status::Pressed => (p.accent_hover, p.on_accent),
        button::Status::Disabled => (p.border, p.muted),
        button::Status::Active => (p.accent, p.on_accent),
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: border::rounded(10).color(background).width(1),
        shadow: if matches!(status, button::Status::Disabled) {
            Shadow::default()
        } else {
            Shadow {
                color: rgba(79, 70, 229, 0x33),
                offset: Vector::new(0.0, 3.0),
                blur_radius: 10.0,
            }
        },
    }
}

/// Вторичная кнопка.
pub fn secondary_button(p: Palette, status: button::Status) -> button::Style {
    let (background, border_color, text_color) = match status {
        button::Status::Hovered | button::Status::Pressed => (p.hover, p.accent, p.accent),
        button::Status::Disabled => (p.panel, p.border, p.muted),
        button::Status::Active => (p.panel, p.border, p.text),
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color,
        border: border::rounded(10).color(border_color).width(1),
        shadow: Shadow::default(),
    }
}

/// Мягкая акцентная кнопка.
pub fn soft_button(p: Palette, status: button::Status) -> button::Style {
    let background = match status {
        button::Status::Hovered | button::Status::Pressed => p.hover,
        button::Status::Disabled => p.panel,
        button::Status::Active => p.accent_soft,
    };
    button::Style {
        background: Some(Background::Color(background)),
        text_color: if matches!(status, button::Status::Disabled) {
            p.muted
        } else {
            p.accent
        },
        border: border::rounded(10).color(background).width(1),
        shadow: Shadow::default(),
    }
}

/// Прозрачная кнопка (иконки, навигация).
pub fn ghost_button(p: Palette, status: button::Status) -> button::Style {
    let (background, text_color) = match status {
        button::Status::Hovered | button::Status::Pressed => (Some(p.hover), p.text),
        button::Status::Disabled => (None, p.muted),
        button::Status::Active => (None, p.muted),
    };
    button::Style {
        background: background.map(Background::Color),
        text_color,
        border: border::rounded(9).width(0),
        shadow: Shadow::default(),
    }
}

/// Пункт боковой навигации.
pub fn nav_button(p: Palette, active: bool, status: button::Status) -> button::Style {
    let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
    let disabled = matches!(status, button::Status::Disabled);
    let background = if active {
        Some(p.accent_soft)
    } else if hovered && !disabled {
        Some(p.hover)
    } else {
        None
    };
    let text_color = if disabled {
        p.muted
    } else if active {
        p.accent
    } else if hovered {
        p.text
    } else {
        p.muted
    };
    button::Style {
        background: background.map(Background::Color),
        text_color,
        border: border::rounded(10).width(0),
        shadow: Shadow::default(),
    }
}

fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color::from_rgba8(r, g, b, a as f32 / 255.0)
}
