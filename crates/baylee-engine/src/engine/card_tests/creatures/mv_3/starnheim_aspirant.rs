//! `cards/creatures/mv_3/starnheim_aspirant.rs`, played.
#[allow(clippy::wildcard_imports)]
use super::*;

fn subject() -> CardIndex {
    card_index("e7b92789-47a9-430a-ad57-be7b6c3d63cb")
}
fn overseer() -> CardIndex {
    card_index("d646e42b-5635-4798-b633-29c093b66a55")
}

#[test]
fn starnheim_aspirant_reduces_only_angel_spells_by_two() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[subject(), plains()])
        .hand(0, &[overseer(), ondu_cleric()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    tap_all_mana(&mut e, p0);
    let offer = priority_offer(&e);
    let angel = in_hand(&e, p0, overseer()).unwrap();
    let human = in_hand(&e, p0, ondu_cleric()).unwrap();
    assert!(
        offer.castable.contains(&angel),
        "{{2}}{{W}} less {{2}} costs one"
    );
    assert!(
        !offer.castable.contains(&human),
        "the Human spell still costs two"
    );
    cast_with_floating(&mut e, p0, overseer());
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
    assert!(on_battlefield(&e, p0, overseer()).is_some());
}
