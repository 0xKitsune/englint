use englint::{Document, Span, TokenKind};

#[test]
fn paragraphs_sentences_and_source_spans() {
    let text = "Dr. Rivera wrote 3.14 pages. \"Really?\" Yes!\r\n\r\nDon't over-explain café.\n";
    let doc = Document::parse(text);
    assert_eq!(doc.source(), text);
    assert_eq!(doc.paragraphs().len(), 2);
    let sentences: Vec<_> = doc
        .sentences()
        .map(|s| doc.text(s.span).unwrap().trim())
        .collect();
    assert_eq!(
        sentences,
        [
            "Dr. Rivera wrote 3.14 pages.",
            "\"Really?\"",
            "Yes!",
            "Don't over-explain café."
        ]
    );
    let words: Vec<_> = doc
        .sentences()
        .flat_map(|s| s.words())
        .map(|t| doc.text(t.span).unwrap())
        .collect();
    assert_eq!(
        words,
        [
            "Dr",
            "Rivera",
            "wrote",
            "pages",
            "Really",
            "Yes",
            "Don't",
            "over-explain",
            "café"
        ]
    );
    for p in doc.paragraphs() {
        let tokens: String = p
            .sentences
            .iter()
            .flat_map(|s| &s.tokens)
            .map(|t| doc.text(t.span).unwrap())
            .collect();
        assert_eq!(tokens, doc.text(p.span).unwrap());
        for s in &p.sentences {
            assert_eq!(s.tokens.first().unwrap().span.start, s.span.start);
            assert_eq!(s.tokens.last().unwrap().span.end, s.span.end);
        }
    }
}

#[test]
fn unicode_contractions_numbers_and_positions() {
    let doc = Document::parse("  Élodie’s cafe\u{301} costs 1,234.50.\nNext.");
    let tokens: Vec<_> = doc.sentences().flat_map(|s| &s.tokens).collect();
    assert!(
        tokens
            .iter()
            .any(|t| t.kind == TokenKind::Word && doc.text(t.span) == Some("Élodie’s"))
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.kind == TokenKind::Word && doc.text(t.span) == Some("cafe\u{301}"))
    );
    assert!(
        tokens
            .iter()
            .any(|t| t.kind == TokenKind::Number && doc.text(t.span) == Some("1,234.50"))
    );
    assert_eq!(doc.position(2).unwrap().column, 3);
    assert!(doc.position(3).is_none());
    let next = doc.source().find("Next").unwrap();
    assert_eq!(doc.position(next).unwrap().line, 2);
    assert_eq!(doc.position(next).unwrap().column, 1);
    assert!(doc.text(Span { start: 4, end: 2 }).is_none());
    assert!(doc.position(usize::MAX).is_none());
}

#[test]
fn empty_blank_and_unterminated_input() {
    for source in ["", "  ", "\n\r\n \t\n"] {
        let doc = Document::parse(source);
        assert!(doc.paragraphs().is_empty());
        assert_eq!(doc.source(), source);
    }
    let doc = Document::parse("One sentence.  \n");
    assert_eq!(doc.sentences().count(), 1);
    assert_eq!(
        doc.text(doc.sentences().next().unwrap().span),
        Some("One sentence.  \n")
    );
    assert_eq!(
        Document::parse("No final punctuation").sentences().count(),
        1
    );
}

#[test]
fn parser_preserves_every_scalar_in_generated_inputs() {
    // Exercise combinations of Unicode, punctuation, blank lines and word joiners.
    let pieces = [
        "a", "é", "界", "🦀", "\u{301}", "\r\n", "\n\n", " ", "'", "’", "-", ".", "!?", "3.14",
        "\"", "Dr.",
    ];
    for a in pieces {
        for b in pieces {
            for c in pieces {
                let source = format!("{a}{b}{c}");
                let doc = Document::parse(&source);
                let mut previous = 0;
                for token in doc.sentences().flat_map(|s| &s.tokens) {
                    assert!(token.span.start >= previous);
                    assert!(token.span.end > token.span.start);
                    assert!(doc.text(token.span).is_some());
                    assert!(
                        source[previous..token.span.start]
                            .chars()
                            .all(char::is_whitespace)
                    );
                    previous = token.span.end;
                }
                assert!(source[previous..].chars().all(char::is_whitespace));
                assert_eq!(doc.source(), source);
            }
        }
    }
}
