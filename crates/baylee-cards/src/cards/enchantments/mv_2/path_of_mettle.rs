//! Path of Mettle // Metzali, Tower of Triumph — {R}{W} — Legendary Enchantment // Legendary Land
//! Oracle: When Path of Mettle enters, it deals 1 damage to each creature that doesn't have first strike, double strike, vigilance, or haste.
//! Oracle: Whenever you attack with at least two creatures that have first strike, double strike, vigilance, and/or haste, transform Path of Mettle.
//! Oracle: (Transforms from Path of Mettle.)
//! Oracle: {T}: Add one mana of any color.
//! Oracle: {1}{R}, {T}: Metzali deals 2 damage to each opponent.
//! Oracle: {2}{W}, {T}: Choose a creature at random that attacked this turn. Destroy that creature.
//! Set: RIX #165 — Rivals of Ixalan | Scryfall ID: 66d9d524-3611-48d9-86c9-48e509e8ae70 | Oracle ID: db9ea3f9-c723-422f-98cc-a3ef7ca2c290
//! Face: Path of Mettle — {R}{W} — Legendary Enchantment
//! Face: Metzali, Tower of Triumph —  — Legendary Land
// PARTIAL — of the four printed abilities only Metzali's "{T}: Add one mana of
// any color" is sayable in the DSL; the other three are listed as
// NOT SUPPORTED below, with the variant each one is missing.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::PATH_OF_METTLE,
    oracle_id = "db9ea3f9-c723-422f-98cc-a3ef7ca2c290",
    scryfall_id = "66d9d524-3611-48d9-86c9-48e509e8ae70",
    color_identity = ColorSet::from_slice(&[Color::Red, Color::White]),
    faces = &[
        face!(
            name = "Path of Mettle",
            mana_cost = mana!("{R}{W}"),
            types = TypeSet::ENCHANTMENT,
            supertypes = SupertypeSet::LEGENDARY,
        ),
        face!(
            name = "Metzali, Tower of Triumph",
            // CR 712.8c: a nonmodal double-faced card is cast as its front
            // face and reaches this one only by transforming.
            castable_from_hand = false,
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            abilities = &[mana_ability!(&[Effect::mana_of_any_color()])],
        ),
    ],
    coverage = Coverage::Partial(
        "no effect deals damage to every object matching a filter, no trigger counts attackers \
         or asks what attacked this turn, nothing selects at random, and no effect transforms; \
         only the back face's mana ability is built",
    ),
);

// NOT SUPPORTED: "When Path of Mettle enters, it deals 1 damage to each
// creature that doesn't have first strike, double strike, vigilance, or
// haste." — Effect::DealDamage names one TargetSpec, and no effect damages
// every object matching a filter (Effect::DestroyAll destroys rather than
// deals damage); the filter itself is sayable, the op is not.
// NOT SUPPORTED: "Whenever you attack with at least two creatures that have
// first strike, double strike, vigilance, and/or haste, transform Path of
// Mettle." — Trigger::Attacks(&filter) fires once per attacking object and
// carries no "at least two" count, and transforming is not an effect
// (Effect::ExileSelfReturnAsFace exiles and returns, which is a new object).
// NOT SUPPORTED: "{1}{R}, {T}: Metzali deals 2 damage to each opponent." —
// "each opponent" is not a target, so Effect::DealDamage has nothing to name;
// Effect::LoseLife { target: PlayerRel::EachOpponent } would make players lose
// life, which is a different instruction from taking damage.
// NOT SUPPORTED: "{2}{W}, {T}: Choose a creature at random that attacked this
// turn. Destroy that creature." — no selection in the DSL is random, and no
// filter can ask what attacked this turn.
