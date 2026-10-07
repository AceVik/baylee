//! `cards/instants/mv_2/heliod_s_intervention.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Heliod's Intervention is a modal instant — `{X}{W}{W}` for "destroy X
/// target artifacts and/or enchantments", or "target player gains twice X
/// life". This scenario plays the destroy mode at X = 1 with exactly three
/// permanents its filter names on the table — my Sol Ring, my Exploration
/// and the opponent's Sol Ring — so the single target the cast names is the
/// only permanent destroyed. The two survivors prove both halves at once:
/// X governs how many, and "artifacts and/or enchantments" governs which
/// kinds appear on the menu (the four Plains do not).
#[test]
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
fn heliods_intervention_destroys_exactly_the_x_artifacts_or_enchantments_it_targets() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                quiet_artifact(),
                exploration(),
            ],
        )
        .battlefield(1, &[quiet_artifact()])
        .hand(0, &[heliods_intervention()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_ring = on_battlefield(&engine, p0, quiet_artifact()).expect("my Sol Ring is out");
    let my_chant = on_battlefield(&engine, p0, exploration()).expect("my Exploration is out");
    let their_ring = on_battlefield(&engine, p1, quiet_artifact()).expect("their Sol Ring is out");

    // Mana first: `can_afford` reads the pool and not the untapped lands, so
    // the spell's own costs are what the offer is filtered against.
    tap_all_mana(&mut engine, p0);
    assert!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::White)
            >= 2,
        "the four Plains are the white the {{W}}{{W}} needs"
    );
    cast_with_floating(&mut engine, p0, heliods_intervention());

    // The three questions the cast asks — X, the mode and the target — each
    // in whichever order the engine raises them.
    let mut chose_x = false;
    let mut chose_mode = false;
    for _ in 0..12 {
        match engine.pending().clone() {
            Pending::ChooseNumber {
                player, min, max, ..
            } => {
                assert_eq!(player, p0, "the caster names X");
                assert!(
                    min <= 1 && 1 <= max,
                    "X = 1 is a legal value: {min}..={max}"
                );
                engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
                chose_x = true;
            }
            Pending::ChooseCastMode {
                player, options, ..
            } => {
                assert_eq!(player, p0, "the caster names the mode");
                let slot = options
                    .iter()
                    .position(|o| matches!(o.kind, CastModeKind::Mode(0)))
                    .expect("the destroy mode is one of the ways to cast this card");
                engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
                chose_mode = true;
            }
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the caster aims the destroy");
                assert_eq!(
                    (min, max),
                    (1, 1),
                    "X = 1: exactly one target, no more and no fewer"
                );
                assert!(
                    options.contains(&their_ring)
                        && options.contains(&my_ring)
                        && options.contains(&my_chant),
                    "\"artifacts and/or enchantments\" is any of the three, on \
                     either side of the table: {options:?}"
                );
                assert_eq!(
                    options.len(),
                    3,
                    "the four Plains are lands, neither artifacts nor \
                     enchantments: {options:?}"
                );
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![their_ring],
                        },
                    )
                    .expect("the Sol Ring X named is one of the options");
                break;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while casting Heliod's Intervention: {other:?}"),
        }
    }
    assert!(chose_x, "X is a question the caster answers");
    assert!(
        chose_mode,
        "with two modes printed, the caster picks one of them"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        in_graveyard(&engine, p1, quiet_artifact()).is_some(),
        "\"destroy X target artifacts and/or enchantments\" — the Sol Ring the \
         target question named is in its owner's graveyard"
    );
    assert!(
        on_battlefield(&engine, p1, quiet_artifact()).is_none(),
        "and it is no longer on the battlefield"
    );
    assert!(
        on_battlefield(&engine, p0, quiet_artifact()).is_some(),
        "the artifact X did not name is untouched: X is a count, not a sweep"
    );
    assert!(
        on_battlefield(&engine, p0, exploration()).is_some(),
        "and the enchantment beside it survives, which is the half of the \
         filter only \"and/or\" can account for: destroying every permanent \
         the filter matched would empty the board"
    );
}

/// Heliod's Intervention with an X its pool cannot pay: the X is taken, so is
/// the player it then names, and the cast is reversed.
///
/// A total cost the caster cannot pay makes the cast illegal: it is reversed
/// (CR 601.2h, 732.1), with the mana back in the pool and the card back in
/// hand. Choosing the offered maximum still exceeds the pool once the
/// spell's fixed {W}{W} is included, so the resource-bounded X menu must
/// preserve the same rollback behavior.
#[test]
fn heliods_intervention_over_an_x_it_cannot_pay_is_taken_and_reversed() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains(), plains()])
        .hand(0, &[heliods_intervention()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    tap_all_mana(&mut engine, p0);
    let floating = engine.state().players[0].mana_pool.total();
    cast_with_floating(&mut engine, p0, heliods_intervention());
    let mut named = None;
    for _ in 0..12 {
        match engine.pending().clone() {
            Pending::ChooseNumber { player, max, .. } => {
                assert!(
                    u64::from(max) > floating.saturating_sub(2),
                    "the offered X plus fixed {{W}}{{W}} must exceed the pool: {max} over {floating}"
                );
                engine
                    .apply(player, PlayerAction::ChooseNumber(max))
                    .expect("an X the question offers is an answer");
            }
            Pending::ChooseCastMode {
                player, options, ..
            } => {
                let slot = options
                    .iter()
                    .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
                    .expect("the lifegain mode is one of the ways to cast this card");
                engine
                    .apply(player, PlayerAction::ChooseMode(slot))
                    .unwrap();
            }
            Pending::ChoosePlayer { player, options } => {
                let chosen = options[0];
                engine
                    .apply(player, PlayerAction::ChoosePlayer(chosen))
                    .expect("a player the question offers is an answer");
                named = Some(chosen);
                break;
            }
            other => panic!("unexpected while casting Heliod's Intervention: {other:?}"),
        }
    }
    assert!(
        named.is_some(),
        "the lifegain mode never asked for its player"
    );
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0),
        "the reversed cast gave the caster their priority back: {:?}",
        engine.pending()
    );
    assert!(
        engine.state().zones.stack_is_empty(),
        "the unpayable spell reached the stack"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        floating,
        "the reversed cast spent nothing"
    );
    assert!(
        in_hand(&engine, p0, heliods_intervention()).is_some(),
        "the card went back to hand"
    );
}
