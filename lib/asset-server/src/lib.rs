use std::any::Any;
use std::path::PathBuf;
use std::sync::{Arc, RwLock, Weak};
use std::thread::JoinHandle;

use uuid::Uuid;

#[derive(Debug)]
pub enum AssetState<T: ?Sized> {
    Queued(PathBuf),
    Loading(JoinHandle<Box<T>>),
    Ready(Box<T>),
}

impl<T: 'static + Send + Sync> AssetState<T> {
    pub fn load(path: impl Into<PathBuf>) -> Self {
        Self::Queued(path.into())
    }

    pub fn generate(func: impl 'static + Send + Sync + FnOnce() -> Box<T>) -> Self {
        Self::Loading(std::thread::spawn(func))
    }
}

impl<T> From<T> for AssetState<T> {
    fn from(value: T) -> Self {
        Self::Ready(Box::new(value))
    }
}

#[derive(Debug)]
struct StoreData<T: ?Sized> {
    id: Uuid,
    filepath: Option<PathBuf>,
    name: Option<String>,
    asset: AssetState<T>,
}

type StorageImpl<T> = RwLock<StoreData<T>>;
type Storage<T> = Arc<StorageImpl<T>>;
type StorageRef<T> = Weak<StorageImpl<T>>;

type ErasedStorage = Storage<dyn Any>;

fn erase<T>(storage: Storage<T>) -> ErasedStorage {
    unsafe { std::mem::transmute(storage) }
}

#[derive(Debug, Clone)]
pub struct Asset<T> {
    value: StorageRef<T>,
}

// impl<T> Asset<T> {
//     pub fn get_value(&self) -> Option<&T> {
//         self.value.upgrade().and_then(|val| val.read().ok()).and_then(|mut val| match &mut val {
//             AssetState::Ready(value) => Some(value),
//             _ => None,
//         }))
//     }
// }

pub struct AssetServer {
    storage: Vec<ErasedStorage>,
}

impl AssetServer {
    pub fn new() -> Self {
        Self { storage: vec![] }
    }

    pub fn insert<T: 'static + Any>(&mut self, value: T) -> Asset<T> {
        let stored = Arc::new(RwLock::new(StoreData {
            asset: AssetState::Ready(Box::new(value)),
            id: Uuid::new_v4(),
            name: None,
            filepath: None,
        }));
        let ret = Asset {
            value: Arc::downgrade(&stored),
        };
        self.storage.push(erase(stored));
        ret
    }
}
