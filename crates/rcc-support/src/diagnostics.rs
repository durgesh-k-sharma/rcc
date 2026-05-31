//! Diagnostic infrastructure for structured error reporting.

use std::fmt;
use std::sync::Arc;

use crate::source::{SourceManager, Span};

/// The severity of a diagnostic message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Additional context or information (not an error or warning).
    Note,
    /// A potential problem that does not prevent compilation.
    Warning,
    /// A definite problem that prevents successful compilation.
    Error,
    /// An internal compiler error (ICE) — something is wrong with rcc itself.
    Bug,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Note => "note",
            Severity::Warning => "warning",
            Severity::Error => "error",
            Severity::Bug => "bug",
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single diagnostic message.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub primary_span: Option<Span>,
    pub secondary_spans: Vec<(Span, String)>,
    pub fix_its: Vec<(Span, String)>,
}

impl Diagnostic {
    pub fn error(msg: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Error,
            message: msg.into(),
            primary_span: None,
            secondary_spans: Vec::new(),
            fix_its: Vec::new(),
        }
    }

    pub fn warning(msg: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Warning,
            message: msg.into(),
            primary_span: None,
            secondary_spans: Vec::new(),
            fix_its: Vec::new(),
        }
    }

    pub fn note(msg: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Note,
            message: msg.into(),
            primary_span: None,
            secondary_spans: Vec::new(),
            fix_its: Vec::new(),
        }
    }

    pub fn with_primary(mut self, span: Span) -> Self {
        self.primary_span = Some(span);
        self
    }

    pub fn with_note(mut self, span: Span, msg: impl Into<String>) -> Self {
        self.secondary_spans.push((span, msg.into()));
        self
    }

    pub fn with_fix_it(mut self, span: Span, replacement: impl Into<String>) -> Self {
        self.fix_its.push((span, replacement.into()));
        self
    }
}

/// Collects and optionally renders diagnostics.
pub struct Diagnostics {
    source_manager: Arc<SourceManager>,
    emitted: Vec<Diagnostic>,
    /// Whether compilation should abort after the current pass.
    abort_on_error: bool,
}

impl Diagnostics {
    pub fn new(source_manager: Arc<SourceManager>) -> Self {
        Diagnostics {
            source_manager,
            emitted: Vec::new(),
            abort_on_error: false,
        }
    }

    /// Record a diagnostic.
    pub fn emit(&mut self, diag: Diagnostic) {
        if diag.severity >= Severity::Error {
            self.abort_on_error = true;
        }
        self.emitted.push(diag);
    }

    /// Whether any errors (or bugs) have been emitted.
    pub fn has_errors(&self) -> bool {
        self.emitted
            .iter()
            .any(|d| d.severity >= Severity::Error)
    }

    /// Whether compilation should abort.
    pub fn abort_on_error(&self) -> bool {
        self.abort_on_error
    }

    /// Return all emitted diagnostics.
    pub fn emitted(&self) -> &[Diagnostic] {
        &self.emitted
    }

    /// Access the source manager associated with this diagnostics session.
    pub fn source_manager(&self) -> &SourceManager {
        &self.source_manager
    }

    /// Clear all emitted diagnostics and reset the abort flag.
    pub fn reset(&mut self) {
        self.emitted.clear();
        self.abort_on_error = false;
    }

    /// Render all diagnostics to a writer.
    ///
    /// Format (GCC-style):
    /// ```text
    /// file.c:line:col: severity: message
    /// ```
    /// With source context if a primary span is present.
    pub fn report_all(&self, writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        for diag in &self.emitted {
            self.render_diagnostic(writer, diag)?;
        }
        Ok(())
    }

