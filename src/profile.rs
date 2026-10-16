use crate::{Diagnostic, Document, Severity};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Style policies, not a judgment of factual or protocol correctness.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Profile {
    #[default]
    Plain,
    TechnicalSpec,
    Tip,
}

impl Profile {
    pub fn required_sections(self) -> &'static [&'static str] {
        match self {
            Self::Plain => &[],
            Self::TechnicalSpec => &[
                "Summary",
                "Motivation",
                "Specification",
                "Compatibility",
                "Security",
                "Test plan",
            ],
            Self::Tip => &[],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForbiddenTerm {
    pub term: String,
    pub reason: String,
    pub replacement: Option<String>,
}

/// Counts describe the text checked, not a universal writing-quality score.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Metrics {
    pub words: usize,
    pub sentences: usize,
    pub longest_sentence_words: usize,
    pub findings: usize,
    pub findings_per_100_words: f64,
    pub by_rule: BTreeMap<String, usize>,
    pub by_severity: BTreeMap<String, usize>,
}

impl Metrics {
    pub fn measure(document: &Document, diagnostics: &[Diagnostic]) -> Self {
        let counts: Vec<_> = document
            .sentences()
            .map(|s| s.words().count())
            .filter(|&n| n > 0)
            .collect();
        let words = counts.iter().sum();
        let mut by_rule = BTreeMap::new();
        let mut by_severity = BTreeMap::new();
        for diagnostic in diagnostics {
            *by_rule.entry(diagnostic.rule.clone()).or_default() += 1;
            let severity = match diagnostic.severity {
                Severity::Advice => "advice",
                Severity::Warning => "warning",
                Severity::Error => "error",
            };
            *by_severity.entry(severity.into()).or_default() += 1;
        }
        Self {
            words,
            sentences: counts.len(),
            longest_sentence_words: counts.into_iter().max().unwrap_or(0),
            findings: diagnostics.len(),
            findings_per_100_words: if words == 0 {
                0.0
            } else {
                diagnostics.len() as f64 * 100.0 / words as f64
            },
            by_rule,
            by_severity,
        }
    }
}
