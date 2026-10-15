use crate::{Document, ForbiddenTerm, Profile, Sentence, Span, TokenKind};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Advice,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Suggestion {
    pub replacement: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub rule: String,
    pub severity: Severity,
    pub span: Span,
    pub message: String,
    /// A proposed edit for human review, not a guaranteed meaning-preserving fix.
    pub suggestion: Option<Suggestion>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct RuleInfo {
    pub id: &'static str,
    pub description: &'static str,
    pub severity: Severity,
}

pub const BUILTIN_RULES: &[RuleInfo] = &[
    RuleInfo {
        id: "forbidden-term",
        description: "Avoid configured words and phrases",
        severity: Severity::Error,
    },
    RuleInfo {
        id: "required-section",
        description: "Include each required Markdown section",
        severity: Severity::Error,
    },
    RuleInfo {
        id: "empty-section",
        description: "Provide content beneath required section headings",
        severity: Severity::Error,
    },
    RuleInfo {
        id: "duplicate-section",
        description: "Use each required section heading once",
        severity: Severity::Warning,
    },
    RuleInfo {
        id: "vague-spec",
        description: "Replace vague requirements with measurable behavior",
        severity: Severity::Warning,
    },
    RuleInfo {
        id: "placeholder",
        description: "Resolve unfinished specification text",
        severity: Severity::Error,
    },
    RuleInfo {
        id: "plain-word",
        description: "Prefer familiar words to inflated vocabulary",
        severity: Severity::Advice,
    },
    RuleInfo {
        id: "wordy-phrase",
        description: "Shorten phrases that add no meaning",
        severity: Severity::Warning,
    },
    RuleInfo {
        id: "vague-attribution",
        description: "Name the source of an attributed claim",
        severity: Severity::Warning,
    },
    RuleInfo {
        id: "chatbot-phrase",
        description: "Remove conversational boilerplate from prose",
        severity: Severity::Advice,
    },
    RuleInfo {
        id: "generic-conclusion",
        description: "Replace stock conclusions with a specific fact",
        severity: Severity::Advice,
    },
    RuleInfo {
        id: "stacked-hedges",
        description: "Avoid multiple hedges in a row",
        severity: Severity::Warning,
    },
    RuleInfo {
        id: "repeated-word",
        description: "Check adjacent duplicate words",
        severity: Severity::Warning,
    },
    RuleInfo {
        id: "long-sentence",
        description: "Consider splitting sentences over the word limit",
        severity: Severity::Advice,
    },
    RuleInfo {
        id: "passive-voice",
        description: "Check possible passive constructions",
        severity: Severity::Advice,
    },
    RuleInfo {
        id: "em-dash",
        description: "Consider a period or comma instead of an em dash",
        severity: Severity::Advice,
    },
    RuleInfo {
        id: "not-only-but-also",
        description: "State the claim without stock contrast framing",
        severity: Severity::Advice,
    },
];

/// Settings for built-in and custom rules. Unknown fields are rejected by serde.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub max_sentence_words: usize,
    pub profile: Profile,
    pub forbidden: Vec<ForbiddenTerm>,
    /// None uses profile defaults; an empty list disables section requirements.
    pub required_sections: Option<Vec<String>>,
    pub disabled: BTreeSet<String>,
    pub severity: BTreeMap<String, Severity>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_sentence_words: 30,
            profile: Profile::Plain,
            forbidden: Vec::new(),
            required_sections: None,
            disabled: BTreeSet::new(),
            severity: BTreeMap::new(),
        }
    }
}

/// Custom rules inspect immutable source structure and return source-based diagnostics.
pub trait Rule: Send + Sync {
    fn info(&self) -> RuleInfo;
    fn check(&self, document: &Document, diagnostics: &mut Vec<Diagnostic>);
}

pub struct Linter {
    config: Config,
    custom: Vec<Box<dyn Rule>>,
}

impl Default for Linter {
    fn default() -> Self {
        Self::new(Config::default())
    }
}

