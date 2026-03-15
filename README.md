# X-Panel

模块化终端面板框架 - 基于 Rust + TUI

![Version](https://img.shields.io/badge/version-0.1.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)

## 功能特性

- 🧩 **模块化设计**: 基于 trait 的模块系统，易于扩展
- 📊 **多模块支持**: 同时显示多个信息模块
- 🔄 **自动刷新**: 每个模块独立配置刷新间隔
- ⏸️ **暂停控制**: 支持暂停/恢复模块更新
- 📱 **系统监控**: CPU、内存、磁盘、GPU、电池、功耗
- 💰 **API 余额**: DeepSeek API 余额实时查询
- 🔔 **桌面通知**: 任务完成自动通知
- ⌨️ **快捷键**: 丰富的键盘交互
- 🎨 **美观 UI**: 基于 ratatui 的现代化终端界面

## 项目结构

```
panel-framework/
├── Cargo.toml              # 项目配置
├── src/
│   ├── main.rs             # 主程序入口
│   ├── lib.rs              # 库导出
│   ├── module_trait.rs     # 模块 trait 定义
│   ├── registry.rs         # 模块注册表
│   ├── panel.rs            # 主面板逻辑
│   └── modules/            # 模块实现
│       ├── mod.rs
│       ├── deepseek_balance.rs  # DeepSeek 余额模块
│       └── system_info.rs       # 系统信息模块
├── notify.py               # 桌面通知脚本
├── .env.example            # 环境变量模板
├── README.md               # 使用文档
└── DESIGN.md               # 设计文档
```

## 快速开始

### 1. 配置环境变量

```bash
cp .env.example .env
# 编辑 .env 填入 DEEPSEEK_API_KEY
```

### 2. 编译运行

```bash
# 开发模式
cargo run

# 发布模式
cargo build --release
./target/release/panel-framework
```

## 快捷键

| 按键 | 功能 |
|------|------|
| `q` | 退出程序 |
| `r` | 刷新所有模块 |
| `u` | 刷新当前选中模块 |
| **空格** | 暂停/恢复当前模块 |
| **+** 或 **=** | 增加刷新间隔 (+10 秒) |
| **-** | 减少刷新间隔 (-5 秒，最小 1 秒) |
| `↑` | 上一个模块 |
| `↓` | 下一个模块 |

## 内置模块

### DeepSeek 余额模块 (`deepseek_balance`)

显示 DeepSeek API 账户余额信息。

**显示内容**:
- 💰 总余额
- 🎁 赠送余额
- 💳 充值余额

**配置项**:
- `DEEPSEEK_API_KEY`: API 密钥
- `REFRESH_INTERVAL`: 刷新间隔（秒）

### 系统信息模块 (`system_info`)

显示系统硬件状态和功耗信息。

**显示内容**:
- CPU 使用率
- 内存使用量和详情
- 磁盘使用率
- GPU 信息
- 电池状态（电量、充电状态、电源连接）
- 实时功耗（GPU 功率）

**刷新间隔**: 1-300 秒（可按 +/- 调整）

## 创建自定义模块

1. 在 `src/modules/` 目录创建新文件

```rust
use crate::module_trait::{ModuleUpdate, PanelModule};
use async_trait::async_trait;
use ratatui::{layout::Rect, widgets::Paragraph, Frame};

pub struct MyModule {
    data: String,
    refresh_interval: u64,
}

impl MyModule {
    pub fn new() -> Self {
        Self {
            data: String::new(),
            refresh_interval: 60,
        }
    }
}

#[async_trait]
impl PanelModule for MyModule {
    fn id(&self) -> &str {
        "my_module"
    }
    
    fn name(&self) -> &str {
        "我的模块"
    }
    
    fn refresh_interval(&self) -> u64 {
        self.refresh_interval
    }
    
    async fn update(&mut self) -> ModuleUpdate {
        // 更新数据逻辑
        ModuleUpdate {
            id: self.id().to_string(),
            success: true,
            error: None,
        }
    }
    
    fn render(&self, frame: &mut Frame, area: Rect) {
        let widget = Paragraph::new(&self.data);
        frame.render_widget(widget, area);
    }
    
    fn height(&self) -> u16 {
        5
    }
    
    fn get_error(&self) -> Option<&str> {
        None
    }
}
```

2. 在 `src/modules/mod.rs` 中导出

```rust
pub mod my_module;
pub use my_module::MyModule;
```

3. 在 `main.rs` 中注册

```rust
panel.register_module(Box::new(MyModule::new()));
```

## 架构设计

```
┌─────────────────────────────────────────────────────────┐
│                      Panel (主面板)                      │
│  ┌─────────────────────────────────────────────────┐   │
│  │              ModuleRegistry (注册表)             │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────┐ │   │
│  │  │ Module 1    │  │ Module 2    │  │ ModuleN │ │   │
│  │  │ DeepSeek    │  │ System      │  │ ...     │ │   │
│  │  └─────────────┘  └─────────────┘  └─────────┘ │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
                      │
                      │ 实现
                      ▼
            ┌─────────────────────┐
            │   PanelModule Trait │
            ├─────────────────────┤
            │ + id()              │
            │ + name()            │
            │ + refresh_interval()│
            │ + update()          │
            │ + render()          │
            │ + height()          │
            │ + get_error()       │
            └─────────────────────┘
```

## 依赖

- `tokio`: 异步运行时
- `reqwest`: HTTP 客户端
- `ratatui`: 终端 UI
- `crossterm`: 终端操作
- `serde/serde_json`: JSON 处理
- `async-trait`: 异步 trait 支持

## 许可证

MIT
