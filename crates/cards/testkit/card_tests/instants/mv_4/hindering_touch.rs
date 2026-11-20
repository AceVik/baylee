//! `cards/instants/mv_4/hindering_touch.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;
use baylee_engine::choice::TargetPrompt;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

/// Hindering Touch — {3}{U} Instant:
/// "Counter target spell unless its controller pays {2}.
/// Storm (When you cast this spell, copy it for each spell cast before it this turn. You may choose new targets for the copies.)"
#[test]
fn hindering_touch_taxes_each_copy_independently_and_retargets_second_spell() {
    let touch = card_index("90c6f338-fc41-4499-8f39-1cd202a2cd31");
    let mut engine = Duel::new(SEED, forest())
        .hand(
            0,
            &[
                index::DARK_RITUAL,
                index::GRIZZLY_BEARS,
                index::LIGHTNING_BOLT,
            ],
        )
        .hand(1, &[touch])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, P0);

    {
        let state = engine.dev_state_mut(P0).unwrap();
        state.players[0].mana_pool.add(ManaColor::Black, 1);
        state.players[0].mana_pool.add(ManaColor::Green, 2);
        state.players[0].mana_pool.add(ManaColor::Red, 1);
        // Exactly {2} left over after casting Ritual + Bears + Bolt: enough to pay ONE {2} tax.
        state.players[0].mana_pool.add(ManaColor::Colorless, 2);
        state.players[1].mana_pool.add(ManaColor::Blue, 4);
    }
    engine.refresh_offer();

    // Spell 1: Dark Ritual (resolves).
    cast_with_floating(&mut engine, P0, index::DARK_RITUAL);
    pass_until(&mut engine, stack_is_empty);

    // Drain Dark Ritual mana so P0 pays Bears cleanly with {G}{G}, then Bolt with {R},
    // leaving exactly {C}{C} to pay only ONE {2} tax.
    {
        let state = engine.dev_state_mut(P0).unwrap();
        state.players[0].mana_pool = baylee_core::mana::ManaPool::default();
        state.players[0].mana_pool.add(ManaColor::Green, 2);
    }
    engine.refresh_offer();

    // Spell 2: Grizzly Bears; Spell 3 (in response): Lightning Bolt at P1.
    let bears = in_hand(&engine, P0, index::GRIZZLY_BEARS).unwrap();
    cast_with_floating(&mut engine, P0, index::GRIZZLY_BEARS);
    {
        let state = engine.dev_state_mut(P0).unwrap();
        state.players[0].mana_pool.add(ManaColor::Red, 1);
        state.players[0].mana_pool.add(ManaColor::Colorless, 2);
    }
    engine.refresh_offer();
    let bolt = in_hand(&engine, P0, index::LIGHTNING_BOLT).unwrap();
    cast_with_floating(&mut engine, P0, index::LIGHTNING_BOLT);
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![P1],
            },
        )
        .unwrap();
    engine.apply(P0, PlayerAction::PassPriority).unwrap();

    // P1 casts Hindering Touch targeting Bears -> 3 Storm copies (4 total Hindering Touches).
    cast_with_floating(&mut engine, P1, touch);
    aim_at(&mut engine, P1, bears);

    let mut retargeted = 0usize;
    let mut paid_once = false;
    let mut tax_prompts = 0usize;
    for _ in 0..80 {
        if stack_is_empty(&engine) && matches!(engine.pending(), Pending::Priority { .. }) {
            break;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::ChooseTargets {
                player,
                reason: TargetPrompt::Retarget { .. },
                ..
            } => {
                assert_eq!(player, P1);
                // Retarget 1st copy to Bolt, keep 2nd and 3rd copies on Bears.
                let objects = if retargeted == 0 { vec![bolt] } else { vec![] };
                retargeted += 1;
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseTargets {
                            objects,
                            players: vec![],
                        },
                    )
                    .unwrap();
            }
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::PayTax { mana },
                ..
            } => {
                assert_eq!(player, P0);
                assert_eq!(mana, 2);
                tax_prompts += 1;
                // Pay {2} on the first tax prompt only; decline subsequent ones (cannot pay again).
                let pay = !paid_once;
                paid_once = true;
                engine.apply(P0, PlayerAction::YesNo(pay)).unwrap();
            }
            other => panic!("unexpected pending during Hindering Touch: {other:?}"),
        }
    }
    assert_eq!(
        retargeted, 3,
        "Storm created three copies for three prior casts"
    );
    assert!(
        tax_prompts >= 2,
        "paying the first {{2}} did not excuse later copies from taxing"
    );
    assert_eq!(engine.state().per_turn.spells_cast, vec![3, 1]);
    assert!(
        on_battlefield(&engine, P0, index::GRIZZLY_BEARS).is_none()
            && in_graveyard(&engine, P0, index::GRIZZLY_BEARS).is_some(),
        "Bears was countered once P0 ran out of mana for the stacked copies"
    );
    assert!(
        in_graveyard(&engine, P0, index::LIGHTNING_BOLT).is_some(),
        "Bolt was also countered by the retargeted copy"
    );
    assert_eq!(
        engine.state().players[1].life,
        20,
        "countered Bolt dealt no damage"
    );
}
