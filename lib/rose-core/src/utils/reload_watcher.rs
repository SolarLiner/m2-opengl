use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread::{self},
};
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "hot-reload")]
use notify::{EventKind, recommended_watcher, RecursiveMode, Watcher};

#[derive(Debug)]
pub struct ReloadWatcher {
    base_path: PathBuf,
    #[cfg(feature = "hot-reload")]
    to_reload: Arc<Mutex<HashSet<PathBuf>>>,
    #[cfg(feature = "hot-reload")]
    cancel_thread: Arc<AtomicBool>,
}

#[cfg(feature = "hot-reload")]
impl Drop for ReloadWatcher {
    fn drop(&mut self) {
        self.cancel_thread.store(true, Ordering::Relaxed);
    }
}

impl ReloadWatcher {
    #[cfg(feature = "hot-reload")]
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        let base_path = base_path.into();
        let to_reload = Arc::new(Mutex::new(HashSet::new()));
        let cancel_thread = Arc::new(AtomicBool::new(false));
        thread::spawn({
            let base_path = base_path.clone();
            let to_reload = to_reload.clone();
            let cancel_thread = cancel_thread.clone();
            move || {
                let (tx, rx) = crossbeam_channel::unbounded();
                let mut watcher = recommended_watcher(tx).unwrap();
                watcher
                    .watch(&base_path, RecursiveMode::Recursive)
                    .unwrap();
                tracing::info!("Watching {}", base_path.display());
                for res in rx {
                    if cancel_thread.load(Ordering::Relaxed) {
                        break;
                    }

                    match res {
                        Ok(event) => {
                            let mut set = to_reload.lock().unwrap();
                            if let EventKind::Modify(..) = event.kind {
                                set.extend(
                                    event
                                        .paths
                                        .into_iter()
                                        .map(|path| {
                                            if !path.is_absolute() {
                                                base_path.join(path)
                                            } else {
                                                path
                                            }
                                        })
                                        .inspect(|path| {
                                            tracing::debug!("Modified: {}", path.display())
                                        }),
                                );
                            }
                        }
                        Err(err) => {
                            tracing::warn!("Could not notify events: {}", err);
                        }
                    }
                    thread::yield_now();
                }
            }
        });

        Self {
            base_path,
            to_reload,
            cancel_thread,
        }
    }

    #[cfg(not(feature = "hot-reload"))]
    pub fn new(base_path: impl AsRef<Path>) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
        }
    }

    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    #[cfg(feature = "hot-reload")]
    pub fn should_reload(&self, path: impl AsRef<Path>) -> bool {
        self.to_reload
            .lock()
            .unwrap()
            .remove(&self.base_path.join(path.as_ref()))
    }

    #[cfg(not(feature = "hot-reload"))]
    #[inline(always)]
    pub fn should_reload(&self, _path: impl AsRef<Path>) -> bool {
        false
    }

    pub fn proxy<'p>(&self, files: impl IntoIterator<Item=&'p Path>) -> ReloadFileProxy {
        ReloadFileProxy::from_watcher(self, files)
    }

    pub fn proxy_single(&self, file: impl AsRef<Path>) -> ReloadFileProxy {
        self.proxy([file.as_ref()])
    }
}

#[derive(Debug)]
pub struct ReloadFileProxy {
    files: Vec<PathBuf>,
    #[cfg(feature = "hot-reload")]
    to_reload: Arc<Mutex<HashSet<PathBuf>>>,
}

impl ReloadFileProxy {
    pub fn from_watcher<'p>(
        watcher: &ReloadWatcher,
        paths: impl IntoIterator<Item=&'p Path>,
    ) -> Self {
        Self {
            files: Vec::from_iter(paths.into_iter().map(|path| watcher.base_path.join(path))),
            #[cfg(feature = "hot-reload")]
            to_reload: watcher.to_reload.clone(),
        }
    }

    #[cfg(feature = "hot-reload")]
    pub fn should_reload(&self) -> bool {
        let mut to_reload = self.to_reload.lock().unwrap();
        let mut reload = false;
        for path in &self.files {
            reload |= to_reload.remove(path);
        }
        reload
    }

    #[cfg(not(feature = "hot-reload"))]
    #[inline(always)]
    pub fn should_reload(&self) -> bool {
        false
    }

    pub fn paths(&self) -> impl '_ + Iterator<Item=&Path> {
        self.files.iter().map(|p| p.as_path())
    }
}
