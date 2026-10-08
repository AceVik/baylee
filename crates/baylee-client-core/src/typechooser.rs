//! The creature-type chooser's lists (the owner, 08.10.2026; DESIGN-v3
//! §6.3, v4 C3-4, v5 §6.3).
//!
//! *"A QUICK-CHOOSE list on top ordered by how many creatures of that type
//! are in MY deck … show the count on each chip, e.g. 'Elf · 14'; top ~8,
//! ties alphabetical; types with 0 not in the quick list), then the full list
//! below (alphabetical, localised names, filtered by the field)."*
//!
//! Renderer-free: what the quick list holds and in what order, what the full
//! list holds under a filter, and the phone's letter groups. Only types the
//! engine offered are ever in either list, and every row keeps the index of
//! the offer it stands for, which is the answer the engine is sent.
//!
//! **A changeling counts for no type.** It is every creature type at once
//! (CR 702.73a), so it would add one to every offered type alike and move
//! nothing in the order; counted, it would only push a deck's real tribes off
//! the top behind types the deck merely contains by rule. The quick list is
//! about what the deck is built around, and a changeling is not that.

use baylee_core::ids::SubtypeId;

/// One card of the deck, as the quick list reads it.
#[derive(Clone, Copy, Debug)]
pub struct DeckCard<'a> {
    /// How many copies.
    pub copies: u32,
    /// Whether it is a creature card (the types of its front face).
    pub creature: bool,
    /// Whether it has changeling, which counts it for no type here.
    pub changeling: bool,
    /// Its printed subtypes.
    pub subtypes: &'a [SubtypeId],
}

/// One row of a list: the offer it answers and what it is called.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TypeRow {
    /// The index of the engine's offer this row answers.
    pub index: usize,
    /// The type's name, in the interface's language.
    pub name: String,
    /// How many of the deck's creatures have it (the quick list only).
    pub count: u32,
}

/// The quick list's length.
pub const QUICK: usize = 8;

