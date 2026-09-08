//! Which picture a token wears.
//!
//! One function, and it exists as its own module for the reason
//! [`crate::manasources`] does: the answer lives in the compiled card
//! registry, and `baylee-client-core` deliberately does not link it. So the
//! policy — cache keys, budgets, URLs — stays down there and this is the
//! lookup handed in from up here.

/// The printed token card a token id wears the picture of.
///
/// `None` for a token nobody has chosen art for, and for anything that is not
/// a registry token at all: a copy token has no entry here, because it is a
/// copy of a *card* rather than of one of `baylee_cards::tokens::ALL`. Both
/// fall back to the drawn face, which is correct — a request built out of an
/// empty id is a guaranteed 404 every time the card is drawn.
#[must_use]
pub fn of(id: u16) -> Option<&'static str> {
    baylee_cards::tokens::by_token_id(id)
        .map(|t| t.scryfall_id)
        .filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    /// The registry and the client have to agree about what a token id is:
    /// the number the engine stamps on the object is an index into `ALL`, and
    /// this is the only place that reads it back.
    #[test]
    fn every_token_the_registry_defines_can_be_drawn() {
        for (i, token) in baylee_cards::tokens::ALL.iter().enumerate() {
            let id = u16::try_from(i).expect("a token id fits in u16");
            assert_eq!(
                super::of(id),
                Some(token.scryfall_id),
                "{} is not reachable by its own id",
                token.name
            );
        }
        assert_eq!(
            super::of(u16::MAX),
            None,
            "an id no token has must not resolve to one that does"
        );
    }
}
