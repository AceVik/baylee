//! Rabbit Battery — {R} — Artifact Creature — Equipment Rabbit
//! Oracle: Haste
//! Oracle: Equipped creature gets +1/+1 and has haste.
//! Oracle: Reconfigure {R} ({R}: Attach to target creature you control; or unattach from a creature. Reconfigure only as a sorcery. While attached, this isn't a creature.)
//! Set: NEO #157 — Kamigawa: Neon Dynasty | Scryfall ID: 5d33a5b7-797b-4079-8d62-edd124c0fb5a | Oracle ID: c739e180-2f14-41ed-8e7e-50b7df985f35
// PARTIAL — haste, the +1/+1-and-haste grant to the attached creature,
// reconfigure's attach half and its "while attached, this isn't a creature".
// The unattach mode has no vocabulary.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::RABBIT_BATTERY,
    oracle_id = "c739e180-2f14-41ed-8e7e-50b7df985f35",
    scryfall_id = "5d33a5b7-797b-4079-8d62-edd124c0fb5a",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    keywords = KeywordSet::HASTE,
    faces = &[face!(
        name = "Rabbit Battery",
        mana_cost = mana!("{R}"),
        types = TypeSet::ARTIFACT.union(TypeSet::CREATURE),
        subtypes = &[subtypes::artifact::EQUIPMENT, subtypes::creature::RABBIT],
        power = Some(1),
        toughness = Some(1),
    ),],
    coverage = Coverage::Partial(
        "reconfigure's \"or unattach from a creature\" mode (no effect detaches the source)"
    ),
    abilities = &[
        static_ability!(Filter::AttachedToBySource, Modifier::ModifyPT(1, 1)),
        static_ability!(
            Filter::AttachedToBySource,
            Modifier::AddKeyword(KeywordSet::HASTE)
        ),
        // NOT SUPPORTED: "or unattach from a creature" — no Effect detaches
        // the source.
        //
        // Reconfigure is two abilities (CR 702.151a), and its attaching half
        // is worded exactly as the equip keyword is: sorcery speed, "target
        // creature you control". So that half is built below by the macro
        // the rules supply everything for (CR 702.6).
        equip!("{R}"),
        // "While attached, this isn't a creature" (CR 702.151b). A creature
        // attached to anything becomes unattached (CR 704.5p), so without
        // this the Battery would come off the creature it just equipped.
        static_ability!(
            Filter::This,
            Modifier::RemoveType(TypeSet::CREATURE),
            condition = Some(Condition::SourceMatches(&Filter::IsAttached))
        ),
    ],
);
