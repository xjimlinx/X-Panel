use crate::module_trait::{ModuleUpdate, PanelModule};
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
use serde::Deserialize;

// ==================== DeepSeek 余额模块 ====================

/// DeepSeek API 余额响应
#[derive(Debug, Deserialize)]
struct BalanceResponse {
    #[serde(default)]
    is_available: bool,
    #[serde(default)]
    balance_infos: Vec<BalanceInfo>,
}

#[derive(Debug, Deserialize, Clone)]
struct BalanceInfo {
    #[serde(default)]
    currency: String,
    #[serde(default)]
    total_balance: String,
    #[serde(default)]
    granted_balance: String,
    #[serde(default)]
    topped_up_balance: String,
}

/// DeepSeek 余额模块
pub struct DeepSeekBalanceModule {
    balance: String,
    currency: String,
    granted_balance: String,
    topped_up_balance: String,
    last_update: String,
    error: Option<String>,
    refresh_interval: u64,
    paused: bool,
    api_key: String,
    client: Client,
}

impl DeepSeekBalanceModule {
    pub fn new(api_key: String, refresh_interval: u64) -> Self {
        Self {
            balance: String::new(),
            currency: "CNY".to_string(),
            granted_balance: String::new(),
            topped_up_balance: String::new(),
            last_update: String::new(),
            error: None,
            refresh_interval,
            paused: false,
            api_key,
            client: Client::new(),
        }
    }
    
    async fn fetch_balance(&mut self) {
        let url = "https://api.deepseek.com/user/balance";
        
        match self
            .client
            .get(url)
            .header("Accept", "application/json")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    match response.json::<BalanceResponse>().await {
                        Ok(data) => {
                            if let Some(info) = data.balance_infos.first() {
                                self.balance = info.total_balance.clone();
                                self.currency = info.currency.clone();
                                self.granted_balance = info.granted_balance.clone();
                                self.topped_up_balance = info.topped_up_balance.clone();
                            }
                            self.last_update = Local::now().format("%H:%M:%S").to_string();
                            self.error = None;
                        }
                        Err(e) => {
                            self.error = Some(format!("解析失败：{}", e));
                        }
                    }
                } else {
                    self.error = Some(format!("API 错误：{}", response.status()));
                }
            }
            Err(e) => {
                self.error = Some(format!("网络错误：{}", e));
            }
        }
    }
}

#[async_trait]
impl PanelModule for DeepSeekBalanceModule {
    fn id(&self) -> &str {
        "deepseek_balance"
    }
    
    fn name(&self) -> &str {
        "💰 DeepSeek 余额"
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
        self.fetch_balance().await;
        ModuleUpdate {
            id: self.id().to_string(),
            success: self.error.is_none(),
            error: self.error.clone(),
        }
    }
    
    fn render(&self, frame: &mut Frame, area: Rect, is_selected: bool) {
        let balance_text = if let Some(error) = &self.error {
            Line::from(Span::styled(
                format!("❌ {}", error),
                Style::default().fg(Color::Red),
            ))
        } else {
            Line::from(vec![
                Span::styled("💰 总余额：", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{} {}", self.balance, self.currency),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
        };

        let granted_text = Line::from(vec![
            Span::styled("🎁 赠送余额：", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{} {}", self.granted_balance, self.currency),
                Style::default().fg(Color::Cyan),
            ),
        ]);

        let topped_up_text = Line::from(vec![
            Span::styled("💳 充值余额：", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{} {}", self.topped_up_balance, self.currency),
                Style::default().fg(Color::Magenta),
            ),
        ]);
        
        // 暂停状态标识
        let pause_indicator = if self.paused {
            Line::from(Span::styled("⏸️ 已暂停", Style::default().fg(Color::Yellow)))
        } else {
            Line::from(Span::styled(
                format!("🕐 每{}秒刷新", self.refresh_interval),
                Style::default().fg(Color::DarkGray),
            ))
        };

        let border_style = if is_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let block = Paragraph::new(vec![balance_text, granted_text, topped_up_text, pause_indicator])
            .block(
                Block::default()
                    .title(self.name())
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .style(Style::default().fg(Color::White)),
            )
            .wrap(Wrap { trim: true });

        frame.render_widget(block, area);
    }
    
    fn height(&self) -> u16 {
        7
    }
    
    fn get_error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}
