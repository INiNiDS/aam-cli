// src/lsp.rs
use aam_rs::error::AamlError;
use aam_rs::pipeline::{DefaultLexer, DefaultParser, Lexer, Parser};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

pub struct AamLsp {
    client: Client,
}

#[tower_lsp::async_trait]
impl LanguageServer for AamLsp {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                document_formatting_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "aam-lsp ready")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.validate(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.validate(params.text_document.uri, change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn formatting(&self, _params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        // Simple formatting implementation for LSP
        // Store document content somewhere - for now use placeholder
        let source = String::new();

        let assist =
            aam_rs::aam::AAM::lsp_assist(&source, &aam_rs::pipeline::FormattingOptions::default());

        if let Some(formatted) = assist.formatted {
            let line_count = source.lines().count() as u32;
            Ok(Some(vec![TextEdit {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: line_count,
                        character: 0,
                    },
                },
                new_text: formatted,
            }]))
        } else {
            Ok(None)
        }
    }
}

impl AamLsp {
    async fn validate(&self, uri: Url, source: String) {
        let diagnostics = run_pipeline(&source);
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

fn run_pipeline(source: &str) -> Vec<Diagnostic> {
    let lexer = DefaultLexer::new();
    let parser = DefaultParser::new();

    match lexer.tokenize(source) {
        Ok(tokens) => {
            let parse_output = parser.parse_with_recovery(&tokens);
            if parse_output.errors.is_empty() {
                vec![]
            } else {
                parse_output
                    .errors
                    .iter()
                    .map(aaml_error_to_diagnostic)
                    .collect()
            }
        }
        Err(e) => vec![aaml_error_to_diagnostic(&e)],
    }
}

fn aaml_error_to_diagnostic(err: &AamlError) -> Diagnostic {
    let (line, col) = extract_position(err);
    // AamlError line/col — 1-based, LSP — 0-based
    let line = line.saturating_sub(1) as u32;
    let col = col.saturating_sub(1) as u32;

    Diagnostic {
        range: Range {
            start: Position {
                line,
                character: col,
            },
            end: Position {
                line,
                character: col + 10,
            },
        },
        severity: Some(DiagnosticSeverity::ERROR),
        message: err.to_string(),
        source: Some("aam-lsp".to_string()),
        ..Default::default()
    }
}

fn extract_position(err: &AamlError) -> (usize, usize) {
    match err {
        AamlError::LexError { line, column, .. } => (*line, *column),
        AamlError::ParseError { line, .. } => (*line, 0),
        AamlError::SchemaValidationError { .. } => (1, 0),
        AamlError::InvalidType { .. } => (1, 0),
        _ => (1, 0),
    }
}

pub fn run_lsp() -> anyhow::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();

        let (service, socket) = LspService::new(|client| AamLsp { client });
        Server::new(stdin, stdout, socket).serve(service).await;

        Ok(())
    })
}
