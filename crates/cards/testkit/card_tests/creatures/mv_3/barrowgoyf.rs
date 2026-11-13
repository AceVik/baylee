//! `cards/creatures/mv_3/barrowgoyf.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn barrowgoyf() -> CardIndex {
    card_index("74c0164c-130f-4572-9066-626c77f6e2ff")
}

#[track_caller]
fn refresh(e: &mut Engine<RegistryLookup>) {
    e.dev_state_mut(P0)
        .expect("the harness may refresh after preparing a graveyard")
        .refresh_characteristics();
}

/// Creature cards count once even across both graveyards; a land adds a type.
#[test]
fn barrowgoyf_defines_power_and_toughness_from_all_graveyards() {
    let mut e = Duel::new(SEED, steadfast_guard())
        .battlefield(0, &[barrowgoyf(), mountain()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let goyf = on_battlefield(&e, P0, barrowgoyf()).unwrap();
    assert_eq!(pt(&e, goyf), (0, 1));

    seed_graveyard(&mut e, P1, 1);
    refresh(&mut e);
    assert_eq!(pt(&e, goyf), (1, 2), "an opposing creature card");
    seed_graveyard(&mut e, P0, 1);
    refresh(&mut e);
    assert_eq!(pt(&e, goyf), (1, 2), "still only one card type");

    let land = on_battlefield(&e, P0, mountain()).unwrap();
    bury(&mut e, &[land]);
    refresh(&mut e);
    assert_eq!(pt(&e, goyf), (2, 3), "creature and land");
}

/// Two power is lethal to the 2/4 Spider with deathtouch, and lifelink gains
/// the same two even though no player was dealt combat damage.
#[test]
fn barrowgoyf_plays_deathtouch_and_lifelink_in_combat() {
    let mut e = Duel::new(SEED, steadfast_guard())
        .battlefield(0, &[barrowgoyf()])
        .battlefield(1, &[giant_spider(), mountain()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    seed_graveyard(&mut e, P1, 1);
    let land = on_battlefield(&e, P1, mountain()).unwrap();
    bury(&mut e, &[land]);
    refresh(&mut e);
    let goyf = on_battlefield(&e, P0, barrowgoyf()).unwrap();
    let spider = on_battlefield(&e, P1, giant_spider()).unwrap();
    assert_eq!(pt(&e, goyf), (2, 3));
    let words = keywords(&e, goyf);
    assert!(words.contains(KeywordSet::DEATHTOUCH));
    assert!(words.contains(KeywordSet::LIFELINK));

    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(goyf, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(spider, goyf)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });

    assert!(in_graveyard(&e, P1, giant_spider()).is_some());
    assert_eq!(e.state().object(goyf).unwrap().damage, 2);
    assert_eq!(e.state().players[0].life, 22, "two lifelink damage");
    assert_eq!(e.state().players[1].life, 20);
}

fn mill_board() -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, steadfast_guard())
        .battlefield(0, &[barrowgoyf()])
        .battlefield(1, &[mountain()])
        .hand(0, &[llanowar_elves(), lightning_bolt(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    seed_graveyard(&mut e, P1, 1);
    let land = on_battlefield(&e, P1, mountain()).unwrap();
    bury(&mut e, &[land]);
    refresh(&mut e);
    let goyf = on_battlefield(&e, P0, barrowgoyf()).unwrap();
    assert_eq!(
        pt(&e, goyf),
        (2, 3),
        "fixture starts with creature and land"
    );
    e
}

#[track_caller]
fn connect(e: &mut Engine<RegistryLookup>) {
    let goyf = on_battlefield(e, P0, barrowgoyf()).unwrap();
    assert_eq!(pt(e, goyf), (2, 3));
    pass_until(e, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(goyf, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(e, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                player: P0,
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
}

/// "That many" is the two combat damage, frozen on the trigger. Yes mills the
/// Elves and Bolt (not the third card), and only the creature may go to hand.
#[test]
fn barrowgoyf_mills_the_damage_and_may_return_a_milled_creature() {
    let mut e = mill_board();
    let forest = hand_to_library_top(&mut e, P0, forest());
    let bolt = hand_to_library_top(&mut e, P0, lightning_bolt());
    let elf = hand_to_library_top(&mut e, P0, llanowar_elves());
    let library = e.state().zones.list(ZoneLocation::Library(P0)).len();
    connect(&mut e);
    e.apply(P0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        options, min, max, ..
    } = e.pending().clone()
    else {
        unreachable!()
    };
    assert_eq!(options, vec![elf], "only the milled creature card");
    assert_eq!((min, max), (0, 1));
    assert_eq!(e.state().object(bolt).unwrap().zone, Zone::Graveyard);
    assert_eq!(e.state().object(forest).unwrap().zone, Zone::Library);
    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(P0)).len(),
        library - 2
    );
    e.apply(P0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(e.state().object(elf).unwrap().zone, Zone::Hand);
    assert_eq!(e.state().players[1].life, 18, "two combat damage");
    assert_eq!(e.state().players[0].life, 22, "two lifelink damage");
}

/// Declining the first "may" does not mill and therefore offers no return.
#[test]
fn barrowgoyf_declining_the_mill_changes_no_cards() {
    let mut e = mill_board();
    hand_to_library_top(&mut e, P0, forest());
    hand_to_library_top(&mut e, P0, lightning_bolt());
    hand_to_library_top(&mut e, P0, llanowar_elves());
    let library = e.state().zones.list(ZoneLocation::Library(P0)).len();
    let hand = e.state().zones.list(ZoneLocation::Hand(P0)).len();
    connect(&mut e);
    e.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        e.state().zones.list(ZoneLocation::Library(P0)).len(),
        library
    );
    assert_eq!(e.state().zones.list(ZoneLocation::Hand(P0)).len(), hand);
    assert_eq!(e.state().players[1].life, 18);
    assert_eq!(e.state().players[0].life, 22);
}
