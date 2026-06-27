use std::cell::Cell;

/// Snapshot of AST construction counts.
///
/// Produced as a side effect of building an AST via
/// [`CountingAstBuilder`](crate::builder::CountingAstBuilder). Intended to be fed to semantic
/// analysis to pre-allocate its node / scope / symbol / reference stores without a separate
/// counting traversal.
///
/// See `oxc_semantic::Stats` for the consumer.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct AstNodeCounts {
    /// Number of visitable AST nodes (types with an `AstKind`).
    pub nodes: u32,
    /// Number of lexical scopes.
    pub scopes: u32,
    /// Upper bound on number of declared symbols.
    pub symbols: u32,
    /// Number of identifier references.
    pub references: u32,
}

/// Mutable counters tracking AST node construction.
///
/// Held inside [`CountingAstBuilder`](crate::builder::CountingAstBuilder); each generated
/// constructor for a visitable AST struct bumps the appropriate counter(s) via the
/// [`AstBuild`](crate::builder::AstBuild) hooks. The parser snapshots the counters at every
/// speculative checkpoint and restores them on rewind, so the totals at end of parse match
/// what a `Visit` traversal would observe.
#[derive(Default, Debug)]
pub struct AstNodeCounter {
    nodes: Cell<u32>,
    scopes: Cell<u32>,
    symbols: Cell<u32>,
    references: Cell<u32>,
}

impl AstNodeCounter {
    /// `+1` to the visitable-node count.
    #[inline]
    pub fn inc_node(&self) {
        self.nodes.set(self.nodes.get().wrapping_add(1));
    }

    /// `+1` to the scope count.
    #[inline]
    pub fn inc_scope(&self) {
        self.scopes.set(self.scopes.get().wrapping_add(1));
    }

    /// `+1` to the symbol count.
    #[inline]
    pub fn inc_symbol(&self) {
        self.symbols.set(self.symbols.get().wrapping_add(1));
    }

    /// `+1` to the reference count.
    #[inline]
    pub fn inc_reference(&self) {
        self.references.set(self.references.get().wrapping_add(1));
    }

    /// Read the current counts.
    #[inline]
    pub fn counts(&self) -> AstNodeCounts {
        AstNodeCounts {
            nodes: self.nodes.get(),
            scopes: self.scopes.get(),
            symbols: self.symbols.get(),
            references: self.references.get(),
        }
    }

    /// Overwrite the counters from a snapshot.
    ///
    /// Used by the parser to roll counts back to a checkpoint after a speculative parse
    /// is abandoned.
    #[inline]
    pub fn restore(&self, counts: AstNodeCounts) {
        self.nodes.set(counts.nodes);
        self.scopes.set(counts.scopes);
        self.symbols.set(counts.symbols);
        self.references.set(counts.references);
    }
}
