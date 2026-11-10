//! Hum of the Radix counts artifacts controlled by each spell's caster.

use super::*;

fn hum_of_the_radix() -> CardIndex {
    card_index("95803a5a-9f98-4764-b6bd-6cf4cdcb740b")
}

#[test]
fn hum_prices_each_caster_against_their_own_artifacts() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                hum_of_the_radix(),
                darksteel_pendant(),
                darksteel_pendant(),
                island(),
                island(),
            ],
        )
        .battlefield(1, &[darksteel_pendant(), island(), island()])
        .hand(0, &[quiet_artifact()])
        .hand(1, &[quiet_artifact()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    tap_all_mana(&mut e, p0);
    let mine = in_hand(&e, p0, quiet_artifact()).unwrap();
    assert!(
        e.apply(p0, PlayerAction::CastSpell { card: mine }).is_err(),
        "three mana is required, not two"
    );

    reach_their_main_phase(&mut e, p1);
    cast_from_hand(&mut e, p1, quiet_artifact());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(all_on_battlefield(&e, p1, quiet_artifact()).len(), 1);
    assert_eq!(
        e.state().players[1].mana_pool.total(),
        0,
        "one mana plus one tax was paid"
    );
}
