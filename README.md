# english-ast

A Rust library and CLI that parse English prose into a source-preserving text AST and flag specific writing patterns.

The AST contains paragraphs, sentences, words, numbers, punctuation, and whitespace. The linter returns byte spans, severity levels, explanations, and optional replacement suggestions. It has no model downloads or network calls at runtime.

## Use the library

```rust
use english_ast::{Document, Linter};

let document = Document::parse("In order to help, utilize plain words.");
let diagnostics = Linter::default().check(&document);

assert_eq!(diagnostics[0].rule, "wordy-phrase");
assert_eq!(document.text(diagnostics[0].span), Some("In order to"));
assert_eq!(
    diagnostics[0].suggestion.as_ref().unwrap().replacement,
    "To"
);
```

`Document::parse` accepts any string. `source()` returns the original text, including blank lines. `paragraphs()` and `sentences()` expose the tree. `Sentence::words()` excludes numbers, punctuation, and whitespace.

Every span is a half-open UTF-8 byte range. `Document::text(span)` validates the range before slicing. `position(offset)` returns a one-based line and Unicode scalar column. Tabs count as one column; columns are not terminal display widths.

## Run the CLI

Requires Rust 1.88 or later.

```sh
cargo install --path .
printf 'In order to help, utilize plain words.\n' | english-ast
english-ast essay.txt notes.txt --format json
english-ast essay.txt --ast
english-ast --list-rules
english-ast essay.txt --config english-ast.toml --disable em-dash
```

The first command that checks prose prints:

```text
<stdin>:1:1: Warning[wordy-phrase]: Use 'to'.
  suggestion: "To"
<stdin>:1:19: Advice[plain-word]: Consider 'use' if it preserves the meaning.
  suggestion: "use"
```

Omit file arguments to read stdin, or use `-` once alongside file paths. Inputs must be UTF-8. `--ast` always prints JSON. Lint JSON is an array of per-file reports, each with `path` and `diagnostics`. AST JSON is an array with `path` and `document`.

Exit codes are `0` for no findings at the selected threshold, `1` for findings, and `2` for argument, configuration, or I/O errors. The default threshold is `warning`; advice still appears in output. Use `--fail-on advice`, `--fail-on error`, or `--fail-on never` to change it. The CLI prints diagnostics to stdout and operational errors to stderr. It reads all inputs before emitting reports, so a missing file cannot leave a partial JSON report.

## Configure the rules

Configuration is explicit; the CLI only loads a file passed with `--config`. CLI flags override the word limit and add to disabled rules.

```toml
max_sentence_words = 30
disabled = ["em-dash", "passive-voice"]

[severity]
wordy-phrase = "error"
plain-word = "advice"
```

Rule IDs and TOML fields are checked for spelling. Severity values are `advice`, `warning`, and `error`. The word limit must be positive. Library users pass a `Config` to `Linter::new` and call `validate()` after registering custom rules.

| Rule | Default | What it checks |
| --- | --- | --- |
| `plain-word` | advice | A small vocabulary list, such as “utilize” and “numerous” |
| `wordy-phrase` | warning | Preambles and phrases such as “in order to” |
| `vague-attribution` | warning | Claims attributed to unnamed experts, studies, or reports |
| `chatbot-phrase` | advice | Stock praise, invitations, and sign-offs |
| `generic-conclusion` | advice | Stock framing such as “the future looks bright” |
| `stacked-hedges` | warning | Adjacent hedges such as “could potentially” |
| `repeated-word` | warning | Adjacent duplicate words |
| `long-sentence` | advice | Sentences with more than the configured number of words |
| `passive-voice` | advice | A form of “be” followed immediately by a likely participle |
| `em-dash` | advice | Each em dash |
| `not-only-but-also` | advice | “not only … but also” within one sentence |

Phrase checks ignore case and accept whitespace between words. They do not match across punctuation or sentence boundaries. Diagnostics are sorted by start offset, end offset, and rule ID. Replacement suggestions preserve initial capitalization or all-uppercase text. Suggestions need human review; the tool does not edit files.

Implement `Rule` and register it with `Linter::with_rule` to add domain checks. Custom rules share configuration and diagnostic ordering with built-in rules. Rule IDs must be unique. Custom rules must return valid source spans and use their registered ID. See [the custom rule example](examples/custom_rule.rs), which flags TODO markers.

## Understand the limits

This is a structural text AST, not a grammatical constituency or dependency parser. It does not assign parts of speech, resolve clauses, infer meaning, check facts, or detect AI authorship. A model-backed parser could later annotate these source spans without changing how callers locate text.

Blank lines separate paragraphs. A sentence ends at `.`, `!`, or `?` followed by whitespace or the paragraph end. Closing quotes and brackets stay with the sentence. Whitespace following a sentence belongs to that sentence. Paragraphs exclude separating blank lines, which remain in `source()`.

Common titles such as `Dr.` and single uppercase initials suppress a sentence break. Decimal numbers, contractions, combining accents, and hyphenated words stay together. These are heuristics. Abbreviations, URLs, ellipses, and unusual dialogue can produce unexpected boundaries. Inputs without terminal punctuation still produce a sentence.

The input is plain text. Markdown, HTML, code, links, and quoted passages are not excluded. Extract prose before linting those formats. Passive-voice checks miss interrupted constructions such as “was carefully written” and can flag adjectives such as “was tired.” Repetition can be grammatical, as in “had had.” Ordinary words can be right in context. Disable rules that do not fit the document.

## Develop

```sh
./scripts/check.sh
```

This runs formatting, Clippy with warnings denied, behavioral and CLI tests, doctests, and the custom-rule example. Tests check literal matched text, suggestions, severities, clean prose, Unicode positions, JSON reports, exit codes, configuration errors, and source preservation across generated input combinations. CI also runs the tests on Rust 1.88.

The parser is in `src/ast.rs`, rules and configuration are in `src/lint.rs`, and CLI I/O is in `src/main.rs`.

Licensed under MIT.
