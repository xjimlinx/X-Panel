# X-Panel 设计文档

## 1. 项目概述

### 1.1 项目名称
X-Panel（模块化终端面板框架）

### 1.2 版本
v0.1.0

### 1.3 项目目标
创建一个基于 Rust + TUI 的模块化终端面板框架，支持：
- 多模块并行显示
- 独立刷新间隔控制
- 暂停/恢复功能
- 易于扩展新模块
- 桌面通知集成

---

## 2. 架构设计

### 2.1 技术栈
- **语言**: Rust 2021 Edition
- **异步运行时**: tokio
- **终端 UI**: ratatui + crossterm
- **HTTP 客户端**: reqwest
- **配置管理**: dotenvy

### 2.2 项目结构

```
panel-framework/
├── Cargo.toml              # 项目配置和依赖
├── src/
│   ├── main.rs             # 程序入口，初始化配置
│   ├── lib.rs              # 库导出
│   ├── module_trait.rs     # PanelModule trait 定义
│   ├── registry.rs         # 模块注册表管理
│   ├── panel.rs            # 主面板逻辑和事件循环
│   └── modules/            # 模块实现目录
│       ├── mod.rs          # 模块导出
│       ├── deepseek_balance.rs  # DeepSeek 余额模块
│       └── system_info.rs       # 系统信息模块
├── notify.py               # 桌面通知脚本（Python）
├── .env.example            # 环境变量模板
├── README.md               # 使用文档
└── DESIGN.md               # 设计文档
```

### 2.3 核心组件关系图

```
┌─────────────────────────────────────────────────────────┐
│                      main.rs                            │
│  - 加载环境变量                                          │
│  - 创建 Panel 实例                                       │
│  - 注册模块                                              │
│  - 运行主循环                                            │
└─────────────────────────────────────────────────────────┘
                            │
                            ▼
┌─────────────────────────────────────────────────────────┐
│                       Panel                             │
│  ┌─────────────────────────────────────────────────┐   │
│  │              ModuleRegistry                      │   │
│  │  - 存储所有模块                                   │   │
│  │  - 按 ID 索引                                     │   │
│  └─────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────┐   │
│  │              Event Loop                          │   │
│  │  - 键盘事件处理                                   │   │
│  │  - 自动刷新调度                                   │   │
│  │  - UI 渲染                                        │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
                            │
                            │ 实现
                            ▼
┌─────────────────────────────────────────────────────────┐
│              PanelModule (Trait)                        │
│  + id() -> &str                                         │
│  + name() -> &str                                       │
│  + refresh_interval() -> u64                            │
│  + set_refresh_interval(interval: u64)                  │
│  + toggle_pause()                                       │
│  + is_paused() -> bool                                  │
│  + update() -> ModuleUpdate                             │
│  + render(frame, area, is_selected)                     │
│  + height() -> u16                                      │
│  + get_error() -> Option<&str>                          │
└─────────────────────────────────────────────────────────┘
```

---

## 3. 模块设计

### 3.1 PanelModule Trait

所有模块必须实现的核心接口：

```rust
#[async_trait]
pub trait PanelModule: Send + Sync {
    // 基本信息
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    
    // 刷新控制
    fn refresh_interval(&self) -> u64;
    fn set_refresh_interval(&mut self, interval: u64);
    fn toggle_pause(&mut self);
    fn is_paused(&self) -> bool { self.refresh_interval() == 0 }
    
    // 数据更新
    async fn update(&mut self) -> ModuleUpdate;
    
    // UI 渲染
    fn render(&self, frame: &mut Frame, area: Rect, is_selected: bool);
    fn height(&self) -> u16;
    
    // 错误处理
    fn get_error(&self) -> Option<&str>;
}
```

### 3.2 模块状态管理

每个模块维护自己的状态：
- **数据**: 从 API 或系统读取的信息
- **刷新间隔**: 用户可配置的更新频率
- **暂停状态**: 是否暂停自动更新
- **错误信息**: 更新失败时的错误提示

### 3.3 DeepSeek 余额模块

```rust
pub struct DeepSeekBalanceModule {
    // 余额数据
    balance: String,
    currency: String,
    granted_balance: String,
    topped_up_balance: String,
    
    // 状态
    last_update: String,
    error: Option<String>,
    
    // 配置
    refresh_interval: u64,
    paused: bool,
    api_key: String,
    client: Client,
}
```

**API 端点**: `https://api.deepseek.com/user/balance`

**响应格式**:
```json
{
  "is_available": true,
  "balance_infos": [{
    "currency": "CNY",
    "total_balance": "7.38",
    "granted_balance": "0.00",
    "topped_up_balance": "7.38"
  }]
}
```

### 3.4 系统信息模块

```rust
pub struct SystemInfoModule {
    // 系统数据
    cpu_usage: String,
    memory_usage: String,
    memory_detail: String,
    disk_usage: String,
    gpu_info: String,
    battery_info: String,
    power_usage: String,
    
    // 状态
    last_update: String,
    
    // 配置
    refresh_interval: u64,
    paused: bool,
    
    // CPU 计算辅助
    prev_cpu_stats: Option<CpuStats>,
    prev_time: Option<Instant>,
}
```

**数据来源**:
| 信息 | 来源 |
|------|------|
| CPU 使用率 | `/proc/stat` |
| 内存使用 | `/proc/meminfo` |
| 磁盘使用 | `df /` 命令 |
| GPU 信息 | `lspci`, `/sys/class/drm/` |
| 电池状态 | `/sys/class/power_supply/BAT0/` |
| 功耗 | `/sys/class/hwmon/*/power1_input` |

