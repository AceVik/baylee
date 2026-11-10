//! `cards/instants/mv_1/dwarven_catapult.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn dwarven_catapult() -> CardIndex {
    card_index("2aae77ed-1a91-41f5-8dc2-e353d63a7586")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Answers the question that names a player.
#[track_caller]
fn name_player(engine: &mut Engine<RegistryLookup>, seat: PlayerId, who: PlayerId) {
    match engine.pending().clone() {
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(player, seat);
            assert_eq!(options, vec![who], "only the opponent is on the menu");
            engine
                .apply(seat, PlayerAction::ChoosePlayer(who))
                .expect("the opponent is a legal choice");
        }
        Pending::ChooseTargets {
            player,
            options,
            player_options,
            ..
        } => {
            assert_eq!(player, seat);
            assert!(options.is_empty(), "no object is a legal target");
            assert_eq!(
                player_options,
                vec![who],
                "only the opponent is on the menu"
            );
            engine
                .apply(
                    seat,
                    PlayerAction::ChooseTargets {
                        objects: vec![],
                        players: vec![who],
                    },
                )
                .expect("the opponent is a legal target");
        }
        other => panic!("expected a player to be named, got {other:?}"),
    }
}

/// Casts the Catapult for `x`, aimed at p1, and lets it resolve.
#[track_caller]
fn fire(engine: &mut Engine<RegistryLookup>, x: u32) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    cast_from_hand(engine, p0, dwarven_catapult());
    engine
        .apply(p0, PlayerAction::ChooseNumber(x))
        .expect("X is chosen");
    name_player(engine, p0, p1);
    pass_until(engine, stack_is_empty);
}

fn board() -> Engine<RegistryLookup> {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                grizzly_bears(),
            ],
        )
        .hand(0, &[dwarven_catapult()])
        .battlefield(1, &[grizzly_bears(), hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");
    engine
}

/// Dwarven Catapult (2aae77ed-1a91-41f5-8dc2-e353d63a7586) — "Dwarven
/// Catapult deals X damage divided evenly, rounded down, among all
/// creatures target opponent controls."
///
/// X = 5 over two creatures is 2 each: the Bears die, the Giant lives with
/// two damage, and my own Bears and the opponent themselves are untouched.
#[test]
fn dwarven_catapult_deals_x_divided_evenly_rounded_down_to_the_opponents_creatures() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = board();
    let mine = on_battlefield(&engine, p0, grizzly_bears()).expect("my Bears are out");
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("the Giant is out");

    fire(&mut engine, 5);

    assert!(
        in_graveyard(&engine, p1, grizzly_bears()).is_some(),
        "2 damage kills their 2/2 Bears"
    );
    let giant_obj = engine.state().object(giant).expect("the Giant lives");
    assert_eq!(giant_obj.zone, Zone::Battlefield);
    assert_eq!(giant_obj.damage, 2, "5 divided by 2, rounded down");
    assert_eq!(
        engine.state().object(mine).expect("my Bears live").damage,
        0,
        "only target opponent's creatures"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "the opponent is not hit"
    );
}

/// Rounded down all the way: X = 1 over two creatures is 0 each.
#[test]
fn dwarven_catapult_with_less_damage_than_creatures_deals_none() {
    let p1 = PlayerId::new(1);
    let mut engine = board();
    let giant = on_battlefield(&engine, p1, hill_giant()).expect("the Giant is out");
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("their Bears are out");

    fire(&mut engine, 1);

    assert_eq!(engine.state().object(giant).expect("alive").damage, 0);
    assert_eq!(engine.state().object(bears).expect("alive").damage, 0);
    assert_eq!(
        engine.state().object(bears).expect("alive").zone,
        Zone::Battlefield
    );
}
