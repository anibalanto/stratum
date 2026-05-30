use std::path::{Path, PathBuf};

/// A single token in a stratum path expression.
#[derive(Debug, PartialEq, Clone)]
pub enum PathToken {
    Down(String),    // `>name`  — navigate into `.stratum/<name>`
    Up,              // `<`      — go up one stratum level (`../..`)
    Root,            // `<*`     — project root (nearest `.git` ancestor)
    TopRoot,         // `*`      — outermost `.git` ancestor (top of the whole project)
    Simple(PathBuf), // traditional path component — joined as-is
}

/// A parsed stratum path: a sequence of tokens.
/// Tokens are resolved left to right.
pub type StratumPath = Vec<PathToken>;

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

/// Parses a stratum path string into a token sequence.
///
/// Rules:
/// - Starts with `>`: sequence of `Down` tokens, terminated by a `/` which begins a `Simple`.
/// - Starts with `<*`: `Root` token, followed by optional `Simple` and `Down` tokens.
/// - Starts with `<`: sequence of `Up` tokens, terminated by a `/` which begins a `Simple`.
/// - Starts with `*`: `TopRoot` token (outermost `.git`), followed by optional `Simple`/`Down`.
/// - Anything else: a single `Simple` token (traditional path, returned as-is).
pub fn parse_path(s: &str) -> Result<StratumPath, ParseError> {
    if s.is_empty() {
        return Err(ParseError::Empty);
    }

    if s.starts_with('>') {
        return parse_down_tokens(s);
    }

    // `<*` must be checked before `<` to avoid consuming the `*`
    if s.starts_with("<*") {
        return parse_root_token(s);
    }

    if s.starts_with('<') {
        return parse_up_tokens(s);
    }

    if s.starts_with('*') {
        return parse_top_root_token(s);
    }

    Ok(vec![PathToken::Simple(PathBuf::from(s))])
}

/// Formats a `StratumPath` back to its string representation.
pub fn format_path(tokens: &StratumPath) -> String {
    let mut s = String::new();
    for token in tokens {
        match token {
            PathToken::Down(name) => { s.push('>'); s.push_str(name); }
            PathToken::Up        => s.push('<'),
            PathToken::Root      => s.push_str("<*"),
            PathToken::TopRoot   => s.push('*'),
            PathToken::Simple(p) => s.push_str(&p.display().to_string()),
        }
    }
    s
}

fn parse_root_token(s: &str) -> Result<StratumPath, ParseError> {
    // s starts with "<*"
    let rest = &s[2..];
    let mut tokens = vec![PathToken::Root];

    if rest.is_empty() {
        return Ok(tokens);
    }

    if rest.starts_with('/') {
        if let Some(gt_pos) = rest.find('>') {
            let simple_part = &rest[..gt_pos];
            let down_part   = &rest[gt_pos..]; // starts with '>'
            if !simple_part.is_empty() && simple_part != "/" {
                tokens.push(PathToken::Simple(PathBuf::from(simple_part)));
            }
            tokens.extend(parse_down_tokens(down_part)?);
        } else {
            tokens.push(PathToken::Simple(PathBuf::from(rest)));
        }
    } else if rest.starts_with('>') {
        tokens.extend(parse_down_tokens(rest)?);
    } else {
        return Err(ParseError::InvalidSyntax(s.to_string()));
    }

    Ok(tokens)
}

fn parse_down_tokens(s: &str) -> Result<StratumPath, ParseError> {
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

fn parse_up_tokens(s: &str) -> Result<StratumPath, ParseError> {
    let levels = s.chars().take_while(|&c| c == '<').count();
    let rest = &s[levels..];

    let mut tokens: StratumPath = (0..levels).map(|_| PathToken::Up).collect();

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
    /// `<*` used but no `.git` ancestor found.
    NoProjectRoot,
}

impl std::fmt::Display for ResolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolveError::InsufficientDepth { requested, actual } => {
                write!(f, "profundidad insuficiente: se pidieron {requested}, actual {actual}")
            }
            ResolveError::NotFound(p) => write!(f, "path no encontrado: {}", p.display()),
            ResolveError::NoProjectRoot => write!(f, "raíz de proyecto no encontrada (no hay .git en ningún ancestro)"),
        }
    }
}