impl Linter {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            custom: Vec::new(),
        }
    }

    pub fn with_rule(mut self, rule: impl Rule + 'static) -> Self {
        self.custom.push(Box::new(rule));
        self
    }

    /// Reject misspelled rule IDs and invalid thresholds before checking documents.
    pub fn validate(&self) -> Result<(), String> {
        if self.config.max_sentence_words == 0 {
            return Err("max_sentence_words must be greater than zero".into());
        }
        for term in &self.config.forbidden {
            let parsed = Document::parse(&term.term);
            let words: Vec<_> = parsed.sentences().flat_map(|s| s.words()).collect();
            if words.is_empty()
                || term.reason.trim().is_empty()
                || !contiguous(&parsed, &words)
                || words.first().unwrap().span.start != 0
                || words.last().unwrap().span.end != term.term.len()
            {
                return Err("forbidden terms must contain only words separated by whitespace and a nonempty reason".into());
            }
        }
        if let Some(sections) = &self.config.required_sections {
            let mut seen = BTreeSet::new();
            for section in sections {
                if section.trim().is_empty() || !seen.insert(section.trim().to_lowercase()) {
                    return Err("required_sections must contain nonempty, unique titles".into());
                }
            }
        }
        let mut ids: BTreeSet<_> = BUILTIN_RULES.iter().map(|r| r.id).collect();
        for rule in &self.custom {
            if !ids.insert(rule.info().id) {
                return Err(format!("duplicate rule ID: {}", rule.info().id));
            }
        }
        for id in self
            .config
            .disabled
            .iter()
            .chain(self.config.severity.keys())
        {
            if !ids.contains(id.as_str()) {
                return Err(format!("unknown rule ID: {id}"));
            }
        }
        Ok(())
    }

    pub fn check(&self, document: &Document) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for sentence in document.sentences() {
            check_sentence(
                document,
                sentence,
                self.config.max_sentence_words,
                &mut diagnostics,
            );
        }
        self.check_policy(document, &mut diagnostics);
        for rule in &self.custom {
            if !self.config.disabled.contains(rule.info().id) {
                rule.check(document, &mut diagnostics);
            }
        }
        diagnostics.retain(|d| !self.config.disabled.contains(&d.rule));
        for diagnostic in &mut diagnostics {
            if let Some(severity) = self.config.severity.get(&diagnostic.rule) {
                diagnostic.severity = *severity;
            }
        }
        diagnostics.sort_by(|a, b| {
            (a.span.start, a.span.end, &a.rule).cmp(&(b.span.start, b.span.end, &b.rule))
        });
        diagnostics
    }
    fn check_policy(&self, document: &Document, out: &mut Vec<Diagnostic>) {
        for sentence in document.sentences() {
            for term in &self.config.forbidden {
                check_term(
                    document,
                    sentence,
                    &term.term,
                    "forbidden-term",
                    &term.reason,
                    term.replacement.as_deref(),
                    out,
                );
            }
            if self.config.profile != Profile::Plain {
                for term in [
                    "fast",
                    "efficient",
                    "seamless",
                    "robust",
                    "user-friendly",
                    "as appropriate",
                    "as needed",
                    "etc",
                    "and so on",
                ] {
                    check_term(
                        document,
                        sentence,
                        term,
                        "vague-spec",
                        "Specify the bound, condition, or exhaustive behavior; review context before changing this term.",
                        None,
                        out,
                    );
                }
                for term in ["TODO", "TBD", "FIXME", "XXX"] {
                    check_term(
                        document,
                        sentence,
                        term,
                        "placeholder",
                        "Resolve this placeholder before review.",
                        None,
                        out,
                    );
                }
            }
        }
        let defaults = self
            .config
            .profile
            .required_sections()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();
        let required = self.config.required_sections.as_ref().unwrap_or(&defaults);
        if required.is_empty() {
            return;
        }
        let headings = if document.markdown {
            crate::markdown::headings(document.source())
        } else {
            Vec::new()
        };
        for title in required {
            let matches: Vec<_> = headings
                .iter()
                .filter(|h| h.title.trim().eq_ignore_ascii_case(title.trim()))
                .collect();
            if matches.is_empty() {
                emit(
                    out,
                    "required-section",
                    Span { start: 0, end: 0 },
                    format!("Add a Markdown heading and content for '{title}'."),
                    None,
                );
            }
            for (index, heading) in matches.iter().enumerate() {
                let span = Span {
                    start: heading.range.start,
                    end: heading.range.end,
                };
                if index > 0 {
                    emit(
                        out,
                        "duplicate-section",
                        span,
                        format!("Merge duplicate '{title}' sections."),
                        None,
                    );
                }
                if !heading.content {
                    emit(
                        out,
                        "empty-section",
                        span,
                        format!("Explain '{title}', or state why it does not apply."),
                        None,
                    );
                }
            }
        }
    }
}

struct Phrase {
    rule: &'static str,
    text: &'static str,
    message: &'static str,
    replacement: Option<&'static str>,
}

