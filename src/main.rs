use x_panel::modules::{DeepSeekBalanceModule, SystemInfoModule, NetworkMonitorModule, SystemTempModule, ClockModule};
use x_panel::{Config, Panel};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    // 读取配置文件（作为环境变量的后备）
    let config = Config::load();
    let api_key = std::env::var("DEEPSEEK_API_KEY")
        .unwrap_or_else(|_| config.deepseek_api_key.clone());

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
        panel.log_info("DeepSeek 余额模块已注册");
        println!("✅ 已注册 DeepSeek 余额模块");
    } else {
        panel.log_info("未设置 DEEPSEEK_API_KEY，跳过 DeepSeek 余额模块");
        println!("⚠️  未设置 DEEPSEEK_API_KEY，跳过 DeepSeek 余额模块");
    }

    // 注册系统信息模块
    panel.register_module(Box::new(SystemInfoModule::new(30)));
    panel.log_info("系统信息模块已注册");
    println!("✅ 已注册 系统信息模块");

    // 注册网络监控模块
    panel.register_module(Box::new(NetworkMonitorModule::new(5)));
    panel.log_info("网络监控模块已注册");
    println!("✅ 已注册 网络监控模块");

    // 注册系统温度模块
    panel.register_module(Box::new(SystemTempModule::new(10)));
    panel.log_info("系统温度模块已注册");
    println!("✅ 已注册 系统温度模块");

    // 注册时钟模块
    panel.register_module(Box::new(ClockModule::new(1)));
    panel.log_info("时钟日历模块已注册");
    println!("✅ 已注册 时钟日历模块");

    // 应用已保存的配置
    panel.apply_config();

    panel.log_info(&format!("启动完成，共 {} 个模块", panel.module_count()));
    println!("\n🚀 启动面板... 按 'q' 退出\n");

    // 运行面板
    let result = panel.run().await;

    result?;

    Ok(())
}
