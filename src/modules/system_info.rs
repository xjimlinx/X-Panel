use crate::module_trait::{ModuleUpdate, PanelModule};
use async_trait::async_trait;
use chrono::Local;
use ratatui::{
    layout::Rect,
    prelude::{Line, Modifier, Span},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::fs;
use std::time::Instant;

// CPU 使用率计算所需的数据
struct CpuStats {
    user: u64,
    nice: u64,
    system: u64,
    idle: u64,
    iowait: u64,
}

impl CpuStats {
    fn from_stat_line(line: &str) -> Option<Self> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 8 {
            return None;
        }
        
        Some(CpuStats {
            user: parts[1].parse().unwrap_or(0),
            nice: parts[2].parse().unwrap_or(0),
            system: parts[3].parse().unwrap_or(0),
            idle: parts[4].parse().unwrap_or(0),
            iowait: parts[5].parse().unwrap_or(0),
        })
    }
    
    fn total(&self) -> u64 {
        self.user + self.nice + self.system + self.idle + self.iowait
    }
    
    fn active(&self) -> u64 {
        self.user + self.nice + self.system
    }
}

/// 系统信息模块
pub struct SystemInfoModule {
    cpu_usage: String,
    memory_usage: String,
    memory_detail: String,
    disk_usage: String,
    gpu_info: String,
    battery_info: String,
    power_usage: String,
    last_update: String,
    refresh_interval: u64,
    paused: bool,
    prev_cpu_stats: Option<CpuStats>,
    prev_time: Option<Instant>,
}

impl SystemInfoModule {
    pub fn new(refresh_interval: u64) -> Self {
        Self {
            cpu_usage: "N/A".to_string(),
            memory_usage: "N/A".to_string(),
            memory_detail: String::new(),
            disk_usage: "N/A".to_string(),
            gpu_info: "N/A".to_string(),
            battery_info: "N/A".to_string(),
            power_usage: "N/A".to_string(),
            last_update: String::new(),
            refresh_interval,
            paused: false,
            prev_cpu_stats: None,
            prev_time: None,
        }
    }
    
    fn update_info(&mut self) {
        self.cpu_usage = Self::get_cpu_usage(&mut self.prev_cpu_stats, &mut self.prev_time);
        let (mem_usage, mem_detail) = Self::get_memory_usage();
        self.memory_usage = mem_usage;
        self.memory_detail = mem_detail;
        self.disk_usage = Self::get_disk_usage();
        self.gpu_info = Self::get_gpu_info();
        self.battery_info = Self::get_battery_info();
        self.power_usage = Self::get_power_usage();
        self.last_update = Local::now().format("%H:%M:%S").to_string();
    }
    
    fn get_cpu_usage(prev_stats: &mut Option<CpuStats>, prev_time: &mut Option<Instant>) -> String {
        let content = match fs::read_to_string("/proc/stat") {
            Ok(c) => c,
            Err(_) => return "N/A".to_string(),
        };

        let first_line = content.lines().next().unwrap_or("");
        let current_stats = match CpuStats::from_stat_line(first_line) {
            Some(s) => s,
            None => return "N/A".to_string(),
        };

        // 如果是第一次获取，保存数据并返回一个估算值
        match (prev_stats.as_ref(), prev_time.as_ref()) {
            (Some(prev_s), Some(prev_t)) => {
                let elapsed = prev_t.elapsed();
                
                // 如果间隔太短，使用上次的计算结果
                if elapsed.as_millis() < 50 {
                    return format!("{:.1}%", Self::calculate_usage(prev_s, &current_stats));
                }

                let usage = Self::calculate_usage(prev_s, &current_stats);

                // 更新数据
                *prev_stats = Some(current_stats);
                *prev_time = Some(Instant::now());

                format!("{:.1}%", usage)
            }
            _ => {
                // 第一次获取：保存数据，返回 0% 作为初始值
                *prev_stats = Some(current_stats);
                *prev_time = Some(Instant::now());
                "0.0%".to_string()
            }
        }
    }
    
    fn calculate_usage(prev: &CpuStats, curr: &CpuStats) -> f64 {
        let prev_total = prev.total() as f64;
        let curr_total = curr.total() as f64;
        let prev_active = prev.active() as f64;
        let curr_active = curr.active() as f64;
        
        let total_diff = curr_total - prev_total;
        let active_diff = curr_active - prev_active;
        
        if total_diff == 0.0 {
            return 0.0;
        }
        
        (active_diff / total_diff) * 100.0
    }
    
