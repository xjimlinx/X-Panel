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
use std::time::Duration;
use crate::theme::Theme;

// ==================== DeepSeek 余额模块 ====================

/// DeepSeek API 余额响应
#[derive(Debug, Deserialize)]
struct BalanceResponse {
    #[serde(default)]
    #[allow(dead_code)]
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
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("创建 HTTP 客户端失败"),
        }
    }
    
    async fn fetch_balance(&mut self) {
        let url = "https://api.deepseek.com/user/balance";
        
        for attempt in 0..2 {
            let result = self
                .client
                .get(url)
                .header("Accept", "application/json")
                .header("Authorization", format!("Bearer {}", self.api_key))
                .send()
                .await;

            match result {
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
                                return;
                            }
                            Err(e) => {
                                self.error = Some(format!("解析失败：{}", e));
                            }
                        }
                    } else {
                        self.error = Some(format!("API 错误：{}", response.status()));
                        // 429 限流时重试一次
                        if response.status().as_u16() == 429 && attempt == 0 {
                            tokio::time::sleep(Duration::from_secs(2)).await;
                            continue;
                        }
                    }
                }
                Err(e) => {
                    self.error = Some(format!("网络错误：{}", e));
                    // 超时等网络错误重试一次
                    if attempt == 0 {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        continue;
                    }
                }
            }
            break;
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
    
    fn render(&self, frame: &mut Frame, area: Rect, is_selected: bool, theme: &Theme) {
        let show_data = !self.balance.is_empty();
        let balance_text = if show_data {
            Line::from(vec![
                Span::styled("💰 总余额：", Style::default().fg(Color::Yellow)),
                Span::styled(
                    format!("{} {}", self.balance, self.currency),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
            ])
        } else if let Some(error) = &self.error {
            Line::from(Span::styled(
                format!("❌ {}", error),
                Style::default().fg(Color::Red),
            ))
        } else {
            Line::from(Span::styled("⏳ 加载中...", Style::default().fg(Color::DarkGray)))
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
            Style::default().fg(theme.border_selected).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.border_default)
        };

        let error_line = if show_data {
            if let Some(error) = &self.error {
                Some(Line::from(Span::styled(
                    format!("⚠️ {}", error),
                    Style::default().fg(Color::Red),
                )))
            } else {
                None
            }
        } else {
            None
        };

        let mut lines = vec![balance_text, granted_text, topped_up_text];
        if let Some(e) = error_line {
            lines.push(e);
        }
        lines.push(pause_indicator);

        let block = Paragraph::new(lines)
            .block(
                Block::default()
                    .title(self.name())
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .style(Style::default().fg(theme.text).bg(theme.bg)),
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
