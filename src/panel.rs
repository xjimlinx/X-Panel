use crate::module_trait::PanelModule;
use crate::registry::ModuleRegistry;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    prelude::Line,
    style::{Color, Modifier, Style},
    widgets::{Block, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::{io, time::Duration};

/// 主面板
pub struct Panel {
    registry: ModuleRegistry,
    running: bool,
    status_message: String,
    current_module_idx: usize,
}

impl Panel {
    pub fn new() -> Self {
        Self {
            registry: ModuleRegistry::new(),
            running: true,
            status_message: String::new(),
            current_module_idx: 0,
        }
    }
    
    /// 注册模块
    pub fn register_module(&mut self, module: Box<dyn PanelModule>) {
        self.registry.register(module);
    }
    
    /// 运行面板
    pub async fn run(&mut self) -> anyhow::Result<()> {
        // 初始化终端
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        
        // 立即更新所有模块
        for (_, module) in self.registry.modules_mut() {
            let _ = module.update().await;
        }
        
        // 运行主循环
        let result = self.run_loop(&mut terminal).await;
        
        // 恢复终端
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
        let _last_update = std::time::Instant::now();
        // 记录每个模块上次更新的时间
        let mut module_last_update: std::collections::HashMap<String, std::time::Instant> = 
            std::collections::HashMap::new();

        loop {
            // 绘制界面
            terminal.draw(|f| self.ui(f))?;

            // 检查事件
            if event::poll(Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    match key.code {
                        KeyCode::Char('q') => self.running = false,
                        KeyCode::Char('r') => {
                            // 手动刷新所有模块
                            for (_, module) in self.registry.modules_mut() {
                                let _ = module.update().await;
                            }
                            self.status_message = "已刷新所有模块".to_string();
                        }
                        KeyCode::Char('u') => {
                            // 刷新当前选中的模块
                            if let Some((_, module)) = self.registry.modules_mut().nth(self.current_module_idx) {
                                let _ = module.update().await;
                                self.status_message = format!("已刷新：{}", module.name());
                            }
                        }
                        KeyCode::Char(' ') => {
                            // 暂停/恢复当前模块
                            if let Some((_, module)) = self.registry.modules_mut().nth(self.current_module_idx) {
                                module.toggle_pause();
                                let status = if module.is_paused() { "已暂停" } else { "已恢复" };
                                self.status_message = format!("{}: {}", module.name(), status);
                            }
                        }
                        KeyCode::Char('+') | KeyCode::Char('=') => {
                            // 增加刷新间隔
                            if let Some((_, module)) = self.registry.modules_mut().nth(self.current_module_idx) {
                                let current = module.refresh_interval();
                                let new_interval = (current + 10).min(300);
                                module.set_refresh_interval(new_interval);
                                self.status_message = format!("{}: 刷新间隔 {} 秒", module.name(), new_interval);
                            }
                        }
                        KeyCode::Char('-') => {
                            // 减少刷新间隔
                            if let Some((_, module)) = self.registry.modules_mut().nth(self.current_module_idx) {
                                let current = module.refresh_interval();
                                let new_interval = if current <= 5 { 1 } else { current - 5 };
                                module.set_refresh_interval(new_interval);
                                self.status_message = format!("{}: 刷新间隔 {} 秒", module.name(), new_interval);
                            }
                        }
                        KeyCode::Up => {
                            if self.current_module_idx > 0 {
                                self.current_module_idx -= 1;
                                self.status_message = format!("选中：{}", self.get_current_module_name());
                            }
                        }
                        KeyCode::Down => {
                            let len = self.registry.len();
                            if self.current_module_idx < len - 1 {
                                self.current_module_idx += 1;
                                self.status_message = format!("选中：{}", self.get_current_module_name());
                            }
                        }
                        _ => {}
                    }
                }
            }

            // 自动更新模块 - 根据各自的刷新间隔
            let now = std::time::Instant::now();
            for (id, module) in self.registry.modules_mut() {
                let interval = module.refresh_interval();
                
                // 如果间隔为 0，表示暂停，跳过更新
                if interval == 0 {
                    continue;
                }
                
                // 初始化或获取上次更新时间
                let last_time = module_last_update.entry(id.clone()).or_insert(now);
                
                // 检查是否到达更新时间
                if now.duration_since(*last_time).as_secs() >= interval {
                    let _ = module.update().await;
                    *last_time = now;
                }
            }

            if !self.running {
                break;
            }
        }

        Ok(())
    }
    
    fn ui(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Length(3),  // 标题
                Constraint::Min(0),     // 模块内容
                Constraint::Length(3),  // 状态栏
                Constraint::Length(4),  // 帮助
            ])
            .split(f.size());

        // 标题
        let title = Paragraph::new("📊 X-Panel - 模块化面板")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
            .block(Block::default().borders(ratatui::widgets::Borders::ALL));
        f.render_widget(title, chunks[0]);

        // 模块内容
        self.render_modules(f, chunks[1]);

        // 状态栏
        let status = Paragraph::new(format!(
            "{} | 模块数：{} | 空格 - 暂停/恢复 | +/- - 刷新间隔",
            self.status_message,
            self.registry.len()
        ))
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(ratatui::widgets::Borders::ALL));
        f.render_widget(status, chunks[2]);

        // 帮助信息
        let help = List::new(vec![
            ListItem::new(Line::from("↑/↓ - 切换 | 空格 - 暂停 | +/- - 间隔 | r - 刷新全部 | u - 刷新当前 | q - 退出")),
        ])
        .block(Block::default().title("帮助").borders(ratatui::widgets::Borders::ALL));
        f.render_widget(help, chunks[3]);
    }

    fn get_current_module_name(&self) -> String {
        self.registry.modules()
            .nth(self.current_module_idx)
            .map(|(_, m)| m.name().to_string())
            .unwrap_or_else(|| "无".to_string())
    }
    
    fn render_modules(&mut self, f: &mut Frame, area: ratatui::layout::Rect) {
        let modules: Vec<_> = self.registry.modules().collect();
        if modules.is_empty() {
            let empty = Paragraph::new("没有注册任何模块")
                .block(Block::default().title("模块列表").borders(ratatui::widgets::Borders::ALL));
            f.render_widget(empty, area);
            return;
        }

        // 垂直布局显示所有模块
        let constraints: Vec<Constraint> = modules
            .iter()
            .map(|(_, m)| Constraint::Length(m.height() + 2))
            .collect();

        let module_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);

        for (i, (_, module)) in modules.iter().enumerate() {
            if i < module_chunks.len() {
                // 传递选中状态给模块
                let is_selected = i == self.current_module_idx;
                module.render(f, module_chunks[i], is_selected);
            }
        }
    }
}

impl Default for Panel {
    fn default() -> Self {
        Self::new()
    }
}
