//! `cards/creatures/mv_3/everglove_courier.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

#[test]
fn everglove_courier_pumps_target_elf_while_courier_remains_tapped() {
    let p0 = PlayerId::new(0);
    let courier = card_index("31da3df3-5b79-42d0-9018-76bc73411954");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[courier, llanowar_elves(), forest(), forest(), forest()],
        )
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let courier_id = on_battlefield(&engine, p0, courier).unwrap();
    let elf = on_battlefield(&engine, p0, llanowar_elves()).unwrap();
    assert_eq!(pt(&engine, elf), (1, 1));

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    activate(&mut engine, p0, courier, 1);
    aim_at(&mut engine, p0, elf);
    pass_until(&mut engine, stack_is_empty);

    assert!(is_tapped(&engine, courier_id));
    assert_eq!(pt(&engine, elf), (3, 3));
    assert!(keywords(&engine, elf).contains(KeywordSet::TRAMPLE));

    // On p0's next untap step, choose not to untap Everglove Courier.
    for _ in 0..100 {
        if let Pending::ChooseCards {
            prompt: ChoicePrompt::LeaveTapped,
            ..
        } = engine.pending().clone()
        {
            engine
                .apply(
                    p0,
                    PlayerAction::ChooseObjects {
                        objects: vec![courier_id],
                    },
                )
                .unwrap();
            break;
        }
        let (player, action) = answer_one(&engine).expect("walk to untap");
        engine.apply(player, action).unwrap();
    }
    reach_main_phase(&mut engine, p0);
    assert!(is_tapped(&engine, courier_id));
    assert_eq!(pt(&engine, elf), (3, 3));

    // On the following turn, allow Everglove Courier to untap normally.
    reach_their_main_phase(&mut engine, PlayerId::new(1));
    reach_their_main_phase(&mut engine, p0);
    assert!(!is_tapped(&engine, courier_id));
    assert_eq!(pt(&engine, elf), (1, 1));
}
