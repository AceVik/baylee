//! Takklemaggot — {2}{B}{B} — Enchantment — Aura
//! Oracle: Enchant creature
//! Oracle: At the beginning of the upkeep of enchanted creature's controller, put a -0/-1 counter on that creature.
//! Oracle: When enchanted creature dies, that creature's controller chooses a creature that this card could enchant. If the player does, return this card to the battlefield under your control attached to that creature. If they don't, return this card to the battlefield under your control as a non-Aura enchantment. It loses "enchant creature" and gains "At the beginning of that player's upkeep, this enchantment deals 1 damage to that player."
//! Set: ME3 #76 — Masters Edition III | Scryfall ID: a1171f5b-5b92-4a65-a59d-97ac6ce1283d | Oracle ID: 31d01633-86f8-4a3f-9c28-f6968e056ba3
// PARTIAL — enchant creature and the upkeep -0/-1 counter are written; the
// death clause is off the card (see NOT SUPPORTED below).

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::TAKKLEMAGGOT,
    oracle_id = "31d01633-86f8-4a3f-9c28-f6968e056ba3",
    scryfall_id = "a1171f5b-5b92-4a65-a59d-97ac6ce1283d",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    coverage = Coverage::Partial(
        "the death clause's controller chooses a creature this Aura could enchant \
         and this card returns attached to it, or returns as a non-Aura enchantment \
         that loses \"enchant creature\" and gains a new upkeep trigger; no effect \
         chooses a legal host, reattaches the Aura as it returns, or makes a \
         permanent non-Aura with a granted trigger"
    ),
    faces = &[face!(
        name = "Takklemaggot",
        mana_cost = mana!("{2}{B}{B}"),
        types = TypeSet::ENCHANTMENT,
        subtypes = &[subtypes::enchantment::AURA],
    ),],
    // NOT SUPPORTED: "When enchanted creature dies, that creature's controller
    // chooses a creature that this card could enchant. If the player does,
    // return this card to the battlefield under your control attached to that
    // creature. If they don't, return this card to the battlefield under your
    // control as a non-Aura enchantment. It loses \"enchant creature\" and
    // gains \"At the beginning of that player's upkeep, this enchantment deals
    // 1 damage to that player.\"" — `GraveyardToBattlefield` cannot reattach
    // the returned Aura or change its subtypes and keyword, and there is no
    // effect that grants a trigger to a permanent or remembers the dead
    // creature's controller for it.
    abilities = &[
        spell!(
            &[Effect::AttachSelf {
                target: TargetSpec::Object(&Filter::CREATURE)
            }],
            targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))
        ),
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::ControllerOfAttached
            },
            &[Effect::AddCounterFilter {
                filter: &Filter::AttachedToBySource,
                kind: CounterKind::Minus {
                    power: 0,
                    toughness: 1
                },
                amount: Amount::Fixed(1),
            }]
        ),
    ],
);