    fn get_memory_usage() -> (String, String) {
        let content = match fs::read_to_string("/proc/meminfo") {
            Ok(c) => c,
            Err(_) => return ("N/A".to_string(), String::new()),
        };
        
        let mut mem_total = 0u64;
        let mut mem_available = 0u64;
        
        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let value = parts[1].parse().unwrap_or(0);
                match parts[0] {
                    "MemTotal:" => mem_total = value,
                    "MemAvailable:" => mem_available = value,
                    _ => {}
                }
            }
        }
        
        if mem_total == 0 {
            return ("N/A".to_string(), String::new());
        }
        
        let mem_used = mem_total - mem_available;
        let percent = (mem_used as f64 / mem_total as f64) * 100.0;
        
        let usage = format!("{:.1}%", percent);
        let detail = format!(
            "{:.1}GB / {:.1}GB",
            mem_used as f64 / 1024.0 / 1024.0,
            mem_total as f64 / 1024.0 / 1024.0
        );
        
        (usage, detail)
    }
    
    fn get_disk_usage() -> String {
        // 使用 df 命令获取磁盘使用情况
        match std::process::Command::new("df")
            .arg("/")
            .output()
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = stdout.lines().collect();
                if lines.len() >= 2 {
                    let parts: Vec<&str> = lines[1].split_whitespace().collect();
                    if parts.len() >= 5 {
                        return parts[4].to_string();
                    }
                }
            }
            Err(_) => {}
        }
        
        "N/A".to_string()
    }
    
    fn get_gpu_info() -> String {
        // 尝试多种方式获取 GPU 信息
        
        // 1. 尝试 nvidia-smi (NVIDIA GPU)
        if let Ok(output) = std::process::Command::new("nvidia-smi")
            .args(["--query-gpu=name,utilization.gpu,memory.used,memory.total", "--format=csv,noheader"])
            .output()
        {
            if output.status.success() {
                let info = String::from_utf8_lossy(&output.stdout);
                if !info.trim().is_empty() {
                    let parts: Vec<&str> = info.trim().split(", ").collect();
                    if parts.len() >= 4 {
                        return format!("{} | 使用：{} | 显存：{}/{}", 
                            parts[0], parts[1], parts[2], parts[3]);
                    }
                }
            }
        }
        
        // 2. 尝试读取 AMD GPU 信息
        if let Ok(content) = fs::read_to_string("/sys/class/drm/card0/device/gpu_busy_percent") {
            let usage = content.trim();
            if !usage.is_empty() {
                let name = Self::get_gpu_name();
                return format!("{} | 使用：{}%", name, usage);
            }
        }
        
        // 3. 尝试 lspci 获取 GPU 名称
        let gpu_name = Self::get_gpu_name();
        if !gpu_name.is_empty() {
            return format!("{} | 使用：N/A", gpu_name);
        }
        
        "N/A".to_string()
    }
    
    fn get_gpu_name() -> String {
        // 尝试从 lspci 获取 GPU 名称
        if let Ok(output) = std::process::Command::new("lspci")
            .arg("-v")
            .output()
        {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("VGA compatible controller") || line.contains("3D controller") {
                    // 提取 GPU 名称
                    if let Some(start) = line.find(": ") {
                        let name = line[start + 2..].trim();
                        // 简化名称，去掉版本信息
                        let name = name.split('(').next().unwrap_or(name).trim();
                        return name.to_string();
                    }
                }
            }
        }
        
        // 尝试从 DRM 获取
        if let Ok(entries) = fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let path = entry.path().join("device/uevent");
                if let Ok(content) = fs::read_to_string(&path) {
                    for line in content.lines() {
                        if line.starts_with("DRM_DEVICE_NAME=") {
                            return line.trim_start_matches("DRM_DEVICE_NAME=").to_string();
                        }
                    }
                }
            }
        }
        
        String::new()
    }
    
    fn get_battery_info() -> String {
        // 尝试读取电池信息
        let mut battery_percent = String::new();
        let mut battery_status = String::new();
        let mut power_online = false;
        
        // 读取电池容量
        if let Ok(content) = fs::read_to_string("/sys/class/power_supply/BAT0/capacity") {
            battery_percent = content.trim().to_string();
        }
        
        // 读取电池状态
        if let Ok(content) = fs::read_to_string("/sys/class/power_supply/BAT0/status") {
            battery_status = content.trim().to_string();
        }
        
        // 检查是否接通电源
        for adapter in &["ADP0", "ADP1", "AC", "ACAD"] {
            if let Ok(content) = fs::read_to_string(format!("/sys/class/power_supply/{}/online", adapter)) {
                if content.trim() == "1" {
                    power_online = true;
                    break;
                }
            }
        }
        
        if battery_percent.is_empty() {
            return "N/A".to_string();
        }
        
        // 构建电池信息字符串
        let status_icon = match battery_status.as_str() {
            "Full" => "✓",
            "Charging" => "⚡",
            "Discharging" => "🔋",
            "Not charging" => "⏸️",
            _ => "🔋",
        };
        
        let power_icon = if power_online { "🔌" } else { "" };
        
        format!("{} {}% {}{}", status_icon, battery_percent, power_icon, battery_status)
    }
    
    fn get_power_usage() -> String {
        // 尝试从 hwmon 读取 GPU 功耗 (power1_input 单位是微瓦)
        if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                let power_path = entry.path().join("power1_input");
                if let Ok(content) = fs::read_to_string(power_path) {
                    if let Ok(power) = content.trim().parse::<i64>() {
                        if power > 0 {
                            let watts = power as f64 / 1_000_000.0;
                            return format!("{:.1}W", watts);
                        }
                    }
                }
            }
        }
        
        // 尝试从 AMD GPU 读取
        if let Ok(content) = fs::read_to_string("/sys/kernel/debug/dri/0/amdgpu_pm_info") {
            for line in content.lines() {
                if line.contains("power") || line.contains("Power") {
                    return line.trim().to_string();
                }
            }
        }
        
        // 尝试从 AMD GPU hwmon 读取
        if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
            for entry in entries.flatten() {
                let name_path = entry.path().join("name");
                if let Ok(name) = fs::read_to_string(name_path) {
                    if name.contains("amdgpu") {
                        let power_path = entry.path().join("power1_input");
                        if let Ok(content) = fs::read_to_string(power_path) {
                            if let Ok(power) = content.trim().parse::<i64>() {
                                let watts = power as f64 / 1_000_000.0;
                                return format!("{:.1}W", watts);
                            }
                        }
                    }
                }
            }
        }
        
        "N/A".to_string()
    }
}

