//! Tower of the Magistrate — (no cost) — Land
//! Oracle: {T}: Add {C}.
//! Oracle: {1}, {T}: Target creature gains protection from artifacts until end of turn.
//! Set: MMQ #330 — Mercadian Masques | Scryfall ID: ee0481db-15ae-46b4-89a3-01c95a9626c7 | Oracle ID: ac08fae8-208c-4602-8d39-9bfd29b53a5e
// IMPLEMENTED — {C} mana + protection-from-artifacts grant until EOT.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::TOWER_OF_THE_MAGISTRATE,
    oracle_id = "ac08fae8-208c-4602-8d39-9bfd29b53a5e",
    scryfall_id = "ee0481db-15ae-46b4-89a3-01c95a9626c7",
    faces = &[face!(
        name = "Tower of the Magistrate",
        types = TypeSet::LAND,
    )],
    coverage = Coverage::Implemented,
    abilities = &[
        mana_ability!(&[Effect::mana(ManaColor::Colorless, 1)]),
        activated!(
            cost!("{1}", TapSelf),
            &[Effect::continuous(
                &Filter::This,
                Modifier::ProtectionFrom(&Filter::ARTIFACT),
                Duration::UntilEndOfTurn
            )],
            target = Some(TargetSpec::Object(&Filter::CREATURE))
        ),
    ],
);