---

## 4. 刷新调度机制

### 4.1 独立刷新间隔

每个模块有自己的刷新间隔，互不影响：

```rust
// 为每个模块记录上次更新时间
let mut module_last_update: HashMap<String, Instant> = HashMap::new();

// 主循环中检查每个模块
for (id, module) in registry.modules_mut() {
    let interval = module.refresh_interval();
    
    // 间隔为 0 表示暂停
    if interval == 0 { continue; }
    
    // 检查是否到达更新时间
    let last_time = module_last_update.entry(id.clone()).or_insert(now);
    if now.duration_since(*last_time).as_secs() >= interval {
        module.update().await;
        *last_time = now;
    }
}
```

### 4.2 暂停功能

- 按 **空格** 切换暂停状态
- 暂停时 `refresh_interval()` 返回 0
- 暂停的模块不会被调度更新
- UI 显示 `⏸️ 已暂停` 标识

### 4.3 刷新间隔调整

- **增加**: 按 `+` 或 `=`，+10 秒，最大 300 秒
- **减少**: 按 `-`，>5 秒时 -5 秒，≤5 秒时到 1 秒
- 调整后立即生效

---

## 5. UI 设计

### 5.1 布局结构

```
┌─────────────────────────────────────────┐
│ 标题栏 (3 行)                            │
├─────────────────────────────────────────┤
│                                         │
│  模块 1 (动态高度)                       │
│                                         │
├─────────────────────────────────────────┤
│  模块 2 (动态高度)                       │
│                                         │
├─────────────────────────────────────────┤
│ 状态栏 (3 行)                            │
├─────────────────────────────────────────┤
│ 帮助信息 (4 行)                          │
└─────────────────────────────────────────┘
```

### 5.2 选中高亮

- 当前选中的模块：**黄色粗边框**
- 未选中的模块：**灰色边框**

### 5.3 状态指示

- 暂停：`⏸️ 已暂停`（黄色）
- 运行：`🕐 每 X 秒刷新`（灰色）

---

## 6. 通知系统

### 6.1 通知脚本 (notify.py)

支持两种通知方式：
1. **GNOME 桌面通知**: 使用 `notify-send`
2. **TCP 网络通知**: 同一网络内推送

### 6.2 使用方式

```bash
# 任务完成通知
python3 notify.py task --name "任务名"

# 输出格式
# X-Panel: 任务{任务名}已完成，请查阅。
```

### 6.3 Rust 集成

```rust
fn send_task_notification(task_name: &str) {
    let script_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("notify.py");
    let _ = Command::new("python3")
        .arg(&script_path)
        .arg("task")
        .arg("--name")
        .arg(task_name)
        .output();
}
```

---

## 7. 快捷键设计

| 按键 | 功能 | 作用域 |
|------|------|--------|
| `q` | 退出程序 | 全局 |
| `r` | 刷新所有模块 | 全局 |
| `u` | 刷新当前模块 | 当前选中 |
| `空格` | 暂停/恢复 | 当前选中 |
| `+` / `=` | 增加间隔 | 当前选中 |
| `-` | 减少间隔 | 当前选中 |
| `↑` | 上一个模块 | 全局 |
| `↓` | 下一个模块 | 全局 |

---

## 8. 扩展指南

### 8.1 创建新模块

1. 在 `src/modules/` 创建新文件
2. 实现 `PanelModule` trait
3. 在 `mod.rs` 中导出
4. 在 `main.rs` 中注册

### 8.2 模块模板

```rust
use crate::module_trait::{ModuleUpdate, PanelModule};
use async_trait::async_trait;
use ratatui::{layout::Rect, prelude::*, widgets::*, Frame};

pub struct MyModule {
    data: String,
    refresh_interval: u64,
    paused: bool,
}

impl MyModule {
    pub fn new() -> Self {
        Self {
            data: String::new(),
            refresh_interval: 60,
            paused: false,
        }
    }
}

#[async_trait]
impl PanelModule for MyModule {
    fn id(&self) -> &str { "my_module" }
    fn name(&self) -> &str { "我的模块" }
    
    fn refresh_interval(&self) -> u64 {
        if self.paused { 0 } else { self.refresh_interval }
    }
    
    fn set_refresh_interval(&mut self, interval: u64) {
        self.refresh_interval = interval;
        self.paused = interval == 0;
    }
    
    fn toggle_pause(&mut self) { self.paused = !self.paused; }
    
    async fn update(&mut self) -> ModuleUpdate {
        // 更新数据逻辑
        ModuleUpdate {
            id: self.id().to_string(),
            success: true,
            error: None,
        }
    }
    
    fn render(&self, frame: &mut Frame, area: Rect, is_selected: bool) {
        // 渲染逻辑
    }
    
    fn height(&self) -> u16 { 5 }
    fn get_error(&self) -> Option<&str> { None }
}
```

---

## 9. 版本历史

| 版本 | 日期 | 变更 |
|------|------|------|
| 0.1.0 | 2026-03-15 | 初始版本，包含 DeepSeek 余额和系统信息模块 |

---

## 10. 待办事项

- [ ] 添加更多系统监控模块（网络、温度）
- [ ] 支持配置文件（TOML/YAML）
- [ ] 添加 Web API 模块
- [ ] 支持模块排序配置
- [ ] 添加日志系统
- [ ] 支持主题切换
