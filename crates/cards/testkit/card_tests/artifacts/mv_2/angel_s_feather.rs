//! Oracle: whenever any player casts a white spell, you may gain 1 life.
#[allow(clippy::wildcard_imports)]
use super::*;
fn angel_s_feather() -> CardIndex {
    card_index("3d0d4ba4-d5aa-4d9f-89ab-46687637fecf")
}
/// Checklist: red spell excluded; white spells cast by either player trigger; controller may decline/accept; life gain precedes spell resolution.
#[test]
fn optional_life_trigger_tracks_white_spell_cast_by_any_player() {
    let (p, q) = (PlayerId::new(0), PlayerId::new(1));
    let healing_hands = CardIndex::new(15_002);
    let mut g = Duel::new(SEED, plains())
        .battlefield(0, &[angel_s_feather(), plains(), plains(), plains()])
        .battlefield(1, &[mountain(), mountain(), plains(), plains(), plains()])
        .hand(0, &[healing_hands])
        .hand(1, &[lightning_bolt(), healing_hands])
        .life(0, 10)
        .life(1, 10)
        .start();
    keep_mulligans(&mut g);
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
    pass_until(&mut g, stack_is_empty);
    assert_eq!(
        g.state().players[0].life,
        10,
        "a red spell does not trigger the Feather"
    );

    // The opponent casts a white spell. The Feather's controller is asked;
    // declining leaves life unchanged while Healing Hands remains on stack.
    cast_from_hand(&mut g, q, healing_hands);
    g.apply(q, PlayerAction::ChoosePlayer(q)).unwrap();
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
    assert!(on_stack(&g, healing_hands).is_some());
    assert_eq!(g.state().players[0].life, 10);
    assert_eq!(g.state().players[1].life, 7);
    g.apply(p, PlayerAction::YesNo(false)).unwrap();
    assert!(on_stack(&g, healing_hands).is_some());
    assert_eq!(g.state().players[0].life, 10);
    pass_until(&mut g, stack_is_empty);
    assert_eq!(g.state().players[1].life, 11);

    pass_until(&mut g, |e| {
        e.state().turn.active == p && e.state().turn.phase == Phase::FirstMain
    });
    cast_from_hand(&mut g, p, healing_hands);
    g.apply(p, PlayerAction::ChoosePlayer(p)).unwrap();
    pass_until(&mut g, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    g.apply(p, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut g, |e| e.state().players[0].life == 11);
    assert!(
        on_stack(&g, healing_hands).is_some(),
        "the accepted trigger resolves before the white spell"
    );
    pass_until(&mut g, stack_is_empty);
    assert_eq!(g.state().players[0].life, 15);
}
