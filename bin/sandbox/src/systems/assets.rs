use std::path::Path;
use assets_manager::AssetCache;
use eyre::Result;

pub struct AssetSystem {
    pub assets: &'static AssetCache,
}

impl AssetSystem {
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self> {
        let assets = Box::leak(Box::new(AssetCache::new(base_dir)?));
        assets.enhance_hot_reloading();
        Ok(Self {
            assets
        })
    }
    
    pub fn on_frame(&self) {
        self.assets.hot_reload();
    }
}