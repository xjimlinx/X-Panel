use crate::module_trait::PanelModule;
use std::collections::HashMap;

struct ModuleEntry {
    module: Box<dyn PanelModule>,
    visible: bool,
}

pub struct ModuleRegistry {
    modules: HashMap<String, ModuleEntry>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
        }
    }

    pub fn register(&mut self, module: Box<dyn PanelModule>) {
        let id = module.id().to_string();
        self.modules.insert(id, ModuleEntry { module, visible: true });
    }

    pub fn modules(&self) -> impl Iterator<Item = (&String, &Box<dyn PanelModule>)> {
        self.modules.iter().map(|(id, e)| (id, &e.module))
    }

    pub fn modules_mut(&mut self) -> impl Iterator<Item = (&String, &mut Box<dyn PanelModule>)> {
        self.modules.iter_mut().map(|(id, e)| (id, &mut e.module))
    }

    pub fn len(&self) -> usize {
        self.modules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    pub fn is_visible(&self, id: &str) -> bool {
        self.modules.get(id).map(|e| e.visible).unwrap_or(true)
    }

    pub fn set_visible(&mut self, id: &str, visible: bool) {
        if let Some(e) = self.modules.get_mut(id) {
            e.visible = visible;
        }
    }

    pub fn visible_count(&self) -> usize {
        self.modules.values().filter(|e| e.visible).count()
    }

    pub fn nth_visible(&self, n: usize) -> Option<(&String, &Box<dyn PanelModule>)> {
        self.modules.iter().filter(|(_, e)| e.visible).nth(n).map(|(id, e)| (id, &e.module))
    }

    pub fn nth_id(&self, n: usize) -> Option<String> {
        self.modules.iter().nth(n).map(|(id, _)| id.clone())
    }

    pub fn visible_indices(&self) -> Vec<usize> {
        self.modules.iter().enumerate()
            .filter(|(_, (_, e))| e.visible)
            .map(|(i, _)| i)
            .collect()
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}
