use englint::{Config, Document, ForbiddenTerm, Linter, Metrics, Profile, Severity};

fn policy(profile: Profile) -> Config {
    Config {
        profile,
        ..Config::default()
    }
}

#[test]
fn markdown_skips_code_metadata_html_images_and_link_destinations() {
    let source = "---\ntitle: utilize\n---\n# A title\n\nÉlodie: utilize [tools](https://utilize.test). `utilize`\n\n```rust\nutilize();\n```\n\n    utilize();\n\n<!-- utilize -->\n\n![utilize](utilize.png)\n\n<div>utilize</div>\n";
    let doc = Document::parse_markdown(source);
    assert_eq!(doc.source(), source);
    let findings = Linter::default().check(&doc);
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(doc.text(findings[0].span), Some("utilize"));
    assert_eq!(
        findings[0].span.start,
        source.find("Élodie: utilize").unwrap() + "Élodie: ".len()
    );
    assert_eq!(doc.position(findings[0].span.start).unwrap().line, 6);
}

#[test]
fn markdown_retains_visible_link_labels_and_soft_line_breaks() {
    let doc = Document::parse_markdown(
        "[Utilize](https://example.com) tools.\n\nIn order\nto help.\n\nIn `x` order to help.",
    );
    let findings = Linter::default().check(&doc);
    assert_eq!(
        findings.iter().map(|d| d.rule.as_str()).collect::<Vec<_>>(),
        ["plain-word", "wordy-phrase"]
    );
    assert_eq!(doc.text(findings[1].span), Some("In order\nto"));
}

#[test]
fn forbidden_phrases_have_precise_spans_case_and_configuration() {
    let mut config = Config {
        forbidden: vec![ForbiddenTerm {
            term: "game changer".into(),
            reason: "Describe the observable change.".into(),
            replacement: Some("change".into()),
        }],
        ..Config::default()
    };
    let linter = Linter::new(config.clone());
    linter.validate().unwrap();
    let doc = Document::parse("Élodie: GAME CHANGER. Game, changer. Game. Changer. Game changers.");
    let ds = linter.check(&doc);
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].rule, "forbidden-term");
    assert_eq!(ds[0].severity, Severity::Error);
    assert_eq!(doc.text(ds[0].span), Some("GAME CHANGER"));
    assert_eq!(ds[0].suggestion.as_ref().unwrap().replacement, "CHANGE");
    config.disabled.insert("forbidden-term".into());
    assert!(Linter::new(config).check(&doc).is_empty());
}

#[test]
fn invalid_policies_fail_before_linting() {
    for term in ["", " ", "hello, there", "42", "word!", " word"] {
        let mut config = Config::default();
        config.forbidden.push(ForbiddenTerm {
            term: term.into(),
            reason: "reason".into(),
            replacement: None,
        });
        assert!(Linter::new(config).validate().is_err(), "{term}");
    }
    for sections in [vec!["".into()], vec!["Summary".into(), "summary".into()]] {
        let config = Config {
            required_sections: Some(sections),
            ..Config::default()
        };
        assert!(Linter::new(config).validate().is_err());
    }
    assert!(toml::from_str::<Config>("profile = 'typo'").is_err());
    assert!(
        toml::from_str::<Config>(
            "[[forbidden]]\nterm = 'word'\nreason = 'why'\nreplacment = 'other'"
        )
        .is_err()
    );
}

#[test]
fn tip_does_not_require_or_enforce_section_structure() {
    let linter = Linter::new(policy(Profile::Tip));
    for source in [
        "The validator checks the signature before execution.",
        "## Abstract\n\n## Motivation\n\nA reason.\n\n## Motivation\n\nAnother reason.",
    ] {
        assert!(linter.check(&Document::parse_markdown(source)).is_empty());
    }
}

