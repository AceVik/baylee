//! Echo Chamber — {4} — Artifact
//! Oracle: {4}, {T}: An opponent chooses target creature they control. Create a token that's a copy of that creature. That token gains haste until end of turn. Exile the token at the beginning of the next end step. Activate only as a sorcery.
//! Set: TMP #282 — Tempest | Scryfall ID: 06425615-6c10-4766-8128-a1a09a35649d | Oracle ID: 75960649-93a4-4d56-b40f-f9fcbb4341d4
// PARTIAL

use baylee_cards_dsl::prelude::*;

card!(
    index = index::ECHO_CHAMBER,
    oracle_id = "75960649-93a4-4d56-b40f-f9fcbb4341d4",
    scryfall_id = "06425615-6c10-4766-8128-a1a09a35649d",
    coverage = Coverage::Partial(
        "{4}, {T} as a sorcery has an opponent choose target creature they control and creates a token copy of it with haste that is sacrificed at the beginning of the next end step; exiling (rather than sacrificing) the token at the next end step and limiting haste to until end of turn are approximated by CopyMod::AddKeyword(HASTE) and sacrifice_at_next_end_step."
    ),
    faces = &[face!(
        name = "Echo Chamber",
        mana_cost = mana!("{4}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{4}", TapSelf),
        &[Effect::CreateTokenCopyOfTarget {
            mods: &[CopyMod::AddKeyword(KeywordSet::HASTE)],
            sacrifice_at_next_end_step: true,
        }],
        targets = Some(
            TargetReq::one(TargetSpec::Object(&Filter::OPPONENT_CREATURE)).chosen_by_opponent()
        ),
        timing = ActivationTiming::SorcerySpeed,
    )],
);
