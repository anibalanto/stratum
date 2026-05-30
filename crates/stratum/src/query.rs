use crate::path::{parse_path, StratumPath, ParseError};

/// A CLI query: either a path expression or an info command.
#[derive(Debug, PartialEq)]
pub enum Query {
    Path(StratumPath),
    ListDown,  // `>?`
    ListUp,    // `<?`
}

#[derive(Debug, PartialEq)]
pub enum QueryError {
    Empty,
    ParseError(ParseError),
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryError::Empty => write!(f, "query vacía"),
            QueryError::ParseError(e) => write!(f, "{e}"),
        }
    }
}

pub fn parse(s: &str) -> Result<Query, QueryError> {
    if s.is_empty() {
        return Err(QueryError::Empty);
    }
    if s == ">?" {
        return Ok(Query::ListDown);
    }
    if s == "<?" {
        return Ok(Query::ListUp);
    }
    parse_path(s).map(Query::Path).map_err(QueryError::ParseError)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path::PathToken;
    use std::path::PathBuf;

    #[test]
    fn list_down() {
        assert_eq!(parse(">?"), Ok(Query::ListDown));
    }

    #[test]
    fn list_up() {
        assert_eq!(parse("<?"), Ok(Query::ListUp));
    }

    #[test]
    fn path_down() {
        assert_eq!(
            parse(">impl"),
            Ok(Query::Path(vec![PathToken::Down("impl".into())]))
        );
    }

    #[test]
    fn path_traditional() {
        assert_eq!(
            parse("src/main.rs"),
            Ok(Query::Path(vec![PathToken::Simple(PathBuf::from("src/main.rs"))]))
        );
    }

    #[test]
    fn empty_is_error() {
        assert_eq!(parse(""), Err(QueryError::Empty));
    }
}
