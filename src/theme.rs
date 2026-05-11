use ratatui::style::Color;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Clone, Copy)]
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
    pub name: String,
    pub theme: Theme,
}

#[derive(Debug, Deserialize)]
struct TomlTheme {
    name: String,
    title_fg: String,
    title_border: String,
    status_fg: String,
    border_selected: String,
    border_default: String,
    highlight: String,
    dim: String,
    error: String,
    accent: String,
    bg: String,
    text: String,
}

fn parse_hex(hex: &str) -> Option<Color> {
    let hex = hex.trim_start_matches('#');
    if hex.len() == 6 {
        let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
        Some(Color::Rgb(r, g, b))
    } else {
        None
    }
}

impl TomlTheme {
    fn to_theme(&self) -> Option<(String, Theme)> {
        Some((self.name.clone(), Theme {
            title_fg: parse_hex(&self.title_fg)?,
            title_border: parse_hex(&self.title_border)?,
            status_fg: parse_hex(&self.status_fg)?,
            border_selected: parse_hex(&self.border_selected)?,
            border_default: parse_hex(&self.border_default)?,
            highlight: parse_hex(&self.highlight)?,
            dim: parse_hex(&self.dim)?,
            error: parse_hex(&self.error)?,
            accent: parse_hex(&self.accent)?,
            bg: parse_hex(&self.bg)?,
            text: parse_hex(&self.text)?,
        }))
    }
}

fn themes_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".config/x-panel/themes")
}

fn write_sample(themes: &[(&str, &Theme)]) {
    let dir = themes_dir();
    let _ = fs::create_dir_all(&dir);
    for (name, theme) in themes {
        let path = dir.join(format!("{}.toml", name));
        if path.exists() { continue; }
        let (tf_r, tf_g, tf_b) = rgb_tuple(theme.title_fg);
        let (tb_r, tb_g, tb_b) = rgb_tuple(theme.title_border);
        let (sf_r, sf_g, sf_b) = rgb_tuple(theme.status_fg);
        let (bs_r, bs_g, bs_b) = rgb_tuple(theme.border_selected);
        let (bd_r, bd_g, bd_b) = rgb_tuple(theme.border_default);
        let (hl_r, hl_g, hl_b) = rgb_tuple(theme.highlight);
        let (dm_r, dm_g, dm_b) = rgb_tuple(theme.dim);
        let (er_r, er_g, er_b) = rgb_tuple(theme.error);
        let (ac_r, ac_g, ac_b) = rgb_tuple(theme.accent);
        let (bg_r, bg_g, bg_b) = rgb_tuple(theme.bg);
        let (tx_r, tx_g, tx_b) = rgb_tuple(theme.text);

        let content = format!(
            r##"name = "{}"
title_fg = "#{:02X}{:02X}{:02X}"
title_border = "#{:02X}{:02X}{:02X}"
status_fg = "#{:02X}{:02X}{:02X}"
border_selected = "#{:02X}{:02X}{:02X}"
border_default = "#{:02X}{:02X}{:02X}"
highlight = "#{:02X}{:02X}{:02X}"
dim = "#{:02X}{:02X}{:02X}"
error = "#{:02X}{:02X}{:02X}"
accent = "#{:02X}{:02X}{:02X}"
bg = "#{:02X}{:02X}{:02X}"
text = "#{:02X}{:02X}{:02X}"
"##,
            name,
            tf_r, tf_g, tf_b,
            tb_r, tb_g, tb_b,
            sf_r, sf_g, sf_b,
            bs_r, bs_g, bs_b,
            bd_r, bd_g, bd_b,
            hl_r, hl_g, hl_b,
            dm_r, dm_g, dm_b,
            er_r, er_g, er_b,
            ac_r, ac_g, ac_b,
            bg_r, bg_g, bg_b,
            tx_r, tx_g, tx_b,
        );
        let _ = fs::write(&path, content);
    }
}

fn rgb_tuple(c: Color) -> (u8, u8, u8) {
    match c {
        Color::Rgb(r, g, b) => (r, g, b),
        _ => (0, 0, 0),
    }
}

