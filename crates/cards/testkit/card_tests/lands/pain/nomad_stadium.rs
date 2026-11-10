//! `cards/lands/pain/nomad_stadium.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Nomad Stadium prints two sentences, and only the first is written: the
/// mana ability that adds {W} *and* deals 1 damage to you, because the
/// threshold gate on the "{W}, {T}, Sacrifice this land: You gain 4 life"
/// line has no condition to be spelled in. So the card is played as a land
/// drop and then tapped for real, which is the only way the two halves of
/// the rider can be read at once: the white mana has to land in the pool
/// *and* one life has to be missing from the seat that tapped, in one
/// activation that never touches the stack (CR 605.3b). The opponent's life
/// is the counter-half — a damage rider written as "target player" instead
/// of "you" would read the same in the card file and would take the point
/// off the wrong seat here.
#[test]
fn nomad_stadium_taps_for_white_and_bites_its_controller_for_one() {
    let p0 = PlayerId::new(0);
    let _p1 = PlayerId::new(1);
    let mut engine = Duel::new(7801, forest())
        .hand(0, &[nomad_stadium()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, nomad_stadium());
    assert!(
        on_battlefield(&engine, p0, nomad_stadium()).is_some(),
        "the land drop put it on the battlefield"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "arriving costs nothing; only the tap carries the rider"
    );
    assert_eq!(engine.state().players[0].mana_pool.total(), 0);
    assert!(!is_tapped(&engine, land), "and it arrives untapped");

    // `tap_all_mana` takes both lists (#159), and this land's {T} is a
    // printed mana ability rather than a basic-type shortcut — with one
    // source on the board the count is one either way.
    let taken = tap_all_mana(&mut engine, p0);
    assert_eq!(taken, 1, "the one land on the board is one mana route");

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(
        pool.available(ManaColor::White),
        1,
        "{{T}}: Add {{W}}, and the one white is white and not a default"
    );
    assert_eq!(pool.total(), 1, "one mana, off one tap");
    assert!(is_tapped(&engine, land), "the Stadium paid its own {{T}}");
    assert!(
        stack_is_empty(&engine),
        "CR 605.3b: a mana ability uses no stack, so the damage is not \
         waiting on one"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"this land deals 1 damage to you\", off the same activation"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the point belongs to the seat that tapped, not to the opponent"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the seat holds priority again, got {:?}",
        engine.pending()
    );
}

// Nomad Stadium, Centaur Garden, Cephalid Coliseum and Nantuko Monastery: the
// other four lands `Condition::GraveyardCountAtLeast` finished, each played
// once. The gate itself is proved at the boundary in
// `cabal_pit_offers_its_ability_only_at_threshold`; what each of these adds is
// that the *effect* behind the gate is the printed one — a condition that let
// the wrong ability through would pass a test that only asked whether
// something was offered.

/// Nomad Stadium: "You gain 4 life."
#[test]
fn nomad_stadium_gains_four_life_once_the_graveyard_is_full() {
    let p0 = PlayerId::new(0);

    let mut engine = Duel::new(13, forest())
        .battlefield(0, &[nomad_stadium(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let land = on_battlefield(&engine, p0, nomad_stadium()).expect("in play");
    tap_mana_except(&mut engine, p0, land);
    seed_graveyard(&mut engine, p0, 7);
    let life = engine.state().players[0].life;
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: land,
                ability_index: 1,
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().players[0].life, life + 4);
}
