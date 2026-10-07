//! `cards/lands/deserts/shefet_dunes.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Shefet Dunes prints `{T}: Add {C}`, `{T}, Pay 1 life: Add {W}`, and a
/// sorcery-speed `{2}{W}{W}, {T}, Sacrifice a Desert: Creatures you control
/// get +1/+1 until end of turn`, and one turn plays all three: both mana
/// abilities are pressed by index (the white one's price is not its own tap
/// alone, so `tap_all_mana` never presses it, #159) and what they make pays
/// for the pump. The load-bearing word in the cost is the *subtype* — the
/// menu has to be the Deserts this seat controls, the source among them and
/// the opponent's Desert not — and the Elf across the table is what separates
/// "creatures you control" from "all creatures".
#[test]
#[allow(clippy::too_many_lines)] // one land, played through every clause it prints
fn shefet_dunes_taps_for_its_two_mana_and_pumps_the_team_for_a_desert() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                shefet_dunes(),
                shefet_dunes(),
                shefet_dunes(),
                llanowar_elves(),
                llanowar_elves(),
            ],
        )
        // A Desert on the other side of the table: "sacrifice a Desert" is
        // not an invitation to eat somebody else's.
        .battlefield(1, &[llanowar_elves(), desert()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // Ability 0: "{T}: Add {C}". A Desert has no basic land type, so this is
    // the land's own printed mana ability rather than a CR 305.6 shortcut,
    // and it lands with nothing on the stack (CR 605.3b).
    activate(&mut engine, p0, shefet_dunes(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Colorless), 1, "{{T}}: Add {{C}}");
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert_eq!(
        pool.available(ManaColor::White),
        0,
        "the {{C}} line is not the white one"
    );
    assert!(stack_is_empty(&engine), "a mana ability uses no stack");

    // Ability 1: "{T}, Pay 1 life: Add {W}". Its price is not its own tap, so
    // it is activated by index and the life is read as a difference.
    let life = engine.state().players[0].life;
    activate(&mut engine, p0, shefet_dunes(), 1);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "one white, for one life"
    );
    assert_eq!(
        pool.available(ManaColor::Colorless),
        1,
        "beside the {{C}} already floating"
    );
    assert_eq!(
        engine.state().players[0].life,
        life - 1,
        "and the life is gone"
    );
    assert!(stack_is_empty(&engine), "still a mana ability");

    // The {2}{W}{W} the pump costs, off the Plains and the Elves. The Dunes
    // are kept back: every one of them is a potential {T}, and one has to be
    // standing when the pump is pressed.
    tap_all_mana_but(&mut engine, p0, Some(shefet_dunes()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "{{C}} and {{W}} off the two Dunes, two more white off the Plains and \
         two green off the Elves"
    );

    let dunes = all_on_battlefield(&engine, p0, shefet_dunes());
    assert_eq!(
        dunes.len(),
        3,
        "three Deserts, and one stays to be the source"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    let (source, index) = legal
        .abilities
        .iter()
        .copied()
        .find(|(id, ability_index)| {
            *ability_index == 2
                && engine
                    .state()
                    .object(*id)
                    .is_some_and(|o| o.card.is_some_and(|c| c.index == shefet_dunes()))
        })
        .expect("with {2}{W}{W} in the pool the pump is offered");
    assert!(
        !is_tapped(&engine, source),
        "and it is the Dune the two mana abilities left standing"
    );
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source,
                ability_index: index,
            },
        )
        .expect("the pump activates");

    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which Desert, got {:?}", engine.pending())
    };
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostSacrifice,
        "a cost and not a search, which is all a client has to tell the two apart"
    );
    assert_eq!((min, max), (1, 1), "one Desert, and the cost asks once");
    for land in &dunes {
        assert!(
            options.contains(land),
            "every Desert this seat controls is on the menu: {options:?}"
        );
    }
    assert!(
        options.contains(&source),
        "\"a Desert\", with no \"another\": the land paying its own {{T}} is on its own menu"
    );
    assert_eq!(
        options.len(),
        3,
        "and the three Deserts are the whole menu: {options:?}"
    );
    let theirs = on_battlefield(&engine, p1, desert()).expect("their Desert is on the table");
    assert!(
        !options.contains(&theirs),
        "a seat sacrifices only what it controls, whatever the filter says: \
         {options:?}"
    );
    let mine = on_battlefield(&engine, p0, plains()).expect("a Plains is out");
    assert!(
        !options.contains(&mine),
        "a Plains carries no Desert subtype"
    );
    let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("an Elf is out");
    assert!(!options.contains(&elf), "and a creature is none either");

    let fodder = *dunes
        .iter()
        .find(|id| **id != source)
        .expect("a second Desert to pay with");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .expect("the Desert the question offered pays the cost");

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(fodder).map(|o| o.zone),
        Some(crate::zone::Zone::Graveyard),
        "a sacrificed Desert goes to its owner's graveyard"
    );
    assert_eq!(
        all_on_battlefield(&engine, p0, shefet_dunes()).len(),
        2,
        "and leaves the battlefield"
    );
    assert!(
        is_tapped(&engine, source),
        "tapping the land paid the other half of the cost"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        2,
        "the {{2}}{{W}}{{W}} came out of the pool"
    );

    let elves = all_on_battlefield(&engine, p0, llanowar_elves());
    assert_eq!(elves.len(), 2, "both Elves are still on the table");
    for elf in &elves {
        assert_eq!(
            pt(&engine, *elf),
            (2, 2),
            "\"creatures you control get +1/+1 until end of turn\""
        );
    }
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf is out");
    assert_eq!(
        pt(&engine, their_elf),
        (1, 1),
        "and nothing at all across the table"
    );
}
