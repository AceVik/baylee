//! `cards/enchantments/auras/mv_1/rancor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Rancor: the whole card, which is three sentences and a return trip.
///
/// The Aura is cast on a creature, the creature is +2/+0 with trample, and
/// when the Aura goes to the graveyard it comes back to its owner's hand —
/// the last of which is what makes this the card it is, and the only way to
/// see it is to kill the host.
#[test]
fn rancor_pumps_its_host_and_comes_back_when_it_dies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(397, forest())
        .battlefield(0, &[forest(), llanowar_elves()])
        .hand(0, &[rancor()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is seated");
    cast_from_hand(&mut engine, p0, rancor());
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal host");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(pt(&engine, elf), (3, 1), "a 1/1 with +2/+0 is a 3/1");
    assert!(
        keywords(&engine, elf).contains(KeywordSet::TRAMPLE),
        "and it tramples"
    );

    // The host leaves, so the Aura is put into the graveyard (CR 704.5m) and
    // its own trigger sends it home.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, swords_to_plowshares());
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("the Elf is a legal target");
    assert!(
        matches!(drive_to_rest(&mut engine, p0), Rest::Reached),
        "the removal and the Aura's trigger both resolve"
    );

    assert!(
        in_hand(&engine, p0, rancor()).is_some(),
        "the Aura went to the graveyard and its trigger returned it to hand"
    );
    assert!(
        in_graveyard(&engine, p0, rancor()).is_none(),
        "so it is not lying in the graveyard"
    );
}
