use std::collections::HashSet;
use std::{
    io,
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

pub fn load_and_parse(path: impl AsRef<Path>) -> io::Result<Vec<(PathBuf, String)>> {
    let contents = std::fs::read_to_string(path.as_ref())?;
    let dirname = path.as_ref().parent().unwrap();
    let mut paths = HashSet::new();
    let (contents, imports) = parse_imports(&contents);
    Ok(imports
        .into_iter()
        .map(|p| dirname.join(p))
        .map(load_and_parse)
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .filter(|(p, _)| paths.insert(p.clone()))
        .chain(std::iter::once((path.as_ref().to_path_buf(), contents)))
        .collect())
}
