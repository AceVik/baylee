//! `cards/enchantments/mv_5/zendikar_s_roil.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn zendikar_s_roil() -> CardIndex {
    card_index("a842cc2b-52eb-4dc5-86c6-6575c2ed913d")
}

/// "Landfall — Whenever a land you control enters, create a 2/2 green
/// Elemental creature token." My land makes one; the opponent's does not
/// make one for either of us.
#[test]
fn zendikar_s_roil_makes_an_elemental_for_my_lands_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[zendikar_s_roil()])
        .hand(0, &[forest()])
        .hand(1, &[plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    assert!(tokens_of(&engine, p0).is_empty());
    play_land(&mut engine, p0, forest());
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 1);
    assert_eq!(pt(&engine, tokens[0]), (2, 2));

    reach_their_main_phase(&mut engine, p1);
    play_land(&mut engine, p1, plains());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(tokens_of(&engine, p0).len(), 1, "theirs adds none to me");
    assert!(tokens_of(&engine, p1).is_empty(), "nor to them");
}
