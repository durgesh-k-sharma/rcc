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

    /// ANSI colour code for the severity label.
    fn ansi_color(&self) -> &'static str {
        match self {
            Severity::Note => "\x1b[36m",    // cyan
            Severity::Warning => "\x1b[35m", // magenta
            Severity::Error => "\x1b[31m",   // red
            Severity::Bug => "\x1b[31;1m",   // bright red
        }
    }
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

const RESET: &str = "\x1b[0m";

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

    pub fn bug(msg: impl Into<String>) -> Self {
        Diagnostic {
            severity: Severity::Bug,
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
    /// Whether to emit ANSI colour codes (auto-detected from terminal).
    color_enabled: bool,
}

impl Diagnostics {
    pub fn new(source_manager: Arc<SourceManager>) -> Self {
        let color_enabled = cfg!(not(windows)) && atty::is(atty::Stream::Stdout);
        Diagnostics {
            source_manager,
            emitted: Vec::new(),
            abort_on_error: false,
            color_enabled,
        }
    }

    /// Create a diagnostics session with an explicit colour setting.
    pub fn new_with_color(source_manager: Arc<SourceManager>, color: bool) -> Self {
        Diagnostics {
            source_manager,
            emitted: Vec::new(),
            abort_on_error: false,
            color_enabled: color,
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
        self.emitted.iter().any(|d| d.severity >= Severity::Error)
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
}

/// Renders diagnostics to a writer.
///
/// Output style (Clang-inspired):
/// ```text
/// error: message
///   --> file.c:line:col
///    |
///  1 | int main() {
///    | ^^^
/// ```
pub fn render_diagnostics(
    diagnostics: &Diagnostics,
    writer: &mut dyn std::io::Write,
    color: Option<bool>,
) -> std::io::Result<()> {
    let use_color = color.unwrap_or(diagnostics.color_enabled);
    for diag in &diagnostics.emitted {
        render_one(diagnostics, writer, diag, use_color)?;
    }
    Ok(())
}

fn render_one(
    diags: &Diagnostics,
    writer: &mut dyn std::io::Write,
    diag: &Diagnostic,
    color: bool,
) -> std::io::Result<()> {
    let sev_color = if color {
        diag.severity.ansi_color()
    } else {
        ""
    };

    // Severity label + message
    if color {
        write!(writer, "{sev_color}{}{RESET}: {}\n", diag.severity, diag.message)?;
    } else {
        write!(writer, "{}: {}\n", diag.severity.as_str(), diag.message)?;
    }

    // Source location + snippet
    if let Some(span) = &diag.primary_span {
        if let Some(file) = diags.source_manager.get(span.file_id) {
            if let Some((line, _col)) = file.line_col(span.start) {
                let arrow = if color { "\x1b[32m-->\x1b[0m" } else { "-->" };
                writeln!(writer, "  {} {}:{}:{}", arrow, file.path().display(), line, _col)?;
                writeln!(writer, "   |")?;

                if let Some(line_src) = file.line(line) {
                    let line_src = line_src.trim_end_matches('\n');
                    if line_src.len() <= 256 {
                        let line_num = line as usize;
                        let col = (span.start - file.line_starts()[line_num - 1]) as usize;
                        let caret_count = (span.end - span.start).max(1) as usize;
                        let col = col.min(line_src.len());

                        writeln!(writer, " {:>4} | {}", line, line_src)?;
                        let caret_color = if color { "\x1b[32m" } else { "" };
                        writeln!(writer, "      | {}{}{}", " ".repeat(col), caret_color, "^".repeat(caret_count))?;
                        if color {
                            write!(writer, "{RESET}")?;
                        }
                    }
                }
            }
        }
    }

    // Secondary spans
    for (span, msg) in &diag.secondary_spans {
        if let Some(file) = diags.source_manager.get(span.file_id) {
            if let Some((s_line, s_col)) = file.line_col(span.start) {
                if color {
                    writeln!(
                        writer,
                        "  {}= note: {}:{}:{}:{}: {}",
                        "\x1b[36m", RESET, file.path().display(), s_line, s_col, msg,
                    )?;
                } else {
                    writeln!(
                        writer,
                        "  = note: {}:{}:{}: {}",
                        file.path().display(), s_line, s_col, msg,
                    )?;
                }
            }
        }
    }

    // Fix-it hints
    for (span, replacement) in &diag.fix_its {
        if color {
            writeln!(
                writer,
                "  {}= help: replace {}..{} with \"{}\"{}",
                "\x1b[35m", span.start, span.end, replacement, RESET,
            )?;
        } else {
            writeln!(
                writer,
                "  = help: replace {}..{} with \"{}\"",
                span.start, span.end, replacement,
            )?;
        }
    }

    Ok(())
}

/// Render diagnostics to a string (useful for tests).
pub fn diagnostics_to_string(diags: &Diagnostics) -> String {
    let mut buf = Vec::new();
    render_diagnostics(diags, &mut buf, Some(false)).ok();
    String::from_utf8(buf).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Golden-test helpers
// ---------------------------------------------------------------------------

/// Check that at least one emitted diagnostic matches the given severity and
/// message substring.
pub fn assert_diagnostic_emitted(emitted: &[Diagnostic], severity: Severity, msg_substring: &str) {
    let found = emitted
        .iter()
        .any(|d| d.severity == severity && d.message.to_lowercase().contains(&msg_substring.to_lowercase()));
    assert!(
        found,
        "expected diagnostic not found: severity={:?}, message contains \"{}\"",
        severity,
        msg_substring,
    );
}

/// Check the total number of emitted diagnostics.
pub fn assert_diagnostic_count(emitted: &[Diagnostic], n: usize) {
    assert_eq!(
        emitted.len(),
        n,
        "expected {} diagnostics, got {}",
        n,
        emitted.len(),
    );
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
        let mut diags = Diagnostics::new_with_color(sm, false);
        diags.emit(Diagnostic::error("something went wrong"));
        assert!(diags.has_errors());
        assert!(diags.abort_on_error());
    }

    #[test]
    fn warning_does_not_set_abort_flag() {
        let sm = make_diag_source_manager();
        let mut diags = Diagnostics::new_with_color(sm, false);
        diags.emit(Diagnostic::warning("this is a warning"));
        assert!(!diags.has_errors());
        assert!(!diags.abort_on_error());
    }

    #[test]
    fn report_all_produces_location_prefix() {
        let sm = make_diag_source_manager();
        let mut diags = Diagnostics::new_with_color(Arc::clone(&sm), false);
        let file = sm.get(crate::source::FileId(0)).unwrap();
        let span = crate::source::Span::new(file.id(), 0, 3);
        diags.emit(Diagnostic::error("test message").with_primary(span));

        let output = diagnostics_to_string(&diags);
        assert!(output.contains("error: test message"));
        assert!(output.contains("--> test.c:1:0"));
        assert!(output.contains("| int"));
    }

    #[test]
    fn reset_clears_all_state() {
        let sm = make_diag_source_manager();
        let mut diags = Diagnostics::new_with_color(sm, false);
        diags.emit(Diagnostic::error("err"));
        assert!(diags.has_errors());
        diags.reset();
        assert!(!diags.has_errors());
        assert!(!diags.abort_on_error());
    }

    #[test]
    fn primary_span_without_source_does_not_panic() {
        let sm = make_diag_source_manager();
        let mut diags = Diagnostics::new_with_color(sm, false);
        let bad_span = crate::source::Span::new(crate::source::FileId(999), 0, 5);
        diags.emit(Diagnostic::error("no file").with_primary(bad_span));
        let output = diagnostics_to_string(&diags);
        assert!(output.contains("error: no file"));
    }

    #[test]
    fn secondary_span_is_rendered() {
        let sm = make_diag_source_manager();
        let mut diags = Diagnostics::new_with_color(Arc::clone(&sm), false);
        let file = sm.get(crate::source::FileId(0)).unwrap();
        let primary = crate::source::Span::new(file.id(), 0, 3);
        let secondary = crate::source::Span::new(file.id(), 15, 17);
        diags.emit(
            Diagnostic::error("mismatch error")
                .with_primary(primary)
                .with_note(secondary, "defined here"),
        );
        let output = diagnostics_to_string(&diags);
        assert!(output.contains("note:"));
        assert!(output.contains("defined here"));
    }

    #[test]
    fn fix_it_hint_is_rendered() {
        let sm = make_diag_source_manager();
        let mut diags = Diagnostics::new_with_color(Arc::clone(&sm), false);
        let file = sm.get(crate::source::FileId(0)).unwrap();
        let span = crate::source::Span::new(file.id(), 5, 9);
        diags.emit(
            Diagnostic::warning("old style")
                .with_primary(span)
                .with_fix_it(span, "new_style"),
        );
        let output = diagnostics_to_string(&diags);
        assert!(output.contains("help:"));
        assert!(output.contains("new_style"));
    }

    #[test]
    fn render_colored_output_contains_escape_sequences() {
        let sm = make_diag_source_manager();
        let mut diags = Diagnostics::new_with_color(Arc::clone(&sm), true);
        let file = sm.get(crate::source::FileId(0)).unwrap();
        let span = crate::source::Span::new(file.id(), 0, 3);
        diags.emit(Diagnostic::error("colored test").with_primary(span));

        let mut buf = Vec::new();
        render_diagnostics(&diags, &mut buf, Some(true)).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("\x1b["));
        assert!(output.contains("error"));
    }

    #[test]
    fn note_severity() {
        let sm = make_diag_source_manager();
        let mut diags = Diagnostics::new_with_color(sm, false);
        diags.emit(Diagnostic::note("just fyi"));
        assert!(!diags.has_errors());
        assert!(!diags.abort_on_error());
        let output = diagnostics_to_string(&diags);
        assert!(output.contains("note: just fyi"));
    }

    #[test]
    fn bug_severity_sets_abort() {
        let sm = make_diag_source_manager();
        let mut diags = Diagnostics::new_with_color(sm, false);
        diags.emit(Diagnostic::bug("ICE: null pointer"));
        assert!(diags.has_errors());
        assert!(diags.abort_on_error());
        let output = diagnostics_to_string(&diags);
        assert!(output.contains("bug: ICE"));
    }

    #[test]
    fn multiple_diagnostics_are_all_rendered() {
        let sm = make_diag_source_manager();
        let mut diags = Diagnostics::new_with_color(sm, false);
        diags.emit(Diagnostic::error("first error"));
        diags.emit(Diagnostic::warning("second warning"));
        diags.emit(Diagnostic::note("third note"));
        let output = diagnostics_to_string(&diags);
        assert!(output.contains("error: first error"));
        assert!(output.contains("warning: second warning"));
        assert!(output.contains("note: third note"));
    }

    #[test]
    fn golden_assert_diagnostic_emitted() {
        let sm = make_diag_source_manager();
        let mut diags = Diagnostics::new_with_color(sm, false);
        diags.emit(Diagnostic::error("unexpected token ';' at line 5"));
        assert_diagnostic_emitted(diags.emitted(), Severity::Error, "unexpected token");
    }

    #[test]
    fn golden_assert_diagnostic_count() {
        let sm = make_diag_source_manager();
        let mut diags = Diagnostics::new_with_color(sm, false);
        diags.emit(Diagnostic::error("error 1"));
        diags.emit(Diagnostic::error("error 2"));
        diags.emit(Diagnostic::warning("warning 1"));
        assert_diagnostic_count(diags.emitted(), 3);
    }
}
