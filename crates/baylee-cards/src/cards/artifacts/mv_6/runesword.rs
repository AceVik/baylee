//! Runesword — {6} — Artifact
//! Oracle: {3}, {T}: Target attacking creature gets +2/+0 until end of turn. When that creature leaves the battlefield this turn, sacrifice this artifact. If the creature deals damage to a creature this turn, the creature dealt damage can't be regenerated this turn. If a creature dealt damage by the targeted creature would die this turn, exile that creature instead.
//! Set: CHR #106 — Chronicles | Scryfall ID: a11ec2f6-a0c7-4164-a562-fab7aab7d211 | Oracle ID: 964c2003-cef0-4ac7-9f66-4e20893c8e50

use baylee_cards_dsl::prelude::*;

card!(
    index = index::RUNESWORD,
    oracle_id = "964c2003-cef0-4ac7-9f66-4e20893c8e50",
    scryfall_id = "a11ec2f6-a0c7-4164-a562-fab7aab7d211",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Runesword",
        mana_cost = mana!("{6}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{3}", TapSelf),
        &[
            Effect::ModifyTargets {
                modifier: Modifier::ModifyPT(2, 0),
                duration: Duration::UntilEndOfTurn,
            },
            Effect::WhenTargetLeavesBattlefieldThisTurn {
                effects: &[Effect::SacrificeSelf],
            },
            Effect::RuneswordTargetRider,
        ],
        target = Some(TargetSpec::Object(&Filter::ATTACKING_CREATURE))
    )],
);
