use async_trait::async_trait;
use ratatui::layout::Rect;
use ratatui::Frame;

/// 模块更新结果
#[derive(Debug, Clone)]
pub struct ModuleUpdate {
    /// 模块 ID
    pub id: String,
    /// 是否成功
    pub success: bool,
    /// 错误信息（如果有）
    pub error: Option<String>,
}

/// 模块 trait - 所有面板模块必须实现此 trait
#[async_trait]
pub trait PanelModule: Send + Sync {
    /// 获取模块 ID
    fn id(&self) -> &str;
    
    /// 获取模块名称（显示用）
    fn name(&self) -> &str;
    
    /// 获取模块刷新间隔（秒），0 表示暂停
    fn refresh_interval(&self) -> u64;
    
    /// 设置刷新间隔（秒），0 表示暂停
    fn set_refresh_interval(&mut self, interval: u64);
    
    /// 是否暂停
    fn is_paused(&self) -> bool {
        self.refresh_interval() == 0
    }
    
    /// 切换暂停状态
    fn toggle_pause(&mut self);
    
    /// 更新模块数据
    async fn update(&mut self) -> ModuleUpdate;
    
    /// 渲染模块
    fn render(&self, frame: &mut Frame, area: Rect, is_selected: bool);
    
    /// 获取模块高度（行数）
    fn height(&self) -> u16;
    
    /// 处理错误
    fn get_error(&self) -> Option<&str>;
}
