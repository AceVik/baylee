//! `cards/sorceries/mv_4/aftershock.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Aftershock — {2}{R}{R} sorcery: "Destroy target artifact, creature, or
/// land. Aftershock deals 3 damage to you."
///
/// The filter is three words wide, so the board carries one of each on the
/// opponent's side plus a permanent that is none of them — an Exploration,
/// which a bare "target permanent" would have offered and this must decline.
/// The menu is read as the enumeration it is, the land is then destroyed for
/// real, and the artifact and creature beside it are checked to still be
/// standing so the reading is a targeted destruction and not a sweeper. The
/// drawback is the other printed sentence and is read on *both* seats: three
/// damage to the caster and the opponent's total untouched, which a card that
/// had aimed the damage at the target's controller would fail.
#[test]
fn aftershock_destroys_one_artifact_creature_or_land_and_bites_its_caster() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                exploration(),
            ],
        )
        .battlefield(1, &[forest(), quiet_creature(), quiet_artifact()])
        .hand(0, &[aftershock()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");
    let beast = on_battlefield(&engine, p1, quiet_creature()).expect("their Elf is out");
    let rock = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    let chant = on_battlefield(&engine, p0, exploration()).expect("the Exploration is out");
    let mine = on_battlefield(&engine, p0, mountain()).expect("my own Mountain is out");

    // `cast_from_hand` taps the four Mountains first: the offer for a
    // {2}{R}{R} spell is read off the pool and not off the untapped lands.
    cast_from_hand(&mut engine, p0, aftershock());

    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target artifact, creature, or land\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat casting Aftershock names its target");
    assert_eq!((min, max), (1, 1), "one target, and the spell asks once");
    assert!(
        options.contains(&land) && options.contains(&beast) && options.contains(&rock),
        "all three words of the filter are read: {options:?}"
    );
    assert!(
        options.contains(&mine),
        "\"target land\" is any land and not the opponent's alone: {options:?}"
    );
    assert!(
        !options.contains(&chant),
        "an enchantment is a permanent and none of the three: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![land],
            },
        )
        .expect("the Forest was one of the options the question enumerated");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, forest()).is_some(),
        "\"destroy target land\": the Forest is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_creature()).is_some(),
        "the creature the spell did not name is still standing"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "and so is the artifact: one target, one destruction"
    );
    assert!(
        on_battlefield(&engine, p0, exploration()).is_some(),
        "and the permanent the filter declined never moved"
    );
    assert!(
        in_graveyard(&engine, p0, aftershock()).is_some(),
        "the sorcery resolved and went to its owner's graveyard"
    );
    assert_eq!(
        engine.state().players[0].life,
        17,
        "\"Aftershock deals 3 damage to you\" — the caster pays the drawback"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the damage belongs to the seat that cast it, not to the opponent"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "four Mountains paid the {{2}}{{R}}{{R}} and left nothing floating"
    );
}
