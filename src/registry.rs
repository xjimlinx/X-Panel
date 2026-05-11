use crate::module_trait::PanelModule;
use std::collections::HashMap;

struct ModuleEntry {
    module: Box<dyn PanelModule>,
    visible: bool,
}

pub struct ModuleRegistry {
    modules: HashMap<String, ModuleEntry>,
    order: Vec<String>,
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            order: Vec::new(),
        }
    }

    pub fn register(&mut self, module: Box<dyn PanelModule>) {
        let id = module.id().to_string();
        self.order.push(id.clone());
        self.modules.insert(id, ModuleEntry { module, visible: true });
    }

    pub fn modules(&self) -> impl Iterator<Item = (&String, &Box<dyn PanelModule>)> {
        self.order.iter().filter_map(move |id| {
            self.modules.get(id).map(|e| (id, &e.module))
        })
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

    pub fn get_mut(&mut self, idx: usize) -> Option<(String, &mut Box<dyn PanelModule>)> {
        let id = self.order.get(idx)?.clone();
        self.modules.get_mut(&id).map(|e| (id, &mut e.module))
    }

    pub fn get_mut_by_id(&mut self, id: &str) -> Option<(String, &mut Box<dyn PanelModule>)> {
        if self.modules.contains_key(id) {
            self.modules.get_mut(id).map(|e| (id.to_string(), &mut e.module))
        } else {
            None
        }
    }

    pub fn nth_id(&self, n: usize) -> Option<String> {
        self.order.get(n).cloned()
    }

    pub fn swap(&mut self, i: usize, j: usize) {
        if i < self.order.len() && j < self.order.len() {
            self.order.swap(i, j);
        }
    }

    pub fn move_up(&mut self, idx: usize) {
        if idx > 0 && idx < self.order.len() {
            self.order.swap(idx, idx - 1);
        }
    }

    pub fn move_down(&mut self, idx: usize) {
        if idx + 1 < self.order.len() {
            self.order.swap(idx, idx + 1);
        }
    }

    pub fn order(&self) -> &[String] {
        &self.order
    }

    pub fn set_order(&mut self, order: Vec<String>) {
        self.order = order;
    }
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}
