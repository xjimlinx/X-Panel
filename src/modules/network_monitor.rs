use crate::module_trait::{ModuleUpdate, PanelModule};
use async_trait::async_trait;
use chrono::Local;
use ratatui::{
    layout::Rect,
    prelude::{Line, Modifier, Span},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Wrap, Widget},
    Frame,
};
use std::fs;
use std::time::Instant;

#[derive(Debug, Clone, Default)]
struct NetworkStats {
    rx_bytes: u64,
    tx_bytes: u64,
}

pub struct NetworkMonitorModule {
    interface: String,
    wifi_ssid: String,
    wifi_signal: String,
    local_ip: String,
    rx_speed: String,
    tx_speed: String,
    total_rx: String,
    total_tx: String,
    last_update: String,
    error: Option<String>,
    refresh_interval: u64,
    paused: bool,
    prev_stats: Option<NetworkStats>,
    prev_time: Option<Instant>,
}

fn get_default_interface() -> String {
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "lo" || name.starts_with("docker") || name.starts_with("br-") {
                continue;
            }
            if name.starts_with("wlan") || name.starts_with("wl") {
                return name;
            }
            if name.starts_with("eth") || name.starts_with("en") {
                return name;
            }
        }
    }
    "eth0".to_string()
}

impl NetworkMonitorModule {
    pub fn new(refresh_interval: u64) -> Self {
        Self {
            interface: get_default_interface(),
            wifi_ssid: String::new(),
            wifi_signal: String::new(),
            local_ip: String::new(),
            rx_speed: "0 B/s".to_string(),
            tx_speed: "0 B/s".to_string(),
            total_rx: "0 MB".to_string(),
            total_tx: "0 MB".to_string(),
            last_update: String::new(),
            error: None,
            refresh_interval,
            paused: false,
            prev_stats: None,
            prev_time: None,
        }
    }

    fn update_info(&mut self) {
        let stats = Self::get_network_stats(&self.interface);
        let now = Instant::now();
        
        if let Some(prev) = &self.prev_stats {
            if let Some(prev_time) = self.prev_time {
                let elapsed = now.duration_since(prev_time).as_secs_f64();
                if elapsed > 0.0 {
                    let rx_diff = stats.rx_bytes.saturating_sub(prev.rx_bytes) as f64;
                    let tx_diff = stats.tx_bytes.saturating_sub(prev.tx_bytes) as f64;
                    self.rx_speed = Self::format_speed(rx_diff / elapsed);
                    self.tx_speed = Self::format_speed(tx_diff / elapsed);
                }
            }
        }
        
        self.prev_stats = Some(stats.clone());
        self.prev_time = Some(now);
        self.total_rx = Self::format_bytes(stats.rx_bytes);
        self.total_tx = Self::format_bytes(stats.tx_bytes);
        
        let (wifi_ssid, wifi_signal) = Self::get_wifi_info();
        self.wifi_ssid = wifi_ssid;
        self.wifi_signal = wifi_signal;
        self.local_ip = Self::get_local_ip();
        self.last_update = Local::now().format("%H:%M:%S").to_string();
        self.error = None;
    }

    fn get_network_stats(interface: &str) -> NetworkStats {
        let mut stats = NetworkStats::default();
        if let Ok(content) = fs::read_to_string("/proc/net/dev") {
            for line in content.lines().skip(2) {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() == 2 && parts[0].trim() == interface {
                    let values: Vec<&str> = parts[1].split_whitespace().collect();
                    if values.len() >= 10 {
                        stats.rx_bytes = values[0].parse().unwrap_or(0);
                        stats.tx_bytes = values[8].parse().unwrap_or(0);
                    }
                    break;
                }
            }
        }
        stats
    }

