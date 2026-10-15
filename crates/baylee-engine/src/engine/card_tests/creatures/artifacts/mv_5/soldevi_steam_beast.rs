//! `cards/creatures/artifacts/mv_5/soldevi_steam_beast.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn soldevi_steam_beast() -> CardIndex {
    card_index("935728dd-ad97-47a9-97ca-068af7960d07")
}

/// Soldevi Steam Beast — "Whenever this creature becomes tapped, target
/// opponent gains 2 life." Attacking taps it (no vigilance): the opponent
/// gains 2 and then takes the 4 combat damage.
#[test]
fn soldevi_steam_beast_gives_an_opponent_two_life_when_it_becomes_tapped() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[soldevi_steam_beast()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let beast = on_battlefield(&engine, p0, soldevi_steam_beast()).expect("seated");
    assert_eq!(pt(&engine, beast), (4, 2));
    let (mine, theirs) = (life_of(&engine, 0), life_of(&engine, 1));

    attack_alone(&mut engine, p0, beast, p1);
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!(
            "the trigger asks for its target, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(
        player_options,
        vec![p1],
        "\"target opponent\": not the controller"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("the opponent is the target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        life_of(&engine, 1),
        theirs + 2,
        "the trigger gave the opponent 2"
    );
    through_combat(&mut engine);
    assert_eq!(life_of(&engine, 1), theirs + 2 - 4, "then 4 damage");
    assert_eq!(life_of(&engine, 0), mine, "the controller gains nothing");
}

/// "{2}: Regenerate this creature."
#[test]
fn soldevi_steam_beast_regenerates_for_two() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[soldevi_steam_beast(), forest(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let beast = on_battlefield(&engine, p0, soldevi_steam_beast()).expect("seated");
    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, beast, 1);
    kill(&mut engine, beast);
    assert_eq!(
        engine.state().object(beast).map(|o| o.zone),
        Some(Zone::Battlefield),
        "the shield saved it"
    );
}
