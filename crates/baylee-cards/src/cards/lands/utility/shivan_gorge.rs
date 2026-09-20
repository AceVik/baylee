//! Shivan Gorge — (no cost) — Legendary Land
//! Oracle: {T}: Add {C}.
//! Oracle: {2}{R}, {T}: Shivan Gorge deals 1 damage to each opponent.
//! Set: DSC #297 — Duskmourn: House of Horror Commander | Scryfall ID: c1752b65-dace-4c87-b102-23df276b2e41 | Oracle ID: e90a1381-c9c1-4f57-928c-5d19dc065274
// PARTIAL — {T}: Add {C}; the {2}{R} ability is not expressible (see NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::SHIVAN_GORGE,
    oracle_id = "e90a1381-c9c1-4f57-928c-5d19dc065274",
    scryfall_id = "c1752b65-dace-4c87-b102-23df276b2e41",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "the {2}{R} ability deals 1 damage to each opponent, which no effect can say"
    ),
    faces = &[face!(
        name = "Shivan Gorge",
        types = TypeSet::LAND,
        supertypes = SupertypeSet::LEGENDARY,
    ),],
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        // NOT SUPPORTED: "{2}{R}, {T}: Shivan Gorge deals 1 damage to each
        // opponent." — Effect::DealDamage takes one TargetSpec, and
        // TargetSpec::Player names a single seat (You/Opponent), so there is
        // no way to reach a whole group of players. The nearest variant that
        // exists, Effect::LoseLife with PlayerRel::EachOpponent, is life loss
        // and not damage, which is a different sentence to every card that
        // reads damage (prevention, "dealt damage this turn", deathtouch).
    ],
);
