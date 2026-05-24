use std::path::{Path, PathBuf};

/// A single token in an estrato path expression.
#[derive(Debug, PartialEq, Clone)]
pub enum PathToken {
    Down(String),   // `>name`  — navigate into `.estrato/<name>`
    Up,             // `<`      — go up one estrato level (`../..`)
    Simple(PathBuf), // traditional path component — joined as-is
}

/// A parsed estrato path: a sequence of tokens.
/// Tokens are resolved left to right.
pub type EstratPath = Vec<PathToken>;

#[derive(Debug, PartialEq)]
pub enum ParseError {
    Empty,
    InvalidSegment(String),
    InvalidSyntax(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Empty => write!(f, "path vacío"),
            ParseError::InvalidSegment(s) => write!(f, "segmento inválido: '{s}'"),
            ParseError::InvalidSyntax(s) => write!(f, "sintaxis inválida: '{s}'"),
        }
    }
}

/// Parses an estrato path string into a token sequence.
///
/// Rules:
/// - Starts with `>`: sequence of `Down` tokens, terminated by a `/` which begins a `Simple`.
/// - Starts with `<`: sequence of `Up` tokens, terminated by a `/` which begins a `Simple`.
/// - Anything else: a single `Simple` token (traditional path, returned as-is).
pub fn parse_path(s: &str) -> Result<EstratPath, ParseError> {
    if s.is_empty() {
        return Err(ParseError::Empty);
    }

    if s.starts_with('>') {
        return parse_down_tokens(s);
    }

    if s.starts_with('<') {
        return parse_up_tokens(s);
    }

    Ok(vec![PathToken::Simple(PathBuf::from(s))])
}

fn parse_down_tokens(s: &str) -> Result<EstratPath, ParseError> {
    let mut tokens = Vec::new();
    let mut rest = &s[1..]; // skip leading '>'

    loop {
        // Find the next '>' or '/' or end of string
        let next_cmd = rest.find('>');
        let next_slash = rest.find('/');

        match (next_cmd, next_slash) {
            // '/' comes before '>' (or there's no '>') → this is the last layer + Simple
            (cmd, Some(slash)) if cmd.map_or(true, |c| slash < c) => {
                let layer = &rest[..slash];
                let simple = &rest[slash..]; // keep leading '/'
                if layer.is_empty() {
                    return Err(ParseError::InvalidSegment(layer.to_string()));
                }
                tokens.push(PathToken::Down(layer.to_string()));
                if simple != "/" {
                    tokens.push(PathToken::Simple(PathBuf::from(simple)));
                }
                break;
            }
            // '>' found (before any '/') → another layer command follows
            (Some(cmd), _) => {
                let layer = &rest[..cmd];
                if layer.is_empty() {
                    return Err(ParseError::InvalidSegment(layer.to_string()));
                }
                tokens.push(PathToken::Down(layer.to_string()));
                rest = &rest[cmd + 1..];
            }
            // '/' found but no '>' (or '>' comes after) — same as first arm, handled there.
            // No '>' and no '/' → last layer, no suffix
            (None, None) => {
                if rest.is_empty() {
                    return Err(ParseError::InvalidSegment(rest.to_string()));
                }
                tokens.push(PathToken::Down(rest.to_string()));
                break;
            }
            // No '>' but there is a '/' → last layer with Simple suffix
            (None, Some(slash)) => {
                let layer = &rest[..slash];
                let simple = &rest[slash..]; // keep leading '/'
                if layer.is_empty() {
                    return Err(ParseError::InvalidSegment(layer.to_string()));
                }
                tokens.push(PathToken::Down(layer.to_string()));
                if simple != "/" {
                    tokens.push(PathToken::Simple(PathBuf::from(simple)));
                }
                break;
            }
        }
    }

    Ok(tokens)
}

fn parse_up_tokens(s: &str) -> Result<EstratPath, ParseError> {
    let levels = s.chars().take_while(|&c| c == '<').count();
    let rest = &s[levels..];

    let mut tokens: EstratPath = (0..levels).map(|_| PathToken::Up).collect();

    if rest.starts_with('/') {
        if rest.len() > 1 {
            tokens.push(PathToken::Simple(PathBuf::from(rest))); // keep leading '/'
        }
    } else if !rest.is_empty() {
        return Err(ParseError::InvalidSyntax(s.to_string()));
    }

    Ok(tokens)
}

/// Resolution errors.
#[derive(Debug, PartialEq)]
pub enum ResolveError {
    InsufficientDepth { requested: usize, actual: usize },
    NotFound(PathBuf),
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::InsufficientDepth { requested, actual } => {
                write!(f, "profundidad insuficiente: se pidieron {requested}, actual {actual}")
            }
            ResolveError::NotFound(p) => write!(f, "path no encontrado: {}", p.display()),
        }
    }
}

