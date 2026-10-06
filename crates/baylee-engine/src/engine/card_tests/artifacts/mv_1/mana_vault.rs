//! `cards/artifacts/mv_1/mana_vault.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Mana Vault's `Coverage::Partial` note leaves three printed sentences
/// live: it does not untap during its controller's untap step, it taps for
/// {C}{C}{C}, and its draw-step trigger charges a *tapped* Vault one life.
/// One turn cycle reads all three at once, because each is what keeps the
/// others honest — the Forests beside it coming back in the same step proves
/// the untap step really ran (CR 502.3) rather than the game never
/// advancing, the three colourless are the mana ability landing with no
/// stack (CR 605.3b), and the life p0 is missing on the following draw step
/// fires only because the artifact is *still* tapped. The `{4}` upkeep untap
/// payment is declined here; `mana_vault_untaps_for_four_at_upkeep` pays it.
#[test]
fn mana_vault_taps_for_three_never_untaps_and_bites_its_controller_on_the_draw_step() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(5171, forest())
        .battlefield(0, &[mana_vault(), forest(), forest(), forest()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut engine);
    decline_the_vaults_upkeep(&mut engine);
    reach_main_phase(&mut engine, p0);

    let vault = on_battlefield(&engine, p0, mana_vault()).expect("the Vault is on the table");
    let forests = all_on_battlefield(&engine, p0, forest());
    assert_eq!(forests.len(), 3, "three Forests were dealt beside it");

    // `tap_all_mana` takes both lists (#159), so the Vault has to be named
    // as the one thing kept back — its {T} is the printed ability this test
    // activates by index, and it is the permanent that must still be tapped
    // on the draw step below.
    tap_all_mana_but(&mut engine, p0, Some(mana_vault()));
    activate(&mut engine, p0, mana_vault(), 1);
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        3,
        "{{T}}: Add {{C}}{{C}}{{C}}, in the pool the moment it is activated"
    );
    assert!(is_tapped(&engine, vault), "which tapped the Vault");
    assert!(
        forests.iter().all(|id| is_tapped(&engine, *id)),
        "and the Forests were tapped in the same turn"
    );

    // Across the opponent's turn and back. The Vault's draw-step trigger
    // fires on p0's *own* draw step, so the one question this walk can meet
    // is who the damage is aimed at — answered with p0, which is what "you"
    // means on the card.
    reach_their_main_phase(&mut engine, p1);
    let mut reached_next_main = false;
    for _ in 0..300 {
        if matches!(engine.state().turn.phase, Phase::FirstMain)
            && engine.state().turn.active == p0
            && matches!(engine.pending(), Pending::Priority { player, .. } if *player == p0)
        {
            reached_next_main = true;
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseAttackers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareAttackers { attackers: vec![] })
                    .unwrap();
            }
            Pending::ChooseBlockers { player, .. } => {
                engine
                    .apply(player, PlayerAction::DeclareBlockers { blockers: vec![] })
                    .unwrap();
            }
            Pending::ChooseTargets {
                player,
                options,
                player_options,
                ..
            } => {
                let players: Vec<PlayerId> = player_options.into_iter().take(1).collect();
                let objects = if players.is_empty() {
                    options.into_iter().take(1).collect()
                } else {
                    Vec::new()
                };
                engine
                    .apply(player, PlayerAction::ChooseTargets { objects, players })
                    .unwrap();
            }
            // The upkeep's "you may pay {4}", declined: this test is about
            // the Vault staying tapped.
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana: 4 },
                ..
            } => {
                engine.apply(player, PlayerAction::YesNo(false)).unwrap();
            }
            other => panic!("unexpected on the way to the next turn: {other:?}"),
        }
    }
    assert!(reached_next_main, "the game walks a whole turn cycle");

    assert!(
        forests.iter().all(|id| !is_tapped(&engine, *id)),
        "the untap step ran: every Forest came back"
    );
    assert!(
        is_tapped(&engine, vault),
        "and the Vault alone stayed down — \"This artifact doesn't untap \
         during your untap step\" (CR 502.3), an effect rather than a \
         characteristic"
    );
    assert_eq!(
        engine.state().players[0].life,
        19,
        "\"At the beginning of your draw step, if this artifact is tapped, it \
         deals 1 damage to you\" — the life is gone only because the untap \
         step left the artifact tapped"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "and the damage belongs to the Vault's controller, not the opponent"
    );
}

/// Mana Vault — "At the beginning of your upkeep, you may pay {4}. If you
/// do, untap this artifact." Tapped on p0's first turn, it is asked about at
/// p0's next upkeep; four green floated from the Forests pay for it, the
/// Vault untaps, and so the draw step that follows charges no life.
#[test]
fn mana_vault_untaps_for_four_at_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(5173, forest())
        .battlefield(0, &[mana_vault(), forest(), forest(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    decline_the_vaults_upkeep(&mut engine);
    reach_main_phase(&mut engine, p0);
    let vault = on_battlefield(&engine, p0, mana_vault()).expect("the Vault is out");
    activate(&mut engine, p0, mana_vault(), 1);
    assert!(is_tapped(&engine, vault));

    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0
            && e.state().turn.step == crate::turn::Step::Upkeep
            && !stack_is_empty(e)
    });
    assert!(is_tapped(&engine, vault), "its own untap step left it down");
    tap_all_mana_but(&mut engine, p0, Some(mana_vault()));
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayTax { mana: 4 },
                ..
            }
        )
    });
    let life = engine.state().players[0].life;
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    assert!(!is_tapped(&engine, vault), "paid, so it untaps");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "{{4}} spent"
    );
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == crate::turn::Step::Main
    });
    assert_eq!(
        engine.state().players[0].life,
        life,
        "untapped by the draw step, so it deals no damage"
    );
}
