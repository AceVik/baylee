//! `cards/enchantments/auras/mv_2/armor_of_thorns.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::{ABYSSAL_HUNTER, GRIZZLY_BEARS};

fn armor_of_thorns() -> CardIndex {
    card_index("31ead5a5-aa01-4601-8d3e-267b65fce3bb")
}

#[test]
fn armor_of_thorns_gives_enchanted_nonblack_creature_plus_two_plus_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[armor_of_thorns()])
        .battlefield(0, &[GRIZZLY_BEARS, forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();

    cast_from_hand(&mut engine, p0, armor_of_thorns());
    aim_at(&mut engine, p0, bear);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, bear), (4, 4));
}

#[test]
fn armor_of_thorns_cannot_enchant_a_black_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[armor_of_thorns()])
        .battlefield(0, &[GRIZZLY_BEARS, forest(), forest()])
        .battlefield(1, &[ABYSSAL_HUNTER])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bear = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let hunter = on_battlefield(&engine, p1, ABYSSAL_HUNTER).unwrap();
    cast_from_hand(&mut engine, p0, armor_of_thorns());
    let options = aim_at(&mut engine, p0, bear);
    assert!(options.contains(&bear));
    assert!(!options.contains(&hunter), "Abyssal Hunter is black");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, bear), (4, 4));
    assert_eq!(pt(&engine, hunter), (1, 1), "only the enchanted creature");
}

/// Armor of Thorns (Partial): "You may cast this spell as though it had
/// flash. If you cast it any time a sorcery couldn't have been cast …" The
/// flash is not played, so the Aura is only castable when a sorcery is. A
/// pinned limitation: this test moves when the flash can be said whole.
#[test]
fn armor_of_thorns_is_not_castable_when_a_sorcery_could_not_be_cast() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[armor_of_thorns()])
        .battlefield(0, &[GRIZZLY_BEARS, forest(), forest()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    let aura = in_hand(&engine, p0, armor_of_thorns()).unwrap();
    // On p1's turn, p0 gets priority only after p1 passes: no sorcery timing.
    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    let Pending::Priority { player, legal, .. } = engine.pending().clone() else {
        panic!("priority, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert!(!legal.castable.contains(&aura));
}
