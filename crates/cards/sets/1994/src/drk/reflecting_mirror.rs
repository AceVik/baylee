//! Reflecting Mirror — {4} — Artifact
//! Oracle: {X}, {T}: Change the target of target spell with a single target if that target is you. The new target must be a player. X is twice the mana value of that spell.
//! Set: DRK #106 — The Dark | Scryfall ID: d551ff93-d8da-4c21-bc3c-6451c0dde07e | Oracle ID: 2fb505c6-3d28-4b83-9820-4b9db64c8304

use baylee_cards_dsl::prelude::*;

static SINGLE_TARGET_SPELL_TWICE_X: Filter =
    Filter::And(&[Filter::WithSingleTarget, Filter::TwiceCmcExactlyX]);

card!(
    index = index::REFLECTING_MIRROR,
    oracle_id = "2fb505c6-3d28-4b83-9820-4b9db64c8304",
    scryfall_id = "d551ff93-d8da-4c21-bc3c-6451c0dde07e",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Reflecting Mirror",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{X}", TapSelf),
        &[Effect::ChangeTargetToPlayerIfTargetsYou],
        target = Some(TargetSpec::Spell(&SINGLE_TARGET_SPELL_TWICE_X))
    )],
);
