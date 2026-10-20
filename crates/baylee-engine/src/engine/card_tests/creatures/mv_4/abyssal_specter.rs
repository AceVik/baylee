//! `cards/creatures/mv_4/abyssal_specter.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Phase;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn abyssal_specter() -> CardIndex {
    card_index("51833cff-6519-4806-8d91-0040e9a02189")
}

/// Abyssal Specter (51833cff-6519-4806-8d91-0040e9a02189) — {2}{B}{B} 2/3
/// Creature — Specter:
/// - Flying
/// - "Whenever this creature deals damage to a player, that player discards a card."
///
/// Unlike Hypnotic Specter ("at random"), the damaged player chooses which card
/// to discard. When blocked by a flying creature, no damage is dealt to a
/// player and no discard trigger fires.
#[test]
fn abyssal_specter_flies_and_makes_damaged_player_choose_a_card_to_discard() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[abyssal_specter()])
        .hand(1, &[forest(), island()])
        .start();
    keep_mulligans(&mut engine);
    let specter = on_battlefield(&engine, P0, abyssal_specter()).expect("Abyssal Specter seated");
    assert_eq!(pt(&engine, specter), (2, 3));
    assert!(keywords(&engine, specter).contains(KeywordSet::FLYING));

    let keep_card = in_hand(&engine, P1, forest()).expect("Forest in P1 hand");
    let pitch_card = in_hand(&engine, P1, island()).expect("Island in P1 hand");

    let blocks = attack_and_collect_blocks(&mut engine, specter, P1);
    assert!(blocks.is_empty(), "flying prevents ground blockers");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "expected ChooseCards for P1 discard, got {:?}",
            engine.pending()
        );
    };
    assert_eq!(
        player, P1,
        "the damaged player chooses which card to discard"
    );
    assert_eq!((min, max), (1, 1));
    assert!(options.contains(&keep_card) && options.contains(&pitch_card));

    engine
        .apply(
            P1,
            PlayerAction::ChooseObjects {
                objects: vec![pitch_card],
            },
        )
        .expect("P1 chooses the Island to discard");

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(engine.state().players[1].life, 18, "2 combat damage dealt");
    assert!(
        in_graveyard(&engine, P1, island()).is_some(),
        "chosen card discarded"
    );
    assert!(
        in_hand(&engine, P1, forest()).is_some(),
        "unchosen card kept in hand"
    );
}

#[test]
fn abyssal_specter_blocked_by_flier_deals_no_player_damage_and_triggers_no_discard() {
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(0, &[abyssal_specter()])
        .battlefield(1, &[wall_of_swords()])
        .hand(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    let specter = on_battlefield(&engine, P0, abyssal_specter()).expect("Abyssal Specter seated");
    let blocker = on_battlefield(&engine, P1, wall_of_swords()).expect("blocker seated");

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            P0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(specter, Defender::Player(P1))],
            },
        )
        .expect("attack with Abyssal Specter");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    engine
        .apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, specter)],
            },
        )
        .expect("Hypnotic Specter blocks Abyssal Specter");

    pass_until(&mut engine, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        engine.state().players[1].life,
        20,
        "blocked: no player damage"
    );
    assert!(
        in_hand(&engine, P1, forest()).is_some(),
        "no discard triggered"
    );
}
