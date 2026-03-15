use crate::module_trait::{ModuleUpdate, PanelModule};
use async_trait::async_trait;
use chrono::Local;
use ratatui::{
    layout::Rect,
    prelude::{Line, Modifier, Span},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Widget},
    Frame,
};
use std::fs;

pub struct SystemTempModule {
    cpu_temp: String,
    gpu_temp: String,
    motherboard_temp: String,
    fan_speed: String,
    last_update: String,
    error: Option<String>,
    refresh_interval: u64,
    paused: bool,
}

impl SystemTempModule {
    pub fn new(refresh_interval: u64) -> Self {
        Self {
            cpu_temp: "N/A".to_string(),
            gpu_temp: "N/A".to_string(),
            motherboard_temp: "N/A".to_string(),
            fan_speed: "N/A".to_string(),
            last_update: String::new(),
            error: None,
            refresh_interval,
            paused: false,
        }
    }
    
    fn update_info(&mut self) {
        self.cpu_temp = Self::get_cpu_temp();
        self.gpu_temp = Self::get_gpu_temp();
        self.motherboard_temp = Self::get_motherboard_temp();
        self.fan_speed = Self::get_fan_speed();
        self.last_update = Local::now().format("%H:%M:%S").to_string();
        self.error = None;
    }

    fn get_cpu_temp() -> String {
        if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                if let Ok(name) = fs::read_to_string(entry.path().join("name")) {
                    let name = name.trim().to_lowercase();
                    if name.contains("coretemp") || name.contains("k10temp") || name.contains("zenpower") {
                        if let Ok(content) = fs::read_to_string(entry.path().join("temp1_input")) {
                            if let Ok(temp) = content.trim().parse::<i64>() {
                                return format!("{:.1}°C", temp as f64 / 1000.0);
                            }
                        }
                    }
                }
            }
        }
        if let Ok(entries) = fs::read_dir("/sys/class/thermal") {
            for entry in entries.flatten() {
                if let Ok(type_name) = fs::read_to_string(entry.path().join("type")) {
                    if type_name.trim() == "x86_pkg" || type_name.trim() == "cpu" {
                        if let Ok(content) = fs::read_to_string(entry.path().join("temp")) {
                            if let Ok(temp) = content.trim().parse::<i64>() {
                                return format!("{:.1}°C", temp as f64 / 1000.0);
                            }
                        }
                    }
                }
            }
        }
        "N/A".to_string()
    }

    fn get_gpu_temp() -> String {
        if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                if let Ok(name) = fs::read_to_string(entry.path().join("name")) {
                    if name.trim().to_lowercase().contains("amdgpu") {
                        if let Ok(content) = fs::read_to_string(entry.path().join("temp1_input")) {
                            if let Ok(temp) = content.trim().parse::<i64>() {
                                return format!("{:.1}°C", temp as f64 / 1000.0);
                            }
                        }
                    }
                }
            }
        }
        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=temperature.gpu", "--format=csv,noheader"]).output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Ok(temp) = stdout.trim().parse::<i32>() {
                return format!("{}°C", temp);
            }
        }
        "N/A".to_string()
    }

    fn get_motherboard_temp() -> String {
        if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                if let Ok(name) = fs::read_to_string(entry.path().join("name")) {
                    let name = name.trim().to_lowercase();
                    if name.contains("nct") || name.contains("it87") || name.contains("acpitz") {
                        if let Ok(content) = fs::read_to_string(entry.path().join("temp1_input")) {
                            if let Ok(temp) = content.trim().parse::<i64>() {
                                return format!("{:.1}°C", temp as f64 / 1000.0);
                            }
                        }
                    }
                }
            }
        }
        "N/A".to_string()
    }

    fn get_fan_speed() -> String {
        if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                if let Ok(content) = fs::read_to_string(entry.path().join("fan1_input")) {
                    if let Ok(rpm) = content.trim().parse::<i32>() {
                        if rpm > 0 { return format!("{} RPM", rpm); }
                    }
                }
            }
        }
        "N/A".to_string()
    }

    fn get_temp_color(temp: &str) -> Color {
        if let Some(value) = temp.strip_suffix("°C") {
            if let Ok(t) = value.parse::<f64>() {
                if t < 50.0 { return Color::Green; }
                else if t < 70.0 { return Color::Yellow; }
                else if t < 85.0 { return Color::LightRed; }
                else { return Color::Red; }
            }
        }
        Color::White
    }
}

#[async_trait]
impl PanelModule for SystemTempModule {
    fn id(&self) -> &str { "system_temp" }
    fn name(&self) -> &str { "🌡️ 系统温度" }
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
            Line::from(vec![Span::styled("CPU:  ", Style::default().fg(Color::Yellow)), Span::styled(&self.cpu_temp, Style::default().fg(Self::get_temp_color(&self.cpu_temp)).add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::styled("GPU:  ", Style::default().fg(Color::Yellow)), Span::styled(&self.gpu_temp, Style::default().fg(Self::get_temp_color(&self.gpu_temp)).add_modifier(Modifier::BOLD))]),
            Line::from(vec![Span::styled("主板：", Style::default().fg(Color::Yellow)), Span::styled(&self.motherboard_temp, Style::default().fg(Self::get_temp_color(&self.motherboard_temp)))]),
            Line::from(vec![Span::styled("风扇：", Style::default().fg(Color::Yellow)), Span::styled(&self.fan_speed, Style::default().fg(Color::Cyan))]),
        ];
        
        let pause_line = if self.paused {
            Line::from(Span::styled("⏸️ 已暂停", Style::default().fg(Color::Yellow)))
        } else {
            Line::from(Span::styled(format!("🕐 每{}秒刷新", self.refresh_interval), Style::default().fg(Color::DarkGray)))
        };
        
        let all_lines = [lines, vec![pause_line]].concat();
        Paragraph::new(all_lines)
            .block(Block::default().title(self.name()).borders(Borders::ALL).border_style(border_style))
            .render(area, frame.buffer_mut());
    }
    
    fn height(&self) -> u16 { 8 }
    fn get_error(&self) -> Option<&str> { self.error.as_deref() }
}
