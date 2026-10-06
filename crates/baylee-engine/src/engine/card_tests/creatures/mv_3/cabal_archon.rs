//! `cards/creatures/mv_3/cabal_archon.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Cabal Archon — {2}{B} — 2/2 Human Cleric: "{B}, Sacrifice a Cleric: Target
/// player loses 2 life and you gain 2 life."
///
/// No part of the line is readable from the card. "Sacrifice a Cleric" names
/// no particular creature, so the engine must ask — and its menu is half the
/// card: both Clerics on this side are on it (the Archon itself is a Human
/// Cleric; the cost does not say "another"), while the Elf next to it
/// (a Druid) and the Cleric on the other side (CR 701.21a) are missing.
/// "Target player" is the second question and reaches both sides, and only
/// together with the sacrifice does the life total explain that two
/// *different* players are meant: one loses two, the other gains two — a line
/// that credited the two only to the target would leave p0 at 20. The {2}{B}
/// of the cast leaves exactly one {B} in the mana pool, so the cost of the
/// activation is a real payment and not a label.
#[test]
#[allow(clippy::too_many_lines)] // eine Aktivierung, zwei Fragen, und jede Klausel am Brett gelesen
fn cabal_archon_sacrifices_a_cleric_to_drain_two_life_and_gain_two() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                ondu_cleric(),
                llanowar_elves(),
            ],
        )
        // A Cleric on the table: "Sacrifice a Cleric" says nothing about
        // "you control", so it must stay out of the menu.
        .battlefield(1, &[ondu_cleric()])
        .hand(0, &[cabal_archon()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // Four Swamps into the pool; the Elf remains untapped, because its
    // own `{T}: Add {G}` would otherwise have been tapped as well, and it
    // is the creature on which the sacrifice menu reads its "you control".
    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "vier Sümpfe, und der Elf hat nichts beigesteuert"
    );
    cast_with_floating(&mut engine, p0, cabal_archon());
    pass_until(&mut engine, |e| at_rest(e, p0));

    let archon = on_battlefield(&engine, p0, cabal_archon()).expect("the Archon resolved");
    let fodder = on_battlefield(&engine, p0, ondu_cleric()).expect("der Kleriker steht");
    let theirs = on_battlefield(&engine, p1, ondu_cleric()).expect("ihr Kleriker steht");
    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("der Elf steht");
    assert_eq!(pt(&engine, archon), (2, 2), "der gedruckte 2/2-Körper");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{2}}{{B}} has been spent and exactly the {{B}} that the \
         ability requires is still in the pool"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(archon, 0)),
        "with {{B}} in the pool, the only line of the card is in the offer: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, cabal_archon(), 0);

    // Two questions in one activation: whom it targets (CR 601.2c) and which
    // Cleric pays (CR 601.2h) — answered in the order they arrive, not in the
    // order one expects them.
    let mut asked_whom = false;
    let mut menu: Vec<ObjectId> = Vec::new();
    for _ in 0..16 {
        if asked_whom && !menu.is_empty() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                player_options,
                ..
            } => {
                assert_eq!(player, p0, "der aktivierende Platz zielt");
                assert!(
                    player_options.contains(&p0) && player_options.contains(&p1),
                    "\"target player\" erreicht beide Plätze: {player_options:?}"
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
                asked_whom = true;
            }
            Pending::ChoosePlayer { player, options } => {
                assert_eq!(player, p0, "der aktivierende Platz zielt");
                assert!(
                    options.contains(&p0) && options.contains(&p1),
                    "beide Plätze sind legal: {options:?}"
                );
                engine
                    .apply(player, PlayerAction::ChoosePlayer(p1))
                    .unwrap();
                asked_whom = true;
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "der aktivierende Platz zahlt seine Kosten");
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostSacrifice,
                    "a question of cost and not a search, that is all a \
                     client can use to distinguish the two"
                );
                assert_eq!(
                    (min, max),
                    (1, 1),
                    "a Cleric, nothing more and nothing less"
                );
                menu = options;
                let paying = on_battlefield(&engine, p0, ondu_cleric())
                    .expect("der Kleriker steht noch zum Opfern");
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![paying],
                        },
                    )
                    .unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Archon's activation resolves: {other:?}"),
        }
    }
    assert!(asked_whom, "\"target player\" ist eine Zielwahl");
    assert_eq!(
        menu.len(),
        2,
        "die beiden Kleriker, die dieser Platz kontrolliert: {menu:?}"
    );
    assert!(
        menu.contains(&fodder),
        "der Kleriker steht auf dem Menü: {menu:?}"
    );
    assert!(
        menu.contains(&archon),
        "and the Archon is a Human Cleric, so it appears on its own menu — \
         the cost does not say \"another\": {menu:?}"
    );
    assert!(
        !menu.contains(&elves),
        "der Elf ist ein Druide und kein Kleriker: {menu:?}"
    );
    assert!(
        !menu.contains(&theirs),
        "`CR 701.21a`: a Cleric on the other side is not to be sacrificed: {menu:?}"
    );

    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        18,
        "\"target player loses 2 life\" — the seat that was named, not the other"
    );
    assert_eq!(
        engine.state().players[0].life,
        22,
        "and \"you gain 2 life\" belongs to the slot that paid the cost"
    );
    assert!(
        on_battlefield(&engine, p0, ondu_cleric()).is_none(),
        "der geopferte Kleriker hat das Schlachtfeld verlassen"
    );
    assert!(
        in_graveyard(&engine, p0, ondu_cleric()).is_some(),
        "and is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p0, cabal_archon()).is_some(),
        "the Archon ate the Cleric and not itself"
    );
    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "and the creature that the menu never named is still there"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{B}} is spent, and no more was in the pool"
    );
}
