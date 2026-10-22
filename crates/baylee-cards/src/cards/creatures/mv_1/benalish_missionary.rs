//! Benalish Missionary — {W} — Creature — Human Cleric
//! Oracle: {1}{W}, {T}: Prevent all combat damage that would be dealt by target blocked creature this turn.
//! Set: WTH #10 — Weatherlight | Scryfall ID: e9ac1992-6212-4f05-af16-c892dfc40643 | Oracle ID: 84fdcfd3-2b22-4570-af34-7e3f55f97466
// NOT SUPPORTED: a dedicated Filter::Blocked variant to prevent targeting an attacker during the declare attackers step before blockers are declared is not yet modeled.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static BLOCKED_CREATURE: Filter = Filter::And(&[
    Filter::CREATURE,
    Filter::Attacking,
    Filter::Not(&Filter::Unblocked),
]);

card!(
    index = index::BENALISH_MISSIONARY,
    oracle_id = "84fdcfd3-2b22-4570-af34-7e3f55f97466",
    scryfall_id = "e9ac1992-6212-4f05-af16-c892dfc40643",
    color_identity = ColorSet::from_slice(&[Color::White]),
    coverage = Coverage::Partial(
        "activates for {1}{W}, {T} targeting a blocked attacking creature so it assigns no combat damage this turn; a dedicated Filter::Blocked variant to prevent targeting an attacker during the declare attackers step before blockers are declared is not yet modeled",
    ),
    faces = &[face!(
        name = "Benalish Missionary",
        mana_cost = mana!("{W}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::CLERIC],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[activated!(
        cost!("{1}{W}", TapSelf),
        &[Effect::continuous(
            &Filter::This,
            Modifier::AssignsNoCombatDamage,
            Duration::UntilEndOfTurn,
        )],
        target = Some(TargetSpec::Object(&BLOCKED_CREATURE)),
    )],
);
