use std::path::Path;

pub struct Context {
    pub depth: usize,
    pub sub_layers: Vec<String>,
}

impl Context {
    pub fn from_dir(dir: &Path) -> Self {
        let depth = crate::up::depth(dir);
        let sub_layers = list_sub_layers(dir);
        Context { depth, sub_layers }
    }

    pub fn up_path(&self) -> Option<String> {
        if self.depth == 0 {
            return None;
        }
        let s = (0..self.depth)
            .map(|_| "../..")
            .collect::<Vec<_>>()
            .join("/");
        Some(s)
    }
}

fn list_sub_layers(dir: &Path) -> Vec<String> {
    let estrato_dir = dir.join(".estrato");
    let Ok(entries) = std::fs::read_dir(&estrato_dir) else {
        return vec![];
    };
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn context_at_root_with_sub_layers() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join(".estrato/impl")).unwrap();
        std::fs::create_dir_all(dir.path().join(".estrato/tech-decisions")).unwrap();

        let ctx = Context::from_dir(dir.path());
        assert_eq!(ctx.depth, 0);
        assert_eq!(ctx.sub_layers, vec!["impl", "tech-decisions"]);
        assert_eq!(ctx.up_path(), None);
    }

    #[test]
    fn context_one_deep_no_sub_layers() {
        let dir = tempdir().unwrap();
        let inner = dir.path().join(".estrato/impl");
        std::fs::create_dir_all(&inner).unwrap();

        let ctx = Context::from_dir(&inner);
        assert_eq!(ctx.depth, 1);
        assert!(ctx.sub_layers.is_empty());
        assert_eq!(ctx.up_path(), Some("../..".to_string()));
    }
}
