//! `cards/sorceries/mv_2/drain_life.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Drain Life — {X}{1}{B} — Sorcery: "Drain Life deals X damage to any
/// target." Cast twice with two different X: two damage marks a vanilla
/// 2/3 without killing it, and five reaches the opponent's life total by
/// exactly five. "Any target" (CR 115.4) is the one menu a creature and
/// both players share. The life-gain half of the card, and the
/// black-mana-only rule on X, are not implemented and are not asked here.
#[allow(clippy::too_many_lines)] // One printed card, played end to end.
#[test]
fn drain_life_deals_the_x_its_controller_names_to_a_creature_and_then_to_a_player() {
    // A vanilla 2/3 body with no ward or other rider, so two marked
    // damage neither kills it nor is lost in the noise of a smaller one,
    // and no unrelated payment is dragged into a test about Drain Life.
    // oracle_id = "8f1dae40-b307-446e-bbd2-86aa35813871"
    fn hurloon_minotaur() -> CardIndex {
        card_index("8f1dae40-b307-446e-bbd2-86aa35813871")
    }

    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[swamp(); 12])
        .battlefield(1, &[hurloon_minotaur()])
        .hand(0, &[drain_life(), drain_life()])
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let minotaur = on_battlefield(&engine, p1, hurloon_minotaur()).expect("the 2/3 is out");
    assert_eq!(
        pt(&engine, minotaur),
        (2, 3),
        "printed 2/3, and three survives two"
    );

    tap_all_mana(&mut engine, p0);

    // First cast: X = 2, at the creature.
    cast_with_floating(&mut engine, p0, drain_life());
    let mut asked_x = false;
    let mut menu: Vec<ObjectId> = Vec::new();
    let mut menu_players: Vec<PlayerId> = Vec::new();
    for _ in 0..8 {
        if asked_x && !menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseNumber {
                player, min, max, ..
            } => {
                assert!(min <= 2 && 2 <= max, "X = 2 must be payable: {min}..={max}");
                engine
                    .apply(player, PlayerAction::ChooseNumber(2))
                    .expect("the answer came out of the question");
                asked_x = true;
            }
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                ..
            } => {
                menu = options;
                menu_players = player_options;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![minotaur],
                        },
                    )
                    .expect("the 2/3 is a legal target");
            }
            other => panic!("unexpected while casting Drain Life: {other:?}"),
        }
    }
    assert!(
        asked_x,
        "X is a question the engine asks, not a number the card fixes"
    );
    assert!(
        menu.contains(&minotaur),
        "\"any target\" reaches a creature: {menu:?}"
    );
    assert!(
        menu_players.contains(&p0) && menu_players.contains(&p1),
        "any target (CR 115.4) puts both players on the same menu: {menu_players:?}"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(minotaur).unwrap().damage,
        2,
        "exactly the X it was cast for"
    );
    assert_eq!(
        pt(&engine, minotaur),
        (2, 3),
        "marked damage does not change printed power or toughness"
    );
    assert!(
        on_battlefield(&engine, p1, hurloon_minotaur()).is_some(),
        "two marked on a 2/3, one short of lethal, is not lethal"
    );

    // Second cast: X = 5, at p1 directly.
    let life_before = life_of(&engine, p1);
    cast_with_floating(&mut engine, p0, drain_life());
    let mut asked_x2 = false;
    let mut targeted = false;
    for _ in 0..8 {
        if asked_x2 && targeted {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseNumber {
                player, min, max, ..
            } => {
                assert!(min <= 5 && 5 <= max, "X = 5 must be payable: {min}..={max}");
                engine
                    .apply(player, PlayerAction::ChooseNumber(5))
                    .expect("the answer came out of the question");
                asked_x2 = true;
            }
            Pending::ChooseTargets {
                player,
                player_options,
                ..
            } => {
                assert!(
                    player_options.contains(&p1),
                    "\"any target\" reaches the player: {player_options:?}"
                );
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects: vec![],
                            players: vec![p1],
                        },
                    )
                    .expect("p1 is a legal target");
                targeted = true;
            }
            other => panic!("unexpected while casting the second Drain Life: {other:?}"),
        }
    }
    assert!(
        asked_x2 && targeted,
        "both questions were asked and answered"
    );
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        life_of(&engine, p1),
        life_before - 5,
        "exactly the second X, off the player directly"
    );
    assert_eq!(
        engine.state().object(minotaur).unwrap().damage,
        2,
        "the second cast's damage went to the player, not back onto the creature"
    );
}
