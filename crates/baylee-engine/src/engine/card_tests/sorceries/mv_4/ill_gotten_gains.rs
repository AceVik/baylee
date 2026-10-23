#[allow(clippy::wildcard_imports)]
use super::*;

fn ill_gotten_gains() -> CardIndex {
    card_index("c88eff74-def5-42ea-873c-b3b479f6fe18")
}

#[test]
fn ill_gotten_gains_exiles_itself_and_each_player_discards_hand() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .hand(0, &[ill_gotten_gains(), swamp()])
        .hand(1, &[swamp()])
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, ill_gotten_gains());
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, swamp()).is_none());
    assert!(in_hand(&engine, p1, swamp()).is_none());
    assert!(in_graveyard(&engine, p0, ill_gotten_gains()).is_none());
}
