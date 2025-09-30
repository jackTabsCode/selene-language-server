use serde::Deserialize;
use tempfile::NamedTempFile;
use tokio::process;

pub async fn run_selene(text: &str) -> Vec<tower_lsp::lsp_types::Diagnostic> {
    let tmp = make_temp_file(text);

    let output = process::Command::new("selene")
        .arg(tmp.path())
        .arg("--display-style=Json")
        .arg("--no-summary")
        .output()
        .await
        .expect("Failed to run selene");

    let stdout = String::from_utf8(output.stdout).expect("Failed to parse selene output");

    stdout
        .lines()
        .map(|line| {
            let diag: SeleneDiagnostic =
                serde_json::from_str(line).expect("Failed to parse selene output");
            diag.into()
        })
        .collect()
}

fn make_temp_file(text: &str) -> NamedTempFile {
    let mut tmp = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    std::io::Write::write_all(&mut tmp, text.as_bytes()).expect("Failed to write temp file");
    tmp
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
