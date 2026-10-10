//! Phyrexian Gremlins — {2}{B} — Creature — Phyrexian Gremlin
//! Oracle: You may choose not to untap this creature during your untap step.
//! Oracle: {T}: Tap target artifact. It doesn't untap during its controller's untap step for as long as this creature remains tapped.
//! Set: ATQ #18 — Antiquities | Scryfall ID: 21a985a9-5612-4844-982e-fd1aa6249770 | Oracle ID: 407a0761-7ccc-4607-8df6-e744d30a81a0
// IMPLEMENTED — the untap-step choice, the tap, and the lock that keeps the
// artifact down for as long as this creature remains tapped
// (Modifier::DoesNotUntap, Duration::WhileSourceTapped).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::PHYREXIAN_GREMLINS,
    oracle_id = "407a0761-7ccc-4607-8df6-e744d30a81a0",
    scryfall_id = "21a985a9-5612-4844-982e-fd1aa6249770",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    faces = &[face!(
        name = "Phyrexian Gremlins",
        mana_cost = mana!("{2}{B}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::PHYREXIAN, subtypes::creature::GREMLIN],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        static_ability!(Filter::This, Modifier::MayChooseNotToUntap),
        activated!(
            Cost::TAP,
            &[
                Effect::TapTarget,
                Effect::continuous(
                    &Filter::This,
                    Modifier::DoesNotUntap,
                    Duration::WhileSourceTapped
                ),
            ],
            target = Some(TargetSpec::Object(&Filter::ARTIFACT))
        ),
    ],
);