const PHRASES: &[Phrase] = &[
    Phrase {
        rule: "wordy-phrase",
        text: "in order to",
        message: "Use 'to'.",
        replacement: Some("to"),
    },
    Phrase {
        rule: "wordy-phrase",
        text: "due to the fact that",
        message: "Use 'because'.",
        replacement: Some("because"),
    },
    Phrase {
        rule: "wordy-phrase",
        text: "it is important to note that",
        message: "Remove this preamble and state the point.",
        replacement: Some(""),
    },
    Phrase {
        rule: "wordy-phrase",
        text: "it is worth noting that",
        message: "Remove this preamble and state the point.",
        replacement: Some(""),
    },
    Phrase {
        rule: "wordy-phrase",
        text: "in the event that",
        message: "Use 'if'.",
        replacement: Some("if"),
    },
    Phrase {
        rule: "wordy-phrase",
        text: "at this point in time",
        message: "Use 'now'.",
        replacement: Some("now"),
    },
    Phrase {
        rule: "wordy-phrase",
        text: "serves as",
        message: "Consider 'is' if it preserves the meaning.",
        replacement: Some("is"),
    },
    Phrase {
        rule: "vague-attribution",
        text: "experts believe",
        message: "Name the experts and cite their evidence.",
        replacement: None,
    },
    Phrase {
        rule: "vague-attribution",
        text: "studies show",
        message: "Name or cite the study.",
        replacement: None,
    },
    Phrase {
        rule: "vague-attribution",
        text: "some critics argue",
        message: "Name the critics or remove the attribution.",
        replacement: None,
    },
    Phrase {
        rule: "vague-attribution",
        text: "industry reports suggest",
        message: "Identify the report and its date.",
        replacement: None,
    },
    Phrase {
        rule: "chatbot-phrase",
        text: "i hope this helps",
        message: "Remove this stock sign-off from standalone prose.",
        replacement: None,
    },
    Phrase {
        rule: "chatbot-phrase",
        text: "great question",
        message: "Consider starting with the answer.",
        replacement: None,
    },
    Phrase {
        rule: "chatbot-phrase",
        text: "let me know if you",
        message: "Consider removing this stock invitation.",
        replacement: None,
    },
    Phrase {
        rule: "generic-conclusion",
        text: "the future looks bright",
        message: "End with a specific fact or next step.",
        replacement: None,
    },
    Phrase {
        rule: "generic-conclusion",
        text: "in today's ever-evolving landscape",
        message: "Replace this stock opening with the concrete subject.",
        replacement: None,
    },
];

const WORDS: &[(&str, &str)] = &[
    ("utilize", "use"),
    ("utilizes", "uses"),
    ("utilizing", "using"),
    ("leverage", "use"),
    ("facilitate", "help"),
    ("numerous", "many"),
    ("additionally", "also"),
    ("delve", "examine"),
    ("pivotal", "important"),
    ("tapestry", "Describe the concrete subject."),
];

fn emit(
    out: &mut Vec<Diagnostic>,
    rule: &str,
    span: Span,
    message: impl Into<String>,
    replacement: Option<String>,
) {
    let severity = BUILTIN_RULES
        .iter()
        .find(|r| r.id == rule)
        .unwrap()
        .severity;
    out.push(Diagnostic {
        rule: rule.into(),
        severity,
        span,
        message: message.into(),
        suggestion: replacement.map(|replacement| Suggestion { replacement }),
    });
}

