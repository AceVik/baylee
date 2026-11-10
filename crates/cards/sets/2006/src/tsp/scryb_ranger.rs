//! Scryb Ranger — {1}{G} — Creature — Faerie Ranger
//! Oracle: Flash
//! Oracle: Flying, protection from blue
//! Oracle: Return a Forest you control to its owner's hand: Untap target creature. Activate only once each turn.
//! Set: TSR #227 — Time Spiral Remastered | Scryfall ID: 885c122a-d350-420f-ba7b-4523a28e48a6 | Oracle ID: d1961110-575b-4a1b-9cee-db0e1f0fdbc1
// IMPLEMENTED — flash and flying as keyword bits, protection from blue as a
// layer-6 static, and the "return a Forest you control: untap target
// creature" activation with its once-each-turn limit.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::SCRYB_RANGER,
    oracle_id = "d1961110-575b-4a1b-9cee-db0e1f0fdbc1",
    scryfall_id = "885c122a-d350-420f-ba7b-4523a28e48a6",
    color_identity = ColorSet::from_slice(&[Color::Green]),
    keywords = KeywordSet::FLYING.union(KeywordSet::FLASH),
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Scryb Ranger",
        mana_cost = mana!("{1}{G}"),
        types = TypeSet::CREATURE,
        subtypes = &[subtypes::creature::FAERIE, subtypes::creature::RANGER],
        power = Some(1),
        toughness = Some(1),
    ),],
    abilities = &[
        static_ability!(
            Filter::This,
            Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[Color::Blue])))
        ),
        activated!(
            cost!(ReturnToHand(&Filter::And(&[
                Filter::HasSubtype(subtypes::land::FOREST),
                Filter::ControlledByYou,
            ]))),
            &[Effect::UntapTarget],
            target = Some(TargetSpec::Object(&Filter::CREATURE)),
            limit = ActivationLimit::PerTurn(1),
        ),
    ],
);
