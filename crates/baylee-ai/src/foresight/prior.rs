//! What a fair seat supposes the other players hide: a weighted list of
//! cards per player, built from the seat's own view and nothing else
//! (`DeckKnowledge::Prior`, `docs/engine-internals.md` §"Forking a
//! simulation").
//!
//! The supposition is that a deck goes on as it began. A constructed deck
//! repeats its cards, so the cards a player has shown (on the battlefield,
//! in a graveyard, in exile, on the stack, in the command zone) are the
//! likeliest to come again; about two cards in five are lands, the ones
//! shown and basic lands for the colours of the spells shown; and beside
//! them, at a low weight, every finished card of the pool that those
//! colours can cast. A player who has shown nothing yet is supposed to play
//! a deck like the seat's own, which is the one deck the seat knows.
//!
//! It reads [`PlayerView`], which holds no card the seat may not see, and
//! the card registry, which is the same for everyone. Whatever the real
//! lists are, the same view gives the same prior.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use baylee_cards_dsl::Coverage;
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::{CardIndex, PlayerId};
use baylee_core::types::TypeSet;
use baylee_view::PlayerView;

/// One player's supposed hidden cards: each with its weight.
pub type Weighted = Vec<(CardIndex, u32)>;

/// The share of a deck that is lands, in fifths.
const LAND_FIFTHS: u64 = 2;
/// What one shown copy of a spell weighs against the whole of the pool's
/// other castable cards, which together weigh [`POOL`].
const SHOWN: u64 = 12;
/// What the pool's other castable cards weigh together.
const POOL: u64 = 6;
/// Of the land share, the tenths given to basic lands of the colours the
/// spells shown need, where lands were shown too.
const BASIC_TENTHS: u64 = 3;

/// The five basic lands, by the colour each makes, and every finished
/// nonland card with its colour identity: read off the registry once.
struct Pool {
    basics: [Option<CardIndex>; 5],
    spells: Vec<(ColorSet, CardIndex)>,
}

const COLORS: [(Color, &str); 5] = [
    (Color::White, "Plains"),
    (Color::Blue, "Island"),
    (Color::Black, "Swamp"),
    (Color::Red, "Mountain"),
    (Color::Green, "Forest"),
];

fn pool() -> &'static Pool {
    static POOL: OnceLock<Pool> = OnceLock::new();
    POOL.get_or_init(|| {
        let mut basics = [None; 5];
        let mut spells = Vec::new();
        for def in baylee_cards::all() {
            let Some(face) = def.faces.first() else {
                continue;
            };
            if face.types.contains(TypeSet::LAND) {
                if let Some(at) = COLORS.iter().position(|(_, name)| *name == def.name()) {
                    basics[at].get_or_insert(def.index);
                }
            } else if matches!(def.coverage, Coverage::Implemented | Coverage::Generated) {
                spells.push((def.color_identity, def.index));
            }
        }
        Pool { basics, spells }
    })
}

/// Whether `card`'s front face is a land.
fn is_land(card: CardIndex) -> bool {
    baylee_cards::by_index(card)
        .and_then(|def| def.faces.first())
        .is_some_and(|face| face.types.contains(TypeSet::LAND))
}

/// The cards `player` owns that `view` shows with their identity, tokens
/// left out, each with how many copies are shown.
fn shown(view: &PlayerView, player: PlayerId) -> BTreeMap<CardIndex, u64> {
    let at = usize::from(player.get());
    let mut cards = BTreeMap::new();
    let everywhere = view
        .battlefield
        .iter()
        .chain(&view.stack)
        .chain(view.graveyards.get(at).into_iter().flatten())
        .chain(view.exile.iter().flatten())
        .chain(view.command.get(at).into_iter().flatten());
    for object in everywhere {
        if object.owner != player || object.token.is_some() {
            continue;
        }
        if let Some(card) = object.card {
            *cards.entry(card.index).or_insert(0) += 1;
        }
    }
    cards
}

