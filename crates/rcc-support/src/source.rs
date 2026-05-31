//! Source file management and source location tracking.

use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

/// Opaque identifier for a loaded source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub u32);

impl FileId {
    pub const fn new(id: u32) -> Self {
        FileId(id)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl std::fmt::Display for FileId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An owned copy of a source file loaded from disk.
#[derive(Debug, Clone)]
pub struct SourceFile {
    id: FileId,
    path: PathBuf,
    source: Arc<str>,
    line_starts: Vec<u32>,
}

impl SourceFile {
    /// Create a new `SourceFile` from its components.
    ///
    /// The `line_starts` vector must contain the byte offset of every line's first
    /// byte, starting with 0 for line 1. If empty, line starts are computed from
    /// `source`.
    pub fn new(id: FileId, path: PathBuf, source: String) -> Self {
        let line_starts = compute_line_starts(&source);
        SourceFile {
            id,
            path,
            source: Arc::from(source),
            line_starts,
        }
    }

    /// The `FileId` assigned to this file.
    pub fn id(&self) -> FileId {
        self.id
    }

    /// The path this file was loaded from.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The full source text.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// The length of the source text in bytes.
    pub fn len(&self) -> usize {
        self.source.len()
    }

    /// Whether the source is empty.
    pub fn is_empty(&self) -> bool {
        self.source.is_empty()
    }

    /// Return the (1-indexed) line and (0-indexed) column for the given byte offset.
    ///
    /// Returns `None` if `offset` is out of range.
    pub fn line_col(&self, offset: u32) -> Option<(u32, u32)> {
        let offset = offset as usize;
        if offset > self.source.len() {
            return None;
        }

        // Binary search for the line that contains this offset.
        let line_idx = match self.line_starts.binary_search(&(offset as u32)) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };

        let line_start = self.line_starts[line_idx] as usize;
        let col = offset.saturating_sub(line_start);

        Some(((line_idx + 1) as u32, col as u32))
    }

    /// Return the source text of a specific 1-indexed line.
    pub fn line(&self, line_num: u32) -> Option<&str> {
        let idx = (line_num as usize).checked_sub(1)?;
        let start = *self.line_starts.get(idx)? as usize;
        let end = self
            .line_starts
            .get(idx + 1)
            .map(|&o| o as usize)
            .unwrap_or(self.source.len());
        Some(&self.source[start..end])
    }

    /// The number of lines in this file.
    pub fn num_lines(&self) -> u32 {
        self.line_starts.len() as u32
    }

    /// The list of line-start offsets (byte positions).
    pub fn line_starts(&self) -> &[u32] {
        &self.line_starts
    }
}

fn compute_line_starts(source: &str) -> Vec<u32> {
    let mut starts = vec![0u32];
    for (i, b) in source.bytes().enumerate() {
        if b == b'\n' {
            starts.push((i + 1) as u32);
        }
    }
    starts
}

/// A half-open byte range `[start, end)` within a source file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub file_id: FileId,
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub const fn new(file_id: FileId, start: u32, end: u32) -> Self {
        Span { file_id, start, end }
    }
}

/// Manages all loaded source files and provides lookup by [`FileId`] or path.
#[derive(Debug, Default)]
pub struct SourceManager {
    files: Vec<SourceFile>,
}

impl SourceManager {
    pub fn new() -> Self {
        SourceManager { files: Vec::new() }
    }

    /// Load a file from disk, assign it a new `FileId`, and return the id.
    pub fn load_file(path: &Path) -> std::io::Result<(FileId, SourceFile)> {
        let source = std::fs::read_to_string(path)?;
        let id = FileId(0); // caller should assign via add_file
        let sf = SourceFile::new(id, path.to_path_buf(), source);
        Ok((sf.id(), sf))
    }

    /// Add an already-constructed SourceFile, returning its assigned FileId.
    pub fn add(&mut self, mut sf: SourceFile) -> FileId {
        let id = FileId(self.files.len() as u32);
        sf.id = id;
        self.files.push(sf);
        id
    }

    /// Look up a file by its id.
    pub fn get(&self, id: FileId) -> Option<&SourceFile> {
        self.files.get(id.0 as usize)
    }

    /// The total number of registered files.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Whether any files are registered.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Iterate over all registered files.
    pub fn iter(&self) -> impl Iterator<Item = &SourceFile> {
        self.files.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_source_file(source: &str) -> SourceFile {
        SourceFile::new(FileId(0), PathBuf::from("test.c"), source.to_string())
    }

    #[test]
    fn line_col_returns_correct_position() {
        let sf = make_source_file("int main() {\n    return 42;\n}\n");
        assert_eq!(sf.line_col(0), Some((1, 0))); // 'i' of 'int'
        assert_eq!(sf.line_col(12), Some((1, 12))); // '('
        assert_eq!(sf.line_col(13), Some((2, 0))); // newline -> line 2
        assert_eq!(sf.line_col(18), Some((2, 5))); // 'r' of 'return'
    }

    #[test]
    fn out_of_range_offset_returns_none() {
        let sf = make_source_file("int main() {}");
        assert_eq!(sf.line_col(100), None);
    }

    #[test]
    fn line_returns_correct_content() {
        let sf = make_source_file("int main() {\n    return 42;\n}\n");
        assert_eq!(sf.line(1), Some("int main() {\n"));
        assert_eq!(sf.line(2), Some("    return 42;\n"));
        assert_eq!(sf.line(3), Some("}\n"));
        // Trailing \n creates an empty line 4.
        assert_eq!(sf.line(4), Some(""));
        assert_eq!(sf.line(5), None);
    }

    #[test]
    fn source_manager_assigns_ids() {
        let mut sm = SourceManager::new();
        let sf1 = SourceFile::new(FileId(0), PathBuf::from("a.c"), "a".into());
        let sf2 = SourceFile::new(FileId(0), PathBuf::from("b.c"), "b".into());
        let id1 = sm.add(sf1);
        let id2 = sm.add(sf2);
        assert_ne!(id1, id2);
        assert_eq!(sm.get(id1).unwrap().path(), Path::new("a.c"));
        assert_eq!(sm.get(id2).unwrap().path(), Path::new("b.c"));
    }

    #[test]
    fn empty_source_has_one_line() {
        let sf = make_source_file("");
        assert_eq!(sf.num_lines(), 1);
    }
}
