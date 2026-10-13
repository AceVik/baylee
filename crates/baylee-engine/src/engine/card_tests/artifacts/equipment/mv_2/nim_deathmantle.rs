//! `cards/artifacts/equipment/mv_2/nim_deathmantle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::index;

/// Equipping pays the real cost and changes only the attached creature.
#[test]
fn nim_deathmantle_equips_for_four_and_clamps_only_the_creature_it_holds() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(41, forest())
        .battlefield(
            0,
            &[
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[nim_deathmantle()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let host = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    assert_eq!(pt(&engine, host), (1, 1), "nothing is equipped yet");

    // The artifact has to arrive, not merely be believed in: six tapped
    // Forests pay the {2} and leave exactly the {4} the equip asks for. The
    // Elf is kept back so that "exactly" is the six Forests and not seven
    // sources — it is the creature the Equipment is about to hold.
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_with_floating(&mut engine, p0, nim_deathmantle());
    pass_until(&mut engine, stack_is_empty);
    let mantle = on_battlefield(&engine, p0, nim_deathmantle()).expect("the Deathmantle resolved");
    assert!(
        engine
            .state()
            .object(mantle)
            .is_some_and(|o| o.attached_to.is_none()),
        "an Equipment enters holding nobody"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the {{2}} is spent and the {{4}} the equip will charge is still in the pool"
    );
    assert_eq!(
        pt(&engine, host),
        (1, 1),
        "an Equipment attached to nothing modifies nothing"
    );

    // Equip {4}: ability 3 on the card, behind the three statics that do the
    // granting. That it is offered at all is the pool reading above.
    activate(&mut engine, p0, nim_deathmantle(), 3);
    let Pending::ChooseTargets { options, .. } = engine.pending().clone() else {
        panic!(
            "equip targets a creature you control, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        options,
        vec![host],
        "target creature *you* control — the Elves across the table are not offered"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![host],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        e.state()
            .object(mantle)
            .is_some_and(|o| o.attached_to == Some(host))
    });

    assert_eq!(
        pt(&engine, host),
        (3, 3),
        "+2/+2 for the creature the Equipment holds"
    );
    let host_chars = engine.state().object(host).unwrap().characteristics();
    assert_eq!(host_chars.colors, ColorSet::from_slice(&[Color::Black]));
    assert!(
        host_chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::ZOMBIE)
    );
    assert!(
        !host_chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::ELF)
    );
    assert!(host_chars.keywords.contains(KeywordSet::INTIMIDATE));
    assert_eq!(
        pt(&engine, theirs),
        (1, 1),
        "and nothing at all for a creature it does not hold"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the equip's {{4}} came out of the pool"
    );
}

/// p0 holds the Deathmantle, an Elf and four Forests, and has tapped the Forests:
/// the {4} the trigger asks for is floating, so any refusal below is the
/// trigger's filter and not a shortage of mana.
fn deathmantle_table(theirs: &[CardIndex]) -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                nim_deathmantle(),
                llanowar_elves(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the four Forests are floating the {{4}} the trigger would charge"
    );
    engine
}

/// Destroys `victim` through the harness door and gives priority one pass, which
/// is when a trigger of that death is put on the stack.
#[track_caller]
fn destroy_and_pass(engine: &mut Engine<RegistryLookup>, victim: ObjectId) {
    crate::sba::destroy(engine.dev_state_mut(PlayerId::new(0)).unwrap(), victim);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    engine.apply(player, PlayerAction::PassPriority).unwrap();
}

/// Passes priority to the end of the turn and fails if anyone is ever asked a
/// yes/no question ("you may pay {4}") on the way.
#[track_caller]
fn pass_to_turns_end_asking_nothing(engine: &mut Engine<RegistryLookup>) {
    let turn = engine.state().turn.number;
    for _ in 0..80 {
        if engine.state().turn.number != turn {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            other => panic!("nothing should have been asked, got {other:?}"),
        }
    }
    panic!("the turn never ended");
}

/// "Whenever a nontoken creature is put into **your** graveyard from the
/// battlefield": an opponent's creature going to the opponent's graveyard asks
/// nothing, even with the {4} floating.
#[test]
fn nim_deathmantle_ignores_a_creature_that_goes_to_an_opponents_graveyard() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = deathmantle_table(&[llanowar_elves()]);
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elves are out");
    let mantle = on_battlefield(&engine, p0, nim_deathmantle()).unwrap();

    destroy_and_pass(&mut engine, theirs);
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the Elves died into their owner's graveyard"
    );
    assert!(
        stack_is_empty(&engine),
        "\"into *your* graveyard\": a death into the opponent's puts no Deathmantle trigger on the stack"
    );
    assert!(
        !matches!(engine.pending(), Pending::YesNo { .. }),
        "and nobody is asked to pay {{4}}: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the floating {{4}} is untouched"
    );
    pass_to_turns_end_asking_nothing(&mut engine);
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "their Elves stay in their graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none()
            && on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and nothing came back to the battlefield under either seat but p0's untouched Elves"
    );
    assert!(
        engine
            .state()
            .object(mantle)
            .is_some_and(|o| o.attached_to.is_none()),
        "the Deathmantle holds nobody"
    );
}

/// "Whenever a **nontoken** creature": a token of yours that dies asks nothing.
#[test]
fn nim_deathmantle_ignores_a_token_of_yours_that_dies() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[nim_deathmantle(), plains(), plains()])
        .hand(0, &[index::RAISE_THE_ALARM])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, index::RAISE_THE_ALARM);
    pass_until(&mut engine, stack_is_empty);
    let tokens = tokens_of(&engine, p0);
    assert_eq!(tokens.len(), 2, "Raise the Alarm made its two Soldiers");
    let mantle = on_battlefield(&engine, p0, nim_deathmantle()).unwrap();
    engine.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Colorless, 4);
    engine.refresh_offer();

    destroy_and_pass(&mut engine, tokens[0]);
    assert!(
        stack_is_empty(&engine),
        "a token is no \"nontoken creature\": its death puts no Deathmantle trigger on the stack"
    );
    assert!(
        !matches!(engine.pending(), Pending::YesNo { .. }),
        "and nobody is asked to pay {{4}}: {:?}",
        engine.pending()
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the floating {{4}} is untouched"
    );
    pass_to_turns_end_asking_nothing(&mut engine);
    assert!(
        engine
            .state()
            .object(mantle)
            .is_some_and(|o| o.attached_to.is_none()),
        "the Deathmantle holds nobody"
    );
}

/// "You *may* pay {4}. If you do, ...": declining leaves the card in the
/// graveyard, the {4} unspent and the Equipment unattached.
#[test]
fn nim_deathmantle_declined_payment_returns_nothing_and_spends_nothing() {
    let p0 = PlayerId::new(0);
    let mut engine = deathmantle_table(&[]);
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("my Elves are out");
    let mantle = on_battlefield(&engine, p0, nim_deathmantle()).unwrap();

    destroy_and_pass(&mut engine, elves);
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Stack).len(),
        1,
        "my own nontoken creature dying puts the trigger on the stack"
    );
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "declined: the Elves stay in the graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_none(),
        "declined: nothing returned to the battlefield"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "declined: the {{4}} was not spent"
    );
    assert!(
        engine
            .state()
            .object(mantle)
            .is_some_and(|o| o.attached_to.is_none()),
        "declined: the Equipment attached to nothing"
    );
}
