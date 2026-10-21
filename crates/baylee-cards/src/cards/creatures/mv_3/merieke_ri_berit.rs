//! Merieke Ri Berit — {W}{U}{B} — Legendary Creature — Human
//! Oracle: Merieke Ri Berit doesn't untap during your untap step.
//! Oracle: {T}: Gain control of target creature for as long as you control Merieke Ri Berit. When Merieke Ri Berit leaves the battlefield or becomes untapped, destroy that creature. It can't be regenerated.
//! Set: TSB #95 — Time Spiral Timeshifted | Scryfall ID: 63f7cf94-78f6-4f30-ab24-8b90231520f1 | Oracle ID: 288fdfbb-1236-4732-afd7-e7eaba860007
// NOT SUPPORTED: destroying the creature without regeneration when Merieke Ri Berit leaves the battlefield or becomes untapped is not supported; Merieke Ri Berit doesn't untap during your untap step and {T} gains control of target creature for as long as you control Merieke Ri Berit.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::MERIEKE_RI_BERIT,
    oracle_id = "288fdfbb-1236-4732-afd7-e7eaba860007",
    scryfall_id = "63f7cf94-78f6-4f30-ab24-8b90231520f1",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Blue, Color::White]),
    commander = CommanderRule::Legendary,
    coverage = Coverage::Partial(
        "destroying the creature without regeneration when Merieke Ri Berit leaves the battlefield or becomes untapped is not supported; Merieke Ri Berit doesn't untap during your untap step and {T} gains control of target creature for as long as you control Merieke Ri Berit",
    ),
    faces = &[face!(
        name = "Merieke Ri Berit",
        mana_cost = mana!("{W}{U}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        static_ability!(Filter::This, Modifier::DoesNotUntap),
        activated!(
            Cost::TAP,
            &[Effect::continuous(
                &Filter::This,
                Modifier::GainControl,
                Duration::WhileYouControlSource,
            )],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
        ),
    ],
);
