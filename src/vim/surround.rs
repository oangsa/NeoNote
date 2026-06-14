#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SurroundSpec {
    Pair(char, char), // e.g. '(', ')'
    Tag {
        name: String,
        attributes: String,
    },
    AnyTag,
}

pub enum ParseResult {
    Complete(SurroundSpec),
    Incomplete,
    Invalid,
}

impl SurroundSpec {
    pub fn parse(buffer: &str) -> ParseResult {
        if buffer.is_empty() {
            return ParseResult::Incomplete;
        }

        let first = buffer.chars().next().unwrap();

        if first == 't' {
            if buffer.len() == 1 {
                return ParseResult::Complete(SurroundSpec::AnyTag);
            }
            return ParseResult::Invalid;
        }

        if first == '<' {
            if buffer.ends_with('>') && buffer.len() > 1 {
                let inner = &buffer[1..buffer.len() - 1];
                let mut parts = inner.splitn(2, ' ');
                let name = parts.next().unwrap_or("").to_string();
                let attributes = parts.next().unwrap_or("").to_string();
                if name.is_empty() {
                    return ParseResult::Invalid;
                }
                return ParseResult::Complete(SurroundSpec::Tag { name, attributes });
            }
            return ParseResult::Incomplete;
        }

        // 2. Single character pairs
        if buffer.len() > 1 {
            return ParseResult::Invalid; // Only tags can be multiple chars
        }

        let pair = match first {
            'b' | '(' | ')' => ('(', ')'),
            'B' | '{' | '}' => ('{', '}'),
            'r' | '[' | ']' => ('[', ']'),
            'a' | '<' | '>' => ('<', '>'),
            '\'' => ('\'', '\''),
            '"' => ('"', '"'),
            '`' => ('`', '`'),
            '*' => ('*', '*'),
            '_' => ('_', '_'),
            // fallback: surround with the exact character
            c => (c, c),
        };

        // Note: We don't implement inner/outer spacing nuances (e.g. `(` adds spaces, `)` doesn't)
        // for simplicity, we just use the characters.
        ParseResult::Complete(SurroundSpec::Pair(pair.0, pair.1))
    }

    pub fn strings(&self) -> (String, String) {
        match self {
            SurroundSpec::Pair(l, r) => (l.to_string(), r.to_string()),
            SurroundSpec::Tag { name, attributes } => {
                if attributes.is_empty() {
                    (format!("<{}>", name), format!("</{}>", name))
                } else {
                    (format!("<{} {}>", name, attributes), format!("</{}>", name))
                }
            }
            SurroundSpec::AnyTag => (String::new(), String::new()),
        }
    }
}
