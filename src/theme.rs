use ratatui::style::Color;

pub struct Theme {
    pub title_fg: Color,
    pub title_border: Color,
    pub status_fg: Color,
    pub border_selected: Color,
    pub border_default: Color,
    pub highlight: Color,
    pub dim: Color,
    pub error: Color,
    pub accent: Color,
    pub bg: Color,
    pub text: Color,
}

pub struct ThemeDef {
    pub name: &'static str,
    pub theme: Theme,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

// === Default Dark ===
pub const DARK: Theme = Theme {
    title_fg: Color::Cyan,
    title_border: Color::Cyan,
    status_fg: Color::Gray,
    border_selected: Color::Yellow,
    border_default: Color::DarkGray,
    highlight: Color::Yellow,
    dim: Color::DarkGray,
    error: Color::Red,
    accent: Color::Cyan,
    bg: Color::Black,
    text: Color::White,
};

// === Default Light ===
pub const LIGHT: Theme = Theme {
    title_fg: Color::Blue,
    title_border: Color::Blue,
    status_fg: Color::DarkGray,
    border_selected: Color::Blue,
    border_default: Color::Gray,
    highlight: Color::Blue,
    dim: Color::Gray,
    error: Color::Red,
    accent: Color::Blue,
    bg: Color::White,
    text: Color::Black,
};

// === Dracula ===
pub const DRACULA: Theme = Theme {
    title_fg: rgb(189, 147, 249),   // purple
    title_border: rgb(189, 147, 249),
    status_fg: rgb(98, 114, 164),   // comment
    border_selected: rgb(80, 250, 123), // green
    border_default: rgb(68, 71, 90),    // selection
    highlight: rgb(80, 250, 123),
    dim: rgb(98, 114, 164),
    error: rgb(255, 85, 85),
    accent: rgb(139, 233, 253),     // cyan
    bg: rgb(40, 42, 54),
    text: rgb(248, 248, 242),
};

// === Solarized Light ===
pub const SOLARIZED_LIGHT: Theme = Theme {
    title_fg: rgb(38, 139, 210),    // blue
    title_border: rgb(38, 139, 210),
    status_fg: rgb(88, 110, 117),   // base01
    border_selected: rgb(203, 75, 22), // orange
    border_default: rgb(147, 161, 161), // base0
    highlight: rgb(203, 75, 22),
    dim: rgb(147, 161, 161),
    error: rgb(220, 50, 47),
    accent: rgb(42, 161, 152),      // cyan
    bg: rgb(253, 246, 227),
    text: rgb(88, 110, 117),
};

// === Nord ===
pub const NORD: Theme = Theme {
    title_fg: rgb(136, 192, 208),   // frost 8
    title_border: rgb(136, 192, 208),
    status_fg: rgb(76, 86, 106),    // polar 3
    border_selected: rgb(163, 190, 140), // aurora green
    border_default: rgb(59, 66, 82),    // polar 2
    highlight: rgb(163, 190, 140),
    dim: rgb(76, 86, 106),
    error: rgb(191, 97, 106),
    accent: rgb(143, 188, 187),     // frost 9
    bg: rgb(46, 52, 64),
    text: rgb(216, 222, 233),
};

// === Catppuccin Mocha ===
pub const MOCHA: Theme = Theme {
    title_fg: rgb(137, 180, 250),   // blue
    title_border: rgb(137, 180, 250),
    status_fg: rgb(147, 153, 178),  // overlay0
    border_selected: rgb(245, 194, 231), // pink
    border_default: rgb(69, 71, 90),     // surface1
    highlight: rgb(245, 194, 231),
    dim: rgb(147, 153, 178),
    error: rgb(243, 139, 168),
    accent: rgb(166, 227, 161),     // green
    bg: rgb(30, 30, 46),
    text: rgb(205, 214, 244),
};

// === Catppuccin Latte ===
pub const LATTE: Theme = Theme {
    title_fg: rgb(30, 102, 245),    // blue
    title_border: rgb(30, 102, 245),
    status_fg: rgb(156, 160, 176),  // overlay0
    border_selected: rgb(221, 120, 120), // maroon
    border_default: rgb(204, 196, 194),  // surface1
    highlight: rgb(221, 120, 120),
    dim: rgb(156, 160, 176),
    error: rgb(210, 15, 57),
    accent: rgb(64, 160, 43),       // green
    bg: rgb(239, 241, 245),
    text: rgb(76, 79, 105),
};

pub const THEMES: &[ThemeDef] = &[
    ThemeDef { name: "Dracula", theme: DRACULA },
    ThemeDef { name: "Solarized Light", theme: SOLARIZED_LIGHT },
    ThemeDef { name: "Nord", theme: NORD },
    ThemeDef { name: "Catppuccin Mocha", theme: MOCHA },
];

pub fn theme_index_by_name(name: &str) -> Option<usize> {
    match name {
        "dark" | "Default Dark" | "Default Light" => Some(0),
        _ => THEMES.iter().position(|t| t.name == name),
    }
}
