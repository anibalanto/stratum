/// Parsed representation of a query string like `'>tech-decisions>impl'` or `'<<'`.
#[derive(Debug, PartialEq)]
pub enum Query {
    Down(Vec<String>),
    Up(usize),
    ListDown,
    ListUp,
}

#[derive(Debug, PartialEq)]
pub enum QueryError {
    Empty,
    InvalidSyntax(String),
    InvalidSegment(String),
}

impl std::fmt::Display for QueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryError::Empty => write!(f, "query vacía"),
            QueryError::InvalidSyntax(s) => write!(f, "sintaxis inválida: '{s}'"),
            QueryError::InvalidSegment(s) => write!(f, "segmento inválido: '{s}'"),
        }
    }
}

pub fn parse(query: &str) -> Result<Query, QueryError> {
    if query.is_empty() {
        return Err(QueryError::Empty);
    }

    if query == ">?" {
        return Ok(Query::ListDown);
    }

    if query == "<?" {
        return Ok(Query::ListUp);
    }

    if query.starts_with('>') {
        let segments: Vec<String> = query[1..]
            .split('>')
            .map(|s| s.to_string())
            .collect();

        for seg in &segments {
            if seg.is_empty() {
                return Err(QueryError::InvalidSegment(seg.clone()));
            }
        }

        return Ok(Query::Down(segments));
    }

    if query.chars().all(|c| c == '<') {
        let n = query.len();
        return Ok(Query::Up(n));
    }

    Err(QueryError::InvalidSyntax(query.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn down_one_level() {
        assert_eq!(parse(">impl"), Ok(Query::Down(vec!["impl".into()])));
    }

    #[test]
    fn down_two_levels() {
        assert_eq!(
            parse(">tech-decisions>impl"),
            Ok(Query::Down(vec!["tech-decisions".into(), "impl".into()]))
        );
    }

    #[test]
    fn up_one() {
        assert_eq!(parse("<"), Ok(Query::Up(1)));
    }

    #[test]
    fn up_three() {
        assert_eq!(parse("<<<"), Ok(Query::Up(3)));
    }

    #[test]
    fn list_down() {
        assert_eq!(parse(">?"), Ok(Query::ListDown));
    }

    #[test]
    fn list_up() {
        assert_eq!(parse("<?"), Ok(Query::ListUp));
    }

    #[test]
    fn empty_is_error() {
        assert_eq!(parse(""), Err(QueryError::Empty));
    }

    #[test]
    fn invalid_syntax() {
        assert!(matches!(parse("xyz"), Err(QueryError::InvalidSyntax(_))));
    }

    #[test]
    fn empty_segment_is_error() {
        assert!(matches!(
            parse(">tech-decisions>>impl"),
            Err(QueryError::InvalidSegment(_))
        ));
    }
}
