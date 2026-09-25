//! Reads a stored deck's rows into what the lobby's deck list says about it
//! (#254): [`baylee_core::deckdigest`]'s types, against this registry.
//!
//! The gateway answers `GET /decks` with this, and the client's offline
//! lobby answers the same request with it, so a deck reads the same with and
//! without a gateway. Nothing here touches a database or a catalog. A row
//! this build cannot parse or name is counted where it can be and otherwise
//! skipped, because a summary that refused a whole deck over one row would
//! be a sidebar that went blank.

use baylee_cards_dsl::CardDef;
use baylee_core::color::ColorSet;
use baylee_core::deckdigest::{Digest, Leader};
use baylee_core::deckrow;

use crate::pool::letters;

/// The registry's card for a name, if this build knows it.
fn named(name: &str) -> Option<&'static CardDef> {
    crate::decks::by_name(name).and_then(crate::by_index)
}

/// The colours any of these cards' identities holds (CR 903.4), as `WUBRG`
/// letters.
fn identity<'a>(cards: impl Iterator<Item = &'a str>) -> String {
    letters(cards.filter_map(named).fold(ColorSet::EMPTY, |held, card| {
        held.union(card.color_identity)
    }))
}

/// Describes one stored deck: its rows, its sideboard's rows and the names
/// of its commanders, the three lists the store keeps.
#[must_use]
pub fn digest(cards: &[String], sideboard: &[String], commanders: &[String]) -> Digest {
    let main: Vec<deckrow::Row> = cards
        .iter()
        .filter_map(|r| deckrow::parse(r).ok())
        .collect();
    // Saturating: the gateway caps a deck at 250 cards, but the offline
    // lobby reads a file anyone can edit, and a row may claim a million.
    let copies = main.iter().fold(0u32, |n, r| n.saturating_add(r.count));
    let side_copies = sideboard
        .iter()
        .filter_map(|r| deckrow::parse(r).ok())
        .fold(0u32, |n, r| n.saturating_add(r.count));

    let leaders = commanders
        .iter()
        .map(|name| {
            let card = named(name);
            // The commander's own row, for the printing the player chose. By
            // the card it names, not its spelling: a row may spell a
            // two-faced card as the pool does or as Scryfall does.
            let chosen = main
                .iter()
                .find(|r| {
                    r.name == *name
                        || card.is_some_and(|c| named(&r.name).is_some_and(|n| n.index == c.index))
                })
                .map(|r| &r.print);
            Leader {
                name: name.clone(),
                scryfall_id: chosen
                    .and_then(|p| p.scryfall_id.clone())
                    .or_else(|| card.map(|c| c.scryfall_id.to_string()))
                    .unwrap_or_default(),
                lang: chosen.map_or("en", |p| p.lang_or_default()).to_string(),
                finish: chosen
                    .map(deckrow::PrintChoice::finish_or_default)
                    .unwrap_or_default(),
                has_back_image: card.is_some_and(|c| crate::sides::has_back_image(c.index)),
            }
        })
        .collect();

    // A commander bounds the deck; without one the deck is what it plays.
    let identity = if commanders.is_empty() {
        identity(main.iter().map(|r| r.name.as_str()))
    } else {
        identity(commanders.iter().map(String::as_str))
    };

    Digest {
        copies,
        side_copies,
        identity,
        leaders,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::{PoolCard, rows as registry_rows};
    use baylee_core::preset::Finish;

    /// The first registry card that can lead a deck and has this identity.
    fn leader_with(identity: &str) -> &'static PoolCard {
        registry_rows()
            .iter()
            .find(|c| c.commander && c.identity == identity)
            .unwrap_or_else(|| panic!("the pool has a {identity} commander"))
    }

    /// A card that is in the registry and has exactly this identity.
    fn card_with(identity: &str) -> &'static PoolCard {
        registry_rows()
            .iter()
            .find(|c| !c.commander && !c.basic_land && c.identity == identity)
            .unwrap_or_else(|| panic!("the pool has a {identity} card"))
    }

    fn rows(lines: &[&str]) -> Vec<String> {
        lines.iter().map(ToString::to_string).collect()
    }

    /// Copies, not lines: a four-of is four cards, and the sideboard is
    /// counted on its own.
    #[test]
    fn a_deck_is_counted_in_cards_not_lines() {
        let red = card_with("R");
        let green = card_with("G");
        let d = digest(
            &rows(&[
                &format!("4 {}", red.english_name),
                &format!("3 {}", green.english_name),
            ]),
            &rows(&[&format!("2 {}", red.english_name)]),
            &[],
        );
        assert_eq!((d.copies, d.side_copies), (7, 2));
    }

    /// Without a commander a deck's colours are its cards': the union in
    /// `WUBRG` order, whatever order the rows come in, and a row this build
    /// cannot name is skipped rather than refusing the deck.
    #[test]
    fn a_deck_without_a_commander_is_the_colours_of_its_cards() {
        let green = card_with("G");
        let white = card_with("W");
        let d = digest(
            &rows(&[
                &format!("1 {}", green.english_name),
                "1 A Card No Build Has Ever Heard Of",
                &format!("1 {}", white.english_name),
                "not a row at all",
            ]),
            &[],
            &[],
        );
        assert_eq!(d.identity, "WG");
        assert_eq!(d.copies, 3, "the unknown card is still a card");
        assert!(d.leaders.is_empty());
    }

    /// A commander deck is its commanders' identity (CR 903.4), a partner
    /// pair's together, whatever else the rows hold, and each commander is
    /// pictured by the printing its own row names.
    #[test]
    fn a_commander_deck_is_the_identity_of_both_its_commanders() {
        let first = leader_with("W");
        let second = leader_with("B");
        let stray = card_with("R");
        let chosen = "11111111-2222-3333-4444-555555555555";
        let d = digest(
            &rows(&[
                &format!("1 {} *F* scryfall={chosen}", first.english_name),
                &format!("1 {}", second.english_name),
                &format!("1 {}", stray.english_name),
            ]),
            &[],
            &[first.english_name.clone(), second.english_name.clone()],
        );
        assert_eq!(d.identity, "WB", "the leaders', not the stray red card's");
        assert_eq!(d.leaders.len(), 2);
        assert_eq!(d.leaders[0].name, first.english_name);
        assert_eq!(d.leaders[0].scryfall_id, chosen);
        assert_eq!(d.leaders[0].finish, Finish::Foil);
        assert_eq!(
            d.leaders[1].scryfall_id, second.scryfall_id,
            "no printing named: the registry's"
        );
        assert_eq!(d.leaders[1].finish, Finish::Normal);
        assert_eq!(d.leaders[1].lang, "en");
    }

    /// A commander's row is found by the card it names, so a row in
    /// Scryfall's whole spelling of a two-faced card still gives the
    /// printing the player chose, and the picture knows it has a back.
    #[test]
    fn a_commanders_row_is_found_under_either_spelling() {
        let leader = registry_rows()
            .iter()
            .find(|c| c.commander && !c.alt_names.is_empty())
            .expect("the pool has a commander Scryfall spells otherwise");
        let whole = &leader.alt_names[0];
        let d = digest(
            &rows(&[&format!("1 {whole} *F*")]),
            &[],
            std::slice::from_ref(&leader.english_name),
        );
        let [pictured] = d.leaders.as_slice() else {
            panic!("one commander: {d:?}")
        };
        assert_eq!(pictured.finish, Finish::Foil, "the row named as {whole}");
        assert_eq!(pictured.has_back_image, leader.has_back_image);
        assert_eq!(d.identity, leader.identity);
    }

    /// A count too big for the number says the most it can, rather than
    /// taking the deck list down with it.
    #[test]
    fn a_deck_too_big_to_count_says_the_most_it_can() {
        let red = card_with("R");
        let huge = format!("{} {}", deckrow::MAX_COUNT, red.english_name);
        let many = vec![huge; 5_000];
        let d = digest(&many, &many, &[]);
        assert_eq!((d.copies, d.side_copies), (u32::MAX, u32::MAX));
    }
}
