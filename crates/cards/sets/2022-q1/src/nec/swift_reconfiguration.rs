//! Swift Reconfiguration — {W} — Enchantment — Aura
//! Oracle: Flash
//! Oracle: Enchant creature or Vehicle
//! Oracle: Enchanted permanent is a Vehicle artifact with crew 5 and it loses all other card types. (It's not a creature unless it's crewed.)
//! Set: NEC #10 — Neon Dynasty Commander | Scryfall ID: 975dcfab-0281-4fee-92aa-021ea6c524c7 | Oracle ID: 5d47e820-913f-441a-a6cc-37ab3181d79a

// IMPLEMENTED — all printed abilities; exercised in engine card tests.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static ENCHANTABLE: Filter = Filter::Or(&[
    Filter::CREATURE,
    Filter::HasSubtype(subtypes::artifact::VEHICLE),
]);

card!(
    index = index::SWIFT_RECONFIGURATION,
    oracle_id = "5d47e820-913f-441a-a6cc-37ab3181d79a",
    scryfall_id = "975dcfab-0281-4fee-92aa-021ea6c524c7",
    color_identity = ColorSet::from_slice(&[Color::White]),
    faces = &[face!(
        name = "Swift Reconfiguration",
        mana_cost = mana!("{W}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    keywords = KeywordSet::FLASH,
    coverage = Coverage::Implemented,
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&ENCHANTABLE),
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&ENCHANTABLE)))
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::BecomeType {
                types: TypeSet::ARTIFACT,
                subtype: subtypes::artifact::VEHICLE,
            }
        ),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::GrantActivated {
                cost: cost!(Crew(5)),
                mana_ability: false,
                effects: &[Effect::continuous(
                    &Filter::This,
                    Modifier::AddType(TypeSet::ARTIFACT.union(TypeSet::CREATURE)),
                    Duration::UntilEndOfTurn
                )],
            }
        ),
    ],
);
