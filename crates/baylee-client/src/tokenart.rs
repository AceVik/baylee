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

/// The registry token printed under a name, if one is.
///
/// The lookup a *copy of a token* needs. A permanent copying a Soldier keeps
/// its own cardboard and projects the name "Soldier", and asking
/// [`crate::cardart::wearing`] about that name gets `None` — no card is
/// printed with it — which is the same answer a permanent copying nothing
/// gives. So a Clone on a Soldier was drawn as a Clone, wearing no mark, with
/// "Soldier" written underneath it.
///
/// **First of a name wins**, and unlike [`crate::cardart::wearing`]'s
/// front-first rule that tie is being played today: `ALL` holds two tokens
/// named "Shapeshifter", the 1/1 changeling and the 2/2 blue one, and a copy
/// of either is drawn as the 1/1. There is nothing in a projected name to
/// tell them apart — two tokens with one name are one name — and the card a
/// player is looking at is drawn correctly in every other respect. What must
/// *not* happen is the reverse, and does not: a Shapeshifter token on the
/// table is drawn from its own id, never from its name.
#[must_use]
pub fn wearing(name: &str) -> Option<u16> {
    baylee_cards::tokens::ALL
        .iter()
        .position(|t| t.name == name)
        .and_then(|i| u16::try_from(i).ok())
}

/// The name a token id is printed with.
///
/// The other half of [`wearing`], and the half that keeps a token from being
/// read as a copy of itself: a token's own name is only reachable by id,
/// because two of them share one.
#[must_use]
pub fn name(id: u16) -> Option<&'static str> {
    baylee_cards::tokens::by_token_id(id).map(|t| t.name)
}

#[cfg(test)]
mod tests {
    /// The registry and the client have to agree about what a token id is:
    /// the number the engine stamps on the object is an index into `ALL`, and
    /// this is the only place that reads it back.
    ///
    /// A token with no printing chosen for it is the *other* half of that
    /// agreement and not an omission: [`super::of`] answers `None` and the
    /// chit is drawn from its face, which is what an empty `scryfall_id`
    /// means. This test asked `Some(token.scryfall_id)` of every row, which
    /// was true only while every token in the table was one a person had
    /// written — the first token a reader wrote broke it, and the thing it
    /// broke on was the fallback working.
    ///
    /// The floor is the fourteen the ledger was seeded with, so a table that
    /// lost every picture it has fails here rather than passing an
    /// assertion about nothing.
    #[test]
    fn every_token_the_registry_defines_can_be_drawn() {
        let mut printed = 0usize;
        for (i, token) in baylee_cards::tokens::ALL.iter().enumerate() {
            let id = u16::try_from(i).expect("a token id fits in u16");
            if token.scryfall_id.is_empty() {
                assert_eq!(
                    super::of(id),
                    None,
                    "{} has no printing chosen and must fall back to its face",
                    token.name
                );
                continue;
            }
            printed += 1;
            assert_eq!(
                super::of(id),
                Some(token.scryfall_id),
                "{} is not reachable by its own id",
                token.name
            );
        }
        assert!(
            printed >= 14,
            "only {printed} tokens wear a printing; the ledger was seeded with fourteen"
        );
        assert_eq!(
            super::of(u16::MAX),
            None,
            "an id no token has must not resolve to one that does"
        );
    }

    /// A name reaches a token, and every token's own name reaches itself.
    ///
    /// The second half is the one the board leans on: [`super::name`] is how
    /// a token on the table says what it is called, and a token whose id
    /// answered nothing would be judged a copy of whatever its name found.
    #[test]
    fn a_token_can_be_found_by_its_name_and_asked_for_it_back() {
        for (i, token) in baylee_cards::tokens::ALL.iter().enumerate() {
            let id = u16::try_from(i).expect("a token id fits in u16");
            assert_eq!(super::name(id), Some(token.name));
            let found = super::wearing(token.name).expect("its own name finds a token");
            assert_eq!(
                super::name(found),
                Some(token.name),
                "{} was found under another name",
                token.name
            );
        }
        assert_eq!(super::wearing("Not A Token"), None);
        assert_eq!(super::name(u16::MAX), None);
    }

    /// The tie `wearing` documents is real, and it is the first of the two.
    ///
    /// Two tokens are printed "Shapeshifter". A copy of either is drawn as
    /// the first, because a projected name is all a copy carries — and the
    /// chits themselves are drawn from their own ids, so the *tokens* are
    /// never confused with each other. That is the whole reason `name`
    /// exists beside `wearing`.
    #[test]
    fn two_tokens_share_a_name_and_the_first_of_them_answers() {
        let twins: Vec<u16> = (0..baylee_cards::tokens::ALL.len())
            .filter_map(|i| u16::try_from(i).ok())
            .filter(|id| super::name(*id) == Some("Shapeshifter"))
            .collect();
        assert_eq!(twins.len(), 2, "the pool mints two Shapeshifters");
        assert_eq!(super::wearing("Shapeshifter"), Some(twins[0]));
        assert_ne!(super::of(twins[0]), super::of(twins[1]), "and two pictures");
    }
}