/// Resolves a `StratumPath` starting from `base`.
/// `current` is used to validate depth for `Up` tokens.
/// The resolved path is validated for existence.
pub fn resolve(base: &Path, current: &Path, tokens: &StratumPath) -> Result<PathBuf, ResolveError> {
    use crate::up::depth;

    let using_root     = matches!(tokens.first(), Some(PathToken::Root));
    let using_top_root = matches!(tokens.first(), Some(PathToken::TopRoot));
    let using_up       = !using_root && !using_top_root
                         && tokens.iter().any(|t| *t == PathToken::Up);

    if using_up {
        let actual_depth = depth(current);
        let up_count = tokens.iter().filter(|t| **t == PathToken::Up).count();
        if up_count > actual_depth {
            return Err(ResolveError::InsufficientDepth {
                requested: up_count,
                actual: actual_depth,
            });
        }
    }

    let mut path = if using_top_root {
        find_outermost_git_root(base).ok_or(ResolveError::NoProjectRoot)?
    } else if using_root {
        find_git_root(base).ok_or(ResolveError::NoProjectRoot)?
    } else if using_up {
        PathBuf::new() // relative
    } else {
        base.to_path_buf()
    };

    for token in tokens {
        match token {
            PathToken::Root | PathToken::TopRoot => {} // initial path already set above
            PathToken::Down(name) => {
                path = path.join(".stratum").join(name);
            }
            PathToken::Up => {
                path = path.join("..").join("..");
            }
            PathToken::Simple(p) => {
                // Strip leading '/' so PathBuf::join doesn't treat it as absolute.
                let p = p.strip_prefix("/").unwrap_or(p);
                path = path.join(p);
            }
        }
    }

    if path.exists() || using_up || using_root || using_top_root {
        Ok(path)
    } else {
        Err(ResolveError::NotFound(path))
    }
}

fn parse_top_root_token(s: &str) -> Result<StratumPath, ParseError> {
    // s starts with "*"
    let rest = &s[1..];
    let mut tokens = vec![PathToken::TopRoot];

    if rest.is_empty() {
        return Ok(tokens);
    }

    if rest.starts_with('/') {
        if let Some(gt_pos) = rest.find('>') {
            let simple_part = &rest[..gt_pos];
            let down_part   = &rest[gt_pos..];
            if !simple_part.is_empty() && simple_part != "/" {
                tokens.push(PathToken::Simple(PathBuf::from(simple_part)));
            }
            tokens.extend(parse_down_tokens(down_part)?);
        } else {
            tokens.push(PathToken::Simple(PathBuf::from(rest)));
        }
    } else if rest.starts_with('>') {
        tokens.extend(parse_down_tokens(rest)?);
    } else {
        return Err(ParseError::InvalidSyntax(s.to_string()));
    }

    Ok(tokens)
}

/// Converts the current working directory into a stratum path starting from `*` (TopRoot).
///
/// Example: `/home/user/acreta/subsystems/stratum/.stratum/impl`
///          → `[TopRoot, Simple("/subsystems/stratum"), Down("impl")]`
///          → `*/subsystems/stratum>impl`
pub fn cwd_as_stratum_path(cwd: &Path) -> Option<StratumPath> {
    let top_root = find_outermost_git_root(cwd)?;
    let cwd_canon = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());
    let rel = cwd_canon.strip_prefix(&top_root).ok()?;

    let mut tokens = vec![PathToken::TopRoot];
    let components: Vec<_> = rel.components().collect();

    let mut i = 0;
    let mut simple_parts: Vec<String> = Vec::new();

    while i < components.len() {
        let comp = components[i].as_os_str().to_string_lossy();
        if comp == ".stratum" && i + 1 < components.len() {
            if !simple_parts.is_empty() {
                let path_str = format!("/{}", simple_parts.join("/"));
                tokens.push(PathToken::Simple(PathBuf::from(path_str)));
                simple_parts.clear();
            }
            i += 1; // skip ".stratum"
            let layer = components[i].as_os_str().to_string_lossy().to_string();
            tokens.push(PathToken::Down(layer));
            i += 1;
        } else {
            simple_parts.push(comp.to_string());
            i += 1;
        }
    }

    if !simple_parts.is_empty() {
        tokens.push(PathToken::Simple(PathBuf::from(format!("/{}", simple_parts.join("/")))));
    }

    Some(tokens)
}

