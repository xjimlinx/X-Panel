use crate::module_trait::{ModuleUpdate, PanelModule};
use crate::theme::Theme;
use async_trait::async_trait;
use chrono::Local;
use ratatui::{
    layout::Rect,
    prelude::{Line, Span},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};
use reqwest::Client;
use std::time::{Duration, Instant};

pub struct WebApiModule {
    name: String,
    url: String,
    status: String,
    response_time: String,
    error: Option<String>,
    last_update: String,
    refresh_interval: u64,
    paused: bool,
    client: Client,
    last_response: String,
}

impl WebApiModule {
    pub fn new(name: String, url: String, refresh_interval: u64) -> Self {
        Self {
            name,
            url,
            status: String::new(),
            response_time: String::new(),
            error: None,
            last_update: String::new(),
            refresh_interval,
            paused: false,
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("创建 HTTP 客户端失败"),
            last_response: String::new(),
        }
    }

    fn shorten_url(url: &str) -> String {
        url.trim_start_matches("https://")
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_string()
    }

    async fn fetch(&mut self) {
        let start = Instant::now();
        match self.client.get(&self.url).send().await {
            Ok(response) => {
                let elapsed = start.elapsed();
                let ms = elapsed.as_secs_f64() * 1000.0;
                self.response_time = format!("{:.0}ms", ms);
                self.status = format!("{} {}", response.status().as_u16(), response.status().canonical_reason().unwrap_or(""));
                self.error = None;

                // 读取前 200 字节作为预览
                self.last_response = match response.text().await {
                    Ok(text) => {
                        let trimmed = text.trim();
                        if trimmed.len() > 200 {
                            format!("{}...", &trimmed[..200])
                        } else {
                            trimmed.to_string()
                        }
                    }
                    Err(e) => format!("[读取失败] {}", e),
                };
                self.last_update = Local::now().format("%H:%M:%S").to_string();
            }
            Err(e) => {
                self.response_time = String::new();
                self.status = "ERR".to_string();
                self.error = Some(format!("{}", e));
                self.last_response = String::new();
                self.last_update = Local::now().format("%H:%M:%S").to_string();
            }
        }
    }
}

#[async_trait]
impl PanelModule for WebApiModule {
    fn id(&self) -> &str {
        &self.name
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn refresh_interval(&self) -> u64 {
        if self.paused { 0 } else { self.refresh_interval }
    }

    fn set_refresh_interval(&mut self, interval: u64) {
        self.refresh_interval = interval;
        if interval > 0 { self.paused = false; }
    }

    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }

    async fn update(&mut self) -> ModuleUpdate {
        self.fetch().await;
        ModuleUpdate {
            id: self.id().to_string(),
            success: self.error.is_none(),
            error: self.error.clone(),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, is_selected: bool, theme: &Theme) {
        let border_style = if is_selected {
            Style::default().fg(theme.border_selected).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.border_default)
        };

        let url_short = Self::shorten_url(&self.url);

        let status_style = if self.error.is_some() {
            Style::default().fg(theme.error)
        } else if self.status.starts_with("2") {
            Style::default().fg(Color::Green)
        } else if self.status.starts_with("3") {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::Yellow)
        };

        let mut lines = vec![
            Line::from(vec![
                Span::styled("🔗 ", Style::default().fg(theme.accent)),
                Span::styled(url_short, Style::default().fg(theme.accent)),
            ]),
            Line::from(vec![
                Span::styled("状态：", Style::default().fg(Color::Yellow)),
                Span::styled(
                    if self.status.is_empty() { "等待中".to_string() } else { self.status.clone() },
                    status_style,
                ),
            ]),
            Line::from(vec![
                Span::styled("耗时：", Style::default().fg(Color::Yellow)),
                Span::styled(
                    if self.response_time.is_empty() { "--" } else { &self.response_time },
                    Style::default().fg(Color::Cyan),
                ),
            ]),
        ];

        if let Some(err) = &self.error {
            lines.push(Line::from(vec![
                Span::styled("❌ ", Style::default().fg(theme.error)),
                Span::styled(err.clone(), Style::default().fg(theme.error)),
            ]));
        } else if !self.last_response.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("响应：", Style::default().fg(Color::Yellow)),
                Span::styled(&self.last_response, Style::default().fg(Color::DarkGray)),
            ]));
        }

        let pause = if self.paused {
            Line::from(Span::styled("⏸️ 已暂停", Style::default().fg(Color::Yellow)))
        } else {
            Line::from(Span::styled(
                format!("🕐 每{}秒刷新", self.refresh_interval),
                Style::default().fg(Color::DarkGray),
            ))
        };
        lines.push(pause);

        let block = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(format!("🌐 {}", self.name))
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .style(Style::default().fg(theme.text).bg(theme.bg)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(block, area);
    }

    fn height(&self) -> u16 {
        if self.error.is_some() || !self.last_response.is_empty() { 8 } else { 7 }
    }

    fn get_error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}
