use std::collections::HashMap;
use std::sync::Arc;

use crate::tool::Tool;

pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self { tools: HashMap::new() }
    }

    pub fn register(&mut self, tool: impl Tool + 'static) {
        self.tools.insert(tool.name().to_string(), Arc::new(tool));
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    pub fn names(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    pub fn schemas(&self) -> Vec<serde_json::Value> {
        self.tools.values().map(|t| serde_json::json!({
            "name": t.name(),
            "description": t.description(),
            "parameters": t.parameters_schema(),
        })).collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self { Self::new() }
}