#[test]
fn explicit_section_policy_still_checks_missing_empty_and_duplicate_headings() {
    let mut config = policy(Profile::Tip);
    config.required_sections = Some(vec![
        "Abstract".into(),
        "Motivation".into(),
        "Specification".into(),
    ]);
    let linter = Linter::new(config);
    let doc = Document::parse_markdown(
        "## Abstract\n\n## Motivation\n\nA reason.\n\n## Motivation\n\nAnother reason.",
    );
    let ds = linter.check(&doc);
    assert_eq!(
        ds.iter().filter(|d| d.rule == "required-section").count(),
        1
    );
    assert_eq!(ds.iter().filter(|d| d.rule == "empty-section").count(), 1);
    assert_eq!(
        ds.iter().filter(|d| d.rule == "duplicate-section").count(),
        1
    );
}

#[test]
fn headings_in_code_are_not_sections_and_nested_content_counts() {
    let mut config = policy(Profile::TechnicalSpec);
    config.required_sections = Some(vec!["Specification".into()]);
    let linter = Linter::new(config);
    let fake = Document::parse_markdown("```markdown\n# Specification\nContent.\n```");
    assert_eq!(linter.check(&fake)[0].rule, "required-section");
    for source in [
        "# Specification\n\n## Encoding\n\nBytes.",
        "Specification\n=============\n\nBytes.",
        "# **Specification**\n\nBytes.",
    ] {
        assert!(
            linter.check(&Document::parse_markdown(source)).is_empty(),
            "{source}"
        );
    }
    assert_eq!(
        linter.check(&Document::parse_markdown(
            "# Specification\n\n<!-- write this -->"
        ))[0]
            .rule,
        "empty-section"
    );
}

#[test]
fn technical_rules_are_profile_specific_and_can_be_overridden() {
    let source = "The fast path is robust. TODO: define bounds.";
    assert!(Linter::default().check(&Document::parse(source)).is_empty());
    let mut config = policy(Profile::TechnicalSpec);
    config.required_sections = Some(vec![]);
    config
        .severity
        .insert("vague-spec".into(), Severity::Advice);
    let ds = Linter::new(config).check(&Document::parse(source));
    assert_eq!(
        ds.iter().map(|d| d.rule.as_str()).collect::<Vec<_>>(),
        ["vague-spec", "vague-spec", "placeholder"]
    );
    assert_eq!(ds[0].severity, Severity::Advice);
    assert_eq!(ds[2].severity, Severity::Error);
}

#[test]
fn metrics_count_actual_checked_words_and_findings() {
    let doc = Document::parse("Utilize tools. In order to help.");
    let ds = Linter::default().check(&doc);
    let metrics = Metrics::measure(&doc, &ds);
    assert_eq!(metrics.words, 6);
    assert_eq!(metrics.sentences, 2);
    assert_eq!(metrics.longest_sentence_words, 4);
    assert_eq!(metrics.findings, 2);
    assert!((metrics.findings_per_100_words - 100.0 / 3.0).abs() < 1e-10);
    assert_eq!(metrics.by_rule["wordy-phrase"], 1);
    assert_eq!(metrics.by_severity["advice"], 1);
    let empty = Metrics::measure(&Document::parse(""), &[]);
    assert_eq!(empty.findings_per_100_words, 0.0);
    assert_eq!(empty.longest_sentence_words, 0);
}

#[test]
fn markdown_spans_stay_valid_across_generated_syntax_and_unicode() {
    let pieces = [
        "é",
        "界",
        "**word**",
        "`utilize`",
        "&amp;",
        "\\*",
        "\n\n",
        "# title\n",
        "![alt](url)",
        "[word](url)",
        "<!-- x -->",
        "\r\n",
    ];
    let linter = Linter::new(policy(Profile::Tip));
    for a in pieces {
        for b in pieces {
            for c in pieces {
                let source = format!("{a}{b}{c}");
                let document = Document::parse_markdown(&source);
                assert_eq!(document.source(), source);
                for token in document.sentences().flat_map(|s| &s.tokens) {
                    assert!(document.text(token.span).is_some());
                    assert!(token.span.start < token.span.end);
                }
                for diagnostic in linter.check(&document) {
                    assert!(document.text(diagnostic.span).is_some());
                    assert!(document.position(diagnostic.span.start).is_some());
                }
            }
        }
    }
}