fn find_outermost_git_root(start: &Path) -> Option<PathBuf> {
    let mut candidate = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
    let mut outermost = None;
    loop {
        if candidate.join(".git").exists() {
            outermost = Some(candidate.clone());
        }
        match candidate.parent() {
            Some(p) => candidate = p.to_path_buf(),
            None    => break,
        }
    }
    outermost
}

fn find_git_root(start: &Path) -> Option<PathBuf> {
    let mut candidate = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
    loop {
        if candidate.join(".git").exists() {
            return Some(candidate);
        }
        match candidate.parent() {
            Some(p) => candidate = p.to_path_buf(),
            None => return None,
        }
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

    #[test]
    fn root_alone() {
        assert_eq!(parse_path("<*"), Ok(vec![PathToken::Root]));
    }

    #[test]
    fn root_with_simple() {
        assert_eq!(
            parse_path("<*/subsystems/stratum"),
            Ok(vec![
                PathToken::Root,
                PathToken::Simple(PathBuf::from("/subsystems/stratum")),
            ])
        );
    }

    #[test]
    fn root_with_down() {
        assert_eq!(
            parse_path("<*>impl"),
            Ok(vec![PathToken::Root, PathToken::Down("impl".into())])
        );
    }

    #[test]
    fn root_full_example() {
        assert_eq!(
            parse_path("<*/subsystems/stratum>impl/crates/cli/src"),
            Ok(vec![
                PathToken::Root,
                PathToken::Simple(PathBuf::from("/subsystems/stratum")),
                PathToken::Down("impl".into()),
                PathToken::Simple(PathBuf::from("/crates/cli/src")),
            ])
        );
    }

    #[test]
    fn root_invalid_suffix() {
        assert!(matches!(parse_path("<*foo"), Err(ParseError::InvalidSyntax(_))));
    }

    #[test]
    fn format_path_root() {
        assert_eq!(format_path(&vec![PathToken::Root]), "<*");
    }

    #[test]
    fn format_path_root_with_down() {
        let tokens = vec![
            PathToken::Root,
            PathToken::Simple(PathBuf::from("/subsystems/stratum")),
            PathToken::Down("impl".into()),
            PathToken::Simple(PathBuf::from("/crates/cli/src")),
        ];
        assert_eq!(format_path(&tokens), "<*/subsystems/stratum>impl/crates/cli/src");
    }

    #[test]
    fn roundtrip_root_path() {
        let s = "<*/subsystems/stratum>impl/crates/cli/src";
        let tokens = parse_path(s).unwrap();
        assert_eq!(format_path(&tokens), s);
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
        let layer = dir.path().join(".stratum/a/.stratum/b");
        let file = layer.join("src/main.rs");
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, "").unwrap();

        let tokens = parse_path(">a>b/src/main.rs").unwrap();
        let result = resolve(dir.path(), dir.path(), &tokens).unwrap();
        assert_eq!(result, file);
    }

    #[test]
    fn resolve_up() {
        let current = PathBuf::from("/project/.stratum/impl");
        let tokens = parse_path("<").unwrap();
        let result = resolve(&current, &current, &tokens).unwrap();
        assert_eq!(result, PathBuf::from("../.."));
    }

    #[test]
    fn resolve_up_insufficient_depth() {
        let current = PathBuf::from("/project/.stratum/impl");
        let tokens = parse_path("<<<").unwrap();
        let err = resolve(&current, &current, &tokens).unwrap_err();
        assert!(matches!(err, ResolveError::InsufficientDepth { requested: 3, actual: 1 }));
    }

    #[test]
    fn resolve_root_finds_git() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        // Create .git at root
        std::fs::create_dir(root.join(".git")).unwrap();
        // Create a nested layer
        let layer = root.join("subsystems").join("foo").join(".stratum").join("impl");
        std::fs::create_dir_all(&layer).unwrap();

        let tokens = parse_path("<*").unwrap();
        let result = resolve(&layer, &layer, &tokens).unwrap();
        assert_eq!(result, root.canonicalize().unwrap());
    }

    #[test]
    fn resolve_root_with_simple_path() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir(root.join(".git")).unwrap();
        let docs = root.join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        let layer = root.join(".stratum").join("impl");
        std::fs::create_dir_all(&layer).unwrap();

        let tokens = parse_path("<*/docs").unwrap();
        let result = resolve(&layer, &layer, &tokens).unwrap();
        assert_eq!(result, docs.canonicalize().unwrap());
    }

    #[test]
    fn resolve_root_no_git_is_error() {
        let dir = tempdir().unwrap();
        let tokens = parse_path("<*").unwrap();
        let err = resolve(dir.path(), dir.path(), &tokens).unwrap_err();
        assert_eq!(err, ResolveError::NoProjectRoot);
    }

    // --- TopRoot (*) ---

    #[test]
    fn top_root_alone() {
        assert_eq!(parse_path("*"), Ok(vec![PathToken::TopRoot]));
    }

    #[test]
    fn top_root_with_simple() {
        assert_eq!(
            parse_path("*/subsystems/stratum"),
            Ok(vec![
                PathToken::TopRoot,
                PathToken::Simple(PathBuf::from("/subsystems/stratum")),
            ])
        );
    }

    #[test]
    fn top_root_with_down() {
        assert_eq!(
            parse_path("*>impl"),
            Ok(vec![PathToken::TopRoot, PathToken::Down("impl".into())])
        );
    }

    #[test]
    fn top_root_full_example() {
        assert_eq!(
            parse_path("*/subsystems/stratum>impl/crates/stratum/src"),
            Ok(vec![
                PathToken::TopRoot,
                PathToken::Simple(PathBuf::from("/subsystems/stratum")),
                PathToken::Down("impl".into()),
                PathToken::Simple(PathBuf::from("/crates/stratum/src")),
            ])
        );
    }

    #[test]
    fn top_root_invalid_suffix() {
        assert!(matches!(parse_path("*foo"), Err(ParseError::InvalidSyntax(_))));
    }

    #[test]
    fn format_top_root() {
        assert_eq!(format_path(&vec![PathToken::TopRoot]), "*");
    }

    #[test]
    fn roundtrip_top_root_path() {
        let s = "*/subsystems/stratum>impl/crates/stratum/src";
        assert_eq!(format_path(&parse_path(s).unwrap()), s);
    }

    #[test]
    fn resolve_top_root_finds_outermost_git() {
        let dir = tempdir().unwrap();
        let outer = dir.path();
        // outer git root
        std::fs::create_dir(outer.join(".git")).unwrap();
        // inner git root nested inside
        let inner = outer.join("subsystems").join("foo");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::create_dir(inner.join(".git")).unwrap();

        // Resolve from inner: should return outer, not inner
        let tokens = parse_path("*").unwrap();
        let result = resolve(&inner, &inner, &tokens).unwrap();
        assert_eq!(result, outer.canonicalize().unwrap());
    }

    #[test]
    fn resolve_top_root_with_path() {
        let dir = tempdir().unwrap();
        let outer = dir.path();
        std::fs::create_dir(outer.join(".git")).unwrap();
        let sub = outer.join("subsystems").join("foo");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::create_dir(sub.join(".git")).unwrap();
        let docs = outer.join("docs");
        std::fs::create_dir_all(&docs).unwrap();

        let tokens = parse_path("*/docs").unwrap();
        let result = resolve(&sub, &sub, &tokens).unwrap();
        assert_eq!(result, docs.canonicalize().unwrap());
    }

    #[test]
    fn resolve_top_root_no_git_is_error() {
        let dir = tempdir().unwrap();
        let tokens = parse_path("*").unwrap();
        let err = resolve(dir.path(), dir.path(), &tokens).unwrap_err();
        assert_eq!(err, ResolveError::NoProjectRoot);
    }
}
