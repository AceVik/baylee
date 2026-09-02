//! What a generator produces for one card.
//!
//! Two readers fill this in — [`landgen`](crate::landgen) from a land's
//! printed text and [`forgegen`](crate::forgegen) from a forge-reference
//! script — and [`stubgen`](crate::stubgen) renders it. Both obey the same
//! rule: a body is produced only when the whole card was understood, so a
//! generated `Coverage::Implemented` never means "most of it".

use std::fmt::Write as _;

/// The generated body of one recognised card.
#[derive(Debug, Default)]
pub struct CardBody {
    /// Extra `static` items (filters) to emit above the card literal.
    pub statics: String,
    /// `EnterModifier` expressions for the front face.
    pub enter_modifiers: Vec<String>,
    /// `KeywordSet` constants to union.
    pub keywords: Vec<String>,
    /// Ability expressions for the card literal.
    pub abilities: Vec<String>,
    /// Short human notes for the `// IMPLEMENTED — …` line.
    pub notes: Vec<String>,
}

impl CardBody {
    /// Whether anything at all was read.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.abilities.is_empty() && self.enter_modifiers.is_empty() && self.keywords.is_empty()
    }

    /// Declares a `static <NAME>: Filter = <expr>;` and returns its name.
    ///
    /// `TargetSpec::Object` and friends hold `&'static Filter`, so a filter
    /// built from a card's own text has to be hoisted out of the literal.
    pub fn filter_static(&mut self, prefix: &str, expr: &str) -> String {
        let n = self.statics.matches("static ").count() + 1;
        let name = format!("{prefix}{n}");
        let _ = write!(self.statics, "static {name}: Filter = {expr};\n\n");
        name
    }
}
