//! Which card a projected name belongs to, and which picture that card wears.
//!
//! The companion to [`crate::tokenart`] and it exists for the same reason: the
//! answer is in the compiled card registry, which `baylee-client-core`
//! deliberately does not link, so the policy stays down there and the lookup
//! is handed in from up here.
//!
//! What needs it is the copy. A permanent that has become a copy of another
//! card keeps its own cardboard — [`baylee_view::PublicObject::card`] is the
//! Clone, in every zone the Clone ever visits — while `name` is the
//! projection. So the view says "Clone" and "Llanowar Elves" in one breath
//! and a client drawing the first under the second was drawing a card that
//! does not exist. A token a copy effect made is the same question with the
//! harder half missing: it has no card and no registry token either, so its
//! name was all there ever was to go on and it drew as a coloured rectangle.

use baylee_core::ids::CardIndex;
use std::collections::HashMap;
use std::sync::OnceLock;

/// The printed card an index wears the picture of.
///
/// `None` for an index no card claims and for a card codegen recorded no
/// printing for, which are both the same answer to the renderer: draw the
/// face rather than fetch a certain 404.
#[must_use]
pub fn of(index: CardIndex) -> Option<&'static str> {
    baylee_cards::by_index(index)
        .map(|c| c.scryfall_id)
        .filter(|s| !s.is_empty())
}

/// The card in the registry that is printed with `name`, and which of its
/// faces carries it.
///
/// A face rather than a card, because a copy of a transformed permanent takes
/// the name of the face that is *up* (CR 707.2 through CR 711.4a), and a
/// lookup answering only an index would draw the front of a card the table is
/// showing the back of.
///
/// Front faces are indexed first and a back face only fills a name no front
/// face claimed, so a card whose back happens to share another card's printed
/// name cannot take that name away from it. No name in today's pool is
/// carried by two cards at all, and
/// `every_name_the_registry_prints_leads_back_to_a_picture` is what says so
/// rather than a comment claiming it; the ordering is the answer prepared in
/// advance for the day one appears, and the front is the one a player means.
#[must_use]
pub fn wearing(name: &str) -> Option<(CardIndex, u8)> {
    static BY_NAME: OnceLock<HashMap<&'static str, (CardIndex, u8)>> = OnceLock::new();
    BY_NAME
        .get_or_init(|| {
            let mut map: HashMap<&'static str, (CardIndex, u8)> = HashMap::new();
            for card in baylee_cards::all() {
                if let Some(front) = card.faces.first() {
                    map.entry(front.name).or_insert((card.index, 0));
                }
            }
            for card in baylee_cards::all() {
                for (i, face) in card.faces.iter().enumerate().skip(1) {
                    let face_index = u8::try_from(i).unwrap_or(u8::MAX);
                    map.entry(face.name).or_insert((card.index, face_index));
                }
            }
            map
        })
        .get(name)
        .copied()
}

#[cfg(test)]
mod tests {
    use super::{of, wearing};

    /// The two halves have to meet: a name is looked up, and the index that
    /// comes back has to be one `of` can turn into a picture. This is the
    /// only place either is read, so a registry that renumbered under them
    /// would be caught nowhere else.
    ///
    /// It asserts that for *every* face and not only for fronts, which is a
    /// claim about the pool rather than about the lookup: no face in it is
    /// printed with a name another card also carries. Were one, the second
    /// card would answer with the first card's index and the copy of a
    /// transformed permanent would be drawn as somebody else entirely. The
    /// front-first pass in [`wearing`] decides who wins such a tie; this is
    /// what says the tie is not being played today, and it is the thing to
    /// read first if it ever fails.
    #[test]
    fn every_name_the_registry_prints_leads_back_to_a_picture() {
        let mut checked = 0usize;
        for card in baylee_cards::all() {
            for face in card.faces {
                let Some((index, _)) = wearing(face.name) else {
                    panic!("{} is printed and cannot be found by its name", face.name);
                };
                assert_eq!(
                    index, card.index,
                    "{} does not answer with its own card",
                    face.name
                );
                assert!(
                    of(index).is_some(),
                    "{} has a name but no printing to draw",
                    face.name
                );
                checked += 1;
            }
        }
        assert!(checked > 1000, "only {checked} faces were reachable");
    }

    /// The counter-test. A projected name is a `String` off the wire and the
    /// engine may name a token anything at all; an answer for one of those
    /// would be a picture of the wrong card, which is worse than none.
    #[test]
    fn a_name_no_card_is_printed_with_answers_nothing() {
        assert_eq!(wearing("Soldier"), None);
        assert_eq!(wearing(""), None);
        assert_eq!(wearing("Llanowar  Elves"), None, "and it is not fuzzy");
        assert_eq!(of(baylee_core::ids::CardIndex::new(u32::MAX)), None);
    }

    /// The case the whole module exists for, spelled out with a card that is
    /// actually in the pool: what a Clone becomes when it copies one.
    #[test]
    fn the_card_a_copy_wears_is_found_by_the_name_it_projects() {
        let (index, face) = wearing("Llanowar Elves").expect("the pool has Llanowar Elves");
        assert_eq!(face, 0);
        let card = baylee_cards::by_index(index).expect("the index is the registry's own");
        assert_eq!(card.faces[0].name, "Llanowar Elves");
        assert_eq!(of(index), Some(card.scryfall_id));
    }
}
