//! What a seat's plate and its chip say (the owner's requests of 08.10.2026).
//!
//! Two surfaces describe a seat: its **chip** in the players' strip at the
//! bottom left, and its **plate** on the table at its mat's outer edge. Both
//! are now the same few lines, in the same order, read from one place:
//!
//! 1. the name and the life, and a crown on the monarch (CR 724);
//! 2. the details: hand (and ∞ where no maximum hand size applies), library,
//!    graveyard and exile, then poison, energy and the worst single
//!    commander's damage when they are not zero;
//! 3. on the plate only, the mana floating in the seat's pool, when there is
//!    any (and, on the seat's own plate in a payment window, what is still
//!    owed).
//!
//! Only what the view carries is said: experience counters, for instance,
//! are not in [`baylee_view::SeatView`] and so are nowhere here. Nothing is
//! derived that the view could disagree with — this module is a reading of
//! the view, no more, which is what lets both surfaces share it and a test
//! hold it without a renderer.

use baylee_core::ids::PlayerId;
use baylee_view::PlayerView;

use crate::i18n::{Lang, Phrase};
use crate::manapool::Floating;

/// One item of a plate's second line, in the order it is drawn.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Detail {
    /// Cards in hand; `unlimited` when an effect removed the seat's maximum
    /// hand size, which the plate says with the ∞ mark alone.
    Hand {
        /// How many.
        count: u32,
        /// No maximum hand size.
        unlimited: bool,
    },
    /// Cards in the library.
    Library(u32),
    /// Cards in the graveyard.
    Graveyard(u32),
    /// Cards in public exile.
    Exile(u32),
    /// Poison counters (only when not zero).
    Poison(u16),
    /// Energy counters (only when not zero).
    Energy(u16),
    /// The most combat damage one commander has dealt the seat (only when
    /// not zero): the number CR 903.10a's twenty-one is counted against.
    Commander(u16),
}

impl Detail {
    /// Poison at or over this is written in danger: three more is ten.
    pub const POISON_HIGH: u16 = 7;
    /// Commander damage at or over this is written in danger: five more is
    /// twenty-one.
    pub const COMMANDER_HIGH: u16 = 16;

    /// Whether this item is close to losing the seat the game.
    #[must_use]
    pub const fn dangerous(self) -> bool {
        match self {
            Self::Poison(n) => n >= Self::POISON_HIGH,
            Self::Commander(n) => n >= Self::COMMANDER_HIGH,
            _ => false,
        }
    }

    /// The number this item shows.
    #[must_use]
    pub fn number(self) -> String {
        match self {
            Self::Hand { count, .. }
            | Self::Library(count)
            | Self::Graveyard(count)
            | Self::Exile(count) => count.to_string(),
            Self::Poison(n) | Self::Energy(n) | Self::Commander(n) => n.to_string(),
        }
    }

    /// The item said in words, for a tooltip and an accessible name.
    #[must_use]
    pub fn words(self, lang: Lang) -> String {
        let n = self.number();
        match self {
            Self::Hand { unlimited, .. } => {
                let hand = Phrase::PlateHand.fill(lang, &[&n]);
                if unlimited {
                    format!("{hand} ({})", Phrase::NoMaxHandSize.text(lang))
                } else {
                    hand
                }
            }
            Self::Library(_) => Phrase::PlateLibrary.fill(lang, &[&n]),
            Self::Graveyard(_) => Phrase::PlateGraveyard.fill(lang, &[&n]),
            Self::Exile(_) => Phrase::PlateExile.fill(lang, &[&n]),
            Self::Poison(_) => Phrase::PlatePoison.fill(lang, &[&n]),
            Self::Energy(_) => Phrase::PlateEnergy.fill(lang, &[&n]),
            Self::Commander(_) => Phrase::PlateCommander.fill(lang, &[&n]),
        }
    }
}

/// Everything a seat's plate and chip say, read off one view.
#[derive(Clone, PartialEq, Debug)]
pub struct SeatPlate {
    /// Whose.
    pub player: PlayerId,
    /// Life total.
    pub life: i32,
    /// Whether this seat is the monarch (CR 724).
    pub monarch: bool,
    /// Whether the seat is out of the game.
    pub lost: bool,
    /// The second line, in drawing order.
    pub details: Vec<Detail>,
    /// The third line: what floats in the seat's pool, in WUBRG+C order and
    /// then the restricted mana. Empty when nothing floats.
    pub pool: Vec<Floating>,
}

impl SeatPlate {
    /// The plate of `player` in `view`, or `None` for a seat the view does
    /// not list.
    #[must_use]
    pub fn of(view: &PlayerView, player: PlayerId) -> Option<Self> {
        let seat = view.seat(player)?;
        let exile = view
            .exile
            .get(player.get() as usize)
            .map_or(0, |pile| u32::try_from(pile.len()).unwrap_or(u32::MAX));
        let commander = seat
            .commander_damage
            .iter()
            .map(|d| d.amount)
            .max()
            .unwrap_or(0);
        let mut details = vec![
            Detail::Hand {
                count: seat.hand_count,
                unlimited: seat.no_max_hand_size,
            },
            Detail::Library(seat.library_count),
            Detail::Graveyard(seat.graveyard_count),
            Detail::Exile(exile),
        ];
        for (n, detail) in [
            (seat.poison, Detail::Poison(seat.poison)),
            (seat.energy, Detail::Energy(seat.energy)),
            (commander, Detail::Commander(commander)),
        ] {
            if n > 0 {
                details.push(detail);
            }
        }
        Some(Self {
            player,
            life: seat.life,
            monarch: view.monarch == Some(player),
            lost: seat.has_lost(),
            details,
            pool: crate::manapool::row(&seat.mana_pool),
        })
    }

