use eyre::{Context, Result};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use lazy_regex::{lazy_regex, Lazy, Regex};

fn parse_imports(source: &str) -> (String, Vec<PathBuf>) {
    static IMPORT_PRAGMA: Lazy<Regex> = lazy_regex!(r#"#include\s+[<"'](.*)[>"']"#);
    let mut source = source.to_string();
    let mut paths = vec![];
    loop {
        let remove_range = if let Some(capture) = IMPORT_PRAGMA.captures(&source) {
            paths.push(capture.get(1).unwrap().as_str().parse().unwrap());
            capture.get(0).unwrap().range()
        } else {
            break;
        };
        source.replace_range(remove_range, "");
    }
    (source, paths)
}

pub fn load_and_parse(path: impl AsRef<Path>) -> Result<Vec<(PathBuf, String)>> {
    let path = path.as_ref();
    let contents = std::fs::read_to_string(path)?;
    let dirname = path.parent().unwrap();
    let mut paths = HashSet::new();
    let (contents, imports) = parse_imports(&contents);
    Ok(imports
        .into_iter()
        .map(|p| dirname.join(p))
        .map(|p| {
            load_and_parse(&p).with_context(|| format!("Including {}", p.display()))
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .filter(|(p, _)| paths.insert(p.clone()))
        .chain(std::iter::once((path.to_path_buf(), contents)))
        .collect())
}
