//! `cards/creatures/mv_4/phelddagrif.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::partials_w2c_kit::life;

fn phelddagrif() -> CardIndex {
    card_index("d0e99535-0ea0-4aa5-99f5-6c5255b27c38")
}

/// Phelddagrif ({1}{G}{W}{U} 4/4 Legendary Creature — Phelddagrif):
/// - "{G}: Phelddagrif gains trample until end of turn. Target opponent creates a 1/1 green Hippo creature token."
/// - "{W}: Phelddagrif gains flying until end of turn. Target opponent gains 2 life."
/// - "{U}: Return Phelddagrif to its owner's hand. Target opponent may draw a card."
#[test]
fn phelddagrif_all_three_activated_abilities_grant_keywords_or_bounce_and_gift_opponent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[phelddagrif(), forest(), plains(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let pheld = on_battlefield(&engine, p0, phelddagrif()).unwrap();
    let opp_life_before = life(&engine, p1);
    let opp_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    tap_all_mana(&mut engine, p0);

    // 1. {G}: Trample + opponent creates a 1/1 green Hippo token.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: pheld,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, pheld).contains(KeywordSet::TRAMPLE));
    let opp_tokens = tokens_of(&engine, p1);
    assert_eq!(opp_tokens.len(), 1);
    assert_eq!(pt(&engine, opp_tokens[0]), (1, 1));

    // 2. {W}: Flying + opponent gains 2 life.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: pheld,
                ability_index: 1,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(keywords(&engine, pheld).contains(KeywordSet::FLYING));
    assert_eq!(life(&engine, p1), opp_life_before + 2);

    // 3. {U}: Return Phelddagrif to owner's hand + opponent may draw a card.
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: pheld,
                ability_index: 2,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    engine.apply(p1, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, phelddagrif()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        opp_hand_before + 1
    );
}

/// Phelddagrif — "{G}: Phelddagrif gains trample until end of turn. Target
/// opponent creates a 1/1 green Hippo creature token." and "{W}: … flying
/// until end of turn. Target opponent gains 2 life.": the gift goes to the
/// opponent alone, and the keywords end with the turn.
#[test]
fn phelddagrif_gifts_go_to_the_opponent_and_its_keywords_end_with_the_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[phelddagrif(), forest(), plains()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let pheld = on_battlefield(&engine, p0, phelddagrif()).unwrap();
    let (my_life, their_life) = (life(&engine, p0), life(&engine, p1));

    tap_all_mana(&mut engine, p0);
    for ability_index in [0, 1] {
        engine
            .apply(
                p0,
                PlayerAction::ActivateAbility {
                    source: pheld,
                    ability_index,
                },
            )
            .unwrap();
        engine
            .apply(
                p0,
                PlayerAction::ChooseTargets {
                    objects: vec![],
                    players: vec![p1],
                },
            )
            .unwrap();
        pass_until(&mut engine, stack_is_empty);
    }
    assert!(keywords(&engine, pheld).contains(KeywordSet::TRAMPLE));
    assert!(keywords(&engine, pheld).contains(KeywordSet::FLYING));
    assert_eq!(tokens_of(&engine, p1).len(), 1, "the opponent's Hippo");
    assert!(tokens_of(&engine, p0).is_empty(), "none for you");
    assert_eq!(life(&engine, p1), their_life + 2);
    assert_eq!(life(&engine, p0), my_life, "you gain nothing");

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
    });
    assert!(!keywords(&engine, pheld).contains(KeywordSet::TRAMPLE));
    assert!(!keywords(&engine, pheld).contains(KeywordSet::FLYING));
}

/// Phelddagrif — "{U}: Return Phelddagrif to its owner's hand. Target
/// opponent may draw a card.": "may", so the opponent can decline.
#[test]
fn phelddagrif_opponent_may_decline_the_card() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[phelddagrif(), island()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let pheld = on_battlefield(&engine, p0, phelddagrif()).unwrap();
    let opp_hand_before = engine.state().zones.list(ZoneLocation::Hand(p1)).len();

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: pheld,
                ability_index: 2,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::YesNo { .. })
    });
    assert_eq!(engine.pending().asked(), Some(p1), "they decide, not you");
    engine.apply(p1, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_hand(&engine, p0, phelddagrif()).is_some());
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p1)).len(),
        opp_hand_before,
        "no card drawn"
    );
}
