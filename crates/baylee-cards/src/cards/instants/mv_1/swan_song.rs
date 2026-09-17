//! Swan Song — {U} — Instant
//! Oracle: Counter target enchantment, instant, or sorcery spell. Its controller creates a 2/2 blue Bird creature token with flying.
//! Set: EOC #46 — Edge of Eternities Commander | Scryfall ID: 83d0b761-d694-4232-9f40-5bd8c82a05f1 | Oracle ID: 8ddfc283-c9b4-41a5-af88-cf0068e986cc
// PARTIAL — the counter half is built; the 2/2 blue Bird its controller makes
// has no entry in `crate::tokens`, and a card file may not define a token of
// its own (a `TokenDef` outside that file has no art key — the build failure
// `tokens::no_card_file_defines_its_own_token` reads the rule back out of the
// pool).

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
    coverage = Coverage::Partial(
        "the countered spell's controller creates a 2/2 blue Bird creature token with \
         flying, and `crate::tokens` has no such token: the pool's only Bird is \
         `BIRD_1_1_WHITE_FLYING`, and a card file may not declare its own `TokenDef`"
    ),
    abilities = &[spell!(
        &[
            Effect::CounterTargetSpell,
            // NOT SUPPORTED: "Its controller creates a 2/2 blue Bird creature
            // token with flying." — it would be
            // `Effect::CreateTokenForTargetController { token: &BIRD_2_2_BLUE_FLYING }`
            // and there is no 2/2 blue flying Bird in `crate::tokens` to point
            // at; writing the pool's 1/1 white Bird instead would be a
            // different card, and a file-local `TokenDef` is refused.
        ],
        targets = Some(TargetReq::one(TargetSpec::Spell(&COUNTERABLE)))
    )],
);
