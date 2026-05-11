use crate::config::{Config, ModuleConfig};
use crate::logger::{LogLevel, Logger};
use crate::module_trait::PanelModule;
use crate::registry::ModuleRegistry;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    prelude::{Line, Span},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::collections::HashMap;
use std::{io, time::Duration};

/// 布局模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LayoutMode {
    Single,     // 单列
    Double,     // 双列
    Triple,     // 三列
}

impl LayoutMode {
    fn next(&self) -> Self {
        match self {
            LayoutMode::Single => LayoutMode::Double,
            LayoutMode::Double => LayoutMode::Triple,
            LayoutMode::Triple => LayoutMode::Single,
        }
    }
    
    fn columns(&self) -> usize {
        match self {
            LayoutMode::Single => 1,
            LayoutMode::Double => 2,
            LayoutMode::Triple => 3,
        }
    }
}

/// 主面板
pub struct Panel {
    registry: ModuleRegistry,
    running: bool,
    status_message: String,
    current_module_idx: usize,
    layout_mode: LayoutMode,
    column_weights: Vec<u16>,  // 每列的权重
    show_about: bool,
    show_settings: bool,
    settings_cursor: usize,
    show_logs: bool,
    log_scroll: usize,
    logger: Logger,
    config: Config,
    module_height_deltas: Vec<i16>,  // 每个模块的高度偏移
}

impl Panel {
    pub fn new() -> Self {
        Self {
            registry: ModuleRegistry::new(),
            running: true,
            status_message: String::new(),
            current_module_idx: 0,
            layout_mode: LayoutMode::Single,
            column_weights: vec![10],  // 默认单列，权重 10
            show_about: false,
            show_settings: false,
            settings_cursor: 0,
            show_logs: false,
            log_scroll: 0,
            logger: Logger::new(500),
            config: Config::load(),
            module_height_deltas: vec![],
        }
    }

    /// 注册模块
    pub fn register_module(&mut self, module: Box<dyn PanelModule>) {
        let id = module.id().to_string();
        let name = module.name().to_string();
        self.registry.register(module);
        self.module_height_deltas.push(0);
        // 应用已保存的模块配置
        if let Some(mcfg) = self.config.modules.get(&id) {
            let idx = self.module_height_deltas.len() - 1;
            self.module_height_deltas[idx] = mcfg.height_offset;
            self.registry.set_visible(&id, mcfg.visible);
            if let Some((_, m)) = self.registry.modules_mut().nth(idx) {
                m.set_refresh_interval(60); // 恢复时重置间隔
            }
            self.logger.info(&format!("恢复模块配置：{}", name));
        }
        self.logger.info(&format!("注册模块：{}", name));
    }

    /// 写入一条 INFO 日志
    pub fn log_info(&mut self, msg: &str) {
        self.logger.info(msg);
    }

    /// 获取模块总数
    pub fn module_count(&self) -> usize {
        self.registry.len()
    }

    /// 应用已保存的全局配置（注册完所有模块后调用）
    pub fn apply_config(&mut self) {
        self.layout_mode = match self.config.layout.as_str() {
            "Double" => LayoutMode::Double,
            "Triple" => LayoutMode::Triple,
            _ => LayoutMode::Single,
        };
        if self.config.column_weights.len() == self.layout_mode.columns() {
            self.column_weights = self.config.column_weights.clone();
        } else {
            self.column_weights = vec![10; self.layout_mode.columns()];
        }
        self.logger.info(&format!("应用保存的配置：{:?} 列", self.layout_mode.columns()));
    }

    /// 保存当前配置到文件
    fn save_config(&mut self) {
        self.config.layout = match self.layout_mode {
            LayoutMode::Single => "Single",
            LayoutMode::Double => "Double",
            LayoutMode::Triple => "Triple",
        }.to_string();
        self.config.column_weights = self.column_weights.clone();

        // 更新所有模块的配置
        for (i, (id, _)) in self.registry.modules().enumerate() {
            let visible = self.registry.is_visible(id);
            let height_offset = self.module_height_deltas.get(i).copied().unwrap_or(0);
            self.config.modules.insert(id.clone(), ModuleConfig {
                visible,
                height_offset,
            });
        }

        self.config.save();
    }

