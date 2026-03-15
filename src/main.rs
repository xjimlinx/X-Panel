use x_panel::{Panel};
use x_panel::modules::{DeepSeekBalanceModule, SystemInfoModule, NetworkMonitorModule, SystemTempModule, ClockModule};
use std::process::Command;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 加载环境变量
    dotenvy::dotenv().ok();

    // 初始化日志
    env_logger::init();

    // 获取配置
    let api_key = std::env::var("DEEPSEEK_API_KEY")
        .unwrap_or_else(|_| String::new());

    let refresh_interval: u64 = std::env::var("REFRESH_INTERVAL")
        .unwrap_or_else(|_| "60".to_string())
        .parse()
        .unwrap_or(60);

    // 创建面板
    let mut panel = Panel::new();

    // 注册模块
    if !api_key.is_empty() {
        panel.register_module(Box::new(DeepSeekBalanceModule::new(
            api_key,
            refresh_interval,
        )));
        println!("✅ 已注册 DeepSeek 余额模块");
    } else {
        println!("⚠️  未设置 DEEPSEEK_API_KEY，跳过 DeepSeek 余额模块");
    }

    // 注册系统信息模块
    panel.register_module(Box::new(SystemInfoModule::new(30)));
    println!("✅ 已注册 系统信息模块");

    // 注册网络监控模块
    panel.register_module(Box::new(NetworkMonitorModule::new(5)));
    println!("✅ 已注册 网络监控模块");

    // 注册系统温度模块
    panel.register_module(Box::new(SystemTempModule::new(10)));
    println!("✅ 已注册 系统温度模块");

    // 注册时钟模块
    panel.register_module(Box::new(ClockModule::new(1)));
    println!("✅ 已注册 时钟日历模块");

    println!("\n🚀 启动面板... 按 'q' 退出\n");

    // 运行面板
    let result = panel.run().await;

    // 程序退出时发送通知
    // send_task_notification("X-Panel");

    result?;

    Ok(())
}

/// 发送任务完成通知
fn send_task_notification(task_name: &str) {
    let script_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("notify.py");

    if script_path.exists() {
        let _ = Command::new("python3")
            .arg(&script_path)
            .arg("task")
            .arg("--name")
            .arg(task_name)
            .output();
    }
}
