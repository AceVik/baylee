//! `cards/lands/utility/griffin_canyon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "ba642c8b-9ade-4501-8393-672fd53d4955"

/// Griffin Canyon prints two lines — "{T}: Add {C}" and "{T}: Untap target
/// Griffin. If it's a creature, it gets +1/+1 until end of turn" — and only the
/// first of them can be played on this board, because no Griffin is reachable:
/// `card_index` reads a card by oracle id and the kit has no reader by subtype.
/// What the second line is held to instead is the filter it prints, which is the
/// half a widened one would lose — a Llanowar Elves stands there as a creature
/// the word "Griffin" has to decline.
///
/// Both lines are read off the *same* offer, which is what makes that a claim
/// and not a coincidence: the Canyon is untapped, so the `{T}` each of them
/// charges is payable, and the only thing the board lacks is a Griffin. The
/// `{C}` is then spent on a `{1}` artifact, so the pool reading is a payment and
/// not a number.
#[test]
fn griffin_canyon_taps_for_colorless_and_withholds_its_griffin_line_from_a_creature() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[llanowar_elves()])
        .hand(0, &[griffin_canyon(), quiet_artifact()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let canyon = play_land(&mut engine, p0, griffin_canyon());
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is on the board");
    assert!(!is_tapped(&engine, canyon), "a land enters untapped");

    let Pending::Priority { player, legal } = engine.pending().clone() else {
        panic!(
            "the main phase hands priority back, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the seat that played the Canyon holds priority");
    assert!(
        legal.abilities.contains(&(canyon, 0)),
        "an untapped land is a paid {{T}}, so \"{{T}}: Add {{C}}\" is offered \
         without a single mana floating: {:?}",
        legal.abilities
    );
    assert!(
        !legal.abilities.contains(&(canyon, 1)),
        "\"target Griffin\" has nothing to point at — the Llanowar Elves beside \
         it is a creature and no Griffin — so the line is withheld rather than \
         offered and then refused: {:?}",
        legal.abilities
    );

    // Ability 0 is the printed "{T}: Add {C}", and its whole price is its own
    // tap, so the mana is in the pool the moment the activation is applied
    // (CR 605.3b) with nothing on the stack behind it.
    activate(&mut engine, p0, griffin_canyon(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "{{T}}: Add {{C}} — one colourless, and nothing else on this board makes it"
    );
    assert_eq!(
        pool.available(ManaColor::Green),
        0,
        "the Elf was never tapped, so the green it prints is not in the pool"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so nothing is waiting to resolve"
    );
    assert!(is_tapped(&engine, canyon), "the Canyon paid its own {{T}}");
    assert!(
        !is_tapped(&engine, elf),
        "and the Elf beside it never moved"
    );

    // The other half of "this is mana": it pays for a spell that costs it.
    cast_with_floating(&mut engine, p0, quiet_artifact());
    pass_until(&mut engine, stack_is_empty);
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the {{C}} paid for a {{1}} artifact instead of sitting in the pool as a label"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and paying for it emptied the pool the Canyon filled"
    );
}

/// Griffin Canyon: "{T}: Untap target Griffin. If it's a creature, it gets
/// +1/+1 until end of turn." The Griffin is tapped first, so the untap is a
/// change; the Canyon itself taps for its cost.
#[test]
fn griffin_canyon_untaps_a_tapped_griffin_and_pumps_it() {
    let p0 = PlayerId::new(0);
    let canyon = card_index("ba642c8b-9ade-4501-8393-672fd53d4955");
    let griffin = card_index("c643cfe1-5844-4eb1-b1f5-028382411773");
    let mut engine = Duel::new(2101, forest())
        .battlefield(0, &[canyon, griffin])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let g = on_battlefield(&engine, p0, griffin).expect("griffin");
    let c = on_battlefield(&engine, p0, canyon).expect("canyon");
    unf_tap(&mut engine, p0, g);
    assert!(is_tapped(&engine, g));
    assert_eq!(pt(&engine, g), (2, 2));

    activate(&mut engine, p0, canyon, 1);
    unf_aim_and_pay(&mut engine, p0, Some(g), None);

    assert!(!is_tapped(&engine, g), "the Griffin untapped");
    assert_eq!(pt(&engine, g), (3, 3), "and got +1/+1");
    assert!(is_tapped(&engine, c), "the cost tapped the Canyon");
}
