//! Chevill, Bane of Monsters — {B}{G} — Legendary Creature — Human Rogue
//! Oracle: Deathtouch
//! Oracle: At the beginning of your upkeep, if your opponents control no permanents with bounty counters on them, put a bounty counter on target creature or planeswalker an opponent controls.
//! Oracle: Whenever a permanent an opponent controls with a bounty counter on it dies, you gain 3 life and draw a card.
//! Set: IKO #181 — Ikoria: Lair of Behemoths | Scryfall ID: fecbf0a3-ebe1-43b6-a720-462ba19002eb | Oracle ID: 50863075-a30b-4238-8e31-5af32a39d886
// IMPLEMENTED — bounty placement is gated on every opposing board being clear;
// the death trigger reads the departed permanent's counters and controller.

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

static OPPOSING_BOUNTY: Filter = Filter::And(&[
    Filter::ControlledByOpponent,
    Filter::HasCounter(counters::BOUNTY),
]);
static OPPOSING_CREATURE_OR_PLANESWALKER: Filter = Filter::And(&[
    Filter::ControlledByOpponent,
    Filter::Or(&[Filter::CREATURE, Filter::PLANESWALKER]),
]);

card!(
    index = index::CHEVILL_BANE_OF_MONSTERS,
    oracle_id = "50863075-a30b-4238-8e31-5af32a39d886",
    scryfall_id = "fecbf0a3-ebe1-43b6-a720-462ba19002eb",
    color_identity = ColorSet::from_slice(&[Color::Black, Color::Green]),
    commander = CommanderRule::Legendary,
    keywords = KeywordSet::DEATHTOUCH,
    coverage = Coverage::Implemented,
    faces = &[face!(
        name = "Chevill, Bane of Monsters",
        mana_cost = mana!("{B}{G}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::ROGUE],
        power = Some(1),
        toughness = Some(3),
    ),],
    abilities = &[
        triggered!(
            Trigger::StepBegin {
                step: StepKind::Upkeep,
                whose: PlayerRel::You,
            },
            &[Effect::AddCounter {
                kind: counters::BOUNTY,
                amount: Amount::Fixed(1),
            }],
            condition = Some(Condition::BattlefieldCountAtMost(&OPPOSING_BOUNTY, 0)),
            targets = Some(TargetReq::one(TargetSpec::Object(
                &OPPOSING_CREATURE_OR_PLANESWALKER
            )))
        ),
        triggered!(
            Trigger::Dies(&OPPOSING_BOUNTY),
            &[Effect::gain_life(3), Effect::draw(1)]
        ),
    ],
);
