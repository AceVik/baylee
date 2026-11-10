//! Grinding Station — {2} — Artifact
//! Oracle: {T}, Sacrifice an artifact: Target player mills three cards.
//! Oracle: Whenever an artifact enters, you may untap this artifact.
//! Set: 5DN #127 — Fifth Dawn | Scryfall ID: df1df511-b52c-45cd-9503-ffce4271a802 | Oracle ID: 0fcd476f-4db8-4293-9388-1678a0043c9e
// IMPLEMENTED — tap-and-sacrifice-an-artifact mill 3 at a chosen player, and
// an artifact entering lets you untap this artifact.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::GRINDING_STATION,
    oracle_id = "0fcd476f-4db8-4293-9388-1678a0043c9e",
    scryfall_id = "df1df511-b52c-45cd-9503-ffce4271a802",
    faces = &[face!(
        name = "Grinding Station",
        mana_cost = mana!("{2}"),
        types = TypeSet::ARTIFACT,
    ),],
    coverage = Coverage::Implemented,
    abilities = &[
        activated!(
            cost!(TapSelf, Sacrifice(&Filter::ARTIFACT)),
            &[Effect::Mill {
                amount: Amount::Fixed(3),
                target: PlayerRel::Chosen,
            }],
            target = Some(TargetSpec::AnyPlayer)
        ),
        triggered!(
            Trigger::EntersBattlefield(&Filter::ARTIFACT),
            &[Effect::MayDo {
                effects: &[Effect::UntapSelf],
            }]
        ),
    ],
);
