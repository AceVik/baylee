//! Random documents through every format: a format that names no loss must
//! give back what it was given, and what it did write must read the same
//! again.

use baylee_core::deckrow::MAX_NOTE;
use baylee_core::preset::Finish;
use baylee_deckio::document::{Card, Document, Zone};
use baylee_deckio::format::{self, FormatId};

/// A small generator with a fixed stream, so a failure names its own seed.
struct Rng(u64);

impl Rng {
    fn next(&mut self, bound: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        usize::try_from(self.0 % bound as u64).unwrap()
    }

    fn pick<T: Copy>(&mut self, from: &[T]) -> T {
        from[self.next(from.len())]
    }

    /// One of these, or nothing, evenly.
    fn maybe(&mut self, from: &[&str]) -> Option<String> {
        let at = self.next(from.len() + 1);
        from.get(at).map(|text| (*text).to_owned())
    }
}

/// Names as cards have them, and some that look like the formats' own words.
const NAMES: [&str; 16] = [
    "Forest",
    "Lightning Bolt",
    "Fire // Ice",
    "Jetmir's Garden",
    "Æther Vial",
    "Lim-Dûl's Vault",
    "Circle of Protection: Red",
    "Deck",
    "Sideboard",
    "Commander",
    "Main: Event",
    "1996 World Champion",
    "X",
    "+2 Mace",
    "Who // What // When // Where // Why",
    "Erase (Not the Urza's Legacy One)",
];

fn document(rng: &mut Rng) -> Document {
    let zones = [Zone::Main, Zone::Side, Zone::Commander, Zone::Maybe];
    let finishes = [
        None,
        Some(Finish::Normal),
        Some(Finish::Foil),
        Some(Finish::Etched),
        Some(Finish::Holographic),
        Some(Finish::Glitter),
        Some(Finish::Galaxy),
    ];
    let cards = (0..=rng.next(8))
        .map(|_| Card {
            zone: rng.pick(&zones),
            count: u32::try_from(rng.next(60) + 1).unwrap(),
            name: rng.pick(&NAMES).into(),
            set: rng.maybe(&["SOI", "PLST"]),
            collector_number: rng.maybe(&["5", "250s", "DDR-7"]),
            lang: rng.maybe(&["de", "ja"]),
            finish: rng.pick(&finishes),
            scryfall_id: rng.maybe(&["0192d0c5-7b50-7a1f-8c2d-3e4f5a6b7c01"]),
            choices: baylee_core::card_metadata::DeckChoices::default(),
            note: rng.maybe(&["keep", "a # b", "*F* (not foil)", &"n".repeat(MAX_NOTE)]),
        })
        .collect();
    Document {
        name: rng.maybe(&["Burn", "Name: odd"]),
        format: rng.maybe(&["modern"]),
        cards,
        ..Document::default()
    }
}

/// Cards in the order a deck list groups them, each zone in its own order.
fn grouped(document: &Document) -> Document {
    let mut document = document.clone();
    document.cards.sort_by_key(|card| card.zone);
    for card in &mut document.cards {
        // A normal finish is what no marker means.
        card.finish = card.finish.filter(|finish| *finish != Finish::Normal);
    }
    document
}

/// Every document of the first seeds that could be stored at all.
fn storable() -> impl Iterator<Item = (u64, Document)> {
    (1..=600_u64).filter_map(|seed| {
        let mut rng = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let document = document(&mut rng);
        document.validate().is_ok().then_some((seed, document))
    })
}

#[test]
fn most_random_documents_can_be_stored() {
    assert!(storable().count() > 300, "{}", storable().count());
}

#[test]
fn a_format_that_names_no_loss_gives_back_what_it_was_given() {
    for (seed, document) in storable() {
        for id in FormatId::ALL {
            let written = baylee_deckio::export(id, &document);
            if !written.losses.is_empty() {
                assert!(!id.lossless(), "{id:?} claims to lose nothing, seed {seed}");
                continue;
            }
            let read = format::read(id, &written.text)
                .unwrap_or_else(|why| panic!("{id:?}, seed {seed}: {why}\n{}", written.text));
            assert!(read.skipped.is_empty(), "{id:?}, seed {seed}");
            assert_eq!(read.document.name, document.name, "{id:?}, seed {seed}");
            assert_eq!(
                grouped(&read.document).cards,
                grouped(&document).cards,
                "{id:?}, seed {seed}:\n{}",
                written.text
            );
        }
    }
}

#[test]
fn what_a_format_wrote_is_written_the_same_again() {
    for (seed, document) in storable() {
        for id in FormatId::ALL {
            let once = baylee_deckio::export(id, &document).text;
            let Ok(read) = format::read(id, &once) else {
                assert!(
                    document.cards.iter().all(|card| card.zone == Zone::Maybe),
                    "{id:?}, seed {seed}:\n{once}"
                );
                continue;
            };
            let twice = baylee_deckio::export(id, &read.document).text;
            assert_eq!(twice, once, "{id:?}, seed {seed}");
        }
    }
}
