//! `cards/creatures/mv_2/blood_artist.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Blood Artist is {1}{B} for a 0/1 Vampire that prints one sentence:
/// "Whenever this creature or another creature dies, target player loses 1
/// life and you gain 1 life." Both halves of that `or` are played here — an
/// Elf fed to Ashnod's Altar ("another creature") and then the Artist itself
/// ("this creature") — because a card that watched only its neighbours would
/// pass the first half on its own. The player named is the opponent both
/// times: aiming the drain at the Artist's own controller would fold the two
/// effects into a net nothing, which is precisely the score of a card with no
/// rules text at all.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn blood_artist_drains_the_player_it_names_for_every_creature_that_dies() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(), swamp(), ashnods_altar(), llanowar_elves()])
        .hand(0, &[blood_artist()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    // {1}{B} off the two Swamps. `tap_all_mana` leaves the Altar standing:
    // its whole price is a creature and not its own {T}, so it is untapped
    // for both activations below.
    cast_from_hand(&mut engine, p0, blood_artist());
    pass_until(&mut engine, stack_is_empty);
    let artist = on_battlefield(&engine, p0, blood_artist()).expect("the Artist resolved");
    let fodder = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elf is out");
    assert_eq!(pt(&engine, artist), (0, 1), "the body the card prints");

    // Answer the drain's own question and let it resolve. "target player" is
    // a target like any other — it arrives as a choice over the seats and is
    // answered with the seat the assertions below are about.
    let drain = |engine: &mut Engine<RegistryLookup>| {
        pass_until(engine, |e| {
            matches!(
                e.pending(),
                Pending::ChooseTargets { .. } | Pending::ChoosePlayer { .. }
            )
        });
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                player_options,
                ..
            } => {
                assert_eq!(player, p0, "the Artist's controller names the player");
                assert!(
                    player_options.contains(&p0) && player_options.contains(&p1),
                    "\"target player\" is either seat: {player_options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p1],
                        },
                    )
                    .unwrap();
            }
            Pending::ChoosePlayer { player, options } => {
                assert_eq!(player, p0, "the Artist's controller names the player");
                assert!(options.contains(&p1), "both seats are legal: {options:?}");
                engine
                    .apply(player, PlayerAction::ChoosePlayer(p1))
                    .unwrap();
            }
            other => panic!("the drain asks for a player, got {other:?}"),
        }
        pass_until(engine, stack_is_empty);
    };

    // First half: another creature dies. The Altar's cost names no creature,
    // so the engine has to ask which one — and the question is a cost, which
    // both creatures this seat controls can pay.
    activate(&mut engine, p0, ashnods_altar(), 0);
    let Pending::ChooseCards {
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "the Altar's price is a creature, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(prompt, crate::choice::ChoicePrompt::CostSacrifice);
    assert_eq!((min, max), (1, 1), "one creature pays the cost");
    assert_eq!(
        options.len(),
        2,
        "the Artist is a creature too, so both are on the menu: {options:?}"
    );
    assert!(options.contains(&fodder) && options.contains(&artist));
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![fodder],
            },
        )
        .unwrap();
    drain(&mut engine);

    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the Elf died for the Altar's mana"
    );
    assert_eq!(
        engine.state().players[1].life,
        19,
        "the named player loses 1"
    );
    assert_eq!(
        engine.state().players[0].life,
        21,
        "and the Artist's controller gains 1 — the two halves are not the same seat"
    );

    // Second half: the Artist itself. The Altar is still untapped, and the
    // permanent that dies is the trigger's own source — the look-back of
    // CR 603.10a is what makes that readable.
    activate(&mut engine, p0, ashnods_altar(), 0);
    let Pending::ChooseCards { options, .. } = engine.pending().clone() else {
        panic!("the Altar asks again, got {:?}", engine.pending())
    };
    assert_eq!(
        options,
        vec![artist],
        "\"another creature\" here is the Artist itself: it is the only \
         creature left and a legal sacrifice"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![artist],
            },
        )
        .unwrap();
    drain(&mut engine);

    assert!(
        in_graveyard(&engine, p0, blood_artist()).is_some(),
        "\"this creature\" died too"
    );
    assert_eq!(
        engine.state().players[1].life,
        18,
        "and the same player was drained again — the half an `Another`-only \
         filter would have lost"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "one more life to the Artist's controller"
    );
}
