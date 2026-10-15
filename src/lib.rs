//! Source-preserving English text structure and explainable writing checks.
//!
//! ```
//! use englint::{Document, Linter};
//! let document = Document::parse("In order to help, utilize plain words.");
//! let diagnostics = Linter::default().check(&document);
//! assert_eq!(diagnostics[0].rule, "wordy-phrase");
//! assert_eq!(document.text(diagnostics[0].span), Some("In order to"));
//! ```
//! # Check a TIP
//!
//! ```
//! use englint::{Config, Document, Linter, Metrics, Profile};
//! let linter = Linter::new(Config { profile: Profile::Tip, ..Config::default() });
//! linter.validate()?;
//! let document = Document::parse_markdown("## Abstract\n\nA draft proposal.");
//! let diagnostics = linter.check(&document);
//! assert_eq!(Metrics::measure(&document, &diagnostics).findings, 7);
//! # Ok::<(), String>(())
//! ```
mod ast;
mod lint;
mod markdown;
mod profile;

pub use profile::{ForbiddenTerm, Metrics, Profile};

pub use ast::{Document, Paragraph, Position, Sentence, Span, Token, TokenKind};
pub use lint::{BUILTIN_RULES, Config, Diagnostic, Linter, Rule, RuleInfo, Severity, Suggestion};
