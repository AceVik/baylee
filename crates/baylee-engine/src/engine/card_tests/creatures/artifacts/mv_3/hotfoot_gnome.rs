//! `cards/creatures/artifacts/mv_3/hotfoot_gnome.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hotfoot_gnome() -> CardIndex {
    card_index("3efb64a4-04f6-4b78-be1c-5b6320c02a71")
}

/// Hotfoot Gnome — haste 3/1; "{T}: Another target creature gains haste until
/// end of turn." The menu leaves out the Gnome itself, and the target has
/// haste afterwards and not before.
#[test]
fn hotfoot_gnome_gives_another_creature_haste() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hotfoot_gnome(), quiet_creature()])
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let gnome = on_battlefield(&engine, p0, hotfoot_gnome()).expect("seated");
    let mine = on_battlefield(&engine, p0, quiet_creature()).expect("seated");
    let theirs = on_battlefield(&engine, p1, quiet_creature()).expect("seated");
    assert_eq!(pt(&engine, gnome), (3, 1));
    assert!(keywords(&engine, gnome).contains(KeywordSet::HASTE));
    assert!(!keywords(&engine, mine).contains(KeywordSet::HASTE));

    activate(&mut engine, p0, hotfoot_gnome(), 0);
    let options = aim_at(&mut engine, p0, mine);
    assert!(options.contains(&mine) && options.contains(&theirs));
    assert!(!options.contains(&gnome), "\"another\" target creature");
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, mine).contains(KeywordSet::HASTE));
    assert!(!keywords(&engine, theirs).contains(KeywordSet::HASTE));
    assert!(is_tapped(&engine, gnome), "{{T}} is the cost");
}
