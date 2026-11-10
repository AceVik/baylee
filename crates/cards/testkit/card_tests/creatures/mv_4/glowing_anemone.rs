//! `cards/creatures/mv_4/glowing_anemone.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "52305f9d-2bfd-4b35-92b3-c08cdd758cde"

/// Glowing Anemone — {3}{U}, a 1/3 Jellyfish Beast: "When this creature
/// enters, you may return target land to its owner's hand."
///
/// "Target land" names neither a controller nor a side of the table, so lands
/// stand on both: four Islands pay the card and a Forest waits across the
/// table, and the trigger's offer is read before it is answered. Answering it
/// with that Forest is what tells "to its owner's hand" from "to your hand" —
/// one seat aims the bounce and the card goes home to the other — and the "you
/// may" arrives as its own `MayDo` question rather than as an effect that
/// simply happens. The Anemone is on the battlefield while the offer is open
/// and on no part of it, because a creature is no land.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn glowing_anemone_bounces_a_land_of_either_seat_to_its_owners_hand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[island(), island(), island(), island()])
        .hand(0, &[glowing_anemone()])
        .battlefield(1, &[forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_their_main_phase(&mut engine, p0);

    assert_eq!(
        lands_of(&engine, p0).len(),
        4,
        "four Islands pay the {{3}}{{U}}, and they are the only lands this seat has"
    );
    let their_land = on_battlefield(&engine, p1, forest()).expect("their Forest is out");

    cast_from_hand(&mut engine, p0, glowing_anemone());

    // One trigger asks two things — which land (CR 603.3d) and whether to take
    // the offer at all — and they are answered in the order they arrive rather
    // than in the order they are expected.
    let mut menu: Option<Vec<ObjectId>> = None;
    let mut asked_permission = false;
    for _ in 0..20 {
        if menu.is_some() && asked_permission {
            break;
        }
        match engine.pending().clone() {
            Pending::ChooseTargets {
                player,
                options,
                min,
                max,
                ..
            } => {
                assert_eq!(player, p0, "the Anemone's controller names the land");
                assert_eq!((min, max), (1, 1), "one land, and the trigger asks once");
                menu = Some(options);
                engine
                    .apply(
                        p0,
                        PlayerAction::ChooseObjects {
                            objects: vec![their_land],
                        },
                    )
                    .expect("the Forest across the table was one of the options");
            }
            Pending::YesNo { player, prompt, .. } => {
                assert_eq!(
                    prompt,
                    YesNoPrompt::MayDo,
                    "\"you may return …\" is a question and not a command"
                );
                assert_eq!(player, p0, "the seat that controls the trigger answers it");
                engine
                    .apply(player, PlayerAction::YesNo(true))
                    .expect("yes is one of the two legal answers");
                asked_permission = true;
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Anemone's trigger resolves: {other:?}"),
        }
    }
    pass_until(&mut engine, stack_is_empty);

    let menu = menu.expect("the enters-trigger asks for a target land");
    assert!(
        asked_permission,
        "and it asks permission first: \"you may\""
    );
    let anemone = on_battlefield(&engine, p0, glowing_anemone()).expect("the Anemone resolved");
    assert_eq!(pt(&engine, anemone), (1, 3), "the body the card prints");
    assert!(
        !menu.contains(&anemone),
        "the trigger's own source is a creature and no land: {menu:?}"
    );
    assert!(
        menu.contains(&their_land),
        "\"target land\" is not \"a land you control\": the Forest across the \
         table is on the menu: {menu:?}"
    );
    let my_lands = lands_of(&engine, p0);
    assert_eq!(my_lands.len(), 4, "four Islands, still standing");
    assert!(
        my_lands.iter().all(|id| menu.contains(id)),
        "and every one of them came off the same question: {menu:?}"
    );
    assert_eq!(
        menu.len(),
        5,
        "the four Islands and the one Forest — nothing else on the board is a \
         land: {menu:?}"
    );

    assert!(
        on_battlefield(&engine, p1, forest()).is_none(),
        "the land the trigger named left the battlefield"
    );
    assert!(
        in_hand(&engine, p1, forest()).is_some(),
        "\"to its owner's hand\": the Forest went back to the seat that owns it"
    );
    assert!(
        in_hand(&engine, p0, forest()).is_none(),
        "and never to the hand of the seat that aimed the bounce"
    );
    assert!(
        in_graveyard(&engine, p1, forest()).is_none(),
        "a permanent returned to a hand is no graveyard card"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        4,
        "the lands the trigger did not name never moved"
    );
}
