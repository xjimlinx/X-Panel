pub mod clock;
pub mod deepseek_balance;
pub mod network_monitor;
pub mod system_info;
pub mod system_temp;
pub mod web_api;

pub use clock::ClockModule;
pub use deepseek_balance::DeepSeekBalanceModule;
pub use network_monitor::NetworkMonitorModule;
pub use system_info::SystemInfoModule;
pub use system_temp::SystemTempModule;
pub use web_api::WebApiModule;
