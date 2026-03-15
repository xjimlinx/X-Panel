use crate::module_trait::{ModuleUpdate, PanelModule};
use async_trait::async_trait;
use chrono::{Local, Datelike};
use ratatui::{
    layout::Rect,
    prelude::{Line, Modifier, Span},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Widget},
    Frame,
};

pub struct ClockModule {
    time: String,
    date: String,
    weekday: String,
    week_number: String,
    refresh_interval: u64,
    paused: bool,
}

impl ClockModule {
    pub fn new(refresh_interval: u64) -> Self {
        Self {
            time: String::new(),
            date: String::new(),
            weekday: String::new(),
            week_number: String::new(),
            refresh_interval,
            paused: false,
        }
    }
    
    fn update_info(&mut self) {
        let now = Local::now();
        self.time = now.format("%H:%M:%S").to_string();
        self.date = now.format("%Y年%m月%d日").to_string();
        self.weekday = match now.weekday().number_from_sunday() {
            0 => "星期日", 1 => "星期一", 2 => "星期二", 3 => "星期三",
            4 => "星期四", 5 => "星期五", 6 => "星期六", _ => "未知",
        }.to_string();
        self.week_number = format!("第{}周", now.iso_week().week());
    }
}

#[async_trait]
impl PanelModule for ClockModule {
    fn id(&self) -> &str { "clock" }
    fn name(&self) -> &str { "🕐 时钟日历" }
    fn refresh_interval(&self) -> u64 { if self.paused { 0 } else { self.refresh_interval } }
    fn set_refresh_interval(&mut self, interval: u64) { self.refresh_interval = interval; self.paused = interval == 0; }
    fn toggle_pause(&mut self) { self.paused = !self.paused; }
    
    async fn update(&mut self) -> ModuleUpdate {
        self.update_info();
        ModuleUpdate { id: self.id().to_string(), success: true, error: None }
    }
    
    fn render(&self, frame: &mut Frame, area: Rect, is_selected: bool) {
        let border_style = if is_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        
        let lines = vec![
            Line::from(Span::styled(&self.time, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
            Line::from(vec![Span::styled(&self.date, Style::default().fg(Color::White)), Span::raw(" "), Span::styled(&self.weekday, Style::default().fg(Color::Yellow))]),
            Line::from(Span::styled(&self.week_number, Style::default().fg(Color::DarkGray))),
            Line::from(if self.paused {
                Span::styled("⏸️ 已暂停", Style::default().fg(Color::Yellow))
            } else {
                Span::styled(format!("🕐 每{}秒刷新", self.refresh_interval), Style::default().fg(Color::DarkGray))
            }),
        ];
        
        Paragraph::new(lines)
            .block(Block::default().title(self.name()).borders(Borders::ALL).border_style(border_style))
            .render(area, frame.buffer_mut());
    }
    
    fn height(&self) -> u16 { 7 }
    fn get_error(&self) -> Option<&str> { None }
}