    fn render_diagnostic(
        &self,
        writer: &mut dyn std::io::Write,
        diag: &Diagnostic,
    ) -> std::io::Result<()> {
        // Location prefix
        if let Some(span) = &diag.primary_span {
            if let Some(file) = self.source_manager.get(span.file_id) {
                if let Some((line, col)) = file.line_col(span.start) {
                    write!(
                        writer,
                        "{}:{}:{}: ",
                        file.path().display(),
                        line,
                        col
                    )?;
                } else {
                    write!(writer, "{}:{}:{}: ", file.path().display(), "?", "?")?;
                }
            }
        }

        // Severity and message
        writeln!(writer, "{}: {}", diag.severity, diag.message)?;

        // Source context
        if let Some(span) = &diag.primary_span {
            if let Some(file) = self.source_manager.get(span.file_id) {
                if let Some((line, _col)) = file.line_col(span.start) {
                    if let Some(line_text) = file.line(line) {
                        let line_text = line_text.trim_end_matches('\n');
                        // Only render if the line isn't too long
                        if line_text.len() <= 256 {
                            writeln!(writer, " {} | {}", line, line_text)?;
                            // Caret line
                            let col = (span.start - file.line_starts()[line as usize - 1]) as usize;
                            let carets = (span.end - span.start).max(1) as usize;
                            let col = col.min(line_text.len());
                            writeln!(writer, "   | {}{}", " ".repeat(col), "^".repeat(carets))?;
                        }
                    }
                }
            }
        }

        // Secondary spans
        for (span, msg) in &diag.secondary_spans {
            if let Some(file) = self.source_manager.get(span.file_id) {
                if let Some((line, col)) = file.line_col(span.start) {
                    writeln!(
                        writer,
                        "  = {}: {}:{}: {}",
                        Severity::Note,
                        file.path().display(),
                        line,
                        msg,
                    )?;
                }
            }
        }

        // Fix-it hints
        for (span, replacement) in &diag.fix_its {
            if let Some(file) = self.source_manager.get(span.file_id) {
                writeln!(
                    writer,
                    "  = help: replace {}..{} with \"{}\"",
                    span.start,
                    span.end,
                    replacement
                )?;
            }
        }

        Ok(())
    }
}

/// Render diagnostics to a string (useful for tests).
pub fn diagnostics_to_string(diags: &Diagnostics) -> String {
    let mut buf = Vec::new();
    diags.report_all(&mut buf).ok();
    String::from_utf8(buf).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceFile;
    use std::path::PathBuf;

    fn make_diag_source_manager() -> Arc<SourceManager> {
        let mut sm = SourceManager::new();
        let sf = SourceFile::new(
            crate::source::FileId(0),
            PathBuf::from("test.c"),
            "int main() {\n    return 42;\n}\n".into(),
        );
        sm.add(sf);
        Arc::new(sm)
    }

    #[test]
    fn error_diagnostic_sets_abort_flag() {
        let sm = make_diag_source_manager();
        let mut diags = Diagnostics::new(sm);
        diags.emit(Diagnostic::error("something went wrong"));
        assert!(diags.has_errors());
        assert!(diags.abort_on_error());
    }

    #[test]
    fn warning_does_not_set_abort_flag() {
        let sm = make_diag_source_manager();
        let mut diags = Diagnostics::new(sm);
        diags.emit(Diagnostic::warning("this is a warning"));
        assert!(!diags.has_errors());
        assert!(!diags.abort_on_error());
    }

    #[test]
    fn report_all_produces_location_prefix() {
        let sm = make_diag_source_manager();
        let mut diags = Diagnostics::new(Arc::clone(&sm));
        let file = sm.get(crate::source::FileId(0)).unwrap();
        let span = crate::source::Span::new(file.id(), 0, 3); // "int"
        diags.emit(Diagnostic::error("test message").with_primary(span));

        let output = diagnostics_to_string(&diags);
        assert!(output.contains("test.c:1:0: error: test message"));
        assert!(output.contains("| int"));
    }

    #[test]
    fn reset_clears_all_state() {
        let sm = make_diag_source_manager();
        let mut diags = Diagnostics::new(sm);
        diags.emit(Diagnostic::error("err"));
        assert!(diags.has_errors());
        diags.reset();
        assert!(!diags.has_errors());
        assert!(!diags.abort_on_error());
    }
}
