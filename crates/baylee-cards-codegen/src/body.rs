//! What a generator produces for one card.
//!
//! Two readers fill this in — [`landgen`](crate::landgen) from a land's
//! printed text and [`scriptgen`](crate::scriptgen) from a card-script reference
//! script — and [`stubgen`](crate::stubgen) renders it. Both obey the same
//! rule: a body is produced only when the whole card was understood, so a
//! generated `Coverage::Implemented` never means "most of it".

use std::fmt::Write as _;

/// A `Cost` expression, from the mana a reader accumulated and the parts it
/// recognised.
///
/// Both readers build the same two pieces and used to render them the same
/// way twice. `parts` are bare variant names (`TapSelf`, `PayLife(1)`),
/// because `cost!` supplies the `CostPart::` prefix — which is the same word
/// three times on a fetchland and is not what a reader is checking.
///
/// The three forms are the card's, not the code's: an empty cost is
/// `Cost::FREE`, a bare tap is `Cost::TAP`, and everything else reads left to
/// right the way the card prints it — mana, then the rest.
#[must_use]
pub fn cost_literal(mana: &str, parts: &[String]) -> String {
    match (mana.is_empty(), parts) {
        (true, []) => "Cost::FREE".to_string(),
        (true, [one]) if one == "TapSelf" => "Cost::TAP".to_string(),
        (true, _) => format!("cost!({})", parts.join(", ")),
        (false, []) => format!("cost!(\"{mana}\")"),
        (false, _) => format!("cost!(\"{mana}\", {})", parts.join(", ")),
    }
}

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
    /// `TargetSpec::Object` and friends hold a `&'static Filter`, and a
    /// borrow of a `Filter` literal promotes to exactly that inside a
    /// `static` — so this is legibility and not necessity. What it buys is
    /// that a target the card computes is named once, above the literal,
    /// where the reader meets it before the ability that aims with it.
    ///
    /// A filter that is *already* a name buys none of that, so it is handed
    /// straight back: `static TARGET1: Filter = Filter::CREATURE;` gives one
    /// spelling of "a creature" a second, card-local spelling, which is the
    /// duplication this was meant to prevent. Fifteen generated cards
    /// carried a `static` whose whole body was a bare constant — eight of
    /// them that one.
    pub fn filter_static(&mut self, prefix: &str, expr: &str) -> String {
        if !expr.contains(['(', '[', ',', ' ']) {
            return expr.to_string();
        }
        // And the same filter written twice in one card is one filter, for
        // the same reason: Rivendell prints "a legendary creature you
        // control" in two sentences — the clause it enters under and the
        // clause its ability asks — and was given two byte-identical
        // statics, which is one thing under two names in a file whose whole
        // job is to be read.
        if let Some(name) = self.declared(prefix, expr) {
            return name;
        }
        let n = self.statics.matches("static ").count() + 1;
        let name = format!("{prefix}{n}");
        let _ = write!(self.statics, "static {name}: Filter = {expr};\n\n");
        name
    }

    /// The name this card already gave `expr`, if it gave it one.
    ///
    /// Matched on the **whole** declaration rather than by searching for the
    /// expression anywhere in the text, because one filter is often a
    /// substring of another: `Filter::And(&[A, B])` sits inside
    /// `Filter::Or(&[Filter::And(&[A, B]), C])`, and a contains-test would
    /// hand back the name of a filter that says something else.
    ///
    /// The prefix has to match too. A name is what the card calls the
    /// thing, so `TARGET1` reused as the `CHECK` of a clause would read as
    /// an ability aiming at what it is asking about.
    fn declared(&self, prefix: &str, expr: &str) -> Option<String> {
        self.statics.split("static ").skip(1).find_map(|block| {
            let (name, rest) = block.split_once(": Filter = ")?;
            let body = rest.strip_suffix(";\n\n")?;
            (name.starts_with(prefix) && body == expr).then(|| name.to_string())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::CardBody;

    /// Two sentences about the same filter are one `static`, and two
    /// sentences about different ones are two — including the pair where
    /// one filter is spelled inside the other, which is where a reader that
    /// searched the text for the expression would go wrong.
    #[test]
    fn one_filter_is_named_once_however_often_the_card_says_it() {
        let inner = "Filter::And(&[Filter::CREATURE, Filter::ControlledByYou])";
        let outer = format!("Filter::Or(&[{inner}, Filter::LAND])");
        let mut body = CardBody::default();
        assert_eq!(body.filter_static("CHECK", inner), "CHECK1");
        assert_eq!(body.filter_static("CHECK", inner), "CHECK1", "the same one");
        assert_eq!(body.filter_static("CHECK", &outer), "CHECK2", "a wider one");
        assert_eq!(
            body.filter_static("TARGET", inner),
            "TARGET3",
            "the same expression under another prefix is another name"
        );
        assert_eq!(body.statics.matches("static ").count(), 3);
    }
}
