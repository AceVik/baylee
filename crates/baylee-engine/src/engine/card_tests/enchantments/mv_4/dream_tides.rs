//! `cards/enchantments/mv_4/dream_tides.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn dream_tides() -> CardIndex {
    card_index("d9583aab-3eff-4d88-84d5-7219ab982636")
}

/// Taps `ids` behind the engine's back and rebuilds the projections.
fn tap(engine: &mut Engine<RegistryLookup>, ids: &[ObjectId]) {
    for &id in ids {
        engine.state.set_tapped(id, true);
    }
    resync(engine);
}

/// Passes until `seat` is asked which tapped creatures to untap, and returns the menu.
#[track_caller]
fn walk_to_untap_question(engine: &mut Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::ChooseCards {
                prompt: ChoicePrompt::Untap,
                ..
            }
        )
    });
    let Pending::ChooseCards {
        player, options, ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, seat, "the active player is asked");
    options
}

/// Dream Tides ({2}{U}{U}, enchantment), both printed sentences:
/// - "Creatures don't untap during their controllers' untap steps."
/// - "At the beginning of each player's upkeep, that player may choose any number of tapped
///   nongreen creatures they control and pay {2} for each creature chosen this way. If the
///   player does, untap those creatures."
///
/// The opponent's upkeep: its tapped nongreen creature stayed tapped through its untap step and
/// is offered, its tapped green creature stayed tapped and is not; declining untaps nothing.
#[test]
fn dream_tides_keeps_creatures_tapped_and_offers_only_the_nongreen_one_on_each_upkeep() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                dream_tides(),
                gray_ogre(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .battlefield(1, &[grizzly_bears(), air_elemental()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let p1_bears = on_battlefield(&engine, P1, grizzly_bears()).unwrap();
    let p1_air = on_battlefield(&engine, P1, air_elemental()).unwrap();
    tap(&mut engine, &[p1_bears, p1_air]);

    let menu = walk_to_untap_question(&mut engine, P1);
    assert!(
        is_tapped(&engine, p1_bears) && is_tapped(&engine, p1_air),
        "no creature untapped in its controller's untap step"
    );
    assert_eq!(
        menu,
        vec![p1_air],
        "only the tapped nongreen creature is offered"
    );
    assert!(
        engine
            .apply(
                P1,
                PlayerAction::ChooseObjects {
                    objects: vec![p1_bears]
                }
            )
            .is_err(),
        "a green creature cannot be chosen"
    );
    engine
        .apply(P1, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_tapped(&engine, p1_air) && is_tapped(&engine, p1_bears),
        "choosing nothing untaps nothing"
    );
}

/// The active player pays {2} for the one creature chosen: it untaps, the creature it did not
/// choose and the green creature stay tapped, and the lands are spent.
#[test]
fn dream_tides_untaps_the_one_chosen_nongreen_creature_for_two_mana() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                dream_tides(),
                gray_ogre(),
                air_elemental(),
                llanowar_elves(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let ogre = on_battlefield(&engine, P0, gray_ogre()).unwrap();
    let air = on_battlefield(&engine, P0, air_elemental()).unwrap();
    let elves = on_battlefield(&engine, P0, llanowar_elves()).unwrap();
    tap(&mut engine, &[ogre, air, elves]);
    let islands = lands_of(&engine, P0);
    let paying: Vec<ObjectId> = islands.iter().copied().take(2).collect();

    // P1's upkeep first: P1 controls nothing, so nobody is asked; P0's own upkeep asks P0.
    let menu = walk_to_untap_question(&mut engine, P0);
    assert_eq!(
        engine.state().turn.active,
        P0,
        "the question comes on P0's own upkeep"
    );
    assert!(
        is_tapped(&engine, ogre) && is_tapped(&engine, air) && is_tapped(&engine, elves),
        "no creature untapped in P0's untap step"
    );
    assert!(
        menu.contains(&ogre) && menu.contains(&air) && !menu.contains(&elves),
        "both tapped nongreen creatures are offered, the tapped Llanowar Elves is not"
    );
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![ogre],
            },
        )
        .unwrap();
    let Pending::YesNo {
        prompt: YesNoPrompt::PayMana { cost },
        ..
    } = engine.pending().clone()
    else {
        panic!("a price question, got {:?}", engine.pending())
    };
    assert_eq!(cost, baylee_core::mana!("{2}"), "{{2}} for one creature");
    pay_the_price_with(&mut engine, P0, |land| paying.contains(&land));

    assert!(!is_tapped(&engine, ogre), "the chosen creature untapped");
    assert!(
        is_tapped(&engine, air),
        "the creature not chosen stays tapped"
    );
    assert!(is_tapped(&engine, elves), "the green creature stays tapped");
    assert!(
        paying.iter().all(|land| is_tapped(&engine, *land)),
        "the two lands that paid are tapped"
    );
    assert!(
        islands
            .iter()
            .filter(|land| !paying.contains(land))
            .all(|land| !is_tapped(&engine, *land)),
        "{{2}} is the whole price: the other lands are untouched"
    );
}

/// Two creatures chosen cost {4}; refusing to pay untaps neither, and spends nothing.
#[test]
fn dream_tides_charges_two_per_creature_and_untaps_none_when_the_price_is_refused() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(
            0,
            &[
                dream_tides(),
                gray_ogre(),
                air_elemental(),
                island(),
                island(),
                island(),
                island(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    let ogre = on_battlefield(&engine, P0, gray_ogre()).unwrap();
    let air = on_battlefield(&engine, P0, air_elemental()).unwrap();
    tap(&mut engine, &[ogre, air]);
    let islands = lands_of(&engine, P0);

    walk_to_untap_question(&mut engine, P0);
    engine
        .apply(
            P0,
            PlayerAction::ChooseObjects {
                objects: vec![ogre, air],
            },
        )
        .unwrap();
    let Pending::YesNo {
        prompt: YesNoPrompt::PayMana { cost },
        ..
    } = engine.pending().clone()
    else {
        panic!("a price question, got {:?}", engine.pending())
    };
    assert_eq!(
        cost,
        baylee_core::mana!("{4}"),
        "{{2}} for each of two creatures"
    );
    engine.apply(P0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(
        is_tapped(&engine, ogre) && is_tapped(&engine, air),
        "an unpaid price untaps nothing"
    );
    assert!(
        islands.iter().all(|land| !is_tapped(&engine, *land)),
        "no land was spent"
    );
}
