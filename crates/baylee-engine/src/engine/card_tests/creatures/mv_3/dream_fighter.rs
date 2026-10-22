//! `cards/creatures/mv_3/dream_fighter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{DREAM_FIGHTER, GRIZZLY_BEARS};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Dream Fighter (ea8d1c55-e045-4a3f-92f0-4ecd86362b97) — {2}{U} 1/1:
/// "Whenever this creature blocks or becomes blocked by a creature, this
/// creature and that creature phase out."
#[test]
fn dream_fighter_phases_out_when_blocked_in_combat() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[DREAM_FIGHTER])
        .battlefield(1, &[GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);

    let fighter = on_battlefield(&engine, P0, DREAM_FIGHTER).unwrap();
    let bears = on_battlefield(&engine, P1, GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == P0 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(fighter, Defender::Player(P1))],
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
                blockers: vec![(bears, fighter)],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        engine
            .state()
            .object(fighter)
            .unwrap()
            .status
            .contains(Status::PHASED_OUT)
    );
}
