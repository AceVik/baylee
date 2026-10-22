//! `cards/creatures/artifacts/mv_3/aesthir_glider.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn aesthir_glider() -> CardIndex {
    card_index("99dcde0b-4f4d-4d7a-89aa-8a89c256e38f")
}

/// `Aesthir Glider` ({3}, 2/1 Artifact Creature — Bird Construct) has flying
/// and can't block.
#[test]
fn aesthir_glider_flies_over_ground_creatures_and_cannot_block() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(); 3])
        .battlefield(1, &[grizzly_bears()])
        .hand(0, &[aesthir_glider()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, aesthir_glider());
    pass_until(&mut engine, stack_is_empty);
    let glider = on_battlefield(&engine, p0, aesthir_glider()).expect("Aesthir Glider enters");
    assert_eq!(pt(&engine, glider), (2, 1));
    assert!(keywords(&engine, glider).contains(KeywordSet::FLYING.union(KeywordSet::CANT_BLOCK)));

    // On P1's turn, Grizzly Bears attacks and Aesthir Glider cannot block it.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::DeclareAttackers
    });
    let bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    assert!(
        engine
            .apply(
                p0,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(glider, bears)],
                },
            )
            .is_err(),
        "Aesthir Glider has CANT_BLOCK and cannot be declared as a blocker"
    );
    engine
        .apply(p0, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    // On P0's next turn, Aesthir Glider attacks in the air and P1's untapped Grizzly Bears cannot block it.
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(glider, Defender::Player(p1))],
            },
        )
        .unwrap();
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(engine.state().players[1].life, 18);
}
