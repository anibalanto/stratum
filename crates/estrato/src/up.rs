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
                "profundidad insuficiente: se pidieron {requested} niveles, profundidad actual {actual}"
            ),
        }
    }
}

/// Counts how many estrato levels deep `path` is by scanning for `.estrato/<name>` pairs.
pub fn depth(path: &Path) -> usize {
    let components: Vec<_> = path.components().collect();
    let mut count = 0;
    let mut i = 0;
    while i + 1 < components.len() {
        let a = components[i].as_os_str();
        let b = components[i + 1].as_os_str();
        if a == ".estrato" && !b.is_empty() && b != ".estrato" {
            count += 1;
            i += 2;
        } else {
            i += 1;
        }
    }
    count
}

/// Returns a relative path going up `n` estrato levels (each level = `../..`).
pub fn resolve_up(current: &Path, n: usize) -> Result<PathBuf, UpError> {
    if n == 0 {
        return Err(UpError::InsufficientDepth {
            requested: n,
            actual: depth(current),
        });
    }
    let actual = depth(current);
    if n > actual {
        return Err(UpError::InsufficientDepth {
            requested: n,
            actual,
        });
    }
    let mut path = PathBuf::new();
    for _ in 0..n {
        path = path.join("..").join("..");
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn depth_root() {
        assert_eq!(depth(&p("/home/user/project")), 0);
    }

    #[test]
    fn depth_one() {
        assert_eq!(depth(&p("/project/.estrato/impl")), 1);
    }

    #[test]
    fn depth_two() {
        assert_eq!(
            depth(&p("/project/.estrato/tech-decisions/.estrato/impl")),
            2
        );
    }

    #[test]
    fn up_one_from_depth_one() {
        let path = p("/project/.estrato/impl");
        let result = resolve_up(&path, 1).unwrap();
        assert_eq!(result, p("../.."));
    }

    #[test]
    fn up_two_from_depth_two() {
        let path = p("/project/.estrato/tech/.estrato/impl");
        let result = resolve_up(&path, 2).unwrap();
        assert_eq!(result, p("../../..").join(".."));
    }

    #[test]
    fn up_exceeds_depth() {
        let path = p("/project/.estrato/impl");
        let err = resolve_up(&path, 3).unwrap_err();
        assert!(matches!(
            err,
            UpError::InsufficientDepth {
                requested: 3,
                actual: 1
            }
        ));
    }

    #[test]
    fn up_zero_is_error() {
        let path = p("/project/.estrato/impl");
        assert!(resolve_up(&path, 0).is_err());
    }
}
