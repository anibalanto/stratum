use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub enum DownError {
    NotFound(PathBuf),
}

impl std::fmt::Display for DownError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownError::NotFound(p) => write!(f, "capa no encontrada: {}", p.display()),
        }
    }
}

/// Resolves `segments` from `base` into `.estrato/a/.estrato/b/...` and validates existence.
pub fn resolve_down(base: &Path, segments: &[String]) -> Result<PathBuf, DownError> {
    let mut path = base.to_path_buf();
    for seg in segments {
        path = path.join(".estrato").join(seg);
    }
    if path.exists() {
        Ok(path)
    } else {
        Err(DownError::NotFound(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn one_level_exists() {
        let dir = tempdir().unwrap();
        let target = dir.path().join(".estrato").join("impl");
        std::fs::create_dir_all(&target).unwrap();

        let result = resolve_down(dir.path(), &["impl".into()]).unwrap();
        assert_eq!(result, target);
    }

    #[test]
    fn two_levels_exist() {
        let dir = tempdir().unwrap();
        let target = dir
            .path()
            .join(".estrato")
            .join("tech-decisions")
            .join(".estrato")
            .join("impl");
        std::fs::create_dir_all(&target).unwrap();

        let result =
            resolve_down(dir.path(), &["tech-decisions".into(), "impl".into()]).unwrap();
        assert_eq!(result, target);
    }

    #[test]
    fn not_found() {
        let dir = tempdir().unwrap();
        let err = resolve_down(dir.path(), &["noexiste".into()]).unwrap_err();
        assert!(matches!(err, DownError::NotFound(_)));
    }
}
