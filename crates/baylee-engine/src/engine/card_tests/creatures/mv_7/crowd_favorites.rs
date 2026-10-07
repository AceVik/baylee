//! `cards/creatures/mv_7/crowd_favorites.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "1ead750f-14a6-4f25-9eb8-9472c2fdac35"

/// Crowd Favorites — {6}{W}, a 4/4 Human Soldier printing two activated
/// abilities that both cost {3}{W}: "Tap target creature" and "This creature
/// gets +0/+5 until end of turn".
///
/// The pair needs one board and two different witnesses, so the Soldier is cast
/// for real and both printed lines are then played off the mana that is left.
/// "Target creature" is any creature, which is why the Elf across the table and
/// the Soldier itself are both on the menu — and the tap has to actually land
/// on the Elf, since an untapped one at the end would leave the first line
/// unread. The pump takes the creature that prints it, so a (4, 9) beside an
/// untouched (1, 1) is the only pair of numbers that tells a self-pump from a
/// board-wide one. Fifteen Plains pay the arrival and then both {3}{W} lines
/// inside a single main phase (CR 500.5), and the pool is read after each
/// activation so that "four and then four" is a claim about the card rather
/// than about mana the board happened to be holding.
#[test]
fn crowd_favorites_taps_a_creature_across_the_table_and_pumps_itself_for_four_each() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(); 15])
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[crowd_favorites()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert!(!is_tapped(&engine, elf), "nothing has tapped the Elf yet");

    // The card arrives for its printed {6}{W}, and every source on the board is
    // spent doing it: whatever is left in the pool afterwards is the two
    // {3}{W} lines and nothing else.
    cast_from_hand(&mut engine, p0, crowd_favorites());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, crowd_favorites()).is_some()
    });
    let crowd = on_battlefield(&engine, p0, crowd_favorites()).expect("the Soldier resolved");
    assert_eq!(pt(&engine, crowd), (4, 4), "the body the card prints");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "fifteen Plains less the {{6}}{{W}} the creature cost to arrive"
    );

    // Both lines cost mana, and `legal.abilities` is filtered through
    // `can_afford`, which reads the pool and not the untapped lands — so the
    // offer is claimed with the eight already floating.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(crowd, 0)),
        "{{3}}{{W}}: Tap target creature — offered with the mana floating: {:?}",
        legal.abilities
    );
    assert!(
        legal.abilities.contains(&(crowd, 1)),
        "and the pump, which charges the same four: {:?}",
        legal.abilities
    );

    // Ability 0, the tap. CR 601.2c names the target before CR 601.2h pays, so
    // the mana is still in the pool and the Elf is still standing.
    activate(&mut engine, p0, crowd_favorites(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&elf) && options.contains(&crowd),
        "\"target creature\" is any creature, on either side of the table: {options:?}"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        8,
        "nothing is spent while the question stands"
    );
    assert!(
        !is_tapped(&engine, elf),
        "and nothing is tapped yet either — the target is chosen before the cost"
    );

    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf was one of the options the question enumerated");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the {{3}}{{W}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "tapping a creature is no mana ability, so the ability is on the stack"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, elf), "\"Tap target creature\"");
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "the Elf is tapped and otherwise untouched"
    );

    // Ability 1, the pump on the creature that printed it. Four mana is still
    // floating in the same main phase, so the second line is payable too.
    activate(&mut engine, p0, crowd_favorites(), 1);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the second {{3}}{{W}} is the rest of the fifteen Plains"
    );
    assert_eq!(
        pt(&engine, crowd),
        (4, 9),
        "\"this creature gets +0/+5\": power untouched, toughness up five"
    );
    assert_eq!(
        pt(&engine, elf),
        (1, 1),
        "the pump reaches the Soldier and never the creature the other line tapped"
    );
}