fn check_sentence(doc: &Document, sentence: &Sentence, limit: usize, out: &mut Vec<Diagnostic>) {
    let words: Vec<_> = sentence.words().collect();
    let lower: Vec<_> = words
        .iter()
        .map(|t| doc.text(t.span).unwrap().to_lowercase().replace('’', "'"))
        .collect();
    for phrase in PHRASES {
        let pattern: Vec<_> = phrase.text.split_whitespace().collect();
        for (i, window) in lower.windows(pattern.len()).enumerate() {
            if window
                .iter()
                .map(String::as_str)
                .eq(pattern.iter().copied())
                && contiguous(doc, &words[i..i + pattern.len()])
            {
                let span = Span {
                    start: words[i].span.start,
                    end: words[i + pattern.len() - 1].span.end,
                };
                let replacement = phrase
                    .replacement
                    .map(|text| match_case(doc.text(span).unwrap(), text));
                emit(out, phrase.rule, span, phrase.message, replacement);
            }
        }
    }
    for (i, word) in lower.iter().enumerate() {
        if let Some((_, replacement)) = WORDS.iter().find(|(candidate, _)| word == candidate) {
            let span = words[i].span;
            let (message, suggestion) = if *word == "tapestry" {
                (
                    "Check whether this metaphor adds meaning. Describe the subject directly."
                        .into(),
                    None,
                )
            } else {
                (
                    format!("Consider '{replacement}' if it preserves the meaning."),
                    Some(match_case(doc.text(span).unwrap(), replacement)),
                )
            };
            emit(out, "plain-word", span, message, suggestion);
        }
    }
    for i in 1..words.len() {
        if !contiguous(doc, &words[i - 1..=i]) {
            continue;
        }
        let span = Span {
            start: words[i - 1].span.start,
            end: words[i].span.end,
        };
        if lower[i - 1] == lower[i] {
            emit(
                out,
                "repeated-word",
                span,
                "Check this repeated word. Some repetitions are grammatical.",
                None,
            );
        }
        if hedge(&lower[i - 1]) && hedge(&lower[i]) {
            emit(
                out,
                "stacked-hedges",
                span,
                "Choose one hedge, or state the evidence directly.",
                None,
            );
        }
        if matches!(
            lower[i - 1].as_str(),
            "is" | "are" | "was" | "were" | "be" | "been" | "being"
        ) && participle(&lower[i])
        {
            emit(
                out,
                "passive-voice",
                span,
                "Possible passive voice. Name the actor if the actor matters.",
                None,
            );
        }
    }
    if words.len() > limit {
        emit(
            out,
            "long-sentence",
            Span {
                start: words[0].span.start,
                end: words.last().unwrap().span.end,
            },
            format!(
                "This sentence has {} words; the configured limit is {limit}. Consider splitting it.",
                words.len()
            ),
            None,
        );
    }
    for token in &sentence.tokens {
        if token.kind == TokenKind::Punctuation && doc.text(token.span) == Some("—") {
            emit(
                out,
                "em-dash",
                token.span,
                "Consider a period or comma to separate the thought.",
                None,
            );
        }
    }
    if let Some(start) = lower.windows(2).position(|w| w == ["not", "only"])
        && contiguous(doc, &words[start..start + 2])
        && let Some(end) = lower[start + 2..]
            .windows(2)
            .position(|w| w == ["but", "also"])
    {
        let finish = start + 2 + end + 1;
        if contiguous(doc, &words[finish - 1..=finish]) {
            emit(
                out,
                "not-only-but-also",
                Span {
                    start: words[start].span.start,
                    end: words[finish].span.end,
                },
                "Consider stating both claims directly without 'not only ... but also'.",
                None,
            );
        }
    }
}

fn contiguous(doc: &Document, words: &[&crate::Token]) -> bool {
    words.windows(2).all(|w| {
        doc.text(Span {
            start: w[0].span.end,
            end: w[1].span.start,
        })
        .is_some_and(|gap| !gap.is_empty() && gap.chars().all(char::is_whitespace))
    })
}

fn hedge(word: &str) -> bool {
    matches!(
        word,
        "could"
            | "may"
            | "might"
            | "perhaps"
            | "possibly"
            | "potentially"
            | "arguably"
            | "seemingly"
    )
}

fn participle(word: &str) -> bool {
    (word.len() > 4 && word.ends_with("ed"))
        || matches!(
            word,
            "built"
                | "written"
                | "known"
                | "given"
                | "seen"
                | "done"
                | "made"
                | "taken"
                | "shown"
                | "found"
                | "sent"
                | "read"
        )
}

fn match_case(original: &str, replacement: &str) -> String {
    if original
        .chars()
        .filter(|c| c.is_alphabetic())
        .all(char::is_uppercase)
    {
        replacement.to_uppercase()
    } else if original.chars().next().is_some_and(char::is_uppercase) {
        let mut chars = replacement.chars();
        chars.next().map_or_else(String::new, |first| {
            first.to_uppercase().collect::<String>() + chars.as_str()
        })
    } else {
        replacement.into()
    }
}

fn check_term(
    doc: &Document,
    sentence: &Sentence,
    term: &str,
    rule: &str,
    reason: &str,
    replacement: Option<&str>,
    out: &mut Vec<Diagnostic>,
) {
    let pattern: Vec<_> = term
        .split_whitespace()
        .map(|w| w.to_lowercase().replace('’', "'"))
        .collect();
    if pattern.is_empty() {
        return;
    }
    let words: Vec<_> = sentence.words().collect();
    for window in words.windows(pattern.len()) {
        if contiguous(doc, window)
            && window.iter().zip(&pattern).all(|(word, expected)| {
                doc.text(word.span)
                    .unwrap()
                    .to_lowercase()
                    .replace('’', "'")
                    == *expected
            })
        {
            let span = Span {
                start: window[0].span.start,
                end: window.last().unwrap().span.end,
            };
            emit(
                out,
                rule,
                span,
                reason,
                replacement.map(|r| match_case(doc.text(span).unwrap(), r)),
            );
        }
    }
}
