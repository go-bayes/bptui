use ratatui::style::{Color, Modifier, Style};

// catppuccin mocha inspired palette
pub const TEXT: Color = Color::Rgb(205, 214, 244);
pub const SUBTEXT0: Color = Color::Rgb(166, 173, 200);
pub const SURFACE0: Color = Color::Rgb(49, 50, 68);
pub const BASE: Color = Color::Rgb(30, 30, 46);
pub const MAUVE: Color = Color::Rgb(203, 166, 247);
pub const PEACH: Color = Color::Rgb(250, 179, 135);
pub const TEAL: Color = Color::Rgb(148, 226, 213);
pub const SAPPHIRE: Color = Color::Rgb(116, 199, 236);
pub const GREEN: Color = Color::Rgb(166, 227, 161);
pub const YELLOW: Color = Color::Rgb(249, 226, 175);

pub fn title() -> Style {
    Style::default().fg(MAUVE).add_modifier(Modifier::BOLD)
}

pub fn panel_title() -> Style {
    Style::default().fg(MAUVE).add_modifier(Modifier::BOLD)
}

pub fn text() -> Style {
    Style::default().fg(TEXT)
}

pub fn selection() -> Style {
    Style::default()
        .fg(TEXT)
        .bg(SURFACE0)
        .add_modifier(Modifier::BOLD)
}

pub fn border() -> Style {
    Style::default().fg(SURFACE0)
}

pub fn chip_ok() -> Style {
    Style::default().fg(GREEN).add_modifier(Modifier::BOLD)
}

pub fn chip_warn() -> Style {
    Style::default().fg(YELLOW).add_modifier(Modifier::BOLD)
}

pub fn help_border() -> Style {
    Style::default().fg(MAUVE)
}

pub const LOGO: &str = r#"
  ████████████████████████████████████████████
  █                                          █
  █               B   P   T   U   I          █
  █                                          █
  ████████████████████████████████████████████
"#;

pub const TAGLINE: &str = "";
