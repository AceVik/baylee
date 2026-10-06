//! `cards/creatures/mv_2/anaba_ancestor.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Anaba Ancestor — {1}{R}, a 1/1 Minotaur Spirit printing "{T}: Another
/// target Minotaur creature gets +1/+1 until end of turn."
///
/// The Ancestor is itself a Minotaur, so "another" and "Minotaur" are two
/// questions one board has to answer, and each gets its counter-half: a second
/// Ancestor cast off two Mountains — which is also the only tangible reading of
/// "{1}{R}" — an Elf that is a creature and no Minotaur, and the copy standing
/// since the start, the only one of the two that may tap because the newcomer
/// has summoning sickness (CR 302.6). With two Minotaurs on the table an offer
/// holding exactly one option is "another" doing the work, the Elf rules out a
/// filter that asked only for another creature, and the source — a Minotaur
/// itself — rules out one that forgot its own subtype. The pump is read after
/// the target is named, because the tap is paid last (CR 601.2c, then 601.2h).
#[test]
fn anaba_ancestor_pumps_another_minotaur_and_never_itself_or_a_non_minotaur() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[anaba_ancestor(), llanowar_elves(), mountain(), mountain()],
        )
        .hand(0, &[anaba_ancestor()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{R} off the two Mountains. The Elf is named as the source to keep
    // back so that it is still untapped and still a creature when the offer
    // below is read.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, anaba_ancestor());
    pass_until(&mut engine, stack_is_empty);

    let ancestors = all_on_battlefield(&engine, p0, anaba_ancestor());
    assert_eq!(ancestors.len(), 2, "the cast copy landed beside the first");
    assert_eq!(
        pt(&engine, ancestors[0]),
        (1, 1),
        "the Ancestor prints a 1/1"
    );
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert_eq!(pt(&engine, elves), (1, 1), "and the Elf is no Minotaur");

    // Only the copy that has been here since the start may tap; `activate`
    // takes the ability out of the offer rather than guessing at it.
    activate(&mut engine, p0, anaba_ancestor(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!("the pump targets a creature, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the seat that activated chooses");
    assert_eq!(
        options.len(),
        1,
        "two Minotaurs stand here, so an offer of one is `another` doing the \
         work: {options:?}"
    );
    let target = options[0];
    assert!(
        ancestors.contains(&target),
        "and the one offered is a Minotaur: {options:?}"
    );
    assert!(
        !options.contains(&elves),
        "a creature that is no Minotaur is not offered, so `Minotaur` is read \
         and not merely `another`: {options:?}"
    );
    let source = *ancestors
        .iter()
        .find(|id| **id != target)
        .expect("the target is one of the two, so the other is the source");
    assert!(
        !options.contains(&source),
        "the source is a Minotaur itself and it is not in its own offer: {options:?}"
    );
    assert!(
        !is_tapped(&engine, source),
        "the target is named first, so the {{T}} is still unpaid (CR 601.2c)"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    assert!(
        is_tapped(&engine, source),
        "{{T}} is the whole price, and it is paid last (CR 601.2h)"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, target),
        (2, 2),
        "+1/+1 until end of turn on the Minotaur it named"
    );
    assert_eq!(
        pt(&engine, source),
        (1, 1),
        "the source is a Minotaur with nothing on it: `another` reached it and stopped"
    );
    assert_eq!(pt(&engine, elves), (1, 1), "and the Elf is untouched");
}