/// Resolves an `EstratPath` starting from `base`.
/// `current` is used to validate depth for `Up` tokens.
/// The resolved path is validated for existence.
pub fn resolve(base: &Path, current: &Path, tokens: &EstratPath) -> Result<PathBuf, ResolveError> {
    use crate::up::depth;

    let actual_depth = depth(current);
    let up_count = tokens.iter().filter(|t| **t == PathToken::Up).count();

    if up_count > actual_depth {
        return Err(ResolveError::InsufficientDepth {
            requested: up_count,
            actual: actual_depth,
        });
    }

    // Build a relative path for Up tokens, absolute for Down+Simple.
    let using_up = tokens.iter().any(|t| *t == PathToken::Up);

    let mut path = if using_up {
        PathBuf::new() // relative
    } else {
        base.to_path_buf()
    };

    for token in tokens {
        match token {
            PathToken::Down(name) => {
                path = path.join(".estrato").join(name);
            }
            PathToken::Up => {
                path = path.join("..").join("..");
            }
            PathToken::Simple(p) => {
                // Strip leading '/' (separator artifact) before joining,
                // so PathBuf::join doesn't treat it as an absolute path.
                let p = p.strip_prefix("/").unwrap_or(p);
                path = path.join(p);
            }
        }
    }

    if path.exists() || using_up {
        Ok(path)
    } else {
        Err(ResolveError::NotFound(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // --- parse_path ---

    #[test]
    fn simple_traditional() {
        assert_eq!(
            parse_path("subfolder1/subfolder2/file.md"),
            Ok(vec![PathToken::Simple(PathBuf::from("subfolder1/subfolder2/file.md"))])
        );
    }

    #[test]
    fn down_one() {
        assert_eq!(
            parse_path(">impl"),
            Ok(vec![PathToken::Down("impl".into())])
        );
    }

    #[test]
    fn down_two() {
        assert_eq!(
            parse_path(">tech-decisions>impl"),
            Ok(vec![
                PathToken::Down("tech-decisions".into()),
                PathToken::Down("impl".into()),
            ])
        );
    }

    #[test]
    fn down_with_simple() {
        assert_eq!(
            parse_path(">tech-decisions>impl/package1/solution.rs"),
            Ok(vec![
                PathToken::Down("tech-decisions".into()),
                PathToken::Down("impl".into()),
                PathToken::Simple(PathBuf::from("/package1/solution.rs")),
            ])
        );
    }

    #[test]
    fn down_one_with_simple() {
        assert_eq!(
            parse_path(">tasks/pending.md"),
            Ok(vec![
                PathToken::Down("tasks".into()),
                PathToken::Simple(PathBuf::from("/pending.md")),
            ])
        );
    }

    #[test]
    fn up_one() {
        assert_eq!(parse_path("<"), Ok(vec![PathToken::Up]));
    }

    #[test]
    fn up_two_with_simple() {
        assert_eq!(
            parse_path("<</docs/api.md"),
            Ok(vec![
                PathToken::Up,
                PathToken::Up,
                PathToken::Simple(PathBuf::from("/docs/api.md")),
            ])
        );
    }

    #[test]
    fn empty_is_error() {
        assert!(matches!(parse_path(""), Err(ParseError::Empty)));
    }

    #[test]
    fn empty_layer_is_error() {
        assert!(matches!(parse_path(">a>>b"), Err(ParseError::InvalidSegment(_))));
    }

    // --- resolve ---

    #[test]
    fn resolve_simple() {
        let dir = tempdir().unwrap();
        let file = dir.path().join("docs").join("readme.md");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "").unwrap();

        let tokens = parse_path("docs/readme.md").unwrap();
        let result = resolve(dir.path(), dir.path(), &tokens).unwrap();
        assert_eq!(result, file);
    }

    #[test]
    fn resolve_down_two_with_simple() {
        let dir = tempdir().unwrap();
        let layer = dir.path().join(".estrato/a/.estrato/b");
        let file = layer.join("src/main.rs");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "").unwrap();

        let tokens = parse_path(">a>b/src/main.rs").unwrap();
        let result = resolve(dir.path(), dir.path(), &tokens).unwrap();
        assert_eq!(result, file);
    }

    #[test]
    fn resolve_up() {
        let current = PathBuf::from("/project/.estrato/impl");
        let tokens = parse_path("<").unwrap();
        let result = resolve(&current, &current, &tokens).unwrap();
        assert_eq!(result, PathBuf::from("../.."));
    }

    #[test]
    fn resolve_up_insufficient_depth() {
        let current = PathBuf::from("/project/.estrato/impl");
        let tokens = parse_path("<<<").unwrap();
        let err = resolve(&current, &current, &tokens).unwrap_err();
        assert!(matches!(err, ResolveError::InsufficientDepth { requested: 3, actual: 1 }));
    }
}
