# englint

A Rust library and CLI for a source-preserving English text AST, configurable writing checks, and technical specification and Tempo Improvement Proposal profiles.

Formerly `english-ast`. The crate, import, binary, and repository are now `englint`. The parser and writing rules live in this one repository.

The AST contains paragraphs, sentences, words, numbers, punctuation, and whitespace. The linter returns byte spans, severity levels, explanations, and optional replacement suggestions. It has no model downloads or network calls at runtime.

## Check a proposal

```sh
cargo install --git https://github.com/0xKitsune/englint --locked
englint proposal.md --profile tip --stats --fail-on advice
englint spec.md --profile technical-spec --format json
englint proposal.md --config profiles/tip.toml --fail-on advice
```

The built-in profiles work without a configuration file. The files in `profiles/` add example vocabulary policies and are available in a repository checkout.

Use [the authoring prompt](examples/prompt.md) with the implementation context and requirements for your proposal. The [before/after examples](examples/results.md) include prose, a technical specification, and a fictional TIP. Each has its source and generated JSON diagnostics checked into `examples/`.

## Use the library

```rust
use englint::{Document, Linter};

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
printf 'In order to help, utilize plain words.\n' | englint
englint essay.txt notes.txt --format json
englint essay.txt --ast
englint --list-rules
englint essay.txt --config englint.toml --disable em-dash
```

The first command that checks prose prints:

```text
<stdin>:1:1: Warning[wordy-phrase]: Use 'to'.
  suggestion: "To"
<stdin>:1:19: Advice[plain-word]: Consider 'use' if it preserves the meaning.
  suggestion: "use"
```

Omit file arguments to read stdin, or use `-` once alongside file paths. Inputs must be UTF-8. `--ast` always prints JSON. Lint JSON is an array of per-file reports, each with `path`, `diagnostics`, and `metrics`. AST JSON is an array with `path` and `document`.

Exit codes are `0` for no findings at the selected threshold, `1` for findings, and `2` for argument, configuration, or I/O errors. The default threshold is `warning`; advice still appears in output. Use `--fail-on advice`, `--fail-on error`, or `--fail-on never` to change it. The CLI prints diagnostics to stdout and operational errors to stderr. It reads all inputs before emitting reports, so a missing file cannot leave a partial JSON report.

## Configure the rules

Configuration is explicit; the CLI only loads a file passed with `--config`. CLI flags override the profile and word limit and add to disabled rules. A profile selects extra rules and default required sections. It does not silently change severities or the word limit.

```toml
profile = "plain"
max_sentence_words = 30
disabled = ["em-dash", "passive-voice"]

[severity]
wordy-phrase = "error"
plain-word = "advice"
```

Rule IDs and TOML fields are checked for spelling. Severity values are `advice`, `warning`, and `error`. The word limit must be positive. Library users pass a `Config` to `Linter::new` and call `validate()` after registering custom rules.

| Rule | Default | What it checks |
| --- | --- | --- |
| `forbidden-term` | error | Configured whole words and whitespace-separated phrases |
| `required-section` | error | Missing required Markdown headings |
| `empty-section` | error | Required headings with no text or code content |
| `duplicate-section` | warning | Repeated required headings |
| `vague-spec` | warning | Technical profiles: vague terms such as “fast” and “as needed” |
| `placeholder` | error | Technical profiles: TODO, TBD, FIXME, and XXX in prose |
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

## Profiles and forbidden terms

`plain` enables the general prose checks. `technical-spec` and `tip` also check vague requirements and unresolved placeholders. All profiles accept vocabulary policies:

```toml
profile = "technical-spec"
max_sentence_words = 25
required_sections = ["Summary", "Specification", "Security", "Test plan"]

[[forbidden]]
term = "game changer"
reason = "Describe the observable change."

[[forbidden]]
term = "utilize"
reason = "Use the familiar word."
replacement = "use"

[severity]
forbidden-term = "error"
```

Terms match whole words, ignore case, and never cross punctuation or sentence boundaries. Terms must contain words separated by whitespace. A replacement is optional. Use `disabled = ["forbidden-term"]` to disable the vocabulary policy.

The TIP profile does not require sections or enforce a document template. The default technical-spec sections are Summary, Motivation, Specification, Compatibility, Security, and Test plan. Set `required_sections` explicitly if your project needs a section policy.

