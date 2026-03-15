use crate::module_trait::PanelModule;
use std::collections::HashMap;

/// 模块注册表
pub struct ModuleRegistry {
    modules: HashMap<String, Box<dyn PanelModule>>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }
    
    /// 注册模块
    pub fn register(&mut self, module: Box<dyn PanelModule>) {
        let id = module.id().to_string();
        self.modules.insert(id, module);
    }
    
    /// 获取所有模块
    pub fn modules(&self) -> impl Iterator<Item = (&String, &Box<dyn PanelModule>)> {
        self.modules.iter()
    }
    
    /// 获取可变引用（用于更新）
    pub fn modules_mut(&mut self) -> impl Iterator<Item = (&String, &mut Box<dyn PanelModule>)> {
        self.modules.iter_mut()
    }
    
    /// 获取模块数量
    pub fn len(&self) -> usize {
        self.modules.len()
    }
    
    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}
