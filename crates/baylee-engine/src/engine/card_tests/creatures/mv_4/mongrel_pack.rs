//! `cards/creatures/mv_4/mongrel_pack.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn mongrel_pack() -> CardIndex {
    card_index("e1208d39-86e7-4eae-8dd6-b5bfb29def84")
}

/// Mongrel Pack (`e1208d39-86e7-4eae-8dd6-b5bfb29def84`) — {3}{G} Creature — Dog:
/// "When this creature dies during combat, create four 1/1 green Dog creature tokens."
#[test]
fn mongrel_pack_creates_four_dog_tokens_when_it_dies_during_combat() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[mongrel_pack()])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let pack = on_battlefield(&engine, P0, mongrel_pack()).unwrap();
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(pack, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(bears, pack)],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);

    assert!(in_graveyard(&engine, P0, mongrel_pack()).is_some());
    assert_eq!(tokens_of(&engine, P0).len(), 4);
}
