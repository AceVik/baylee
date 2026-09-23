//! Liliana the Repentant — {1}{B} — Legendary Creature — Human Warlock
//! Oracle: Whenever another creature or planeswalker you control enters, mill two cards.
//! Oracle: Exhaust — {5}{B}: Return target creature or planeswalker card from your graveyard to the battlefield. Put a +1/+1 counter on Liliana. Activate only as a sorcery. (Activate each exhaust ability only once.)
//! Set: FRA #231 — Reality Fracture | Scryfall ID: 1eb25a6c-d6b4-465d-990e-f1ab86b26b69 | Oracle ID: 5eb4403f-f199-4f75-a7c6-e76783f9b07d
// PARTIAL — another creature or planeswalker you control entering mills you
// two, and {5}{B} at sorcery speed brings a creature or planeswalker card
// back out of your graveyard and grows Liliana by a +1/+1 counter.
// NOT SUPPORTED: "Exhaust" — "(Activate each exhaust ability only once.)"
// Besides its cost and its timing, the one thing an activated ability may
// state is an `Condition`, and all four of its cases read a board
// fact: a permanent count, a counter count on the source, an opponent's
// graveyard count. None of them can ask what this ability has already done,
// and no object records it, so the once-per-game half of the keyword has
// nothing to be refused by. The card plays as though only the `{5}{B}` and
// the sorcery-speed clause were printed, which here makes it better than
// printed rather than inert: the reanimation is offered again every turn the
// mana is there.

static ANOTHER_CREATURE_OR_PLANESWALKER_YOU_CONTROL: Filter = Filter::And(&[
    Filter::CREATURE_OR_PLANESWALKER,
    Filter::ControlledByYou,
    Filter::Another,
]);

use baylee_cards_dsl::prelude::*;
use baylee_core::generated::subtypes;

card!(
    index = index::LILIANA_THE_REPENTANT,
    oracle_id = "5eb4403f-f199-4f75-a7c6-e76783f9b07d",
    scryfall_id = "1eb25a6c-d6b4-465d-990e-f1ab86b26b69",
    color_identity = ColorSet::from_slice(&[Color::Black]),
    commander = CommanderRule::Legendary,
    faces = &[face!(
        name = "Liliana the Repentant",
        mana_cost = mana!("{1}{B}"),
        types = TypeSet::CREATURE,
        supertypes = SupertypeSet::LEGENDARY,
        subtypes = &[subtypes::creature::HUMAN, subtypes::creature::WARLOCK],
        power = Some(2),
        toughness = Some(2),
    ),],
    coverage = Coverage::Partial("exhaust is not enforced: the ability is offered more than once"),
    abilities = &[
        triggered!(
            Trigger::EntersBattlefield(&ANOTHER_CREATURE_OR_PLANESWALKER_YOU_CONTROL),
            &[Effect::Mill {
                amount: Amount::Fixed(2),
                target: PlayerRel::You,
            }]
        ),
        activated!(
            cost!("{5}{B}"),
            &[
                Effect::reanimate(TargetSpec::CardInGraveyard(
                    &Filter::CREATURE_OR_PLANESWALKER,
                    PlayerRel::You,
                )),
                // `AddCounter` puts its counters on the first target, which
                // here is the card coming back. The printed counter goes on
                // Liliana, so the source is named by filter instead.
                Effect::AddCounterFilter {
                    filter: &Filter::This,
                    kind: CounterKind::P1P1,
                    amount: Amount::Fixed(1),
                },
            ],
            target = Some(TargetSpec::CardInGraveyard(
                &Filter::CREATURE_OR_PLANESWALKER,
                PlayerRel::You
            )),
            timing = ActivationTiming::SorcerySpeed
        ),
    ],
);

// Engine-level coverage belongs in `card_tests`, and two of the three things
// to check are firsts for this pool. `Effect::Mill` has never been written
// with `PlayerRel::You` before — every other Mill here names `Chosen` or
// `ControllerOfTarget` — so the trigger wants a test that it is *your* library
// the two cards come off, with another creature entering, another planeswalker
// entering, and Liliana's own entry milling nothing. And the activation is the
// pool's first targeted reanimation that also puts a counter on its own source:
// the graveyard card arrives under your control (a planeswalker with its
// printed loyalty, which `progress` seeds however the permanent entered) and
// the +1/+1 counter lands on Liliana rather than on it.