/// What the seat of `view` supposes each other player hides, in seat
/// order; `own_deck` is the seat's own list, the supposition for a player
/// who has shown nothing.
///
/// Every list is non-empty unless the registry holds no basic land and
/// `own_deck` is empty.
#[must_use]
pub fn public_prior(view: &PlayerView, own_deck: &[CardIndex]) -> Vec<(PlayerId, Weighted)> {
    view.seats
        .iter()
        .filter(|seat| seat.player != view.seat)
        .map(|seat| (seat.player, supposed(view, seat.player, own_deck)))
        .collect()
}

fn supposed(view: &PlayerView, player: PlayerId, own_deck: &[CardIndex]) -> Weighted {
    let shown = shown(view, player);
    let spells: Vec<(CardIndex, u64)> = shown
        .iter()
        .filter(|(card, _)| !is_land(**card))
        .map(|(card, copies)| (*card, *copies))
        .collect();
    let lands: Vec<(CardIndex, u64)> = shown
        .iter()
        .filter(|(card, _)| is_land(**card))
        .map(|(card, copies)| (*card, *copies))
        .collect();
    if spells.is_empty() && lands.is_empty() {
        return like(own_deck);
    }
    let pool = pool();
    // The colours the deck has shown it plays: its spells' identities, and
    // what its lands are for where no spell is shown yet.
    let identity = |cards: &[(CardIndex, u64)]| {
        cards.iter().fold(0u8, |bits, (card, _)| {
            bits | baylee_cards::by_index(*card).map_or(0, |def| def.color_identity.bits())
        })
    };
    let spell_colors = identity(&spells);
    let colors = if spell_colors == 0 {
        identity(&lands)
    } else {
        spell_colors
    };
    let castable: Vec<CardIndex> = pool
        .spells
        .iter()
        .filter(|(needs, _)| needs.bits() & !colors == 0)
        .map(|(_, card)| *card)
        .collect();
    // Spells first, in units where one pool card weighs `POOL`.
    let others = castable.len().max(1) as u64;
    let mut weights: BTreeMap<CardIndex, u64> = BTreeMap::new();
    for card in &castable {
        *weights.entry(*card).or_insert(0) += POOL;
    }
    for (card, copies) in &spells {
        *weights.entry(*card).or_insert(0) += SHOWN * others * copies;
    }
    let spell_mass: u64 = weights.values().sum();
    // Lands: two fifths of the whole, so two thirds of the spells' mass.
    let land_mass = spell_mass * LAND_FIFTHS / (5 - LAND_FIFTHS);
    let basics: Vec<CardIndex> = COLORS
        .iter()
        .zip(pool.basics)
        .filter(|((color, _), _)| colors == 0 || ColorSet::of(*color).bits() & colors != 0)
        .filter_map(|(_, basic)| basic)
        .collect();
    let shown_lands: u64 = lands.iter().map(|(_, copies)| copies).sum();
    let basic_mass = if shown_lands == 0 {
        land_mass
    } else {
        land_mass * BASIC_TENTHS / 10
    };
    if !basics.is_empty() {
        for basic in &basics {
            *weights.entry(*basic).or_insert(0) += basic_mass / basics.len() as u64;
        }
    }
    for (card, copies) in &lands {
        let share = ((land_mass - basic_mass) * copies)
            .checked_div(shown_lands)
            .unwrap_or(0);
        *weights.entry(*card).or_insert(0) += share;
    }
    scaled(weights)
}

/// A deck like `deck`: each of its cards at the weight of its copies.
fn like(deck: &[CardIndex]) -> Weighted {
    let mut weights: BTreeMap<CardIndex, u64> = BTreeMap::new();
    for card in deck {
        *weights.entry(*card).or_insert(0) += 1;
    }
    scaled(weights)
}

/// `weights` brought into `u32`, keeping their proportions and no card at
/// zero.
fn scaled(weights: BTreeMap<CardIndex, u64>) -> Weighted {
    let most = weights.values().copied().max().unwrap_or(1).max(1);
    let shift = (64 - most.leading_zeros()).saturating_sub(24);
    weights
        .into_iter()
        .filter(|(_, weight)| *weight > 0)
        .map(|(card, weight)| {
            (
                card,
                u32::try_from((weight >> shift).max(1)).unwrap_or(u32::MAX),
            )
        })
        .collect()
}
