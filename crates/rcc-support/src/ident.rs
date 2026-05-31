//! Interned strings and identifiers.

/// A handle to an interned string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ident(u32);

impl Ident {
    pub const fn new(id: u32) -> Self {
        Ident(id)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

/// A bidirectional interner for strings.
///
/// Strings are deduplicated: the same string always maps to the same `Ident`.
/// Uses a Vec<String> for storage and a HashMap<&str, Ident> for lookup.
/// Strings are box-leaked into 'static lifetime for the HashMap keys.
pub struct Interner {
    strings: Vec<String>,
}

impl Interner {
    pub fn new() -> Self {
        Interner {
            strings: Vec::new(),
        }
    }

    /// Intern a string, returning a stable `Ident` handle.
    ///
    /// If the string was already interned, returns the existing handle.
    pub fn intern(&mut self, s: &str) -> Ident {
        // Linear scan for existing string (small interner for MVP).
        if let Some((idx, _)) = self
            .strings
            .iter()
            .enumerate()
            .find(|(_, existing)| existing.as_str() == s)
        {
            return Ident(idx as u32);
        }
        let id = Ident(self.strings.len() as u32);
        self.strings.push(s.to_string());
        id
    }

    /// Look up an `Ident` to get its string representation.
    pub fn lookup(&self, ident: Ident) -> &str {
        &self.strings[ident.0 as usize]
    }

    /// The number of unique interned strings.
    pub fn len(&self) -> usize {
        self.strings.len()
    }

    /// Whether the interner is empty.
    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}

impl Default for Interner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intern_and_lookup() {
        let mut interner = Interner::new();
        let id_foo = interner.intern("foo");
        let id_bar = interner.intern("bar");
        let id_foo2 = interner.intern("foo");

        assert_eq!(id_foo, id_foo2);
        assert_ne!(id_foo, id_bar);
        assert_eq!(interner.lookup(id_foo), "foo");
        assert_eq!(interner.lookup(id_bar), "bar");
    }

    #[test]
    fn empty_interner_has_zero_len() {
        let interner = Interner::new();
        assert!(interner.is_empty());
    }

    #[test]
    fn interner_maintains_distinct_ids() {
        let mut interner = Interner::new();
        let ids: Vec<Ident> = (0..10).map(|i| interner.intern(&i.to_string())).collect();
        for i in 0..ids.len() {
            for j in 0..ids.len() {
                if i != j {
                    assert_ne!(ids[i], ids[j], "ids[{}] should differ from ids[{}]", i, j);
                }
            }
        }
    }
}
