//! Sealed Fate — {X}{U}{B} — Sorcery
//! Oracle: Look at the top X cards of target opponent's library. Exile one of those cards and put the rest back on top of that player's library in any order.
//! Set: MIR #282 — Mirage | Scryfall ID: 36cf9da8-f078-4f6e-8077-bb24a7ed487f | Oracle ID: 0b9b5856-9396-4a55-8675-58cd171df708
// PARTIAL: Looking at the top X cards of target opponent's library, exiling one, and putting the rest back on top in any order is not in the engine; casts with {X}{U}{B} targeting an opponent and looks at the top card of that opponent's library.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SEALED_FATE,
    oracle_id = "0b9b5856-9396-4a55-8675-58cd171df708",
    scryfall_id = "36cf9da8-f078-4f6e-8077-bb24a7ed487f",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue]),
    coverage = Coverage::Partial(
        "looking at the top X cards of target opponent's library, exiling one, and putting the rest back on top in any order is not in the engine; casts with {X}{U}{B} targeting an opponent and looks at the top card of that opponent's library",
    ),
    faces = &[face!(
        name = "Sealed Fate",
        mana_cost = mana!("{X}{U}{B}"),
        types = TypeSet::SORCERY,
    ),],
    abilities = &[spell!(
        &[Effect::LookAtTopOf {
            player: PlayerRel::Chosen,
            count: 1,
        }],
        targets = Some(TargetReq::one(TargetSpec::AnyOpponent)),
    )],
);
