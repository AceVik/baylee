//! `cards/instants/mv_1/heat_ray.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Heat Ray — {X}{R} Instant: "Heat Ray deals X damage to target creature."
///
/// X is the whole card, so the scenario answers a number no fixed amount could
/// have produced: five, which is exactly lethal to the printed 7/5 across the
/// table. The number is read out of the question the engine asks at announce
/// time (CR 601.2b) and the target off the list published for CR 601.2c — a
/// creature and no player, which is what "target creature" is worth telling
/// from "any target". The six Mountains the cast drains are what says the
/// {{5}}{{R}} was paid rather than assumed.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn heat_ray_deals_the_x_its_controller_names_to_the_creature_it_names() {
    // oracle_id = "d3a5a830-cd14-49da-9412-c50049c74c92"
    fn fleshgorger() -> CardIndex {
        card_index("d3a5a830-cd14-49da-9412-c50049c74c92")
    }

    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
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
            ],
        )
        // A 7/5 for five damage to be exactly lethal to, and an artifact that
        // "target creature" has to decline.
        .battlefield(1, &[fleshgorger(), quiet_artifact()])
        .hand(0, &[heat_ray()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let target = on_battlefield(&engine, p1, fleshgorger()).expect("the 7/5 is out");
    let ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");
    assert_eq!(
        pt(&engine, target),
        (7, 5),
        "five damage is exactly lethal to the body the spell is aimed at"
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        6,
        "six Mountains, and six is exactly {{5}}{{R}}"
    );
    cast_with_floating(&mut engine, p0, heat_ray());

    // X is chosen at announce time (CR 601.2b) and the target after it
    // (CR 601.2c); each answer is lifted out of the enumeration its own
    // question carried rather than assumed.
    let mut asked_x = false;
    let mut menu: Vec<ObjectId> = Vec::new();
    let mut a_player_was_on_the_menu = true;
    for _ in 0..8 {
        if asked_x && !menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseNumber {
                player, min, max, ..
            } => {
                assert!(
                    min <= 5 && 5 <= max,
                    "X = 5 must be one of the values six Mountains can pay: {min}..={max}"
                );
                engine
                    .apply(player, PlayerAction::ChooseNumber(5))
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
                a_player_was_on_the_menu = !player_options.is_empty();
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![target],
                        },
                    )
                    .expect("the 7/5 was one of the options");
            }
            other => panic!("unexpected while casting Heat Ray: {other:?}"),
        }
    }
    assert!(
        asked_x,
        "{{X}} is a question the engine asks and not a number the card fixes"
    );
    assert!(
        menu.contains(&target),
        "\"target creature\" offers the creature across the table: {menu:?}"
    );
    assert!(
        !menu.contains(&ring),
        "an artifact is no creature: {menu:?}"
    );
    assert!(
        !a_player_was_on_the_menu,
        "\"target creature\" is not \"any target\" (CR 115.4): both seats \
         would otherwise be on the menu"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{5}}{{R}} is the last step of the activation (CR 601.2h), and it is \
         paid rather than labelled"
    );

    pay_life_ward(&mut engine, p0, 7);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        in_graveyard(&engine, p1, fleshgorger()).is_some(),
        "five damage to a 7/5 is lethal (CR 704.5g) — a fixed three would have \
         left it standing, which is what makes this an X and not a number"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_some(),
        "the permanent the spell did not name never moved"
    );
}
