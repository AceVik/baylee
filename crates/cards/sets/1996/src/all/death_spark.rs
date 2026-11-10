//! Death Spark — {R} — Instant
//! Oracle: Death Spark deals 1 damage to any target.
//! Oracle: At the beginning of your upkeep, if this card is in your graveyard with a creature card directly above it, you may pay {1}. If you do, return this card to your hand.
//! Set: ME2 #123 — Masters Edition II | Scryfall ID: 778d3497-2067-4d61-a322-5efb776dbaf1 | Oracle ID: 4b23a89c-89ba-41ad-b894-ec02221689eb
// NOT SUPPORTED: "a creature card directly above it" in any graveyard order. No condition names the card directly above, so the return is gated on the narrower case it can say: exactly one card above this card, and it is a creature (this never fires when the printed text would not; it misses a creature directly above with more cards on top of it).

use baylee_cards_dsl::prelude::*;

card!(
    index = index::DEATH_SPARK,
    oracle_id = "4b23a89c-89ba-41ad-b894-ec02221689eb",
    scryfall_id = "778d3497-2067-4d61-a322-5efb776dbaf1",
    color_identity = ColorSet::from_slice(&[Color::Red]),
    coverage = Coverage::Partial(
        "deals 1 damage to any target; at the beginning of your upkeep, if it is in your graveyard with exactly one card above it and that card is a creature card, you may pay {1} to return it to your hand; a creature card directly above it with other cards on top of that is not recognised, so the return happens less often than printed",
    ),
    faces = &[face!(
        name = "Death Spark",
        mana_cost = mana!("{R}"),
        types = TypeSet::INSTANT,
    ),],
    abilities = &[
        spell!(
            &[Effect::DealDamage {
                amount: Amount::Fixed(1),
                target: TargetSpec::AnyTarget,
            }],
            targets = Some(TargetReq::one(TargetSpec::AnyTarget)),
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::PlayerMayPayManaThen {
                player: PlayerRel::You,
                cost: mana!("{1}"),
                effects: &[Effect::ReturnToHand {
                    target: TargetSpec::ThisObject,
                }],
            }],
            zone = TriggerZone::Graveyard,
            condition = Some(Condition::All(&[
                Condition::GraveyardCardsAbove(&Filter::CREATURE, 1),
                Condition::Not(&Condition::GraveyardCardsAbove(&Filter::Any, 2)),
            ])),
        ),
    ],
);
