//! `cards/creatures/mv_3/whip_vine.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index::WIND_DRAKE;

const WHIP_VINE: &str = "4e90ddce-1765-44e4-9b3c-e576f3a1bcd4";

#[test]
fn whip_vine_blocks_flyer_taps_and_locks_it_while_whip_vine_stays_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let card = card_index(WHIP_VINE);
    let flyer_card = WIND_DRAKE;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[card, forest()])
        .battlefield(1, &[flyer_card])
        .start();
    keep_mulligans(&mut engine);

    let vine = on_battlefield(&engine, p0, card).unwrap();
    let flyer = on_battlefield(&engine, p1, flyer_card).unwrap();

    // Advance to p1's declare-attackers step; Wind Drake (2/2 flying) attacks p0.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::DeclareAttackers {
                attackers: vec![(flyer, Defender::Player(p0))],
            },
        )
        .unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });

    // Whip Vine (1/4 reach) blocks Wind Drake (2/2 flying).
    engine
        .apply(
            p0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(vine, flyer)],
            },
        )
        .unwrap();

    // Activate Whip Vine's {T} ability targeting the blocked flyer.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });
    activate(&mut engine, p0, card, 1);
    aim_at(&mut engine, p0, flyer);
    pass_until(&mut engine, stack_is_empty);
    assert!(is_tapped(&engine, vine));
    assert!(is_tapped(&engine, flyer));

    // On p0's next untap step, choose not to untap Whip Vine (ChoicePrompt::LeaveTapped).
    let mut asked = false;
    for _ in 0..100 {
        if let Pending::ChooseCards {
            options,
            prompt: ChoicePrompt::LeaveTapped,
            ..
        } = engine.pending().clone()
        {
            assert!(options.contains(&vine));
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseObjects {
                        objects: vec![vine],
                    },
                )
                .unwrap();
            asked = true;
            break;
        }
        let (player, action) = answer_one(&engine).expect("walk to untap");
        engine.apply(player, action).unwrap();
    }
    assert!(asked, "untap step should ask LeaveTapped");
    reach_main_phase(&mut engine, p0);
    assert!(is_tapped(&engine, vine));

    // On p1's next turn, Wind Drake remains tapped because Whip Vine remained tapped.
    reach_their_main_phase(&mut engine, p1);
    assert!(is_tapped(&engine, flyer));
}
