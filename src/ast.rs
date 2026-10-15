use serde::Serialize;

/// Half-open UTF-8 byte offsets into the original source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Position {
    /// One-based line number.
    pub line: usize,
    /// One-based Unicode scalar column, not a byte or display column.
    pub column: usize,
}

/// The source is stored once. Every node refers to it through a byte span.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Document {
    source: String,
    paragraphs: Vec<Paragraph>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Paragraph {
    pub span: Span,
    pub sentences: Vec<Sentence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Sentence {
    pub span: Span,
    pub tokens: Vec<Token>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Token {
    pub span: Span,
    pub kind: TokenKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenKind {
    Word,
    Number,
    Punctuation,
    Whitespace,
}

impl Document {
    pub fn parse(source: impl Into<String>) -> Self {
        let source = source.into();
        let mut paragraphs = Vec::new();
        let mut start = None;
        let mut offset = 0;
        for line in source.split_inclusive('\n') {
            if line.trim().is_empty() {
                if let Some(begin) = start.take() {
                    paragraphs.push(paragraph(&source, begin, offset));
                }
            } else {
                start.get_or_insert(offset);
            }
            offset += line.len();
        }
        if let Some(begin) = start {
            paragraphs.push(paragraph(&source, begin, source.len()));
        }
        Self { source, paragraphs }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn paragraphs(&self) -> &[Paragraph] {
        &self.paragraphs
    }

    pub fn sentences(&self) -> impl Iterator<Item = &Sentence> {
        self.paragraphs.iter().flat_map(|p| &p.sentences)
    }

    /// Returns `None` for out-of-bounds spans or offsets inside a UTF-8 scalar.
    pub fn text(&self, span: Span) -> Option<&str> {
        self.source.get(span.start..span.end)
    }

    pub fn position(&self, offset: usize) -> Option<Position> {
        let prefix = self.source.get(..offset)?;
        Some(Position {
            line: prefix.bytes().filter(|&b| b == b'\n').count() + 1,
            column: prefix.rsplit('\n').next()?.chars().count() + 1,
        })
    }
}

impl Sentence {
    pub fn words(&self) -> impl Iterator<Item = &Token> {
        self.tokens.iter().filter(|t| t.kind == TokenKind::Word)
    }
}

fn paragraph(source: &str, start: usize, end: usize) -> Paragraph {
    let mut sentences = Vec::new();
    let tokens = tokenize(source, start, end);
    let mut pending = Vec::new();
    for (i, token) in tokens.iter().enumerate() {
        pending.push(*token);
        // Retain the whitespace following a sentence in that sentence.
        if i + 1 == tokens.len()
            || (token.kind == TokenKind::Whitespace
                && terminal(source, &pending[..pending.len() - 1]))
        {
            let span = Span {
                start: pending[0].span.start,
                end: token.span.end,
            };
            sentences.push(Sentence {
                span,
                tokens: std::mem::take(&mut pending),
            });
        }
    }
    Paragraph {
        span: Span { start, end },
        sentences,
    }
}

fn terminal(source: &str, tokens: &[Token]) -> bool {
    let Some(last) = tokens.iter().rev().find(|t| {
        !matches!(
            &source[t.span.start..t.span.end],
            "\"" | "'" | "”" | "’" | ")" | "]"
        )
    }) else {
        return false;
    };
    let text = &source[last.span.start..last.span.end];
    if text == "!" || text == "?" {
        return true;
    }
    if text != "." {
        return false;
    }
    let index = tokens.iter().position(|t| t.span == last.span).unwrap();
    let Some(previous) = index.checked_sub(1).and_then(|i| tokens.get(i)) else {
        return true;
    };
    let word = &source[previous.span.start..previous.span.end];
    let lower = word.to_lowercase();
    // Titles and initials do not end sentences. Other abbreviations are ambiguous.
    !(matches!(
        lower.as_str(),
        "mr" | "mrs" | "ms" | "dr" | "prof" | "sr" | "jr"
    ) || (previous.kind == TokenKind::Word
        && word.chars().count() == 1
        && word.chars().all(char::is_uppercase)))
}

fn tokenize(source: &str, start: usize, end: usize) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = source[start..end].char_indices().peekable();
    while let Some((relative, ch)) = chars.next() {
        let begin = start + relative;
        let kind = if ch.is_whitespace() {
            TokenKind::Whitespace
        } else if ch.is_alphabetic() {
            TokenKind::Word
        } else if ch.is_numeric() {
            TokenKind::Number
        } else {
            TokenKind::Punctuation
        };
        let mut finish = begin + ch.len_utf8();
        while let Some(&(offset, next)) = chars.peek() {
            let following = source[start + offset + next.len_utf8()..end].chars().next();
            let joins = match kind {
                TokenKind::Word => {
                    next.is_alphabetic()
                        || is_combining(next)
                        || (matches!(next, '\'' | '’' | '-')
                            && following.is_some_and(char::is_alphabetic))
                }
                TokenKind::Number => {
                    next.is_numeric()
                        || (matches!(next, '.' | ',') && following.is_some_and(char::is_numeric))
                }
                TokenKind::Whitespace => next.is_whitespace(),
                TokenKind::Punctuation => false,
            };
            if !joins {
                break;
            }
            chars.next();
            finish = start + offset + next.len_utf8();
        }
        tokens.push(Token {
            span: Span {
                start: begin,
                end: finish,
            },
            kind,
        });
    }
    tokens
}

fn is_combining(ch: char) -> bool {
    matches!(ch as u32, 0x0300..=0x036f | 0x1ab0..=0x1aff | 0x1dc0..=0x1dff | 0x20d0..=0x20ff | 0xfe20..=0xfe2f)
}