    /// 运行面板
    pub async fn run(&mut self) -> anyhow::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        self.logger.info("X-Panel 启动");

        // 立即更新所有模块
        for (_, module) in self.registry.modules_mut() {
            let result = module.update().await;
            if !result.success {
                let err = result.error.unwrap_or_else(|| "未知错误".to_string());
                self.logger.error(&format!("初始刷新 {}: {}", module.name(), err));
            }
        }

        let result = self.run_loop(&mut terminal).await;

        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        result
    }

    async fn run_loop<B: ratatui::backend::Backend>(
        &mut self,
        terminal: &mut Terminal<B>,
    ) -> anyhow::Result<()> {
        let mut module_last_update: HashMap<String, std::time::Instant> = HashMap::new();

        loop {
            terminal.draw(|f| self.ui(f))?;

            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    if self.show_logs {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('v') | KeyCode::Char('V') => {
                                self.show_logs = false;
                                self.status_message = String::new();
                            }
                            KeyCode::Up => {
                                let entries = self.logger.recent(1000).len();
                                if self.log_scroll < entries.saturating_sub(1) {
                                    self.log_scroll += 1;
                                }
                            }
                            KeyCode::Down => {
                                if self.log_scroll > 0 {
                                    self.log_scroll -= 1;
                                }
                            }
                            _ => {}
                        }
                    } else if self.show_settings {
                        match key.code {
                            KeyCode::Esc => {
                                self.show_settings = false;
                                self.adjust_current_visible();
                                self.status_message = String::new();
                            }
                            KeyCode::Up => {
                                if self.settings_cursor > 0 {
                                    self.settings_cursor -= 1;
                                }
                            }
                            KeyCode::Down => {
                                let len = self.registry.len();
                                if self.settings_cursor < len - 1 {
                                    self.settings_cursor += 1;
                                }
                            }
                            KeyCode::Char(' ') => {
                                if let Some(id) = self.registry.nth_id(self.settings_cursor) {
                                    let visible = self.registry.is_visible(&id);
                                    self.registry.set_visible(&id, !visible);
                                    if visible {
                                        self.logger.info(&format!("隐藏模块：{}", id));
                                    } else {
                                        self.logger.info(&format!("显示模块：{}", id));
                                    }
                                    self.save_config();
                                }
                            }
                            _ => {}
                        }
                    } else {
                        match key.code {
                            KeyCode::Esc => {
                                self.show_about = !self.show_about;
                                self.status_message = if self.show_about {
                                    "关于页面 - 按 ESC 关闭".to_string()
                                } else {
                                    String::new()
                                };
                            }
                            KeyCode::Char('v') | KeyCode::Char('V') => {
                                self.show_logs = true;
                                self.log_scroll = 0;
                                self.show_about = false;
                                self.status_message = "日志 - ESC 返回".to_string();
                                self.logger.info("打开日志查看器");
                            }
                            KeyCode::Char('s') | KeyCode::Char('S') => {
                                self.show_settings = true;
                                self.settings_cursor = 0;
                                self.show_about = false;
                                self.status_message = "模块设置 - ESC 返回".to_string();
                            }
                            KeyCode::Char('q') => self.running = false,
                            KeyCode::Char('r') => {
                                for (_, module) in self.registry.modules_mut() {
                                    let _ = module.update().await;
                                }
                                self.status_message = "已刷新所有模块".to_string();
                                self.logger.info("手动刷新所有模块");
                            }
                            KeyCode::Char('u') => {
                                if let Some((_, module)) = self.registry.modules_mut().nth(self.current_module_idx) {
                                    let _ = module.update().await;
                                    self.status_message = format!("已刷新：{}", module.name());
                                    self.logger.info(&format!("手动刷新：{}", module.name()));
                                }
                            }
                            KeyCode::Char(' ') => {
                                let mut info = None;
                                if let Some((_, module)) = self.registry.modules_mut().nth(self.current_module_idx) {
                                    module.toggle_pause();
                                    info = Some((module.name().to_string(), module.is_paused()));
                                }
                                if let Some((name, paused)) = info {
                                    let status = if paused { "已暂停" } else { "已恢复" };
                                    self.status_message = format!("{}: {}", name, status);
                                    self.logger.info(&format!("{}：{}", name, status));
                                }
                                self.save_config();
                            }
                            KeyCode::Char('l') | KeyCode::Char('L') => {
                                self.layout_mode = self.layout_mode.next();
                                self.adjust_column_weights();
                                self.status_message = format!("布局：{:?} 列", self.layout_mode.columns());
                                self.logger.info(&format!("切换布局：{}列", self.layout_mode.columns()));
                                self.save_config();
                            }
                            KeyCode::Char('[') => {
                                self.adjust_column_width(self.current_module_idx, -1);
                            }
                            KeyCode::Char(']') => {
                                self.adjust_column_width(self.current_module_idx, 1);
                            }
                            KeyCode::Char('+') | KeyCode::Char('=') => {
                                if let Some((_, module)) = self.registry.modules_mut().nth(self.current_module_idx) {
                                    let current = module.refresh_interval();
                                    let new_interval = (current + 10).min(300);
                                    module.set_refresh_interval(new_interval);
                                    self.status_message = format!("{}: 刷新间隔 {} 秒", module.name(), new_interval);
                                }
                            }
                            KeyCode::Char('-') => {
                                if let Some((_, module)) = self.registry.modules_mut().nth(self.current_module_idx) {
                                    let current = module.refresh_interval();
                                    let new_interval = if current <= 5 { 1 } else { current - 5 };
                                    module.set_refresh_interval(new_interval);
                                    self.status_message = format!("{}: 刷新间隔 {} 秒", module.name(), new_interval);
                                }
                            }
                            KeyCode::Up => {
                                if let Some(idx) = self.visible_prev(self.current_module_idx) {
                                    self.current_module_idx = idx;
                                    self.status_message = format!("选中：{}", self.get_current_module_name());
                                }
                            }
                            KeyCode::Down => {
                                if let Some(idx) = self.visible_next(self.current_module_idx) {
                                    self.current_module_idx = idx;
                                    self.status_message = format!("选中：{}", self.get_current_module_name());
                                }
                            }
                            KeyCode::Left => {
                                if self.layout_mode.columns() > 1 {
                                    if let Some(idx) = self.navigate_horizontal(self.current_module_idx, -1) {
                                        self.current_module_idx = idx;
                                        self.adjust_current_visible();
                                        self.status_message = format!("选中：{}", self.get_current_module_name());
                                    }
                                }
                            }
                            KeyCode::Right => {
                                if self.layout_mode.columns() > 1 {
                                    if let Some(idx) = self.navigate_horizontal(self.current_module_idx, 1) {
                                        self.current_module_idx = idx;
                                        self.adjust_current_visible();
                                        self.status_message = format!("选中：{}", self.get_current_module_name());
                                    }
                                }
                            }
                            KeyCode::Char('{') | KeyCode::PageUp => {
                                self.adjust_module_height(self.current_module_idx, -1);
                            }
                            KeyCode::Char('}') | KeyCode::PageDown => {
                                self.adjust_module_height(self.current_module_idx, 1);
                            }
                            _ => {}
                        }
                    }
                }
            }

            // 自动更新模块
            let now = std::time::Instant::now();
            for (id, module) in self.registry.modules_mut() {
                let interval = module.refresh_interval();
                if interval == 0 { continue; }
                
                let last_time = module_last_update.entry(id.clone()).or_insert(now);
                if now.duration_since(*last_time).as_secs() >= interval {
                    let result = module.update().await;
                    if !result.success {
                        let err = result.error.unwrap_or_else(|| "未知错误".to_string());
                        self.logger.error(&format!("刷新 {}: {}", module.name(), err));
                    }
                    *last_time = now;
                }
            }

            if !self.running {
                break;
            }
        }

        Ok(())
    }
    
    fn adjust_column_weights(&mut self) {
        let columns = self.layout_mode.columns();
        self.column_weights = vec![10; columns];
    }
    
    fn adjust_column_width(&mut self, module_idx: usize, delta: i16) {
        let columns = self.layout_mode.columns();
        if columns <= 1 { return; }
        
        // 计算模块所在的列
        let modules_per_column = (self.registry.len() + columns - 1) / columns;
        let column_idx = module_idx / modules_per_column;
        
        if column_idx >= columns { return; }
        
        let new_weight = (self.column_weights[column_idx] as i16 + delta).max(3).min(20) as u16;
        self.column_weights[column_idx] = new_weight;
        
        self.status_message = format!("列{}宽度：{}", column_idx + 1, new_weight);
        self.save_config();
    }

    fn modules_per_column(&self) -> usize {
        let len = self.registry.len();
        let cols = self.layout_mode.columns();
        if cols == 0 { return len; }
        (len + cols - 1) / cols
    }

    fn navigate_horizontal(&self, from_idx: usize, direction: i8) -> Option<usize> {
        let cols = self.layout_mode.columns();
        let mpc = self.modules_per_column();
        let len = self.registry.len();
        let cur_col = from_idx / mpc;
        let row = from_idx % mpc;

        let target_col = if direction > 0 {
            if cur_col + 1 >= cols { return None; }
            cur_col + 1
        } else {
            if cur_col == 0 { return None; }
            cur_col - 1
        };

        let start = target_col * mpc;
        let end = std::cmp::min(start + mpc, len);
        if start >= len { return None; }
        Some(std::cmp::min(start + row, end - 1))
    }

    fn visible_prev(&self, from: usize) -> Option<usize> {
        let mut i = from;
        loop {
            if i == 0 { return None; }
            i -= 1;
            if let Some((id, _)) = self.registry.modules().nth(i) {
                if self.registry.is_visible(id) {
                    return Some(i);
                }
            }
        }
    }

    fn visible_next(&self, from: usize) -> Option<usize> {
        let len = self.registry.len();
        let mut i = from;
        loop {
            i += 1;
            if i >= len { return None; }
            if let Some((id, _)) = self.registry.modules().nth(i) {
                if self.registry.is_visible(id) {
                    return Some(i);
                }
            }
        }
    }

    fn adjust_current_visible(&mut self) {
        if let Some((id, _)) = self.registry.modules().nth(self.current_module_idx) {
            if !self.registry.is_visible(id) {
                if let Some(idx) = self.visible_next(self.current_module_idx) {
                    self.current_module_idx = idx;
                } else if let Some(idx) = self.visible_prev(self.current_module_idx) {
                    self.current_module_idx = idx;
                }
            }
        }
    }

    fn adjust_module_height(&mut self, module_idx: usize, delta: i16) {
        if module_idx >= self.module_height_deltas.len() { return; }
        let len = self.registry.len();
        let mpc = self.modules_per_column();
        let col = module_idx / mpc;
        let start = col * mpc;
        let end = std::cmp::min(start + mpc, len);

        if end - start != 2 {
            self.status_message = "当前列不支持调整高度（仅 2 个模块的列可调）".to_string();
            return;
        }

        let new_delta = (self.module_height_deltas[module_idx] + delta).clamp(-5, 10);
        self.module_height_deltas[module_idx] = new_delta;

        let other = if module_idx == start { start + 1 } else { start };
        let other_delta = (self.module_height_deltas[other] - delta).clamp(-5, 10);
        self.module_height_deltas[other] = other_delta;

        let name = self.get_current_module_name();
        self.status_message = format!("{}: 高度偏移 {} (范围 -5 ~ +10)", name, new_delta);
        self.save_config();
    }

    fn ui(&mut self, f: &mut Frame) {
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),  // 标题
                Constraint::Min(0),     // 模块内容
                Constraint::Length(3),  // 状态栏
                Constraint::Length(5),  // 帮助
            ])
            .split(f.size());

        // 标题
        let title = Paragraph::new("📊 X-Panel - 模块化面板")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(title, main_chunks[0]);

        // 模块内容 - 多列布局
        self.render_modules(f, main_chunks[1]);

        // 状态栏
        let visible_count = self.registry.visible_count();
        let status = Paragraph::new(format!(
            "{} | 模块：{}/{} (可见 {}) | 布局：{}列 | 空格-暂停 | l-布局 | [/]-列宽 | PgUp/PgDn-列高",
            self.status_message,
            self.current_module_idx + 1,
            self.registry.len(),
            visible_count,
            self.layout_mode.columns()
        ))
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(status, main_chunks[2]);

        // 帮助信息
        let help = List::new(vec![
            ListItem::new(Line::from("↑/↓ - 上下切换 | ←/→ - 左右列切换 | 空格 - 暂停 | +/- - 刷新间隔")),
            ListItem::new(Line::from("l - 切换布局 | [/] - 列宽 | PgUp/PgDn - 列高 | r - 刷新全部 | u - 刷新当前")),
            ListItem::new(Line::from("s - 模块设置 | v - 日志 | ESC - 关于 | q - 退出")),
        ])
        .block(Block::default().title("帮助").borders(Borders::ALL));
        f.render_widget(help, main_chunks[3]);

        // 日志查看器叠加层
        if self.show_logs {
            self.render_log_viewer(f);
        }

        // 设置页面叠加层
        if self.show_settings {
            self.render_settings(f);
        }

        // 关于页面叠加层
        if self.show_about {
            self.render_about(f);
        }
    }

    fn render_log_viewer(&mut self, f: &mut Frame) {
        let area = f.size();
        let block = Block::default()
            .title("📋 日志")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        let log_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length((area.height - 4) / 2),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(area)[1];

        let inner = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(2),
                Constraint::Min(0),
                Constraint::Length(2),
            ])
            .split(log_area)[1];

        f.render_widget(Clear, inner);
        f.render_widget(block, inner);

        let content_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0), Constraint::Length(1)])
            .split(inner)[1];

        let entries = self.logger.recent(200);
        let max_display = content_area.height.saturating_sub(1) as usize;
        let start = self.log_scroll.min(entries.len().saturating_sub(max_display).max(0));
        let visible: Vec<(&str, &LogLevel)> = entries.iter()
            .skip(start)
            .take(max_display)
            .map(|(s, l)| (*s, l))
            .collect();

        let mut all_lines: Vec<Line> = visible.iter().map(|(text, level)| {
            let color = match level {
                LogLevel::Error => Color::Red,
                LogLevel::Warn => Color::Yellow,
                LogLevel::Info => Color::White,
            };
            Line::from(Span::styled(*text, Style::default().fg(color)))
        }).collect();

        let hint = format!("共 {} 条 | ↑↓ 滚动 | ESC/v 关闭", entries.len());
        all_lines.push(Line::from(Span::styled(hint, Style::default().fg(Color::DarkGray))));

        let paragraph = Paragraph::new(all_lines).block(Block::default());
        f.render_widget(paragraph, content_area);
    }

    fn render_settings(&mut self, f: &mut Frame) {
        let area = f.size();
        let block = Block::default()
            .title("⚙ 模块设置")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        let settings_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length((area.height - 14) / 2),
                Constraint::Length(14),
                Constraint::Min(0),
            ])
            .split(area)[1];

        let inner = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length((area.width - 50) / 2),
                Constraint::Length(50),
                Constraint::Min(0),
            ])
            .split(settings_area)[1];

        f.render_widget(Clear, inner);
        f.render_widget(block, inner);

        let module_list: Vec<(String, String)> = self.registry.modules()
            .map(|(id, m)| (id.clone(), m.name().to_string()))
            .collect();

        let mut lines: Vec<Line> = Vec::new();
        for (i, (id, name)) in module_list.iter().enumerate() {
            let visible = self.registry.is_visible(id);
            let prefix = if visible { "[✓]" } else { "[✗]" };
            let style = if i == self.settings_cursor {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            lines.push(Line::from(vec![
                Span::styled(format!("{} {}", prefix, name), style),
            ]));
        }

        lines.push(Line::from(""));
        lines.push(Line::from(vec![
            Span::styled("↑/↓ - 选择 | 空格 - 开关 | ESC - 返回", Style::default().fg(Color::DarkGray)),
        ]));

        let inner_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(inner)[1];

        let paragraph = Paragraph::new(lines)
            .block(Block::default())
            .alignment(Alignment::Center);
        f.render_widget(paragraph, inner_area);
    }

    fn render_about(&mut self, f: &mut Frame) {
        let area = f.size();
        let block = Block::default()
            .title("关于 X-Panel")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan));

        let about_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length((area.height - 14) / 2),
                Constraint::Length(14),
                Constraint::Min(0),
            ])
            .split(area)[1];

        let inner = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length((area.width - 50) / 2),
                Constraint::Length(50),
                Constraint::Min(0),
            ])
            .split(about_area)[1];

        f.render_widget(Clear, inner);
        f.render_widget(block, inner);

        let title_style = Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
        let subtitle_style = Style::default().add_modifier(Modifier::UNDERLINED);
        let hint_style = Style::default().fg(Color::DarkGray);

        let text = vec![
            Line::from(vec![Span::styled("X-Panel", title_style)]),
            Line::from(""),
            Line::from("版本: 0.3.0"),
            Line::from("描述: 模块化终端面板框架"),
            Line::from("作者: xein"),
            Line::from("协议: MIT"),
            Line::from(""),
            Line::from(vec![Span::styled("内置模块:", subtitle_style)]),
            Line::from("  - DeepSeek 余额"),
            Line::from("  - 系统信息 (CPU/内存/磁盘/GPU/电池)"),
            Line::from("  - 网络监控 (网速/WiFi/IP)"),
            Line::from("  - 系统温度与风扇"),
            Line::from("  - 时钟日历"),
            Line::from(""),
            Line::from(vec![Span::styled("按 ESC 关闭", hint_style)]),
        ];

        let inner_area = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(inner)[1];

        let paragraph = Paragraph::new(text)
            .block(Block::default())
            .alignment(Alignment::Center);
        f.render_widget(paragraph, inner_area);
    }

    fn render_modules(&mut self, f: &mut Frame, area: Rect) {
        let all_modules: Vec<_> = self.registry.modules().collect();
        if all_modules.is_empty() {
            let empty = Paragraph::new("没有注册任何模块")
                .block(Block::default().title("模块列表").borders(Borders::ALL));
            f.render_widget(empty, area);
            return;
        }

        let modules: Vec<(usize, &Box<dyn PanelModule>)> = all_modules.iter()
            .enumerate()
            .filter(|(_, (id, _))| self.registry.is_visible(id))
            .map(|(i, (_, m))| (i, *m))
            .collect();

        if modules.is_empty() {
            let empty = Paragraph::new("所有模块已隐藏，按 s 打开设置")
                .block(Block::default().title("模块列表").borders(Borders::ALL));
            f.render_widget(empty, area);
            return;
        }

        let max_cols = self.layout_mode.columns();
        let effective_cols = std::cmp::min(max_cols, modules.len());
        let modules_per_column = (modules.len() + effective_cols - 1) / effective_cols;

        let column_constraints: Vec<Constraint> = self.column_weights.iter()
            .take(effective_cols)
            .map(|&w| Constraint::Percentage((w * 100 / self.column_weights.iter().take(effective_cols).sum::<u16>()) as u16))
            .collect();

        let columns_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(column_constraints)
            .split(area);

        for (col_idx, column_area) in columns_layout.iter().enumerate() {
            let start_idx = col_idx * modules_per_column;
            let end_idx = std::cmp::min(start_idx + modules_per_column, modules.len());
            if start_idx >= modules.len() { continue; }

            let constraints: Vec<Constraint> = modules[start_idx..end_idx]
                .iter()
                .enumerate()
                .map(|(_, (orig_idx, m))| {
                    let delta = self.module_height_deltas.get(*orig_idx).copied().unwrap_or(0);
                    let h = (m.height() as i16 + delta).max(3) as u16;
                    Constraint::Length(h + 2)
                })
                .collect();

            let column_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(*column_area);

            for (i, module_idx) in (start_idx..end_idx).enumerate() {
                if i < column_layout.len() {
                    let (orig_idx, module) = &modules[module_idx];
                    let is_selected = *orig_idx == self.current_module_idx;
                    module.render(f, column_layout[i], is_selected);
                }
            }
        }
    }

    fn get_current_module_name(&self) -> String {
        self.registry.modules()
            .nth(self.current_module_idx)
            .map(|(_, m)| m.name().to_string())
            .unwrap_or_else(|| "无".to_string())
    }
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}
