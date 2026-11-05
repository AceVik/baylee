//! Cursed Scroll — {1} — Artifact
//! Oracle: {3}, {T}: Choose a card name, then reveal a card at random from your hand. If that card has the chosen name, this artifact deals 2 damage to any target.
//! Set: TPR #220 — Tempest Remastered | Scryfall ID: 1ad9ba08-91e7-4811-bf19-bc7076d3cf3c | Oracle ID: 514752ea-d453-41d4-97cf-b1dcdb4b4a08
// IMPLEMENTED — Pays {3} and taps to choose a card name (remembered on the artifact), reveals a card at random from your hand, and deals 2 damage to the target only if the revealed card has the chosen name.

use baylee_cards_dsl::prelude::*;

card!(
    index = index::CURSED_SCROLL,
    oracle_id = "514752ea-d453-41d4-97cf-b1dcdb4b4a08",
    scryfall_id = "1ad9ba08-91e7-4811-bf19-bc7076d3cf3c",
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Cursed Scroll",
        mana_cost = mana!("{1}"),
        types = TypeSet::ARTIFACT,
    ),],
    abilities = &[activated!(
        cost!("{3}", TapSelf),
        &[
            Effect::ChooseCardName {
                player: PlayerRel::You,
                types: TypeSet::EMPTY,
                excluded_types: TypeSet::EMPTY,
                remember: true,
            },
            Effect::RevealRandomFromHand {
                who: PlayerRel::You,
                count: Amount::Fixed(1),
            },
            Effect::IfCondition {
                condition: Condition::AmountAtLeast(
                    Amount::ThisWay(ThisWay::Revealed(&Filter::NamedBySource)),
                    1,
                ),
                then: &[Effect::DealDamage {
                    target: TargetSpec::AnyTarget,
                    amount: Amount::Fixed(2),
                }],
                otherwise: &[],
            },
        ],
        target = Some(TargetSpec::AnyTarget),
    )],
);
