//! `cards/enchantments/mv_3/endbringer_s_revel.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

fn endbringer_s_revel() -> CardIndex {
    card_index("4ca2511d-76f0-470c-b522-b75fb08932d9")
}

#[test]
fn endbringers_revel_lets_any_player_return_creature_card_from_graveyard_as_a_sorcery() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, swamp())
        .battlefield(
            0,
            &[endbringer_s_revel(), swamp(), swamp(), swamp(), swamp()],
        )
        .battlefield(1, &[forest(), forest(), forest(), forest()])
        .hand(0, &[index::GRIZZLY_BEARS])
        .hand(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    hand_to_graveyard(&mut engine, p0, index::GRIZZLY_BEARS);
    hand_to_graveyard(&mut engine, p1, llanowar_elves());

    let revel = on_battlefield(&engine, p0, endbringer_s_revel()).unwrap();
    let bears = in_graveyard(&engine, p0, index::GRIZZLY_BEARS).unwrap();

    // Controller activates during their main phase to return Grizzly Bears.
    tap_all_mana(&mut engine, p0);
    activate(&mut engine, p0, endbringer_s_revel(), 0);
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p0, index::GRIZZLY_BEARS).is_some());

    // Opponent activates Endbringer's Revel during opponent's main phase to return Llanowar Elves.
    reach_their_main_phase(&mut engine, p1);
    let elves = in_graveyard(&engine, p1, llanowar_elves()).unwrap();
    tap_all_mana(&mut engine, p1);
    engine
        .apply(
            p1,
            PlayerAction::ActivateAbility {
                source: revel,
                ability_index: 0,
            },
        )
        .unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(in_hand(&engine, p1, llanowar_elves()).is_some());
}
