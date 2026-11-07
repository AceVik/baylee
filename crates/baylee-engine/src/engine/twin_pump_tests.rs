//! Two creatures of one name, each pumped by its own Samut's Sprint (the
//! evidence: game 01a1b305, turn 13). One Sprint resolving must leave the
//! other Burning Prophet at the +1/+0 its own cast trigger gave it.

use super::testkit::{
    Duel, basic_forest, card_index, in_hand, keep_mulligans, pass_until, tap_all_mana,
};
use super::*;
use baylee_core::ids::{CardIndex, ObjectId};

const P0: PlayerId = PlayerId::new(0);

type Game = Engine<super::testkit::RegistryLookup>;

fn prophet() -> CardIndex {
    card_index("1e462ce2-15d3-473c-8f48-9c9363fc65eb")
}

fn sprint() -> CardIndex {
    card_index("7d07821e-d34b-4441-a137-836ffce7127b")
}

fn mountain() -> CardIndex {
    card_index("a3fb7228-e76b-4e96-a40e-20b5fed75685")
}

fn prophets(engine: &Game) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine
                .state()
                .object(*id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == prophet()))
        })
        .collect()
}

fn read(engine: &Game, id: ObjectId) -> ((i16, i16), bool) {
    let c = engine.state().object(id).unwrap().characteristics();
    (
        (c.power.unwrap_or(0), c.toughness.unwrap_or(0)),
        c.keywords.contains(baylee_cards_dsl::KeywordSet::HASTE),
    )
}

fn stack_len(engine: &Game) -> usize {
    engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Stack)
        .len()
}

/// Answers whatever is pending, except that every target choice names `aim`.
fn settle(engine: &mut Game, aim: ObjectId) {
    for _ in 0..60 {
        match engine.pending().clone() {
            Pending::Priority { .. } => return,
            Pending::ChooseTargets { player, .. } => {
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![aim],
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            _ => {
                let (player, action) = super::testkit::answer_one(engine).expect("answerable");
                engine.apply(player, action).unwrap();
            }
        }
    }
    panic!("never reached priority");
}

fn settle_stack_to(engine: &mut Game, len: usize) {
    let mut guard = 0;
    while stack_len(engine) > len {
        let (player, action) = super::testkit::answer_one(engine).expect("answerable");
        engine.apply(player, action).unwrap();
        guard += 1;
        assert!(guard < 80, "stack never shrank");
    }
}

fn cast(engine: &mut Game, aim: ObjectId) {
    let card = in_hand(engine, P0, sprint()).expect("a Sprint in hand");
    engine.apply(P0, PlayerAction::CastSpell { card }).unwrap();
    settle(engine, aim);
}

#[test]
fn one_sprint_resolving_leaves_the_other_prophet_alone() {
    let mut engine = Duel::new(0x5a17_0001, basic_forest())
        .hand(0, &[sprint(), sprint()])
        .battlefield(0, &[mountain(), mountain(), prophet(), prophet()])
        .start();
    keep_mulligans(&mut engine);
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == P0)
    });
    let both = prophets(&engine);
    assert_eq!(both.len(), 2);
    let (five, seven) = (both[0], both[1]);
    tap_all_mana(&mut engine, P0);

    // As in the game: the first Sprint's triggers resolve, then the second
    // is cast on top of the lone first Sprint, and everything above the
    // first Sprint resolves.
    cast(&mut engine, five);
    settle_stack_to(&mut engine, 1);
    cast(&mut engine, seven);
    settle_stack_to(&mut engine, 1);
    // Each Prophet saw two noncreature casts: +1 +1. The second Sprint
    // (aimed at seven) is done; the first (aimed at five) is not.
    assert_eq!(read(&engine, seven), ((5, 4), true), "seven: 1+2+2 / 3+1");
    assert_eq!(
        read(&engine, five),
        ((3, 3), false),
        "five: 1+2, no Sprint yet"
    );
}
