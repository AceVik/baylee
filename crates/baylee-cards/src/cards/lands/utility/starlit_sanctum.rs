//! Starlit Sanctum — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {W}, {T}, Sacrifice a Cleric creature: You gain life equal to the sacrificed creature's toughness.
//! Oracle: {B}, {T}, Sacrifice a Cleric creature: Target player loses life equal to the sacrificed creature's power.
//! Set: CLB #917 — Commander Legends: Battle for Baldur's Gate | Scryfall ID: f5774836-0140-420a-9a0f-8ba291cc5ca8 | Oracle ID: d16298ac-67bd-4f9d-9979-23c1b7e4b359
// PARTIAL — {T}: Add {C} only; the two Cleric-sacrifice abilities cannot be
// said at all (see the NOT SUPPORTED lines at the foot of the file).
// NOT SUPPORTED on both: the sacrifice itself is sayable as a cost
// (`Sacrifice(filter)`), but the effect has nothing to read the sacrificed
// permanent with — see below.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::STARLIT_SANCTUM,
    oracle_id = "d16298ac-67bd-4f9d-9979-23c1b7e4b359",
    scryfall_id = "f5774836-0140-420a-9a0f-8ba291cc5ca8",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::White]),
    faces = &[face!(name = "Starlit Sanctum", types = TypeSet::LAND,)],
    coverage = Coverage::Partial(
        "the two Cleric-sacrifice abilities: no Amount reads the power or \
         toughness of the permanent a cost sacrificed",
    ),
    abilities = &[mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)])],
);

// NOT SUPPORTED: "{W}, {T}, Sacrifice a Cleric creature: You gain life equal
// to the sacrificed creature's toughness." The cost half is expressible
// (`cost!("{W}", TapSelf, Sacrifice(...))` over a Cleric-creature filter), but
// the effect has no way to name what the cost removed: the only power/toughness
// readers in `Amount` are `TargetPower`, which reads the first *target* of this
// ability, and the counted forms (`CountOf`), which count objects. Making the
// Cleric a target in order to reach `TargetPower` would be a different card —
// the printed sentence does not target, so hexproof, shroud and protection
// never answer it and the sacrifice would happen on resolution instead of at
// activation.
//
// NOT SUPPORTED: "{B}, {T}, Sacrifice a Cleric creature: Target player loses
// life equal to the sacrificed creature's power." The same missing amount, and
// the same reason: `Effect::LoseLife { amount, target }` is available and its
// player choice is fine, but nothing can supply "the sacrificed creature's
// power" for `amount`.
