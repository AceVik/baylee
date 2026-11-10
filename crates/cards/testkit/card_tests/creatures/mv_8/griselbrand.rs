//! `cards/creatures/mv_8/griselbrand.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn griselbrand() -> CardIndex {
    card_index("f759d112-76db-4091-a22b-b9f19ab6fa5f")
}

/// Griselbrand is a 7/7 for {4}{B}{B}{B}{B} with flying and lifelink.
#[test]
fn griselbrand_is_cast_for_eight_with_flying_and_lifelink() {
    let mut engine = at_main(&[swamp(); 8], &[griselbrand()]);
    let demon = cast_and_settle(&mut engine, griselbrand());
    assert_eq!(pt(&engine, demon), (7, 7));
    assert!(keywords(&engine, demon).contains(KeywordSet::FLYING));
    assert!(keywords(&engine, demon).contains(KeywordSet::LIFELINK));
}

/// "Pay 7 life: Draw seven cards": the cost is life, not mana, and seven
/// cards arrive.
#[test]
fn griselbrand_pays_seven_life_to_draw_seven() {
    let p0 = PlayerId::new(0);
    let mut engine = at_main(&[griselbrand()], &[]);
    let (life, hand) = (life_of(&engine, 0), hand_size(&engine, 0));
    activate(&mut engine, p0, griselbrand(), 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, 0), life - 7);
    assert_eq!(hand_size(&engine, 0), hand + 7);
}

/// Lifelink: 7 combat damage to the opponent is 7 life for Griselbrand's
/// controller.
#[test]
fn griselbrand_gains_seven_when_it_connects() {
    let p0 = PlayerId::new(0);
    let mut engine = at_main(&[griselbrand()], &[]);
    let demon = on_battlefield(&engine, p0, griselbrand()).expect("Griselbrand");
    let (mine, theirs) = (life_of(&engine, 0), life_of(&engine, 1));
    attack_player(&mut engine, demon);
    no_blocks_and_settle(&mut engine);
    assert_eq!(life_of(&engine, 1), theirs - 7);
    assert_eq!(life_of(&engine, 0), mine + 7);
}
