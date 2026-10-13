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
// IMPLEMENTED — the enter trigger, the attack trigger that transforms it into
// Metzali (two or more attackers with first strike, double strike, vigilance
// and/or haste), and Metzali's mana, damage and random-destroy abilities.

use baylee_cards_dsl::prelude::*;

/// The four keywords that spare a creature. `HasKeyword` matches any of
/// them, so its negation is "has none of the four".
const METTLE: KeywordSet = KeywordSet::FIRST_STRIKE
    .union(KeywordSet::DOUBLE_STRIKE)
    .union(KeywordSet::VIGILANCE)
    .union(KeywordSet::HASTE);

/// "each creature that doesn't have first strike, double strike, vigilance,
/// or haste".
static WITHOUT_METTLE: Filter =
    Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::HasKeyword(METTLE))]);

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
            abilities = &[
                triggered!(Trigger::ETB, &[Effect::damage_each(1, &WITHOUT_METTLE)]),
                triggered!(
                    Trigger::AttackWithAtLeastMatching {
                        n: 2,
                        filter: &Filter::HasKeyword(METTLE),
                    },
                    &[Effect::TransformSource]
                ),
            ],
        ),
        face!(
            name = "Metzali, Tower of Triumph",
            // CR 712.8c: a nonmodal double-faced card is cast as its front
            // face and reaches this one only by transforming.
            castable_from_hand = false,
            types = TypeSet::LAND,
            supertypes = SupertypeSet::LEGENDARY,
            abilities = &[
                mana_ability!(&[Effect::mana_of_any_color()]),
                activated!(
                    cost!("{1}{R}", TapSelf),
                    &[Effect::DealDamage {
                        amount: Amount::Fixed(2),
                        target: TargetSpec::Player(PlayerRel::EachOpponent),
                    }]
                ),
                activated!(
                    cost!("{2}{W}", TapSelf),
                    &[Effect::DestroyRandom {
                        filter: &Filter::And(&[Filter::CREATURE, Filter::AttackedThisTurn]),
                    }]
                ),
            ],
        ),
    ],
    coverage = Coverage::Implemented,
);
