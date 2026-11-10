//! `cards/enchantments/mv_5/dictate_of_heliod.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn dictate_of_heliod() -> CardIndex {
    card_index("b989279c-665d-4f15-afc1-adf3872a4851")
}

/// "Flash": the enchantment is cast in the upkeep, where a sorcery-speed
/// permanent could not be. "Creatures you control get +2/+2": mine grow,
/// the opponent's do not.
#[test]
fn dictate_of_heliod_has_flash_and_pumps_my_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .hand(0, &[dictate_of_heliod()])
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                gray_ogre(),
            ],
        )
        .battlefield(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    assert_ne!(
        engine.state().turn.phase,
        Phase::FirstMain,
        "this is still before the main phase"
    );
    let mine = on_battlefield(&engine, p0, gray_ogre()).expect("my Ogre");
    let theirs = on_battlefield(&engine, p1, gray_ogre()).expect("their Ogre");
    assert_eq!(pt(&engine, mine), (2, 2));
    cast_from_hand(&mut engine, p0, dictate_of_heliod());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, dictate_of_heliod()).is_some()
    });
    assert_eq!(pt(&engine, mine), (4, 4), "+2/+2");
    assert_eq!(pt(&engine, theirs), (2, 2), "not the opponent's");
}