/// The quick list: the offered types the deck's creatures carry most, most
/// first, ties by name, none that no creature carries; at most [`QUICK`].
#[must_use]
pub fn quick<'a>(
    offered: &[SubtypeId],
    deck: impl IntoIterator<Item = DeckCard<'a>>,
    name: impl Fn(SubtypeId) -> Option<String>,
) -> Vec<TypeRow> {
    let mut counts = vec![0u32; offered.len()];
    for card in deck {
        if !card.creature || card.changeling {
            continue;
        }
        for subtype in card.subtypes {
            if let Some(at) = offered.iter().position(|o| o == subtype) {
                counts[at] = counts[at].saturating_add(card.copies);
            }
        }
    }
    let mut rows: Vec<TypeRow> = offered
        .iter()
        .zip(counts)
        .enumerate()
        .filter(|(_, (_, count))| *count > 0)
        .filter_map(|(index, (id, count))| {
            Some(TypeRow {
                index,
                name: name(*id)?,
                count,
            })
        })
        .collect();
    rows.sort_by(|a, b| {
        b.count
            .cmp(&a.count)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    rows.truncate(QUICK);
    rows
}

/// The full list: every offered type whose name (or English name) starts
/// with `filter`, alphabetical in the interface's language. A type with no
/// name in the table keeps a row by its number while nothing is typed.
#[must_use]
pub fn full(
    offered: &[SubtypeId],
    filter: &str,
    name: impl Fn(SubtypeId) -> Option<(String, String)>,
) -> Vec<TypeRow> {
    let needle = filter.trim().to_lowercase();
    let mut rows: Vec<TypeRow> = offered
        .iter()
        .enumerate()
        .filter_map(|(index, id)| match name(*id) {
            Some((shown, english)) => (shown.to_lowercase().starts_with(&needle)
                || english.to_lowercase().starts_with(&needle))
            .then_some(TypeRow {
                index,
                name: shown,
                count: 0,
            }),
            None => needle.is_empty().then(|| TypeRow {
                index,
                name: format!("#{}", id.get()),
                count: 0,
            }),
        })
        .collect();
    rows.sort_by_cached_key(|row| (row.name.starts_with('#'), collated(&row.name)));
    rows
}

/// A name as an alphabet orders it: lower case, an umlaut under its base
/// letter (`Bär` before `Bürger`), `ß` as `ss`.
fn collated(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .flat_map(|c| match c {
            'ä' => vec!['a'],
            'ö' => vec!['o'],
            'ü' => vec!['u'],
            'ß' => vec!['s', 's'],
            other => vec![other],
        })
        .collect()
}

/// The letter groups (DESIGN-v5 §6.3, C3-4): six cells over the full list,
/// a name filed under its first letter, an umlaut under its base letter. The
/// fifth takes Q as well, which the design's labels leave out and no type
/// needs a cell of its own for.
pub const GROUPS: [(char, char); 6] = [
    ('A', 'D'),
    ('E', 'H'),
    ('I', 'L'),
    ('M', 'P'),
    ('Q', 'T'),
    ('U', 'Z'),
];

/// What each of [`GROUPS`] says on its cell.
pub const GROUP_LABELS: [&str; 6] = ["A–D", "E–H", "I–L", "M–P", "R–T", "U–Z"];

/// Which of [`GROUPS`] a name is filed under.
#[must_use]
pub fn group_of(name: &str) -> usize {
    let first = name
        .chars()
        .next()
        .map_or('A', |c| match c.to_ascii_uppercase() {
            'Ä' | 'ä' => 'A',
            'Ö' | 'ö' => 'O',
            'Ü' | 'ü' => 'U',
            other => other.to_ascii_uppercase(),
        });
    GROUPS
        .iter()
        .position(|(from, to)| (*from..=*to).contains(&first))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u16) -> SubtypeId {
        SubtypeId::new(n)
    }

    fn named(n: SubtypeId) -> Option<String> {
        Some(
            match n.get() {
                1 => "Elf",
                2 => "Goblin",
                3 => "Ally",
                4 => "Zombie",
                5 => "Druid",
                _ => return None,
            }
            .to_string(),
        )
    }

    fn creature(copies: u32, subtypes: &[SubtypeId]) -> DeckCard<'_> {
        DeckCard {
            copies,
            creature: true,
            changeling: false,
            subtypes,
        }
    }

    /// Red if the list were alphabetical: Goblin (9) before Elf (6) before
    /// Ally (2), and Zombie with no creature out of it.
    #[test]
    fn the_quick_list_is_ordered_by_the_decks_creatures_not_by_name() {
        let offered = [id(1), id(2), id(3), id(4)];
        let goblins = [id(2)];
        let elves = [id(1), id(5)];
        let allies = [id(3)];
        let deck = [
            creature(4, &goblins),
            creature(5, &goblins),
            creature(6, &elves),
            creature(2, &allies),
        ];
        let rows = quick(&offered, deck, named);
        let said: Vec<(&str, u32)> = rows.iter().map(|r| (r.name.as_str(), r.count)).collect();
        assert_eq!(said, [("Goblin", 9), ("Elf", 6), ("Ally", 2)]);
        // Each row answers with the offer's own index.
        assert_eq!(rows.iter().map(|r| r.index).collect::<Vec<_>>(), [1, 0, 2]);
    }

    /// Only offered types: Druid is on a card but was not offered.
    #[test]
    fn only_offered_types_are_listed() {
        let offered = [id(1)];
        let elves = [id(1), id(5)];
        let rows = quick(&offered, [creature(3, &elves)], named);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].name, "Elf");
        let all = full(&offered, "", |n| named(n).map(|s| (s.clone(), s)));
        assert_eq!(all.len(), 1);
    }

    /// A changeling counts for no type, a noncreature for none, ties by name.
    #[test]
    fn changelings_and_noncreatures_count_for_nothing_and_ties_go_by_name() {
        let offered = [id(2), id(1)];
        let both = [id(1), id(2)];
        let deck = [
            creature(3, &both),
            DeckCard {
                copies: 10,
                creature: true,
                changeling: true,
                subtypes: &[],
            },
            DeckCard {
                copies: 10,
                creature: false,
                changeling: false,
                subtypes: &both,
            },
        ];
        let rows = quick(&offered, deck, named);
        let said: Vec<(&str, u32)> = rows.iter().map(|r| (r.name.as_str(), r.count)).collect();
        assert_eq!(said, [("Elf", 3), ("Goblin", 3)]);
    }

    /// The full list filters on the shown name or the English one, and is
    /// alphabetical.
    #[test]
    fn the_full_list_filters_and_is_alphabetical() {
        let offered = [id(4), id(1), id(2)];
        let german = |n: SubtypeId| named(n).map(|english| (english.clone(), english));
        let all = full(&offered, "", german);
        assert_eq!(
            all.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
            ["Elf", "Goblin", "Zombie"]
        );
        let g = full(&offered, "go", german);
        assert_eq!(g.len(), 1);
        assert_eq!(g[0].index, 2);
    }

    #[test]
    fn an_umlaut_sorts_under_its_base_letter() {
        assert!(collated("Bär") < collated("Bürger"));
        assert!(collated("Bär") < collated("Buschköter"));
    }

    #[test]
    fn umlauts_file_under_their_base_letter() {
        assert_eq!(group_of("Ärger"), 0);
        assert_eq!(group_of("Elf"), 1);
        assert_eq!(group_of("Zombie"), 5);
        assert_eq!(group_of("Ork"), 3);
    }
}
