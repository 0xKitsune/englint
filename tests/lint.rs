use englint::{BUILTIN_RULES, Config, Diagnostic, Document, Linter, Rule, RuleInfo, Severity};

#[test]
fn every_builtin_has_a_literal_behavior_case() {
    let cases = [
        ("plain-word", "We utilize tools.", "utilize", Some("use")),
        (
            "wordy-phrase",
            "In order to help.",
            "In order to",
            Some("To"),
        ),
        (
            "vague-attribution",
            "Experts believe this.",
            "Experts believe",
            None,
        ),
        (
            "chatbot-phrase",
            "I hope this helps.",
            "I hope this helps",
            None,
        ),
        (
            "generic-conclusion",
            "The future looks bright.",
            "The future looks bright",
            None,
        ),
        (
            "stacked-hedges",
            "It could potentially fail.",
            "could potentially",
            None,
        ),
        ("repeated-word", "The the cat.", "The the", None),
        (
            "long-sentence",
            "One two three four.",
            "One two three four",
            None,
        ),
        (
            "passive-voice",
            "The report was written yesterday.",
            "was written",
            None,
        ),
        ("em-dash", "Read this—then act.", "—", None),
        (
            "not-only-but-also",
            "Not only fast, but also small.",
            "Not only fast, but also",
            None,
        ),
    ];
    assert_eq!(cases.len() + 6, BUILTIN_RULES.len());
    for (rule, source, matched, replacement) in cases {
        let config = Config {
            max_sentence_words: 3,
            ..Config::default()
        };
        let doc = Document::parse(source);
        let diagnostics = Linter::new(config).check(&doc);
        let found: Vec<_> = diagnostics.iter().filter(|d| d.rule == rule).collect();
        assert_eq!(found.len(), 1, "{rule}: {diagnostics:?}");
        assert_eq!(doc.text(found[0].span), Some(matched), "{rule}");
        assert_eq!(
            found[0].suggestion.as_ref().map(|s| s.replacement.as_str()),
            replacement,
            "{rule}"
        );
    }
}

#[test]
fn clean_prose_and_punctuation_do_not_trigger_phrase_matches() {
    for source in [
        "The parser stores byte offsets. Run the tests before merging.",
        "The dog runs. The cat sleeps.",
        "Use the tools, the tests, and the report.",
        "In order, to proceed, check the file.",
        "We may, potentially, revisit this.",
        "The leverages differ from the utilization rates.",
    ] {
        assert!(
            Linter::default().check(&Document::parse(source)).is_empty(),
            "{source}"
        );
    }
}

#[test]
fn matches_are_case_insensitive_unicode_safe_and_sentence_local() {
    let doc = Document::parse(
        "Élodie: IN ORDER TO help, utilize tools. Experts. Believe nothing.\n\nThe the end.",
    );
    let diagnostics = Linter::default().check(&doc);
    assert_eq!(
        diagnostics
            .iter()
            .map(|d| d.rule.as_str())
            .collect::<Vec<_>>(),
        ["wordy-phrase", "plain-word", "repeated-word"]
    );
    assert_eq!(
        diagnostics[0].suggestion.as_ref().unwrap().replacement,
        "TO"
    );
    assert_eq!(doc.text(diagnostics[0].span), Some("IN ORDER TO"));
    assert_eq!(doc.position(diagnostics[0].span.start).unwrap().column, 9);
    assert_eq!(doc.position(diagnostics[2].span.start).unwrap().line, 3);
}

#[test]
fn config_disables_rules_and_overrides_severity() {
    let config: Config = toml::from_str(
        "max_sentence_words = 4\ndisabled = ['plain-word']\n[severity]\nrepeated-word = 'error'\n",
    )
    .unwrap();
    let linter = Linter::new(config);
    linter.validate().unwrap();
    let ds = linter.check(&Document::parse("Utilize the the tool."));
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].rule, "repeated-word");
    assert_eq!(ds[0].severity, Severity::Error);
    assert!(toml::from_str::<Config>("max_sentnce_words = 10").is_err());
    let config = Config {
        max_sentence_words: 0,
        ..Config::default()
    };
    assert!(Linter::new(config).validate().is_err());
    let mut config = Config::default();
    config.disabled.insert("typo".into());
    assert_eq!(
        Linter::new(config).validate().unwrap_err(),
        "unknown rule ID: typo"
    );
}

struct Custom;
impl Rule for Custom {
    fn info(&self) -> RuleInfo {
        RuleInfo {
            id: "custom",
            description: "Flag the first word",
            severity: Severity::Error,
        }
    }
    fn check(&self, doc: &Document, out: &mut Vec<Diagnostic>) {
        if let Some(word) = doc.sentences().flat_map(|s| s.words()).next() {
            out.push(Diagnostic {
                rule: "custom".into(),
                severity: Severity::Error,
                span: word.span,
                message: "Example".into(),
                suggestion: None,
            });
        }
    }
}

#[test]
fn custom_rules_use_the_same_configuration_and_source_spans() {
    let doc = Document::parse("Hello.");
    let linter = Linter::default().with_rule(Custom);
    linter.validate().unwrap();
    let ds = linter.check(&doc);
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].rule, "custom");
    assert_eq!(doc.text(ds[0].span), Some("Hello"));
    let mut config = Config::default();
    config.disabled.insert("custom".into());
    let linter = Linter::new(config).with_rule(Custom);
    linter.validate().unwrap();
    assert!(linter.check(&doc).is_empty());
    assert!(
        Linter::default()
            .with_rule(Custom)
            .with_rule(Custom)
            .validate()
            .is_err()
    );
}
