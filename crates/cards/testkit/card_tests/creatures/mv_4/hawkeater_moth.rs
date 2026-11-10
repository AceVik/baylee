//! `cards/creatures/mv_4/hawkeater_moth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Hawkeater Moth` is a 1/2 creature under `Coverage::Implemented` with flying and shroud.
/// When cast from hand off four Forests, it resolves onto the battlefield with both keywords.
/// When the opponent casts targeted removal (`Swords to Plowshares`), shroud prevents the Moth from being targeted while an un-shrouded creature is offered.
#[test]
fn hawkeater_moth_has_flying_and_shroud() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[forest(), forest(), forest(), forest(), llanowar_elves()],
        )
        .hand(0, &[hawkeater_moth()])
        .battlefield(1, &[plains()])
        .hand(1, &[swords_to_plowshares()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("Elf deployed");

    // Float 4 green mana from the Forests while keeping the Elf untapped.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Forests produce four green mana"
    );

    cast_with_floating(&mut engine, p0, hawkeater_moth());
    pass_until(&mut engine, stack_is_empty);

    let moth = on_battlefield(&engine, p0, hawkeater_moth()).expect("Hawkeater Moth resolved");
    assert_eq!(pt(&engine, moth), (1, 2), "printed body is 1/2");
    let kw = keywords_of(&engine, moth);
    assert!(kw.contains(KeywordSet::FLYING), "Hawkeater Moth has flying");
    assert!(kw.contains(KeywordSet::SHROUD), "Hawkeater Moth has shroud");

    // Advance to opponent's turn to test shroud against targeted removal.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, swords_to_plowshares());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected target choice, got {:?}", engine.pending());
    };
    assert_eq!(
        player, p1,
        "opponent chooses target for Swords to Plowshares"
    );
    assert_eq!((min, max), (1, 1), "exactly one target required");
    assert!(
        options.contains(&elf),
        "Elf without shroud is a legal target: {options:?}"
    );
    assert!(
        !options.contains(&moth),
        "Hawkeater Moth with shroud cannot be targeted: {options:?}"
    );
}