    /// How many lines the table's plate draws: two, or three while mana
    /// floats.
    #[must_use]
    pub const fn lines(&self) -> u8 {
        if self.pool.is_empty() { 2 } else { 3 }
    }

    /// The whole plate in one sentence, for a tooltip and an accessible name:
    /// `name` (the seat as its rim calls it) first, then each fact.
    #[must_use]
    pub fn describe(&self, lang: Lang, name: &str) -> String {
        let mut parts = vec![name.to_string()];
        if self.monarch {
            parts.push(Phrase::PlateMonarch.text(lang).to_string());
        }
        if self.lost {
            parts.push(Phrase::PlateLost.text(lang).to_string());
            return parts.join(" · ");
        }
        parts.push(Phrase::PlateLife.fill(lang, &[&self.life.to_string()]));
        parts.extend(self.details.iter().map(|d| d.words(lang)));
        if !self.pool.is_empty() {
            let mana: Vec<String> = self
                .pool
                .iter()
                .map(|f| {
                    let restricted = if f.restricted { "*" } else { "" };
                    format!("{}{restricted}×{}", letter(f.color), f.count)
                })
                .collect();
            parts.push(format!(
                "{}: {}",
                Phrase::ManaPool.text(lang),
                mana.join(" ")
            ));
        }
        parts.join(" · ")
    }
}

/// The letter a mana symbol is written with: `W U B R G C`.
const fn letter(color: baylee_core::mana::ManaColor) -> &'static str {
    use baylee_core::mana::ManaColor;
    match color {
        ManaColor::White => "W",
        ManaColor::Blue => "U",
        ManaColor::Black => "B",
        ManaColor::Red => "R",
        ManaColor::Green => "G",
        ManaColor::Colorless => "C",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::ViewBuilder;

    #[test]
    fn the_hand_says_unlimited_only_when_the_view_does() {
        let mut view = ViewBuilder::new(2).build();
        let plate = SeatPlate::of(&view, PlayerId::new(1)).expect("seat 1");
        assert_eq!(
            plate.details[0],
            Detail::Hand {
                count: 7,
                unlimited: false
            }
        );
        view.seats[1].no_max_hand_size = true;
        let plate = SeatPlate::of(&view, PlayerId::new(1)).expect("seat 1");
        assert_eq!(
            plate.details[0],
            Detail::Hand {
                count: 7,
                unlimited: true
            }
        );
        // The number is the count alone; the ∞ is a mark, never "7/∞".
        assert_eq!(plate.details[0].number(), "7");
    }

    #[test]
    fn the_crown_follows_the_monarch() {
        let mut view = ViewBuilder::new(3).build();
        let crowned = |view: &PlayerView| -> Vec<u8> {
            (0..3u8)
                .filter(|p| {
                    SeatPlate::of(view, PlayerId::new(*p))
                        .expect("a seat")
                        .monarch
                })
                .collect()
        };
        assert!(crowned(&view).is_empty(), "no monarch, no crown");
        view.monarch = Some(PlayerId::new(2));
        assert_eq!(crowned(&view), [2]);
        view.monarch = Some(PlayerId::new(0));
        assert_eq!(crowned(&view), [0], "the crown moves, it is not copied");
    }

    #[test]
    fn the_details_read_in_order_and_counters_only_when_they_count() {
        let mut view = ViewBuilder::new(2).build();
        let plate = SeatPlate::of(&view, PlayerId::new(1)).expect("seat 1");
        assert_eq!(
            plate.details,
            [
                Detail::Hand {
                    count: 7,
                    unlimited: false
                },
                Detail::Library(80),
                Detail::Graveyard(2),
                Detail::Exile(0),
            ]
        );
        view.seats[1].poison = 8;
        view.seats[1].energy = 3;
        view.seats[1]
            .commander_damage
            .push(baylee_view::CommanderDamage {
                source: baylee_core::ids::ObjectId::new(9, 0),
                amount: 11,
            });
        view.seats[1]
            .commander_damage
            .push(baylee_view::CommanderDamage {
                source: baylee_core::ids::ObjectId::new(10, 0),
                amount: 4,
            });
        let plate = SeatPlate::of(&view, PlayerId::new(1)).expect("seat 1");
        assert_eq!(
            &plate.details[4..],
            [Detail::Poison(8), Detail::Energy(3), Detail::Commander(11)]
        );
        assert!(Detail::Poison(8).dangerous());
        assert!(!Detail::Commander(11).dangerous());
    }

    #[test]
    fn a_third_line_stands_only_while_mana_floats() {
        let mut view = ViewBuilder::new(2).build();
        assert_eq!(SeatPlate::of(&view, PlayerId::new(1)).unwrap().lines(), 2);
        view.seats[1].mana_pool.green = 2;
        let plate = SeatPlate::of(&view, PlayerId::new(1)).unwrap();
        assert_eq!(plate.lines(), 3);
        assert_eq!(plate.pool[0].count, 2);
        assert!(
            plate.describe(Lang::De, "Anna").contains("Manavorrat"),
            "{}",
            plate.describe(Lang::De, "Anna")
        );
    }

    #[test]
    fn the_sentence_names_every_fact_it_draws() {
        let mut view = ViewBuilder::new(2).build();
        view.monarch = Some(PlayerId::new(1));
        view.seats[1].no_max_hand_size = true;
        let said = SeatPlate::of(&view, PlayerId::new(1))
            .unwrap()
            .describe(Lang::En, "Anna");
        for word in [
            "Anna",
            "Monarch",
            "40 life",
            "7 in hand",
            "No maximum hand size",
            "80 in library",
            "2 in graveyard",
            "0 in exile",
        ] {
            assert!(said.contains(word), "{word:?} missing from {said:?}");
        }
    }
}