Section matching ignores ASCII case and accepts ATX or setext headings at any level. Content in a subsection counts toward its parent. Code blocks count as section content. Comments alone do not. Override the list with `required_sections`; an empty list disables section requirements. A missing section has an empty span at byte zero. Empty and duplicate sections point to their headings.

Library callers select the same policy:

```rust
use englint::{Config, Document, Linter, Metrics, Profile};

let config = Config { profile: Profile::Tip, ..Config::default() };
let linter = Linter::new(config);
linter.validate()?;
let document = Document::parse_markdown("## Abstract\n\nA draft proposal.");
let diagnostics = linter.check(&document);
let metrics = Metrics::measure(&document, &diagnostics);
Ok::<(), String>(())
```

## Markdown and measurements

The CLI defaults to Markdown for `tip` and `technical-spec`, and plain text for `plain`. Override this with `--input-format markdown` or `--input-format text`. Library callers choose `Document::parse_markdown` or `Document::parse` explicitly. Section checks require Markdown parsing.

Markdown parsing uses CommonMark text ranges. It excludes fenced and indented code, inline code, HTML blocks and tags, images, link destinations, and an initial closed `---` front-matter block. It checks visible link labels, headings, list text, and blockquotes. Inline HTML tags are excluded, but text between them can still be checked. Front matter is excluded, not validated.

The AST preserves the complete original source. Markdown paragraphs represent contiguous prose ranges rather than Markdown block nodes. Formatting markers, escapes, entities, inline code, and links can split a prose range. Phrase matching and sentence counts do not join across those gaps; this can undercount long sentences with inline markup. Entity text uses its source spelling. The parser does not decode it into a separate coordinate system.

JSON always includes checked word count, sentence count, longest sentence word count, total findings, counts by rule and severity, and findings per 100 words. `--stats` prints a compact text summary. Density is `findings * 100 / checked_words`, or zero when no words were checked. Structural findings still count toward the total. Compare drafts with the same profile, input format, and configuration; counts from different policies are not interchangeable.

These measurements identify policy violations, not writing quality in general. Zero findings cannot establish that a proposal is correct, complete, secure, or implementable. The linter does not validate front-matter fields, protocol mechanics, factual claims, citations, or whether a section actually answers its heading. Terms such as “fast” can be precise in context; disable or downgrade rules when appropriate.

## Understand the limits

This is a structural text AST, not a grammatical constituency or dependency parser. It does not assign parts of speech, resolve clauses, infer meaning, check facts, or detect AI authorship. A model-backed parser could later annotate these source spans without changing how callers locate text.

Blank lines separate paragraphs. A sentence ends at `.`, `!`, or `?` followed by whitespace or the paragraph end. Closing quotes and brackets stay with the sentence. Whitespace following a sentence belongs to that sentence. Paragraphs exclude separating blank lines, which remain in `source()`.

Common titles such as `Dr.` and single uppercase initials suppress a sentence break. Decimal numbers, contractions, combining accents, and hyphenated words stay together. These are heuristics. Abbreviations, URLs, ellipses, and unusual dialogue can produce unexpected boundaries. Inputs without terminal punctuation still produce a sentence.

Plain-text mode does not exclude Markdown, HTML, code, links, or quoted passages. Use Markdown mode for proposal files. Neither mode excludes blockquotes by default. Passive-voice checks miss interrupted constructions such as “was carefully written” and can flag adjectives such as “was tired.” Repetition can be grammatical, as in “had had.” Ordinary words can be right in context. Disable rules that do not fit the document.

## Develop

```sh
./scripts/check.sh
```

This runs formatting, Clippy with warnings denied, behavioral and CLI tests, doctests, and the custom-rule example. Tests check literal matched text, suggestions, severities, clean prose, Unicode positions, JSON reports, exit codes, configuration errors, and source preservation across generated input combinations. CI also runs the tests on Rust 1.88.

Regenerate example reports with `python3 scripts/examples.py`. Tests compare the saved JSON with actual CLI output and require each revised example to pass at the advice threshold.

The parser is in `src/ast.rs`, Markdown extraction in `src/markdown.rs`, rules and configuration in `src/lint.rs`, profiles and measurements in `src/profile.rs`, and CLI I/O in `src/main.rs`.

Licensed under MIT.
