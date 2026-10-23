#[allow(clippy::wildcard_imports)]
use super::*;

fn spreading_algae() -> CardIndex {
    card_index("a08189f2-1643-496a-8a91-74864528b4db")
}

#[test]
fn spreading_algae_destroys_enchanted_swamp_when_tapped_and_returns_to_hand() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[spreading_algae()])
        .battlefield(0, &[forest(), swamp()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let swamp_id = on_battlefield(&engine, p0, swamp()).expect("swamp on bf");
    tap_mana_where(&mut engine, p0, |id| id != swamp_id);
    cast_with_floating(&mut engine, p0, spreading_algae());
    aim_at(&mut engine, p0, swamp_id);
    pass_until(&mut engine, stack_is_empty);

    tap_mana_where(&mut engine, p0, |id| id == swamp_id);
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, swamp()).is_none());
    assert!(in_graveyard(&engine, p0, swamp()).is_some());
    assert!(in_hand(&engine, p0, spreading_algae()).is_some());
}
