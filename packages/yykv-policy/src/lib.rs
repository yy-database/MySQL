pub struct StoragePolicy {
    // 负责介质识别与页面策略定义
}

impl StoragePolicy {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for StoragePolicy {
    fn default() -> Self {
        Self::new()
    }
}
