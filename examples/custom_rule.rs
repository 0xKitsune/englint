use english_ast::{Diagnostic, Document, Linter, Rule, RuleInfo, Severity};

struct NoTodo;
impl Rule for NoTodo {
    fn info(&self) -> RuleInfo {
        RuleInfo {
            id: "no-todo",
            description: "Resolve TODO markers before publication",
            severity: Severity::Error,
        }
    }
    fn check(&self, document: &Document, diagnostics: &mut Vec<Diagnostic>) {
        for token in document.sentences().flat_map(|s| s.words()) {
            if document.text(token.span) == Some("TODO") {
                diagnostics.push(Diagnostic {
                    rule: self.info().id.into(),
                    severity: self.info().severity,
                    span: token.span,
                    message: "Resolve this TODO before publishing.".into(),
                    suggestion: None,
                });
            }
        }
    }
}
fn main() {
    let document = Document::parse("TODO: cite the result.");
    let linter = Linter::default().with_rule(NoTodo);
    linter.validate().unwrap();
    assert_eq!(linter.check(&document)[0].rule, "no-todo");
    println!("{:#?}", linter.check(&document));
}
