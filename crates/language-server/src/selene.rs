use serde::Deserialize;
use std::process::Stdio;
use tokio::{io::AsyncWriteExt, process};

pub async fn run_selene(text: &str) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    let mut child = process::Command::new("selene")
        .arg("-")
        .arg("--display-style=Json")
        .arg("--no-summary")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to run selene");

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(text.as_bytes()).await.unwrap();
        drop(stdin);
    }

    let output = child
        .wait_with_output()
        .await
        .expect("Failed to run selene");

    output
        .stdout
        .split(|&b| b == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            let diag: SeleneDiagnostic =
                serde_json::from_slice(line).expect("Failed to parse selene output");
            diag.into()
        })
        .collect()
}

#[derive(Debug, Deserialize)]
struct SeleneDiagnostic {
    severity: String,
    code: Option<String>,
    message: String,
    primary_label: Label,
    notes: Vec<String>,
}

impl From<SeleneDiagnostic> for tower_lsp::lsp_types::Diagnostic {
    fn from(d: SeleneDiagnostic) -> Self {
        let mut message = d.message.clone();
        if !d.notes.is_empty() {
            message.push_str("\nNotes:\n");
            for note in &d.notes {
                message.push_str(&format!("- {}\n", note));
            }
        }

        tower_lsp::lsp_types::Diagnostic {
            range: d.primary_label.span.into(),
            severity: Some(match d.severity.as_str() {
                "Bug" => tower_lsp::lsp_types::DiagnosticSeverity::ERROR,
                "Error" => tower_lsp::lsp_types::DiagnosticSeverity::ERROR,
                "Warning" => tower_lsp::lsp_types::DiagnosticSeverity::WARNING,
                "Note" => tower_lsp::lsp_types::DiagnosticSeverity::INFORMATION,
                "Help" => tower_lsp::lsp_types::DiagnosticSeverity::HINT,
                _ => unimplemented!(),
            }),
            code: d.code.map(tower_lsp::lsp_types::NumberOrString::String),
            source: Some("selene".to_string()),
            message,
            ..Default::default()
        }
    }
}

#[derive(Debug, Deserialize)]
struct Label {
    span: Span,
}

#[derive(Debug, Deserialize)]
struct Span {
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
}

impl From<Span> for tower_lsp::lsp_types::Range {
    fn from(span: Span) -> Self {
        Self {
            start: tower_lsp::lsp_types::Position {
                line: span.start_line as u32,
                character: span.start_column as u32,
            },
            end: tower_lsp::lsp_types::Position {
                line: span.end_line as u32,
                character: span.end_column as u32,
            },
        }
    }
}
