//! `cards/creatures/mv_5/ore_gorger.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

// oracle_id = "e5a9d34b-c7d4-4682-8e26-dd82e38ee84e"

/// Ore Gorger — {3}{R}{R}, a 3/1 Spirit: "Whenever you cast a Spirit or Arcane
/// spell, you may destroy target nonbasic land."
///
/// The printed sentence is mostly a claim about the *menu*, so the board puts a
/// nonbasic land and a basic one under the same opponent: the Farmland is the
/// only thing the trigger may name, and the Forest beside it is the control
/// that says "nonbasic" was read rather than skipped. The Spirit spell is
/// Skyclave Apparition, and the game is walked only as far as the destruction —
/// the spell that triggered the ability is still on the stack when the land
/// hits the graveyard, so nothing else in this game can be mistaken for the
/// Gorger's own effect, and a trigger that had offered the menu and destroyed
/// nothing would satisfy every reading taken from the offer alone.
#[test]
#[allow(clippy::too_many_lines)] // one printed card, played end to end: the length is the card's
fn ore_gorger_destroys_the_nonbasic_land_it_names_and_never_a_basic_one() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[ore_gorger(), plains(), plains(), plains()])
        .hand(0, &[skyclave_apparition()])
        .battlefield(1, &[irrigated_farmland(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    assert!(
        on_battlefield(&engine, p0, ore_gorger()).is_some(),
        "the Gorger is on the table before anything is cast"
    );
    let farm = on_battlefield(&engine, p1, irrigated_farmland()).expect("their Farmland is out");
    let their_forest = on_battlefield(&engine, p1, forest()).expect("their Forest is out");

    // Mana before the claim: whether a spell is castable is read off the pool.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        3,
        "three Plains, which is exactly {{1}}{{W}}{{W}}"
    );
    cast_with_floating(&mut engine, p0, skyclave_apparition());

    // The cast trigger goes on the stack above the spell and asks for a
    // nonbasic land. Answer it, answer the optional clause, and stop as soon as
    // the land is destroyed.
    let mut land_menu: Option<Vec<ObjectId>> = None;
    let mut may_was_asked = false;
    for _ in 0..20 {
        if in_graveyard(&engine, p1, irrigated_farmland()).is_some() {
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
                assert_eq!(player, p0, "the seat that cast the Spirit answers");
                assert!(
                    land_menu.is_none(),
                    "the trigger asks for its target once, and nothing else on \
                     this stack asks for one"
                );
                assert_eq!((min, max), (1, 1), "one nonbasic land, and exactly one");
                land_menu = Some(options.clone());
                engine
                    .apply(
                        player,
                        PlayerAction::ChooseObjects {
                            objects: vec![farm],
                        },
                    )
                    .expect("the nonbasic land the question offered");
            }
            Pending::YesNo {
                player,
                prompt: YesNoPrompt::MayDo,
                ..
            } => {
                may_was_asked = true;
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            other => panic!("unexpected while the Gorger's trigger resolves: {other:?}"),
        }
    }

    let menu = land_menu.expect("the cast trigger asks which land it wants");
    assert!(
        menu.contains(&farm),
        "a nonbasic land across the table is the only thing \"target nonbasic \
         land\" may reach: {menu:?}"
    );
    assert!(
        !menu.contains(&their_forest),
        "\"nonbasic\" is read, not skipped: the basic Forest beside it is not on \
         the menu: {menu:?}"
    );
    assert_eq!(
        menu.len(),
        1,
        "and neither are the three basic Plains under the Gorger itself: {menu:?}"
    );
    assert!(
        may_was_asked,
        "\"you may destroy\" is a question, and it is asked before anything dies"
    );

    assert!(
        in_graveyard(&engine, p1, irrigated_farmland()).is_some(),
        "the land the trigger named was destroyed"
    );
    assert!(
        on_battlefield(&engine, p1, irrigated_farmland()).is_none(),
        "and left the battlefield, which is not where a land in a graveyard is"
    );
    assert!(
        on_battlefield(&engine, p1, forest()).is_some(),
        "the basic land the trigger declined never moved"
    );
    assert!(
        on_battlefield(&engine, p0, ore_gorger()).is_some(),
        "the Gorger is the source of the trigger and not one of its targets"
    );
    assert!(
        on_stack(&engine, skyclave_apparition()).is_some(),
        "and the trigger resolved above the Spirit spell that triggered it, \
         which is still waiting on the stack"
    );
}
