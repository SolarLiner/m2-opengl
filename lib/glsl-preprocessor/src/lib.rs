use std::{collections::HashMap, fmt, ops::Range, path::PathBuf};
use std::cell::RefCell;
use std::fmt::Formatter;
use std::rc::Rc;

use lazy_regex::{Lazy, lazy_regex, Regex};

#[derive(Debug, Clone)]
pub struct TranslationUnit {
    cache: Rc<CompilationCache>,
    path: PathBuf,
    contents: String,
    found_includes: Vec<(Range<usize>, PathBuf)>,
}

impl TranslationUnit {
    pub fn new(
        cache: Rc<CompilationCache>,
        filepath: impl Into<PathBuf>,
        contents: impl ToString,
    ) -> Self {
        static INCLUDE_RE: Lazy<Regex> = lazy_regex!(r#"#include\s+[<"](.*)[">]"#);
        let path = filepath.into();
        let contents = contents.to_string();
        let found_includes = INCLUDE_RE
            .captures_iter(&contents)
            .map(|capture| {
                let range = capture.get(0).unwrap().range();
                let filepath = capture.get(1).unwrap().as_str().parse::<PathBuf>().unwrap();
                (range, filepath)
            })
            .collect();
        Self {
            cache,
            path,
            contents,
            found_includes,
        }
    }

    pub fn process(&self) -> String {
        let mut result = self.contents.clone();
        for (range, path) in &self.found_includes {
            let path = self.path.parent().unwrap().join(path);
            let unit = self.cache.request(path.clone());
            result.replace_range(range.clone(), &unit.process());
        }
        result
    }
}

#[derive(Clone)]
pub struct CompilationCache {
    translation_units: RefCell<HashMap<PathBuf, Rc<TranslationUnit>>>,
}

impl fmt::Debug for CompilationCache {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompilationCache")
            .field(
                "translation_units",
                &format!(
                    "<{} translation units>",
                    self.translation_units.borrow().len()
                ),
            )
            .finish()
    }
}

impl CompilationCache {
    pub fn new() -> Rc<Self> {
        Rc::new(Self {
            translation_units: RefCell::new(HashMap::new()),
        })
    }

    pub fn from_file(file: impl Into<PathBuf>) -> (Rc<Self>, Rc<TranslationUnit>) {
        let this = Self::new();
        let tu = this.request(file.into());
        (this, tu)
    }

    pub fn request(self: &Rc<Self>, file: PathBuf) -> Rc<TranslationUnit> {
        debug_assert!(file.is_absolute());
        self.translation_units
            .borrow_mut()
            .entry(file)
            .or_insert_with_key(|file| {
                let cache = self.clone();
                let file = file.clone();
                Rc::new({
                    TranslationUnit::new(
                        cache,
                        file.as_path(),
                        std::fs::read_to_string(&file).unwrap(),
                    )
                })
            })
            .clone()
    }
}

pub fn process_file(file: impl Into<PathBuf>) -> String {
    let (_, unit) = CompilationCache::from_file(file);
    eprintln!("{:#?}", unit);
    unit.process()
}
