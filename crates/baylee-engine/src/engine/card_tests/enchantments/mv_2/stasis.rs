//! `cards/enchantments/mv_2/stasis.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Stasis: "Players skip their untap steps." A permanent tapped during the
/// turn Stasis is cast is still tapped on its controller's next turn; the
/// counter-check is the identical board without Stasis, where the same land
/// untaps on schedule.
#[test]
fn stasis_skips_every_players_untap_step() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), forest()])
        .hand(0, &[stasis()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let island_id = on_battlefield(&engine, p0, island()).expect("the Island is seated");
    cast_from_hand(&mut engine, p0, stasis());
    pass_until(&mut engine, |e| on_battlefield(e, p0, stasis()).is_some());
    assert!(
        is_tapped(&engine, island_id),
        "the Island paid Stasis's {{1}}{{U}}"
    );

    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == crate::turn::Step::Upkeep
    });
    // The untap step always comes before the upkeep step, so reaching p0's
    // upkeep is itself proof their untap step already came and went.
    assert!(
        is_tapped(&engine, island_id),
        "Stasis skips every player's untap step: the Island that paid for \
         it is still tapped on p0's own next turn"
    );

    // Counter-check: the identical board, minus Stasis, untaps on schedule.
    let mut bare = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .start();
    keep_mulligans(&mut bare);
    reach_main_phase(&mut bare, p0);
    let bare_island = on_battlefield(&bare, p0, island()).expect("the Island is seated");
    bare.apply(
        p0,
        PlayerAction::ActivateManaAbility {
            source: bare_island,
        },
    )
    .unwrap();
    assert!(is_tapped(&bare, bare_island), "tapped for its own mana");

    reach_their_main_phase(&mut bare, p1);
    pass_until(&mut bare, |e| {
        e.state().turn.active == p0 && e.state().turn.step == crate::turn::Step::Upkeep
    });
    assert!(
        !is_tapped(&bare, bare_island),
        "without Stasis the same land untaps on the controller's next turn"
    );
}

/// Stasis: "At the beginning of your upkeep, sacrifice this enchantment
/// unless you pay {U}." Paying keeps it on the battlefield; declining sends
/// it to the graveyard.
#[test]
fn stasis_upkeep_trigger_pays_u_or_sacrifices_itself() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);

    // Paying {U} keeps Stasis.
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), forest(), island()])
        .hand(0, &[stasis()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let islands = all_on_battlefield(&engine, p0, island());
    assert_eq!(islands.len(), 2, "two Islands are seated");
    let reserved = islands[0];
    tap_mana_except(&mut engine, p0, reserved);
    cast_with_floating(&mut engine, p0, stasis());
    pass_until(&mut engine, |e| on_battlefield(e, p0, stasis()).is_some());

    reach_their_main_phase(&mut engine, p1);
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    let Pending::YesNo { player, prompt, .. } = engine.pending().clone() else {
        unreachable!("pass_until stopped on the question")
    };
    assert_eq!(player, p0, "\"your upkeep\" asks Stasis's own controller");
    assert_eq!(
        prompt,
        YesNoPrompt::PayMana {
            cost: baylee_core::mana!("{U}")
        }
    );

    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    tap_all_mana(&mut engine, p0);
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(
        on_battlefield(&engine, p0, stasis()).is_some(),
        "paying {{U}} keeps Stasis on the battlefield"
    );

    // Declining sacrifices it.
    let mut decline = Duel::new(SEED, forest())
        .battlefield(0, &[island(), forest()])
        .hand(0, &[stasis()])
        .start();
    keep_mulligans(&mut decline);
    reach_main_phase(&mut decline, p0);
    cast_from_hand(&mut decline, p0, stasis());
    pass_until(&mut decline, |e| on_battlefield(e, p0, stasis()).is_some());

    reach_their_main_phase(&mut decline, p1);
    pass_until(&mut decline, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    decline.apply(p0, PlayerAction::YesNo(false)).unwrap();
    assert!(
        on_battlefield(&decline, p0, stasis()).is_none(),
        "declining sacrifices Stasis"
    );
    assert!(
        in_graveyard(&decline, p0, stasis()).is_some(),
        "sacrificed means the graveyard, not gone from the game"
    );
}

/// "Players skip their untap steps" — the opponent's too — and the upkeep
/// payment is "your upkeep" only. Stasis cast by p0 leaves p1's tapped Forest
/// and tapped creature tapped through p1's own untap step, while p1's upkeep
/// asks nobody for {U}: the walk to p1's main phase would stop on the
/// payment question otherwise.
#[test]
fn stasis_keeps_the_opponents_permanents_tapped_and_asks_only_its_controller_to_pay() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), forest()])
        .battlefield(1, &[forest(), quiet_creature()])
        .hand(0, &[stasis()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    cast_from_hand(&mut engine, p0, stasis());
    pass_until(&mut engine, |e| on_battlefield(e, p0, stasis()).is_some());

    let land = on_battlefield(&engine, p1, forest()).expect("their Forest");
    let creature = on_battlefield(&engine, p1, quiet_creature()).expect("their creature");
    for id in [land, creature] {
        engine
            .dev_state_mut(p1)
            .expect("the harness may set boards up")
            .object_mut(id)
            .expect("seated")
            .status
            .insert(Status::TAPPED);
    }
    engine.refresh_offer();

    reach_their_main_phase(&mut engine, p1);
    assert!(
        is_tapped(&engine, land) && is_tapped(&engine, creature),
        "p1's untap step was skipped: both are still tapped in p1's main phase"
    );
    assert!(
        on_battlefield(&engine, p0, stasis()).is_some(),
        "no payment was asked of anyone during p1's upkeep"
    );
}
