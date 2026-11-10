//! `cards/sorceries/mv_3/angelic_blessing.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Angelic Blessing — {2}{W} sorcery: "Target creature gets +3/+3 and gains
/// flying until end of turn."
///
/// Both halves of the sentence are read off one play, on a board built so that
/// neither can be mistaken for something else: the named 1/1 Elf becomes a
/// (4, 4) with flying, while a second Elf under the same seat and an Elf
/// across the table stay printed (1, 1)s with no evasion — "target creature"
/// is one creature, and the grant is no anthem. Then the turn ends, which is
/// the printed "until end of turn": the same Elf is a 1/1 on the ground again,
/// from a board where nothing else has changed.
#[test]
fn angelic_blessing_pumps_and_grants_flying_to_the_creature_it_names_only_this_turn() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[angelic_blessing()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which stays bare");
    let (host, bystander) = (elves[0], elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("an Elf across the table");
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "a printed 1/1 before the Blessing"
    );
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FLYING),
        "and nothing has granted it flying yet"
    );

    // Three Plains pay the {2}{W}; the mana is in the pool before the cast.
    cast_from_hand(&mut engine, p0, angelic_blessing());
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the Blessing targets a creature, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that cast it is the one that aims it");
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert!(
        options.contains(&host) && options.contains(&bystander) && options.contains(&theirs),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .expect("the Elf the question offered was chosen");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, host),
        (4, 4),
        "+3/+3 on the creature the spell named"
    );
    assert!(
        keywords(&engine, host).contains(KeywordSet::FLYING),
        "\"and gains flying\" reaches the same creature"
    );
    assert_eq!(
        pt(&engine, bystander),
        (1, 1),
        "the Elf beside it is still the 1/1 it was printed as"
    );
    assert!(
        !keywords(&engine, bystander).contains(KeywordSet::FLYING),
        "the grant reaches the target and no other"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and it never reaches across the table"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::FLYING),
        "\"target creature\" is not \"creatures\", let alone anyone else's"
    );

    // "Until end of turn" is the half no reading inside the turn can see: the
    // opponent's main phase is past p0's cleanup, where the effect expires.
    reach_their_main_phase(&mut engine, p1);
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "the pump lasted only the turn it was cast on"
    );
    assert!(
        !keywords(&engine, host).contains(KeywordSet::FLYING),
        "and the flying went with it"
    );
}