fn load_user_themes() -> Vec<ThemeDef> {
    let dir = themes_dir();
    let _ = fs::create_dir_all(&dir);
    let mut themes = Vec::new();
    let Ok(entries) = fs::read_dir(&dir) else { return themes };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().map_or(true, |e| e != "toml") { continue; }
        match fs::read_to_string(&path) {
            Ok(content) => {
                match toml::from_str::<TomlTheme>(&content) {
                    Ok(t) => {
                        if let Some((name, theme)) = t.to_theme() {
                            themes.push(ThemeDef { name, theme });
                        }
                    }
                    Err(e) => eprintln!("主题文件 {} 解析失败: {}", path.display(), e),
                }
            }
            Err(e) => eprintln!("读取主题文件 {} 失败: {}", path.display(), e),
        }
    }
    themes
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::Rgb(r, g, b)
}

// === Dracula ===
const DRACULA: Theme = Theme {
    title_fg: rgb(189, 147, 249),
    title_border: rgb(189, 147, 249),
    status_fg: rgb(98, 114, 164),
    border_selected: rgb(80, 250, 123),
    border_default: rgb(68, 71, 90),
    highlight: rgb(80, 250, 123),
    dim: rgb(98, 114, 164),
    error: rgb(255, 85, 85),
    accent: rgb(139, 233, 253),
    bg: rgb(40, 42, 54),
    text: rgb(248, 248, 242),
};

// === Solarized Light ===
const SOLARIZED_LIGHT: Theme = Theme {
    title_fg: rgb(38, 139, 210),
    title_border: rgb(38, 139, 210),
    status_fg: rgb(88, 110, 117),
    border_selected: rgb(203, 75, 22),
    border_default: rgb(147, 161, 161),
    highlight: rgb(203, 75, 22),
    dim: rgb(147, 161, 161),
    error: rgb(220, 50, 47),
    accent: rgb(42, 161, 152),
    bg: rgb(253, 246, 227),
    text: rgb(88, 110, 117),
};

// === Nord ===
const NORD: Theme = Theme {
    title_fg: rgb(136, 192, 208),
    title_border: rgb(136, 192, 208),
    status_fg: rgb(76, 86, 106),
    border_selected: rgb(163, 190, 140),
    border_default: rgb(59, 66, 82),
    highlight: rgb(163, 190, 140),
    dim: rgb(76, 86, 106),
    error: rgb(191, 97, 106),
    accent: rgb(143, 188, 187),
    bg: rgb(46, 52, 64),
    text: rgb(216, 222, 233),
};

// === Catppuccin Mocha ===
const MOCHA: Theme = Theme {
    title_fg: rgb(137, 180, 250),
    title_border: rgb(137, 180, 250),
    status_fg: rgb(147, 153, 178),
    border_selected: rgb(245, 194, 231),
    border_default: rgb(69, 71, 90),
    highlight: rgb(245, 194, 231),
    dim: rgb(147, 153, 178),
    error: rgb(243, 139, 168),
    accent: rgb(166, 227, 161),
    bg: rgb(30, 30, 46),
    text: rgb(205, 214, 244),
};

pub fn all_themes() -> &'static Vec<ThemeDef> {
    static THEMES: OnceLock<Vec<ThemeDef>> = OnceLock::new();
    THEMES.get_or_init(|| {
        let builtins: Vec<(&str, &Theme)> = vec![
            ("Dracula", &DRACULA),
            ("Solarized Light", &SOLARIZED_LIGHT),
            ("Nord", &NORD),
            ("Catppuccin Mocha", &MOCHA),
        ];
        write_sample(&builtins);
        let mut themes: Vec<ThemeDef> = builtins.iter().map(|(n, t)| {
            ThemeDef { name: n.to_string(), theme: **t }
        }).collect();
        themes.extend(load_user_themes());
        themes
    })
}

pub fn theme_index_by_name(name: &str) -> Option<usize> {
    match name {
        "dark" | "Default Dark" | "Default Light" => Some(0),
        _ => all_themes().iter().position(|t| t.name == name),
    }
}
