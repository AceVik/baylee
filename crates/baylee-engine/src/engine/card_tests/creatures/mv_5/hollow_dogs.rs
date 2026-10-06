//! `cards/creatures/mv_5/hollow_dogs.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hollow Dogs prints one line: "Whenever this creature attacks, it gets
/// +2/+0 until end of turn." Reading the card file cannot tell that from a
/// board-wide pump, so the board carries two Dogs and only one attacks: the
/// attacker reads 5/3, the one that stayed home 3/3, and both were 3/3 before
/// combat — the printed power half up, the toughness half untouched, and
/// `Filter::This` read as the attacking Dog rather than every Dog in play.
#[test]
fn hollow_dogs_pumps_itself_when_it_attacks_and_never_the_dog_that_stayed_home() {
    fn hollow_dogs() -> CardIndex {
        card_index("16aa2e9e-bfe8-4b40-a1db-bb1da2ede849")
    }

    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[hollow_dogs(), hollow_dogs()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let dogs = all_on_battlefield(&engine, p0, hollow_dogs());
    assert_eq!(dogs.len(), 2, "two copies, one of which stays home");
    let (attacker, homebody) = (dogs[0], dogs[1]);
    assert_eq!(pt(&engine, attacker), (3, 3), "a printed 3/3 before combat");
    assert_eq!(
        pt(&engine, homebody),
        (3, 3),
        "and so is the one that stays home"
    );

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let Pending::ChooseAttackers {
        player, attackers, ..
    } = engine.pending().clone()
    else {
        unreachable!("the pass waited for exactly this")
    };
    assert_eq!(player, p0, "the active seat declares");
    assert!(
        attackers.contains(&attacker) && attackers.contains(&homebody),
        "both Dogs are untapped and unsick, so both are on the offer: {attackers:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p1))],
            },
        )
        .expect("the Dog came out of the list that offered it");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        pt(&engine, attacker),
        (5, 3),
        "\"Whenever this creature attacks, it gets +2/+0 until end of turn\" — \
         the power half up and the toughness half untouched"
    );
    assert_eq!(
        pt(&engine, homebody),
        (3, 3),
        "`Filter::This` is the Dog that attacked, not every Dog on the board"
    );
}
