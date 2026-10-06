//! `cards/creatures/mv_3/battle_rampart.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Battle Rampart prints two things: "Defender" on a 1/3 Wall and
/// "`{T}`: Target creature gains haste until end of turn." Both are read on
/// a creature that is not the source — the ability costs only its own tap
/// symbol and is therefore not even tapped by `tap_all_mana` —, and "target
/// creature" is exactly that: the offer includes its own *and* the Elves on
/// the table, the chosen one gets HASTE and the unchosen ones do not. The
/// Wall itself may not attack a turn later, while the Elf next
/// to it is in the same attack offer.
#[test]
fn battle_rampart_grants_haste_to_the_creature_it_targets_and_never_attacks_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), mountain(), llanowar_elves()])
        .hand(0, &[battle_rampart()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elf is out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::HASTE),
        "nothing has granted haste yet"
    );

    // {2}{R} from the three Mountains; the Elf is named as the source that
    // remains standing, because it is the creature in question below.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, battle_rampart());
    pass_until(&mut engine, stack_is_empty);
    let rampart = on_battlefield(&engine, p0, battle_rampart()).expect("the Rampart resolved");
    assert_eq!(pt(&engine, rampart), (1, 3), "the printed 1/3 body");
    assert!(
        keywords(&engine, rampart).contains(KeywordSet::DEFENDER),
        "and the printed defender keyword reaches the permanent"
    );

    // The whole cost of the ability is its own {T}, so no mana: it is
    // offered even though the mana pool is empty.
    // CR 302.6: a creature that arrived this turn cannot pay a `{T}`, so
    // the turn goes round once before the line is pressed. Both halves are
    // needed — `walk_to_own_main` on its own returns where it stands,
    // because this already *is* p0's own main phase.
    reach_their_main_phase(&mut engine, p1);
    assert!(
        walk_to_own_main(&mut engine, p0),
        "the turn came back round, and the summoning sickness with it"
    );
    activate(&mut engine, p0, battle_rampart(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that activated aims it");
    assert!(
        options.contains(&mine) && options.contains(&theirs),
        "\"target creature\" reaches across the table and not only your own side: {options:?}"
    );
    assert!(
        options.contains(&rampart),
        "and \"target creature\" is not \"another target creature\": the source is a creature: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![theirs],
            },
        )
        .expect("the Elf across the table was one of the options");
    assert!(
        is_tapped(&engine, rampart),
        "{{T}} was the price and is paid"
    );
    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, theirs).contains(KeywordSet::HASTE),
        "the chosen creature has haste until end of turn"
    );
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::HASTE),
        "the creature that no one chose has no haste"
    );
    assert!(
        !keywords(&engine, rampart).contains(KeywordSet::HASTE),
        "and the Wall that distributes it doesn't keep it"
    );

    // One turn later, so that summoning sickness no longer excludes the Wall:
    // then defender alone keeps it from being declared an attacker.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers { attackers, .. } = engine.pending().clone() else {
        unreachable!("pass_until stops on nothing but the attack declaration")
    };
    assert!(
        attackers.contains(&mine),
        "an untapped 1/1 without its own text may attack: {attackers:?}"
    );
    assert!(
        !attackers.contains(&rampart),
        "the 1/3 Wall may not — that is the printed defender: {attackers:?}"
    );
}
