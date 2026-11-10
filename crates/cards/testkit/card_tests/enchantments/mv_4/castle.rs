//! `cards/enchantments/mv_4/castle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Castle — {3}{W} enchantment: "Untapped creatures you control get +0/+2."
///
/// Two words in that sentence carry the card and each needs its own witness on
/// one board: an Elf of mine that has tapped for mana reads the printed 1/1
/// while its standing twin reads 1/3, which is `Untapped`; and an untapped Elf
/// across the table stays a printed 1/1, which is `you control`. Both readings
/// come off a single cast — four Plains and one Elf pay the {3}{W}, with the
/// other Elf named as the source kept back, because the creature the static is
/// about must not be tapped by the helper that pays for it. Tapping the pumped
/// Elf afterwards shows the bonus is read off the live status rather than
/// frozen at the moment the Castle arrived.
#[test]
fn castle_pumps_the_untapped_creatures_its_controller_has_and_no_other() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[castle()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let my_elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(
        my_elves.len(),
        2,
        "two Elves of mine, one of which is about to tap"
    );
    let (standing, drinking) = (my_elves[0], my_elves[1]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        pt(&engine, standing),
        (1, 1),
        "a printed 1/1 before the Castle"
    );

    // `can_afford` reads the pool and not the untapped lands, so nothing is
    // castable until the mana is actually floating — the control for the
    // payment below.
    let card = in_hand(&engine, p0, castle()).expect("the Castle is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        !legal.castable.contains(&card),
        "an empty pool pays no {{3}}{{W}}: {:?}",
        legal.castable
    );

    // Four Plains and one Elf are exactly the {3}{W}, and the standing Elf is
    // named as the thing kept back: it is the creature the static is about, and
    // `tap_all_mana` would have taken its own `{T}: Add {G}` as well (#159).
    tap_mana_except(&mut engine, p0, standing);
    assert!(
        is_tapped(&engine, drinking),
        "the Elf nobody kept back paid with its own tap"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "four Plains and one Elf, which is what a four-mana enchantment asks"
    );
    cast_with_floating(&mut engine, p0, castle());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, castle()).is_some(),
        "the Castle resolved onto the battlefield"
    );

    assert_eq!(
        pt(&engine, standing),
        (1, 3),
        "\"Untapped creatures you control get +0/+2\": an untapped 1/1 is a 1/3"
    );
    assert_eq!(
        pt(&engine, drinking),
        (1, 1),
        "the Elf that tapped for its own mana is tapped, and the static has \
         left it exactly as it was printed"
    );
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "\"you control\" reaches the seat that cast the Castle and never \
         across the table"
    );

    // The status is read live rather than snapshotted when the Castle arrived:
    // tapping the pumped Elf takes the +0/+2 away with it.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, ability_index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, _)| *id == standing)
        .expect("the standing Elf's own mana ability is offered, the tapped one's is not");
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            },
        )
        .expect("its whole price is its own tap");
    assert!(is_tapped(&engine, standing), "the Elf tapped for mana");
    assert_eq!(
        pt(&engine, standing),
        (1, 1),
        "so the +0/+2 went with the untapped status it depended on"
    );
}
