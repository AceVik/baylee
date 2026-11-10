//! Swan Song — {U} — Instant
//! Oracle: Counter target enchantment, instant, or sorcery spell. Its controller creates a 2/2 blue Bird creature token with flying.
//! Set: EOC #46 — Edge of Eternities Commander | Scryfall ID: 83d0b761-d694-4232-9f40-5bd8c82a05f1 | Oracle ID: 8ddfc283-c9b4-41a5-af88-cf0068e986cc
// IMPLEMENTED — the counter, and the 2/2 blue flying Bird its controller is
// left with.

use crate::generated_tokens;
use baylee_cards_dsl::prelude::*;

/// "an enchantment, instant, or sorcery spell" — the instant/sorcery half is
/// the constant that already carries it, not a clause spelled out by hand.
static COUNTERABLE: Filter = Filter::Or(&[Filter::ENCHANTMENT, Filter::INSTANT_OR_SORCERY]);

card!(
    index = index::SWAN_SONG,
    oracle_id = "8ddfc283-c9b4-41a5-af88-cf0068e986cc",
    scryfall_id = "83d0b761-d694-4232-9f40-5bd8c82a05f1",
    color_identity = ColorSet::from_slice(&[Color::Blue]),
    faces = &[face!(
        name = "Swan Song",
        mana_cost = mana!("{U}"),
        types = TypeSet::INSTANT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[spell!(
        &[
            Effect::CounterTargetSpell,
            // "Its controller" is the *countered spell's* controller and
            // not a seat this card names, so the line reads the target: the
            // same `CreateTokenForTargetController` An Offer You Can't
            // Refuse pays its Treasures with. Order matters — the counter
            // runs first and has already put the spell in its owner's
            // graveyard, which is where the effect finds it.
            Effect::CreateTokenForTargetController {
                token: &generated_tokens::BIRD_2_2_BLUE_FLYING,
            },
        ],
        targets = Some(TargetReq::one(TargetSpec::Spell(&COUNTERABLE)))
    )],
);