    fn get_wifi_info() -> (String, String) {
        let mut ssid = "N/A".to_string();
        let mut signal = "N/A".to_string();
        
        if let Ok(output) = std::process::Command::new("iwconfig").output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("ESSID") {
                    if let Some(start) = line.find("ESSID:\"") {
                        if let Some(end) = line[start + 7..].find('\"') {
                            ssid = line[start + 7..start + 7 + end].to_string();
                        }
                    }
                }
                if line.contains("Signal level=") {
                    if let Some(start) = line.find("Signal level=") {
                        signal = line[start + 13..].split_whitespace().next().unwrap_or("N/A").to_string();
                    }
                }
            }
        }
        
        if signal == "N/A" {
            if let Ok(content) = fs::read_to_string("/proc/net/wireless") {
                for line in content.lines().skip(2) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        signal = format!("{}%", parts[2].trim_end_matches('.'));
                        break;
                    }
                }
            }
        }
        (ssid, signal)
    }

    fn get_local_ip() -> String {
        if let Ok(output) = std::process::Command::new("ip").args(["addr", "show"]).output() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("inet ") && !line.contains("127.0.0.1") {
                    if let Some(start) = line.find("inet ") {
                        let ip_part = &line[start + 5..];
                        if let Some(ip) = ip_part.split_whitespace().next() {
                            return ip.split('/').next().unwrap_or("N/A").to_string();
                        }
                    }
                }
            }
        }
        if let Ok(output) = std::process::Command::new("hostname").arg("-I").output() {
            return String::from_utf8_lossy(&output.stdout).split_whitespace().next().unwrap_or("N/A").to_string();
        }
        "N/A".to_string()
    }

    fn format_speed(bytes_per_sec: f64) -> String {
        if bytes_per_sec < 1024.0 { format!("{:.0} B/s", bytes_per_sec) }
        else if bytes_per_sec < 1024.0 * 1024.0 { format!("{:.1} KB/s", bytes_per_sec / 1024.0) }
        else { format!("{:.1} MB/s", bytes_per_sec / (1024.0 * 1024.0)) }
    }

    fn format_bytes(bytes: u64) -> String {
        let mb = bytes as f64 / (1024.0 * 1024.0);
        if mb < 1024.0 { format!("{:.1} MB", mb) }
        else { format!("{:.2} GB", mb / 1024.0) }
    }
}

#[async_trait]
impl PanelModule for NetworkMonitorModule {
    fn id(&self) -> &str { "network_monitor" }
    fn name(&self) -> &str { "🌐 网络监控" }
    fn refresh_interval(&self) -> u64 { if self.paused { 0 } else { self.refresh_interval } }
    
    fn set_refresh_interval(&mut self, interval: u64) {
        self.refresh_interval = interval;
        self.paused = interval == 0;
    }
    
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
        
        let wifi_line = if !self.wifi_ssid.is_empty() && self.wifi_ssid != "N/A" {
            Line::from(vec![
                Span::styled("WiFi: ", Style::default().fg(Color::Yellow)),
                Span::styled(&self.wifi_ssid, Style::default().fg(Color::Green)),
                Span::styled(format!(" ({}%)", self.wifi_signal), Style::default().fg(Color::Cyan)),
            ])
        } else {
            Line::from(vec![
                Span::styled("网络：", Style::default().fg(Color::Yellow)),
                Span::styled(&self.interface, Style::default().fg(Color::White)),
            ])
        };
        
        let lines = vec![
            Line::from(vec![Span::styled("IP:   ", Style::default().fg(Color::Yellow)), Span::styled(&self.local_ip, Style::default().fg(Color::White))]),
            wifi_line,
            Line::from(vec![Span::styled("↓ 下载：", Style::default().fg(Color::Green)), Span::styled(&self.rx_speed, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)), Span::raw("  (总计: "), Span::styled(&self.total_rx, Style::default().fg(Color::DarkGray)), Span::raw(")")]),
            Line::from(vec![Span::styled("↑ 上传：", Style::default().fg(Color::Cyan)), Span::styled(&self.tx_speed, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)), Span::raw("  (总计: "), Span::styled(&self.total_tx, Style::default().fg(Color::DarkGray)), Span::raw(")")]),
        ];
        
        let pause_line = if self.paused {
            Line::from(Span::styled("⏸️ 已暂停", Style::default().fg(Color::Yellow)))
        } else {
            Line::from(Span::styled(format!("🕐 每{}秒刷新", self.refresh_interval), Style::default().fg(Color::DarkGray)))
        };
        
        let all_lines = [lines, vec![pause_line]].concat();
        Paragraph::new(all_lines)
            .block(Block::default().title(self.name()).borders(Borders::ALL).border_style(border_style))
            .wrap(Wrap { trim: true })
            .render(area, frame.buffer_mut());
    }
    
    fn height(&self) -> u16 { 9 }
    fn get_error(&self) -> Option<&str> { self.error.as_deref() }
}
