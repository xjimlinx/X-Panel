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
use std::collections::HashMap;
use std::fs;
use std::time::Instant;
use crate::theme::Theme;

#[derive(Debug, Clone, Default)]
struct NetworkStats {
    rx_bytes: u64,
    tx_bytes: u64,
}

fn list_interfaces() -> Vec<String> {
    let mut interfaces = Vec::new();
    if let Ok(entries) = fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "lo" {
                continue;
            }
            interfaces.push(name);
        }
    }
    interfaces.sort();
    interfaces
}

fn get_default_interface() -> String {
    let interfaces = list_interfaces();
    for name in &interfaces {
        if name.starts_with("wlan") || name.starts_with("wl") {
            return name.clone();
        }
    }
    for name in &interfaces {
        if name.starts_with("eth") || name.starts_with("en") {
            return name.clone();
        }
    }
    interfaces.first().cloned().unwrap_or_else(|| "eth0".to_string())
}

pub struct NetworkMonitorModule {
    interface: String,
    interfaces: Vec<String>,
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

impl NetworkMonitorModule {
    pub fn new(interface: String, refresh_interval: u64) -> Self {
        let interfaces = list_interfaces();
        let iface = if interface.is_empty() {
            get_default_interface()
        } else if interfaces.contains(&interface) {
            interface
        } else {
            get_default_interface()
        };
        Self {
            interface: iface,
            interfaces,
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

    fn cycle_interface(&mut self) {
        if self.interfaces.is_empty() { return; }
        if let Some(pos) = self.interfaces.iter().position(|i| i == &self.interface) {
            let next = (pos + 1) % self.interfaces.len();
            self.interface = self.interfaces[next].clone();
        } else {
            self.interface = self.interfaces[0].clone();
        }
        self.update_info();
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
        self.local_ip = Self::get_local_ip(&self.interface);
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

    fn get_local_ip(interface: &str) -> String {
        if let Ok(output) = std::process::Command::new("ip")
            .args(["addr", "show", interface])
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("inet ") {
                    if let Some(rest) = trimmed.strip_prefix("inet ") {
                        if let Some(ip) = rest.split_whitespace().next() {
                            let ip = ip.split('/').next().unwrap_or("N/A");
                            if ip != "127.0.0.1" {
                                return ip.to_string();
                            }
                        }
                    }
                }
            }
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
    
    fn render(&self, frame: &mut Frame, area: Rect, is_selected: bool, theme: &Theme) {
        let border_style = if is_selected {
            Style::default().fg(theme.border_selected).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.border_default)
        };
        
        let hint = if is_selected { " [n 切换网卡]" } else { "" };
        let iface_line = Line::from(vec![
            Span::styled("接口：", Style::default().fg(Color::Yellow)),
            Span::styled(&self.interface, Style::default().fg(Color::White)),
            Span::styled(hint, Style::default().fg(Color::DarkGray)),
        ]);
        
        let wifi_line = if !self.wifi_ssid.is_empty() && self.wifi_ssid != "N/A" {
            Some(Line::from(vec![
                Span::styled("WiFi: ", Style::default().fg(Color::Yellow)),
                Span::styled(&self.wifi_ssid, Style::default().fg(Color::Green)),
                Span::styled(format!(" ({}%)", self.wifi_signal), Style::default().fg(Color::Cyan)),
            ]))
        } else {
            None
        };
        
        let mut lines = vec![
            Line::from(vec![
                Span::styled("IP:   ", Style::default().fg(Color::Yellow)),
                Span::styled(&self.local_ip, Style::default().fg(Color::White)),
            ]),
            iface_line,
        ];
        if let Some(wifi) = wifi_line {
            lines.push(wifi);
        }
        lines.extend_from_slice(&[
            Line::from(vec![
                Span::styled("↓ ", Style::default().fg(Color::Green)),
                Span::styled(&self.rx_speed, Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                Span::raw("  ("),
                Span::styled(&self.total_rx, Style::default().fg(Color::DarkGray)),
                Span::raw(")"),
            ]),
            Line::from(vec![
                Span::styled("↑ ", Style::default().fg(Color::Cyan)),
                Span::styled(&self.tx_speed, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw("  ("),
                Span::styled(&self.total_tx, Style::default().fg(Color::DarkGray)),
                Span::raw(")"),
            ]),
        ]);
        
        let pause_line = if self.paused {
            Line::from(Span::styled("⏸️ 已暂停", Style::default().fg(Color::Yellow)))
        } else {
            Line::from(Span::styled(format!("🕐 每{}秒刷新", self.refresh_interval), Style::default().fg(Color::DarkGray)))
        };
        lines.push(pause_line);
        
        Paragraph::new(lines)
            .block(Block::default().title(self.name()).borders(Borders::ALL).border_style(border_style).style(Style::default().fg(theme.text).bg(theme.bg)))
            .wrap(Wrap { trim: true })
            .render(area, frame.buffer_mut());
    }
    
    fn height(&self) -> u16 { 9 }
    fn get_error(&self) -> Option<&str> { self.error.as_deref() }

    fn handle_key(&mut self, key: char) -> bool {
        if key == 'n' || key == 'N' {
            self.cycle_interface();
            true
        } else {
            false
        }
    }

    fn save_state(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();
        map.insert("interface".to_string(), self.interface.clone());
        map
    }

    fn load_state(&mut self, data: &HashMap<String, String>) {
        if let Some(iface) = data.get("interface") {
            if self.interfaces.contains(iface) {
                self.interface = iface.clone();
            }
        }
    }
}
