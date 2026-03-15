use crate::module_trait::PanelModule;
use crate::registry::ModuleRegistry;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    prelude::Line,
    style::{Color, Modifier, Style},
    widgets::{Block, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::{collections::HashMap, io, time::Duration};

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
        }
    }

    /// 注册模块
    pub fn register_module(&mut self, module: Box<dyn PanelModule>) {
        self.registry.register(module);
    }

    /// 运行面板
    pub async fn run(&mut self) -> anyhow::Result<()> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // 立即更新所有模块
        for (_, module) in self.registry.modules_mut() {
            let _ = module.update().await;
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
                    match key.code {
                        KeyCode::Char('q') => self.running = false,
                        KeyCode::Char('r') => {
                            for (_, module) in self.registry.modules_mut() {
                                let _ = module.update().await;
                            }
                            self.status_message = "已刷新所有模块".to_string();
                        }
                        KeyCode::Char('u') => {
                            if let Some((_, module)) = self.registry.modules_mut().nth(self.current_module_idx) {
                                let _ = module.update().await;
                                self.status_message = format!("已刷新：{}", module.name());
                            }
                        }
                        KeyCode::Char(' ') => {
                            if let Some((_, module)) = self.registry.modules_mut().nth(self.current_module_idx) {
                                module.toggle_pause();
                                let status = if module.is_paused() { "已暂停" } else { "已恢复" };
                                self.status_message = format!("{}: {}", module.name(), status);
                            }
                        }
                        KeyCode::Char('l') | KeyCode::Char('L') => {
                            // 切换布局模式
                            self.layout_mode = self.layout_mode.next();
                            self.adjust_column_weights();
                            self.status_message = format!("布局：{:?} 列", self.layout_mode.columns());
                        }
                        KeyCode::Char('[') => {
                            // 减小当前列宽度
                            self.adjust_column_width(self.current_module_idx, -1);
                        }
                        KeyCode::Char(']') => {
                            // 增大当前列宽度
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

            // 自动更新模块
            let now = std::time::Instant::now();
            for (id, module) in self.registry.modules_mut() {
                let interval = module.refresh_interval();
                if interval == 0 { continue; }
                
                let last_time = module_last_update.entry(id.clone()).or_insert(now);
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
            .block(Block::default().borders(ratatui::widgets::Borders::ALL));
        f.render_widget(title, main_chunks[0]);

        // 模块内容 - 多列布局
        self.render_modules(f, main_chunks[1]);

        // 状态栏
        let status = Paragraph::new(format!(
            "{} | 模块：{}/{} | 布局：{}列 | 空格 - 暂停 | l - 切换布局 | [/] - 列宽",
            self.status_message,
            self.current_module_idx + 1,
            self.registry.len(),
            self.layout_mode.columns()
        ))
        .style(Style::default().fg(Color::Gray))
        .block(Block::default().borders(ratatui::widgets::Borders::ALL));
        f.render_widget(status, main_chunks[2]);

        // 帮助信息
        let help = List::new(vec![
            ListItem::new(Line::from("↑/↓ - 切换模块 | 空格 - 暂停 | +/- - 刷新间隔")),
            ListItem::new(Line::from("l - 切换布局 (1/2/3 列) | [/] - 调整列宽 | r - 刷新全部 | u - 刷新当前")),
            ListItem::new(Line::from("q - 退出")),
        ])
        .block(Block::default().title("帮助").borders(ratatui::widgets::Borders::ALL));
        f.render_widget(help, main_chunks[3]);
    }

    fn render_modules(&mut self, f: &mut Frame, area: Rect) {
        let modules: Vec<_> = self.registry.modules().collect();
        if modules.is_empty() {
            let empty = Paragraph::new("没有注册任何模块")
                .block(Block::default().title("模块列表").borders(ratatui::widgets::Borders::ALL));
            f.render_widget(empty, area);
            return;
        }

        let columns = self.layout_mode.columns();
        
        // 根据列数分割模块
        let modules_per_column = (modules.len() + columns - 1) / columns;
        
        // 创建列布局
        let column_constraints: Vec<Constraint> = self.column_weights.iter()
            .take(columns)
            .map(|&w| Constraint::Percentage((w * 100 / self.column_weights.iter().take(columns).sum::<u16>()) as u16))
            .collect();
        
        let columns_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(column_constraints)
            .split(area);
        
        // 渲染每一列
        for (col_idx, column_area) in columns_layout.iter().enumerate() {
            let start_idx = col_idx * modules_per_column;
            let end_idx = std::cmp::min(start_idx + modules_per_column, modules.len());
            
            if start_idx >= modules.len() { continue; }
            
            // 计算这一列的总高度
            let column_height: u16 = modules[start_idx..end_idx]
                .iter()
                .map(|(_, m)| m.height() + 2)
                .sum();
            
            // 如果内容超过列高，需要滚动
            let constraints: Vec<Constraint> = modules[start_idx..end_idx]
                .iter()
                .map(|(_, m)| Constraint::Length(m.height() + 2))
                .collect();
            
            let column_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(constraints)
                .split(*column_area);
            
            // 渲染该列的每个模块
            for (i, module_idx) in (start_idx..end_idx).enumerate() {
                if i < column_layout.len() {
                    let (_, module) = &modules[module_idx];
                    let is_selected = module_idx == self.current_module_idx;
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
