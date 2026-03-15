pub mod module_trait;
pub mod registry;
pub mod modules;
pub mod panel;

pub use module_trait::{ModuleUpdate, PanelModule};
pub use registry::ModuleRegistry;
pub use panel::Panel;
