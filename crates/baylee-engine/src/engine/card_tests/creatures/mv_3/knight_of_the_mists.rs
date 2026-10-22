//! `cards/creatures/mv_3/knight_of_the_mists.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn knight_of_the_mists() -> CardIndex {
    card_index("e9893534-3d05-42c2-93ff-f28ffae89aa3")
}

fn knight_of_valor() -> CardIndex {
    card_index("f2edb543-de33-4eed-892c-98038eec96e2")
}

/// `Knight of the Mists` ({2}{U}, 2/2 Human Knight, `Coverage::Implemented`):
/// `Flanking. When this creature enters, you may pay {U}. If you don't, destroy
/// target Knight and it can't be regenerated.`
#[test]
fn knight_of_the_mists_etb_destroys_target_knight_when_u_is_not_paid() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island()])
        .battlefield(1, &[knight_of_valor()])
        .hand(0, &[knight_of_the_mists()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let enemy_knight = on_battlefield(&engine, p1, knight_of_valor()).unwrap();
    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, knight_of_the_mists());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut engine, p0, enemy_knight);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. }) || stack_is_empty(e)
    });
    if matches!(engine.pending(), Pending::YesNo { .. }) {
        engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
        pass_until(&mut engine, stack_is_empty);
    }

    assert_eq!(
        engine.state().object(enemy_knight).unwrap().zone,
        Zone::Graveyard
    );
}
