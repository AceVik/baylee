//! `cards/creatures/mv_4/cho_manno_revolutionary.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);

fn subject_card() -> CardIndex {
    card_index("91af5e35-b3b8-43ce-b1ea-997ed74e4ad2")
}

/// Cho-Manno, Revolutionary — {2}{W}{W} 2/2 Legendary Creature — Human Rebel:
/// "Prevent all damage that would be dealt to Cho-Manno."
#[test]
fn cho_manno_prevents_all_damage_dealt_to_itself() {
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains(), mountain()])
        .hand(0, &[subject_card(), lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    cast_from_hand(&mut engine, P0, subject_card());
    pass_until(&mut engine, stack_is_empty);

    let cho_manno = on_battlefield(&engine, P0, subject_card()).unwrap();
    assert_eq!(pt(&engine, cho_manno), (2, 2));

    cast_from_hand(&mut engine, P0, lightning_bolt());
    aim_at(&mut engine, P0, cho_manno);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, P0, subject_card()).is_some());
    assert_eq!(engine.state().object(cho_manno).unwrap().damage, 0);
}
