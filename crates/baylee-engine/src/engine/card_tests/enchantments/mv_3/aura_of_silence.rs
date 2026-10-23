//! `cards/enchantments/mv_3/aura_of_silence.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::index::{GRIZZLY_BEARS, ROD_OF_RUIN};

fn aura_of_silence() -> CardIndex {
    card_index("e7faf8eb-e829-4109-8dfe-42865a23ba86")
}

/// `Aura of Silence` ({1}{W}{W}, Enchantment) sacrifices itself to destroy
/// target artifact or enchantment.
#[test]
fn aura_of_silence_taxes_opponent_artifacts_and_sacrifices_to_destroy_artifact() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[plains(), plains(), plains()])
        .battlefield(1, &[ROD_OF_RUIN])
        .hand(0, &[aura_of_silence()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, aura_of_silence());
    pass_until(&mut engine, stack_is_empty);

    let rod = on_battlefield(&engine, p1, ROD_OF_RUIN).unwrap();
    activate(&mut engine, p0, aura_of_silence(), 1);
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![rod],
                players: vec![],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);

    assert!(in_graveyard(&engine, p0, aura_of_silence()).is_some());
    assert!(in_graveyard(&engine, p1, ROD_OF_RUIN).is_some());
}

/// "Artifact and enchantment spells your **opponents** cast cost {2} more":
/// the opponent's Sol Ring costs {3} (two floating are not enough, three are),
/// their Llanowar Elves are not taxed, and the Aura's own controller pays
/// the printed {1} for the same Sol Ring.
#[test]
fn aura_of_silence_taxes_only_the_opponents_artifact_spells() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[aura_of_silence(), plains()])
        .battlefield(1, &[forest(), forest()])
        .hand(0, &[quiet_artifact()])
        .hand(1, &[quiet_artifact(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    assert!(
        lk::castable(&engine, p0, quiet_artifact()),
        "the Aura's controller pays {{1}} for an artifact"
    );

    reach_their_main_phase(&mut engine, p1);
    tap_all_mana(&mut engine, p1);
    assert_eq!(lk::pool_total(&engine, p1), 2);
    assert!(
        lk::castable(&engine, p1, llanowar_elves()),
        "creature spells are not taxed"
    );
    assert!(
        !lk::castable(&engine, p1, quiet_artifact()),
        "{{1}} and the {{2}} tax is three mana, and two are floating"
    );
    lk::float(&mut engine, p1, &[(ManaColor::Colorless, 1)]);
    assert!(lk::castable(&engine, p1, quiet_artifact()));
    cast_with_floating(&mut engine, p1, quiet_artifact());
    assert_eq!(
        lk::pool_total(&engine, p1),
        0,
        "all three were spent: the {{1}} and the {{2}} tax"
    );
}

/// "Target artifact or enchantment": the menu holds the opponent's Rod of Ruin
/// and not a creature of either side.
#[test]
fn aura_of_silence_destroy_menu_is_artifacts_and_enchantments_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, plains())
        .battlefield(0, &[aura_of_silence(), GRIZZLY_BEARS])
        .battlefield(1, &[ROD_OF_RUIN, GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let rod = on_battlefield(&engine, p1, ROD_OF_RUIN).unwrap();
    let mine = on_battlefield(&engine, p0, GRIZZLY_BEARS).unwrap();
    let theirs = on_battlefield(&engine, p1, GRIZZLY_BEARS).unwrap();

    activate(&mut engine, p0, aura_of_silence(), 1);
    let menu = aim_at(&mut engine, p0, rod);
    assert!(menu.contains(&rod));
    assert!(
        !menu.contains(&mine) && !menu.contains(&theirs),
        "no creatures"
    );
    pass_until(&mut engine, stack_is_empty);
    assert!(in_graveyard(&engine, p1, ROD_OF_RUIN).is_some());
    assert!(on_battlefield(&engine, p1, GRIZZLY_BEARS).is_some());
}
