//! `cards/artifacts/mv_1/helm_of_chatzuk.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Helm of Chatzuk — "{1}, {T}: Target creature gains banding until end of
/// turn." One land on the board pays the generic cost, and the ability is
/// offered at all only because it is affordable; the target is chosen
/// before either half of the cost is paid (CR 602.2b carries the casting
/// steps of 601.2b–i over to activating an ability, target announcement
/// 601.2c before total-cost payment 601.2h), and only once both are paid
/// does the ability resolve and the target's projected keywords carry
/// banding.
#[test]
fn helm_of_chatzuk_costs_one_and_taps_itself_to_grant_banding() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[helm_of_chatzuk(), forest(), hill_giant()])
        .battlefield(1, &[hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let helm = on_battlefield(&engine, p0, helm_of_chatzuk()).expect("seated");
    let giant = on_battlefield(&engine, p0, hill_giant()).expect("seated");
    let their_giant = on_battlefield(&engine, p1, hill_giant()).expect("seated");
    assert!(
        !keywords(&engine, giant).contains(KeywordSet::BANDING),
        "the Giant prints no banding of its own"
    );
    assert!(!is_tapped(&engine, helm));

    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "one land on the board: the ability is offered on exactly {{1}}"
    );

    activate(&mut engine, p0, helm_of_chatzuk(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0);
    assert_eq!((min, max), (1, 1), "one target");
    assert!(
        options.contains(&giant),
        "\"target creature\": no restriction to a creature you control"
    );
    assert!(
        options.contains(&their_giant),
        "\"target creature\": an opponent's creature is on offer too"
    );
    assert!(
        !is_tapped(&engine, helm),
        "CR 602.2b/601.2c before 601.2h: the target is named before the {{T}} is paid"
    );
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "and before the mana leaves the pool"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .expect("the Giant was one of the options the ability enumerated");
    assert!(is_tapped(&engine, helm), "the {{T}} was part of the price");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "and the {{1}} came out of the pool"
    );

    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, giant).contains(KeywordSet::BANDING),
        "\"target creature gains banding\""
    );
}

/// Helm of Chatzuk's grant is not just a keyword sitting on the
/// projection: a creature it never touches cannot lead a band (CR 508.1e
/// asks only creatures with banding), and one it has just granted banding
/// to, can — in the very turn the ability resolved.
#[test]
fn helm_of_chatzuks_grant_lets_a_creature_lead_a_band_this_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    for granted in [false, true] {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[helm_of_chatzuk(), forest(), hill_giant(), llanowar_elves()],
            )
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let giant = on_battlefield(&engine, p0, hill_giant()).expect("seated");
        let elf = on_battlefield(&engine, p0, llanowar_elves()).expect("seated");

        if granted {
            // Not `tap_all_mana`: the Elf is a mana source too, and it still
            // has to be an untapped attacker once the ability resolves.
            tap_mana_except(&mut engine, p0, elf);
            activate(&mut engine, p0, helm_of_chatzuk(), 0);
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseTargets {
                        objects: vec![giant],
                        players: vec![],
                    },
                )
                .expect("the Giant is a legal target");
            pass_until(&mut engine, stack_is_empty);
            assert!(
                keywords(&engine, giant).contains(KeywordSet::BANDING),
                "granted for this turn"
            );
        }

        pass_until(&mut engine, |e| {
            matches!(e.pending(), Pending::ChooseAttackers { .. })
        });
        engine
            .apply(
                p0,
                PlayerAction::DeclareAttackers {
                    attackers: vec![(giant, Defender::Player(p1)), (elf, Defender::Player(p1))],
                },
            )
            .expect("both may attack");

        let asked = matches!(
            engine.pending(),
            Pending::ChooseCards {
                prompt: crate::choice::ChoicePrompt::Band { with },
                ..
            } if *with == giant
        );
        assert_eq!(
            asked,
            granted,
            "a band question names the Giant as a leader only once it has \
             banding: {:?}",
            engine.pending()
        );
    }
}

/// Helm of Chatzuk's grant is a duration, not a body the target keeps: one
/// turn later the same creature has lost banding again, while the
/// creature itself is still standing.
#[test]
fn helm_of_chatzuks_grant_is_gone_by_the_targets_controllers_next_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[helm_of_chatzuk(), forest(), hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let giant = on_battlefield(&engine, p0, hill_giant()).expect("seated");
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, helm_of_chatzuk(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![giant],
                players: vec![],
            },
        )
        .expect("the Giant is a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert!(
        keywords(&engine, giant).contains(KeywordSet::BANDING),
        "granted for this turn"
    );

    reach_their_main_phase(&mut engine, p1);
    assert!(
        !keywords(&engine, giant).contains(KeywordSet::BANDING),
        "\"until end of turn\": gone already by the opponent's own turn, not \
         held over to \"your next turn\""
    );

    reach_their_main_phase(&mut engine, p0);
    assert!(
        !keywords(&engine, giant).contains(KeywordSet::BANDING),
        "\"until end of turn\": the grant lasted the turn it was made in and no longer"
    );
    assert!(
        on_battlefield(&engine, p0, hill_giant()).is_some(),
        "and the creature is still standing, so the keyword left rather than the creature"
    );
}
