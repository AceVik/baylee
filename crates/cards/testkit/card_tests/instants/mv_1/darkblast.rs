//! `cards/instants/mv_1/darkblast.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_card_testkit::card_tests::draw_kit as kit;
use baylee_card_testkit::card_tests::legends_kit as lk;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

// oracle_id = "64446130-46d0-4129-a718-ed6a04218100"
fn darkblast() -> CardIndex {
    card_index("64446130-46d0-4129-a718-ed6a04218100")
}

/// Darkblast ({B}): target creature gets -1/-1 until end of turn (killing a
/// 1/1 and shrinking a 2/2 until cleanup).
#[test]
fn darkblast_shrinks_target_creature_until_end_of_turn() {
    let mut engine = lk::game(
        SEED,
        &[],
        &[index::GRIZZLY_BEARS, index::LLANOWAR_ELVES],
        &[darkblast(), darkblast()],
        &[],
    );
    let bears = lk::obj(&engine, P1, index::GRIZZLY_BEARS);
    let elf = lk::obj(&engine, P1, index::LLANOWAR_ELVES);

    lk::float(&mut engine, P0, &[(ManaColor::Black, 2)]);
    cast_with_floating(&mut engine, P0, darkblast());
    lk::aim(&mut engine, &[elf], &[]);
    lk::settle(&mut engine);
    assert!(in_graveyard(&engine, P1, index::LLANOWAR_ELVES).is_some());

    cast_with_floating(&mut engine, P0, darkblast());
    lk::aim(&mut engine, &[bears], &[]);
    lk::settle(&mut engine);
    assert_eq!(pt(&engine, bears), (1, 1));

    reach_their_main_phase(&mut engine, P1);
    assert_eq!(pt(&engine, bears), (2, 2), "-1/-1 wore off at end of turn");
}

/// Dredge 3 (CR 702.52a): when drawing with Darkblast in graveyard, you may
/// mill 3 and return Darkblast to hand instead of drawing, or decline to draw.
#[test]
fn darkblast_dredge_3_mills_three_and_returns_to_hand_or_declines() {
    for dredge in [true, false] {
        let mut engine = kit::duel(4303, &kit::tome_board(), &[darkblast()], &[], &[]);
        hand_to_graveyard(&mut engine, kit::P0, darkblast());
        let buried = in_graveyard(&engine, kit::P0, darkblast()).unwrap();
        let library = kit::top(&engine, kit::P0, 60);
        tap_all_mana(&mut engine, kit::P0);
        kit::tome_draw(&mut engine);

        let asked = kit::question(&engine);
        assert_eq!(
            (
                asked.player,
                asked.options,
                asked.min,
                asked.max,
                asked.prompt
            ),
            (kit::P0, vec![buried], 0, 1, ChoicePrompt::Dredge)
        );
        if dredge {
            kit::reply(&mut engine, &[buried]);
            kit::run_stack(&mut engine);
            assert_eq!(kit::hand(&engine, kit::P0), vec![buried]);
            assert_eq!(
                kit::sorted(&kit::graveyard(&engine, kit::P0)),
                kit::sorted(&library[..3]),
                "milled top 3 cards"
            );
            assert_eq!(kit::drawn_this_turn(&engine, kit::P0), 0);
        } else {
            kit::reply(&mut engine, &[]);
            kit::run_stack(&mut engine);
            assert_eq!(kit::hand(&engine, kit::P0), vec![library[0]]);
            assert_eq!(kit::graveyard(&engine, kit::P0), vec![buried]);
        }
    }
}
