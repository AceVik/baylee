//! `cards/artifacts/mv_4/vedalken_orrery.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn vedalken_orrery() -> CardIndex {
    card_index("60d640a4-032b-4279-ac75-8f200ca776fb")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn kodama_s_reach() -> CardIndex {
    card_index("1593ea18-2f2f-4ab4-83fb-6ccc0bec8a90")
}

/// Vedalken Orrery (60d640a4-032b-4279-ac75-8f200ca776fb) — {4} Artifact:
/// "You may cast spells as though they had flash."
///
/// Lets YOU cast creature and sorcery spells on the opponent's turn, while the
/// opponent still cannot cast sorcery-speed spells on your turn.
#[test]
fn vedalken_orrery_grants_flash_to_your_spells_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                vedalken_orrery(),
                forest(),
                forest(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[grizzly_bears(), kodama_s_reach()])
        .battlefield(1, &[forest(), forest()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // 1. On p0's turn, pass priority to p1: opponent does NOT get flash from your Orrery.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert_eq!(engine.pending().asked(), Some(p1));
    let their_bears = in_hand(&engine, p1, grizzly_bears()).unwrap();
    tap_all_mana(&mut engine, p1);
    assert!(
        !priority_offer(&engine).castable.contains(&their_bears),
        "Vedalken Orrery grants flash to YOU, not your opponent"
    );

    // 2. On p1's turn, p1 passes priority to p0: p0 casts both a creature and a sorcery!
    reach_their_main_phase(&mut engine, p1);
    engine.apply(p1, PlayerAction::PassPriority).unwrap();
    assert_eq!(engine.pending().asked(), Some(p0));

    let my_bears = in_hand(&engine, p0, grizzly_bears()).unwrap();
    let my_reach = in_hand(&engine, p0, kodama_s_reach()).unwrap();
    tap_all_mana(&mut engine, p0);
    let castable = &priority_offer(&engine).castable;
    assert!(
        castable.contains(&my_bears) && castable.contains(&my_reach),
        "both creature and sorcery spells are castable on opponent's turn"
    );

    cast_with_floating(&mut engine, p0, grizzly_bears());
    pass_until(&mut engine, stack_is_empty);
    assert!(on_battlefield(&engine, p0, grizzly_bears()).is_some());

    if engine.pending().asked() == Some(p1) {
        engine.apply(p1, PlayerAction::PassPriority).unwrap();
    }
    cast_with_floating(&mut engine, p0, kodama_s_reach());
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![] })
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(my_reach).unwrap().zone,
        Zone::Graveyard,
        "sorcery resolved on opponent's turn"
    );
}
