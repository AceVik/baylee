//! `cards/creatures/mv_1/kris_mage.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kris Mage, {R}, 1/1 Human Spellshaper: "{R}, {T}, Discard a card: This
/// creature deals 1 damage to any target." (Mercadian Masques.)
///
/// A cost in three parts, and each is read on the battlefield. Without mana
/// in the pool, the ability is not even offered — `legal.abilities` is
/// filtered through `can_afford`, and that reads the pool and not the
/// untapped lands —, so the {R} is a real cost and not a line on the
/// card. After that, the question form is the card: the target arrives as
/// `ChooseTargets`, whose `player_options` lists the opponent next to their
/// creature ("any target", CR 115.4), and only *after* target selection
/// does the discard ask as `ChooseCards { prompt: CostDiscard }` for the
/// card that pays the graveyard (CR 601.2c before CR 601.2h). The one lost
/// life point is the number that proves "1 damage" and not "destroy".
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn kris_mage_pays_a_red_a_tap_and_a_card_for_exactly_one_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(4141, forest())
        .battlefield(0, &[mountain(), kris_mage()])
        .hand(0, &[island()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mage = on_battlefield(&engine, p0, kris_mage()).expect("Kris Mage steht");
    let theirs = on_battlefield(&engine, p1, llanowar_elves()).expect("der Elf des Gegners steht");
    assert_eq!(pt(&engine, mage), (1, 1), "ein gedrucktes 1/1");
    let hand_before: Vec<ObjectId> = engine.state().zones.list(ZoneLocation::Hand(p0)).clone();
    assert!(!hand_before.is_empty(), "the discard costs need a card");

    // The empty pool is the control for the {R}.
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!(
            "a main phase of its own gives priority back: {:?}",
            engine.pending()
        )
    };
    assert!(
        !legal.abilities.contains(&(mage, 0)),
        "without mana in the pool the ability is not affordable and not \
         offered: {:?}",
        legal.abilities
    );

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        1,
        "ein Mountain, ein rotes Mana"
    );
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("after tapping, priority returns: {:?}", engine.pending())
    };
    assert!(
        legal.abilities.contains(&(mage, 0)),
        "with {{R}} in the pool the one line of the card is offered: {:?}",
        legal.abilities
    );

    activate(&mut engine, p0, kris_mage(), 0);

    let mut asked_target = false;
    let mut discarded: Option<ObjectId> = None;
    for _ in 0..12 {
        if asked_target && discarded.is_some() {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the activating seat chooses the target");
                assert_eq!((min, max), (1, 1), "\"any target\" ist ein Pflichtziel");
                assert!(
                    options.contains(&theirs),
                    "the opposing creature is a target: {options:?}"
                );
                assert!(
                    player_options.contains(&p1),
                    "\"any target\" takes in the player too (CR 115.4): {player_options:?}"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseTargets {
                            objects: Vec::new(),
                            players: vec![p1],
                        },
                    )
                    .unwrap();
                asked_target = true;
            }
            Pending::ChooseCards {
                player,
                options,
                min,
                max,
                prompt,
                ..
            } => {
                assert_eq!(player, p0, "the activating player gives up the card");
                assert_eq!(
                    prompt,
                    crate::choice::ChoicePrompt::CostDiscard,
                    "a cost choice and not a search, which is the only thing \
                     whereby a client distinguishes the two"
                );
                assert_eq!((min, max), (1, 1), "exactly one card");
                assert!(
                    asked_target,
                    "the cost question comes after target selection (CR \
                     601.2c before CR 601.2h)"
                );
                assert!(
                    options.iter().all(|id| hand_before.contains(id)),
                    "discarded from the hand: {options:?}"
                );
                let pick = options[0];
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![pick],
                        },
                    )
                    .unwrap();
                discarded = Some(pick);
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unerwartet beim Aktivieren von Kris Mage: {other:?}"),
        }
    }

    assert!(asked_target, "the ability asked for its target");
    let gone = discarded.expect("the discard costs asked for a card");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[1].life,
        19,
        "\"deals 1 damage to any target\" on the player: exactly one point"
    );
    assert_eq!(
        engine.state().players[0].life,
        20,
        "and the damage belongs to the target, not to the Mage"
    );
    assert!(
        is_tapped(&engine, mage),
        "{{T}} war der zweite Teil des Preises"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "und das {{R}} ist ausgegeben"
    );
    assert!(
        !engine
            .state()
            .zones
            .list(ZoneLocation::Hand(p0))
            .contains(&gone),
        "the discarded card left the hand"
    );
    assert!(
        engine
            .state()
            .zones
            .list(ZoneLocation::Graveyard(p0))
            .contains(&gone),
        "and lies in its owner's graveyard"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before.len() - 1,
        "exactly one card paid the discard"
    );
}
