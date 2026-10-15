use serde_json::Value;
use std::{
    io::Write,
    process::{Command, Output, Stdio},
};

fn cli(args: &[&str], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_english-ast"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn stdin_json_reports_real_offsets_and_warning_exit() {
    let output = cli(&["--format", "json"], "Élodie: in order to help.");
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json[0]["path"], "<stdin>");
    assert_eq!(json[0]["diagnostics"][0]["rule"], "wordy-phrase");
    assert_eq!(json[0]["diagnostics"][0]["span"]["start"], 9);
    assert_eq!(json[0]["diagnostics"][0]["position"]["column"], 9);
    assert_eq!(json[0]["diagnostics"][0]["suggestion"]["replacement"], "to");
}

#[test]
fn exit_thresholds_and_disabling_are_effective() {
    for (args, expected) in [
        (vec![], 0),
        (vec!["--fail-on", "advice"], 1),
        (vec!["--fail-on", "never"], 0),
        (vec!["--fail-on", "error"], 0),
        (vec!["--fail-on", "advice", "--disable", "plain-word"], 0),
    ] {
        let output = cli(&args, "Utilize tools.");
        assert_eq!(output.status.code(), Some(expected), "{args:?}");
    }
    let output = cli(
        &["--config", "english-ast.toml", "tests/fixtures/clean.txt"],
        "",
    );
    assert_eq!(output.status.code(), Some(0));
    assert!(output.stdout.is_empty());
}

#[test]
fn multiple_files_produce_one_json_array() {
    let output = cli(
        &[
            "--format",
            "json",
            "tests/fixtures/wordy.txt",
            "tests/fixtures/clean.txt",
        ],
        "",
    );
    assert_eq!(output.status.code(), Some(1));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 2);
    assert_eq!(json[0]["diagnostics"].as_array().unwrap().len(), 2);
    assert!(json[1]["diagnostics"].as_array().unwrap().is_empty());
}

#[test]
fn ast_mode_retains_source_and_node_kinds() {
    let output = cli(&["--ast"], "Hi, Rust!\n");
    assert_eq!(output.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json[0]["document"]["source"], "Hi, Rust!\n");
    let tokens = &json[0]["document"]["paragraphs"][0]["sentences"][0]["tokens"];
    assert_eq!(tokens[0]["kind"], "word");
    assert_eq!(tokens[1]["kind"], "punctuation");
    assert_eq!(tokens[2]["kind"], "whitespace");
}

#[test]
fn invalid_inputs_exit_two_without_partial_output() {
    for args in [
        vec!["--disable", "typo"],
        vec!["--max-sentence-words", "0"],
        vec!["-", "-"],
        vec![
            "--format",
            "json",
            "tests/fixtures/clean.txt",
            "tests/fixtures/does-not-exist.txt",
        ],
    ] {
        let output = cli(&args, "");
        assert_eq!(output.status.code(), Some(2), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    let output = Command::new(env!("CARGO_BIN_EXE_english-ast"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut child = output;
    child.stdin.take().unwrap().write_all(&[0xff]).unwrap();
    assert_eq!(child.wait_with_output().unwrap().status.code(), Some(2));
}

#[test]
fn catalog_is_machine_readable() {
    let output = cli(&["--list-rules", "--format", "json"], "");
    assert_eq!(output.status.code(), Some(0));
    let json: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 11);
    assert_eq!(json[0]["id"], "plain-word");
}
