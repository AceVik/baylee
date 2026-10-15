//! Oracle: whenever any player casts a red spell, you may gain 1 life.
#[allow(clippy::wildcard_imports)]
use super::*;
fn dragon_s_claw() -> CardIndex {
    card_index("bd8d707d-1201-42c6-bce6-fbda12ce5ea8")
}
/// Checklist: red spell cast by either player triggers; controller may decline/accept; one life per accepted trigger.
#[test]
fn optional_life_trigger_tracks_red_spell_cast_by_any_player() {
    let (p, q) = (PlayerId::new(0), PlayerId::new(1));
    let mut g = Duel::new(SEED, mountain())
        .battlefield(
            0,
            &[
                dragon_s_claw(),
                mountain(),
                mountain(),
                plains(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[lightning_bolt(), CardIndex::new(15_002)])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .life(0, 10)
        .life(1, 10)
        .start();
    keep_mulligans(&mut g);
    reach_main_phase(&mut g, p);
    cast_from_hand(&mut g, p, lightning_bolt());
    g.apply(
        p,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![q],
        },
    )
    .unwrap();
    pass_until(&mut g, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    g.apply(p, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut g, stack_is_empty);
    assert_eq!(g.state().players[0].life, 10);
    // Healing Hands is white, not red: it draws no Claw trigger.
    cast_from_hand(&mut g, p, CardIndex::new(15_002));
    g.apply(p, PlayerAction::ChoosePlayer(p)).unwrap();
    assert!(!matches!(
        g.pending(),
        Pending::YesNo {
            prompt: YesNoPrompt::MayDo,
            ..
        }
    ));
    assert!(on_stack(&g, CardIndex::new(15_002)).is_some());
    pass_until(&mut g, stack_is_empty);
    assert_eq!(g.state().players[0].life, 14);
    reach_their_main_phase(&mut g, q);
    cast_from_hand(&mut g, q, lightning_bolt());
    g.apply(
        q,
        PlayerAction::ChooseTargets {
            objects: vec![],
            players: vec![q],
        },
    )
    .unwrap();
    pass_until(&mut g, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    let Pending::YesNo { player, .. } = g.pending().clone() else {
        unreachable!()
    };
    assert_eq!(player, p);
    assert!(
        on_stack(&g, lightning_bolt()).is_some(),
        "the red spell is still waiting to resolve"
    );
    g.apply(p, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut g, |e| e.state().players[0].life == 15);
    assert!(
        on_stack(&g, lightning_bolt()).is_some(),
        "the Claw trigger resolved before the spell"
    );
    pass_until(&mut g, stack_is_empty);
    assert_eq!(g.state().players[0].life, 15);
}
