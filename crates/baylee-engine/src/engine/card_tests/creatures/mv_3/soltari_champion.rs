//! `cards/creatures/mv_3/soltari_champion.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::power_toughness;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn soltari_champion() -> CardIndex {
    card_index("765017e5-b43d-4c77-a9a9-0fcf0c0cd098")
}

/// Soltari Champion (`765017e5-b43d-4c77-a9a9-0fcf0c0cd098`) — {2}{W} 2/2 Creature — Soltari Soldier (Implemented):
/// "Shadow. Whenever this creature attacks, other creatures you control get +1/+1 until end of turn."
#[test]
fn soltari_champion_pumps_other_creatures_you_control_when_attacking() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[soltari_champion(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let champ = on_battlefield(&engine, P0, soltari_champion()).unwrap();
    let bear = on_battlefield(&engine, P0, index::GRIZZLY_BEARS).unwrap();

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(champ, Defender::Player(P1))],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(power_toughness(&engine, champ), (2, 2));
    assert_eq!(power_toughness(&engine, bear), (3, 3));
}
