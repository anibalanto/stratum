use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub enum UpError {
    InsufficientDepth { requested: usize, actual: usize },
}

impl std::fmt::Display for UpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpError::InsufficientDepth { requested, actual } => write!(
                f,
                "profundidad insuficiente: se pidieron {requested} niveles, actual {actual}"
            ),
        }
    }
}

/// Counts how many stratum levels deep `path` is by scanning for `.stratum/<name>` pairs.
pub fn depth(path: &Path) -> usize {
    let components: Vec<_> = path.components().collect();
    let mut count = 0;
    let mut i = 0;
    while i + 1 < components.len() {
        let a = components[i].as_os_str();
        let b = components[i + 1].as_os_str();
        if a == ".stratum" && !b.is_empty() && b != ".stratum" {
            count += 1;
            i += 2;
        } else {
            i += 1;
        }
    }
    count
}

/// Returns a relative path going up `levels` stratum levels (each = `../..`),
/// appending `suffix` if provided. Validates that depth >= levels.
pub fn resolve_up(
    current: &Path,
    levels: usize,
    suffix: Option<&Path>,
) -> Result<PathBuf, UpError> {
    if levels == 0 {
        return Err(UpError::InsufficientDepth { requested: 0, actual: depth(current) });
    }
    let actual = depth(current);
    if levels > actual {
        return Err(UpError::InsufficientDepth { requested: levels, actual });
    }
    let mut path = PathBuf::new();
    for _ in 0..levels {
        path = path.join("..").join("..");
    }
    if let Some(s) = suffix {
        path = path.join(s);
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf { PathBuf::from(s) }

    #[test]
    fn depth_root() { assert_eq!(depth(&p("/home/user/project")), 0); }

    #[test]
    fn depth_one() { assert_eq!(depth(&p("/project/.stratum/impl")), 1); }

    #[test]
    fn depth_two() {
        assert_eq!(depth(&p("/project/.stratum/tech/.stratum/impl")), 2);
    }

    #[test]
    fn up_one() {
        assert_eq!(resolve_up(&p("/project/.stratum/impl"), 1, None).unwrap(), p("../.."));
    }

    #[test]
    fn up_two() {
        let result = resolve_up(&p("/project/.stratum/tech/.stratum/impl"), 2, None).unwrap();
        assert_eq!(result, p("../../../.."));
    }

    #[test]
    fn up_with_suffix() {
        let result =
            resolve_up(&p("/project/.stratum/impl"), 1, Some(Path::new("src/main.rs"))).unwrap();
        assert_eq!(result, p("../../src/main.rs"));
    }

    #[test]
    fn up_exceeds_depth() {
        let err = resolve_up(&p("/project/.stratum/impl"), 3, None).unwrap_err();
        assert!(matches!(err, UpError::InsufficientDepth { requested: 3, actual: 1 }));
    }

    #[test]
    fn up_zero_is_error() {
        assert!(resolve_up(&p("/project/.stratum/impl"), 0, None).is_err());
    }
}
