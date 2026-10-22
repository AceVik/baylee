//! `cards/creatures/mv_4/tainted_specter.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn tainted_specter() -> CardIndex {
    card_index("fc638800-6aec-4a48-ad2c-c9a5ce93c236")
}

/// Tainted Specter (`{3}{B}`, `Coverage::Partial`): 2/2 flying Specter with
/// `{1}{B}{B}, {T}` sorcery-speed ability making target player discard a card
/// and dealing 1 damage to each creature and each player.
#[test]
fn tainted_specter_discards_and_deals_1_damage_to_each_creature_and_player() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[tainted_specter(), swamp(), swamp(), swamp()])
        .battlefield(1, &[llanowar_elves()])
        .hand(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let specter = on_battlefield(&engine, p0, tainted_specter()).unwrap();
    assert!(keywords(&engine, specter).contains(KeywordSet::FLYING));

    tap_all_mana(&mut engine, p0);
    engine
        .apply(
            p0,
            PlayerAction::ActivateAbility {
                source: specter,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .unwrap();
    let bear_in_hand = in_hand(&engine, p1, grizzly_bears()).unwrap();
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![bear_in_hand],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p1, grizzly_bears()).is_some());
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "1/1 Llanowar Elves dies to 1 damage"
    );
    assert_eq!(engine.state().players[0].life, 19);
    assert_eq!(engine.state().players[1].life, 19);
}
