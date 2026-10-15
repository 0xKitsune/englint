use clap::{Parser, ValueEnum};
use english_ast::{BUILTIN_RULES, Config, Diagnostic, Document, Linter, Position, Severity};
use serde::Serialize;
use std::{
    fs,
    io::{self, Read},
    path::PathBuf,
    process::ExitCode,
};

/// Parse English prose or check it for specific writing patterns.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// UTF-8 text files. Omit for stdin; '-' also reads stdin once.
    files: Vec<PathBuf>,
    /// Print source structure instead of lint diagnostics.
    #[arg(long, conflicts_with = "list_rules")]
    ast: bool,
    #[arg(long, value_enum, default_value = "text")]
    format: Format,
    /// TOML settings. No implicit config discovery.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Disable a rule by ID. Can be repeated.
    #[arg(long)]
    disable: Vec<String>,
    /// Override the sentence word limit.
    #[arg(long)]
    max_sentence_words: Option<usize>,
    /// Minimum severity that produces exit code 1.
    #[arg(long, value_enum, default_value = "warning")]
    fail_on: Failure,
    /// Print the built-in rule catalog.
    #[arg(long)]
    list_rules: bool,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
}
#[derive(Clone, Copy, ValueEnum)]
enum Failure {
    Advice,
    Warning,
    Error,
    Never,
}

#[derive(Serialize)]
struct LocatedDiagnostic {
    #[serde(flatten)]
    diagnostic: Diagnostic,
    position: Position,
}
#[derive(Serialize)]
struct Report {
    path: String,
    diagnostics: Vec<LocatedDiagnostic>,
}
#[derive(Serialize)]
struct AstReport {
    path: String,
    document: Document,
}

fn main() -> ExitCode {
    let args = Args::parse();
    match run(args) {
        Ok(failed) => ExitCode::from(u8::from(failed)),
        Err(message) => {
            eprintln!("english-ast: {message}");
            ExitCode::from(2)
        }
    }
}

fn run(args: Args) -> Result<bool, String> {
    if args.list_rules {
        match args.format {
            Format::Json => println!(
                "{}",
                serde_json::to_string_pretty(BUILTIN_RULES).map_err(|e| e.to_string())?
            ),
            Format::Text => {
                for rule in BUILTIN_RULES {
                    println!("{}\t{:?}\t{}", rule.id, rule.severity, rule.description);
                }
            }
        }
        return Ok(false);
    }
    let mut config = match args.config {
        Some(path) => toml::from_str::<Config>(
            &fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?,
        )
        .map_err(|e| format!("{}: {e}", path.display()))?,
        None => Config::default(),
    };
    config.disabled.extend(args.disable);
    if let Some(limit) = args.max_sentence_words {
        config.max_sentence_words = limit;
    }
    let linter = Linter::new(config);
    linter.validate()?;
    let files = if args.files.is_empty() {
        vec![PathBuf::from("-")]
    } else {
        args.files
    };
    if files.iter().filter(|p| p.as_os_str() == "-").count() > 1 {
        return Err("stdin ('-') can only appear once".into());
    }
    // Read every input before printing, so an I/O error cannot leave partial JSON.
    let mut inputs = Vec::new();
    for path in files {
        let source = if path.as_os_str() == "-" {
            let mut source = String::new();
            io::stdin()
                .read_to_string(&mut source)
                .map_err(|e| format!("stdin: {e}"))?;
            source
        } else {
            fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?
        };
        inputs.push((
            if path.as_os_str() == "-" {
                "<stdin>".into()
            } else {
                path.display().to_string()
            },
            Document::parse(source),
        ));
    }
    if args.ast {
        let reports: Vec<_> = inputs
            .into_iter()
            .map(|(path, document)| AstReport { path, document })
            .collect();
        println!(
            "{}",
            serde_json::to_string_pretty(&reports).map_err(|e| e.to_string())?
        );
        return Ok(false);
    }
    let reports: Vec<_> = inputs
        .into_iter()
        .map(|(path, document)| Report {
            path,
            diagnostics: linter
                .check(&document)
                .into_iter()
                .map(|diagnostic| LocatedDiagnostic {
                    position: document
                        .position(diagnostic.span.start)
                        .expect("rule must return a valid source offset"),
                    diagnostic,
                })
                .collect(),
        })
        .collect();
    match args.format {
        Format::Json => println!(
            "{}",
            serde_json::to_string_pretty(&reports).map_err(|e| e.to_string())?
        ),
        Format::Text => {
            for report in &reports {
                for located in &report.diagnostics {
                    let d = &located.diagnostic;
                    println!(
                        "{}:{}:{}: {:?}[{}]: {}",
                        report.path,
                        located.position.line,
                        located.position.column,
                        d.severity,
                        d.rule,
                        d.message
                    );
                    if let Some(suggestion) = &d.suggestion {
                        println!("  suggestion: {:?}", suggestion.replacement);
                    }
                }
            }
        }
    }
    let threshold = match args.fail_on {
        Failure::Advice => Some(Severity::Advice),
        Failure::Warning => Some(Severity::Warning),
        Failure::Error => Some(Severity::Error),
        Failure::Never => None,
    };
    Ok(threshold.is_some_and(|minimum| {
        reports
            .iter()
            .flat_map(|r| &r.diagnostics)
            .any(|d| d.diagnostic.severity >= minimum)
    }))
}
