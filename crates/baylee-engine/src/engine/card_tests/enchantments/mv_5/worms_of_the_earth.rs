//! `cards/enchantments/mv_5/worms_of_the_earth.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn worms_of_the_earth() -> CardIndex {
    card_index("9638ec0c-c8bf-4aa5-a7ca-8677e4b9f241")
}

/// Passes priority, answering every yes/no question "no", until `until`
/// holds.
#[track_caller]
fn pass_declining(
    engine: &mut Engine<RegistryLookup>,
    until: impl Fn(&Engine<RegistryLookup>) -> bool,
) {
    for _ in 0..60 {
        if until(engine) {
            return;
        }
        match engine.pending().clone() {
            Pending::YesNo { player, .. } => {
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            _ => pass_until(engine, |e| {
                until(e) || matches!(e.pending(), Pending::YesNo { .. })
            }),
        }
    }
    panic!("condition never reached");
}

fn their_main(e: &Engine<RegistryLookup>) -> bool {
    e.state().turn.active == PlayerId::new(1)
        && matches!(e.state().turn.phase, Phase::FirstMain)
        && matches!(e.pending(), Pending::Priority { player, .. } if *player == PlayerId::new(1))
}

/// Moves `card` from its owner's hand onto the battlefield through the
/// effect door, the way a card that puts a land onto the battlefield does.
fn put_onto_battlefield(
    engine: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: ObjectId,
) -> ObjectId {
    engine
        .dev_state_mut(seat)
        .expect("the harness may move cards")
        .move_object(
            card,
            ZoneLocation::Battlefield,
            crate::zone::ZonePosition::Top,
            crate::event::Cause::Effect,
        )
        .expect("the move is attempted")
}

/// Worms of the Earth (9638ec0c-c8bf-4aa5-a7ca-8677e4b9f241) — "Players
/// can't play lands. Lands can't enter the battlefield."
///
/// Cast for real: neither player can play a land from hand, and a land
/// moved onto the battlefield by an effect stays in hand.
#[test]
fn worms_of_the_earth_stops_land_plays_and_lands_entering() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), swamp(), swamp(), swamp()])
        .hand(0, &[worms_of_the_earth(), forest(), plains()])
        .hand(1, &[mountain()])
        .battlefield(1, &[mountain()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    cast_from_hand(&mut engine, p0, worms_of_the_earth());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, worms_of_the_earth()).is_some());

    let forest_card = in_hand(&engine, p0, forest()).expect("the Forest is in hand");
    assert!(
        engine
            .apply(p0, PlayerAction::PlayLand { card: forest_card })
            .is_err(),
        "I can't play a land"
    );
    assert!(in_hand(&engine, p0, forest()).is_some());

    let plains_card = in_hand(&engine, p0, plains()).expect("the Plains is in hand");
    let moved = put_onto_battlefield(&mut engine, p0, plains_card);
    assert_eq!(
        engine.state().object(moved).map(|o| o.zone),
        Some(Zone::Hand),
        "a land can't enter the battlefield"
    );

    pass_declining(&mut engine, their_main);
    let mountain_card = in_hand(&engine, p1, mountain()).expect("their Mountain is in hand");
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::PlayLand {
                    card: mountain_card
                }
            )
            .is_err(),
        "nor can the opponent"
    );
}

/// The counter-board: with no Worms the same land plays and the same move
/// puts a land onto the battlefield, so the test above is about the card.
#[test]
fn without_worms_of_the_earth_lands_play_and_enter() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp()])
        .hand(0, &[forest(), plains()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let forest_card = in_hand(&engine, p0, forest()).expect("the Forest is in hand");
    engine
        .apply(p0, PlayerAction::PlayLand { card: forest_card })
        .expect("a land plays");
    assert!(on_battlefield(&engine, p0, forest()).is_some());

    let plains_card = in_hand(&engine, p0, plains()).expect("the Plains is in hand");
    let moved = put_onto_battlefield(&mut engine, p0, plains_card);
    assert_eq!(
        engine.state().object(moved).map(|o| o.zone),
        Some(Zone::Battlefield)
    );
}

/// "At the beginning of each upkeep, any player may sacrifice two lands of
/// their choice or have this enchantment deal 5 damage to that player. If a
/// player does either, destroy this enchantment." Players with fewer than two
/// lands are only offered the damage: p0 declines, p1 takes 5 and the Worms
/// are destroyed.
#[test]
fn worms_of_the_earth_is_destroyed_when_a_player_takes_five() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[worms_of_the_earth(), swamp()])
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("the pass stopped on a yes/no question")
    };
    assert_eq!(player, p0, "the active player is asked first");
    assert!(
        matches!(prompt, crate::choice::YesNoPrompt::Generic),
        "one land cannot be sacrificed twice, so only the damage is offered: {prompt:?}"
    );
    engine
        .apply(p0, PlayerAction::YesNo(false))
        .expect("p0 declines");

    let Pending::YesNo { player, .. } = engine.pending().clone() else {
        panic!(
            "expected the opponent to be asked, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p1);
    engine
        .apply(p1, PlayerAction::YesNo(true))
        .expect("p1 takes the damage");
    pass_until(&mut engine, |e| {
        in_graveyard(e, p0, worms_of_the_earth()).is_some()
    });

    assert_eq!(
        engine.state().players[1].life,
        15,
        "5 damage to the player who chose it"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and none to the one who declined"
    );
}

/// Sacrificing two lands pays instead of the damage: two of p0's three
/// Mountains go, the Worms are destroyed and nobody takes any damage.
#[test]
fn worms_of_the_earth_is_destroyed_when_a_player_sacrifices_two_lands() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[worms_of_the_earth(), mountain(), mountain(), mountain()],
        )
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut engine);

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("the pass stopped on a yes/no question")
    };
    assert_eq!(player, p0);
    assert!(
        matches!(
            prompt,
            crate::choice::YesNoPrompt::SacrificeToEnter { count: 2 }
        ),
        "asked whether to sacrifice two lands: {prompt:?}"
    );
    engine
        .apply(p0, PlayerAction::YesNo(true))
        .expect("p0 sacrifices");
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!("expected the lands to choose, got {:?}", engine.pending())
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (2, 2));
    assert_eq!(options.len(), 3, "any of my lands");
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: options[..2].to_vec(),
            },
        )
        .expect("two of the offered lands");
    pass_declining(&mut engine, |e| {
        in_graveyard(e, p0, worms_of_the_earth()).is_some()
    });

    assert_eq!(all_on_battlefield(&engine, p0, mountain()).len(), 1);
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20, "nobody took damage");
}

/// If everybody declines everything, the Worms stay.
#[test]
fn worms_of_the_earth_stays_when_every_player_declines() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[worms_of_the_earth(), swamp()])
        .battlefield(1, &[island()])
        .start();
    keep_mulligans(&mut engine);

    pass_declining(&mut engine, |e| {
        e.state().turn.active == p0
            && matches!(e.state().turn.phase, Phase::FirstMain)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    assert!(on_battlefield(&engine, p0, worms_of_the_earth()).is_some());
    assert_eq!(engine.state().players[0].life, 20);
    assert_eq!(engine.state().players[1].life, 20);
}
