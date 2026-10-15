//! Source-preserving English text structure and explainable writing checks.
//!
//! ```
//! use english_ast::{Document, Linter};
//! let document = Document::parse("In order to help, utilize plain words.");
//! let diagnostics = Linter::default().check(&document);
//! assert_eq!(diagnostics[0].rule, "wordy-phrase");
//! assert_eq!(document.text(diagnostics[0].span), Some("In order to"));
//! ```
mod ast;
mod lint;

pub use ast::{Document, Paragraph, Position, Sentence, Span, Token, TokenKind};
pub use lint::{BUILTIN_RULES, Config, Diagnostic, Linter, Rule, RuleInfo, Severity, Suggestion};