#[async_trait]
impl PanelModule for SystemInfoModule {
    fn id(&self) -> &str {
        "system_info"
    }
    
    fn name(&self) -> &str {
        "🖥️ 系统信息"
    }
    
    fn refresh_interval(&self) -> u64 {
        if self.paused { 0 } else { self.refresh_interval }
    }
    
    fn set_refresh_interval(&mut self, interval: u64) {
        self.refresh_interval = interval;
        if interval > 0 {
            self.paused = false;
        }
    }
    
    fn toggle_pause(&mut self) {
        self.paused = !self.paused;
    }
    
    async fn update(&mut self) -> ModuleUpdate {
        self.update_info();
        ModuleUpdate {
            id: self.id().to_string(),
            success: true,
            error: None,
        }
    }
    
    fn render(&self, frame: &mut Frame, area: Rect, is_selected: bool) {
        let border_style = if is_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        
        // 暂停状态标识
        let pause_line = if self.paused {
            Line::from(Span::styled("⏸️ 已暂停", Style::default().fg(Color::Yellow)))
        } else {
            Line::from(Span::styled(
                format!("🕐 每{}秒刷新", self.refresh_interval),
                Style::default().fg(Color::DarkGray),
            ))
        };

        let lines = vec![
            Line::from(vec![
                ratatui::text::Span::styled("CPU:  ", Style::default().fg(Color::Yellow)),
                ratatui::text::Span::styled(&self.cpu_usage, Style::default().fg(Color::Cyan)),
            ]),
            Line::from(vec![
                ratatui::text::Span::styled("内存：", Style::default().fg(Color::Yellow)),
                ratatui::text::Span::styled(
                    format!("{} ({})", self.memory_usage, self.memory_detail),
                    Style::default().fg(Color::Green),
                ),
            ]),
            Line::from(vec![
                ratatui::text::Span::styled("磁盘：", Style::default().fg(Color::Yellow)),
                ratatui::text::Span::styled(&self.disk_usage, Style::default().fg(Color::Magenta)),
            ]),
            Line::from(vec![
                ratatui::text::Span::styled("GPU:  ", Style::default().fg(Color::Yellow)),
                ratatui::text::Span::styled(&self.gpu_info, Style::default().fg(Color::Blue)),
            ]),
            Line::from(vec![
                ratatui::text::Span::styled("电源：", Style::default().fg(Color::Yellow)),
                ratatui::text::Span::styled(&self.battery_info, Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                ratatui::text::Span::styled("功耗：", Style::default().fg(Color::Yellow)),
                ratatui::text::Span::styled(&self.power_usage, Style::default().fg(Color::Red)),
            ]),
            pause_line,
        ];

        let block = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(self.name())
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .style(Style::default().fg(Color::White)),
            );

        frame.render_widget(block, area);
    }
    
    fn height(&self) -> u16 {
        10
    }
    
    fn get_error(&self) -> Option<&str> {
        None
    }
}
