//! `cards/creatures/mv_3/lieutenant_kirtar.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Lieutenant Kirtar is a 2/2 flier whose whole printed text is one
/// sacrifice: "{1}{W}, Sacrifice Lieutenant Kirtar: Exile target attacking
/// creature."
///
/// The scenario plays that sentence at the only moment it exists — inside the
/// opponent's combat, after attackers are declared — and reads the word
/// "attacking" off the menu: one of p1's two identical Elves was sent and the
/// other stayed home, so an offer holding exactly the first is the filter and
/// not a board buff. Both prices are read where CR 601.2h puts them, after the
/// target is answered, and the exile is read as a zone rather than as an
/// absence, because the Elf leaving the battlefield would look the same.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn lieutenant_kirtar_sacrifices_itself_to_exile_the_creature_that_attacked() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[lieutenant_kirtar(), plains(), plains()])
        .battlefield(1, &[llanowar_elves(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);

    // p1's attack: one Elf is sent across the table, the other stays home.
    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    let elves = all_on_battlefield(&engine, p1, llanowar_elves());
    assert_eq!(elves.len(), 2, "two Elves, one of which is sent");
    let (attacker, home) = (elves[0], elves[1]);
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(attacker, Defender::Player(p0))],
            },
        )
        .unwrap();

    // Walk the combat step until the defending seat holds priority — the only
    // window the printed sentence can be used in — answering the block
    // declaration with nothing on the way.
    let mut offered = false;
    for _ in 0..20 {
        match engine.pending().clone() {
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            Pending::Priority { player, .. } if player == p0 => {
                offered = true;
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Elf attacks: {other:?}"),
        }
    }
    assert!(offered, "the defender is offered priority inside combat");

    let kirtar = on_battlefield(&engine, p0, lieutenant_kirtar()).expect("the Bird is out");
    assert!(
        keywords(&engine, kirtar).contains(KeywordSet::FLYING),
        "the printed flying line reaches the permanent"
    );
    assert_eq!(pt(&engine, kirtar), (2, 2), "the body the card prints");

    // The offer is read off the *pool*, so the two Plains are tapped first.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "two Plains, and the Bird makes no mana of its own"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(kirtar, 0)),
        "with {{1}}{{W}} floating the one line the card prints is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, lieutenant_kirtar(), 0);
    let Pending::ChooseTargets {
        player, options, ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target attacking creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert!(
        options.contains(&attacker),
        "the Elf that attacked is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&home),
        "and the Elf that stayed home is not: {options:?}"
    );
    assert!(
        on_battlefield(&engine, p0, lieutenant_kirtar()).is_some(),
        "CR 601.2h pays last, so the Bird is still on the battlefield while \
         the target question stands"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "and the {{1}}{{W}} is still in the pool for the same reason"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![attacker],
            },
        )
        .expect("the attacking Elf was one of the options it enumerated");

    assert!(
        in_graveyard(&engine, p0, lieutenant_kirtar()).is_some(),
        "sacrificing the Bird is the other half of the price"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{1}}{{W}} came out of the pool"
    );
    assert!(!stack_is_empty(&engine), "exiling is no mana ability");

    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the Elf that did not attack is still standing"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Exile(p1)).len(),
        1,
        "exile is where the card went, which a battlefield count alone would \
         not have told apart from a graveyard"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Exile(p1))
            .contains(&attacker),
        "and the exiled card is the creature that was named"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_none(),
        "\"exile\": nothing of the Elf reached a graveyard"
    );
}
