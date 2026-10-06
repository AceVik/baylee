//! `cards/creatures/mv_3/soratami_rainshaper.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Soratami Rainshaper — {2}{U}, a 2/1 Moonfolk Wizard with flying — prints
/// "{3}, Return a land you control to its owner's hand: Target creature you
/// control gains shroud until end of turn."
///
/// The price lands in two different places, so both halves are read off the
/// state after the target question has been answered (CR 601.2c before
/// CR 601.2h): the {3} leaves the pool the four Islands filled, and one of
/// those Islands — **tapped**, because four tapped Islands are still "a land
/// you control" and the card prints no "untapped" — goes back to hand while
/// the land count falls by one.
///
/// The targeting half is read against an Elf across the table, which a bare
/// "target creature" would have offered and "you control" refuses, and the
/// grant is read off the layers: the chosen Elf has shroud, the Shaper that
/// granted it does not, and the Elf nobody named never gets it.
#[allow(clippy::too_many_lines)] // One printed card, played end to end: the length is the card's.
#[test]
fn soratami_rainshaper_returns_a_tapped_land_to_give_a_creature_shroud() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                soratami_rainshaper(),
                quiet_creature(),
            ],
        )
        .battlefield(1, &[quiet_creature()])
        .start();
    keep_mulligans(&mut engine);
    assert!(walk_to_own_main(&mut engine, p0), "p0 reaches its own main");

    let shaper =
        on_battlefield(&engine, p0, soratami_rainshaper()).expect("the Rainshaper is on the table");
    let mine =
        on_battlefield(&engine, p0, quiet_creature()).expect("a creature this seat controls");
    let theirs =
        on_battlefield(&engine, p1, quiet_creature()).expect("a creature across the table");
    assert_eq!(pt(&engine, shaper), (2, 1), "the body the card prints");
    assert!(
        keywords(&engine, shaper).contains(KeywordSet::FLYING),
        "and the flying it prints"
    );
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::SHROUD),
        "nothing has been granted yet"
    );
    let hand_before = engine.state().zones.list(ZoneLocation::Hand(p0)).len();
    assert_eq!(lands_of(&engine, p0).len(), 4, "four Islands to give up");

    // Four Islands, and the Elf named as the one source kept back: its own
    // `{T}: Add {G}` is a mana ability whose whole price is its tap (#159),
    // and it is the creature this ability is about to aim at.
    tap_all_mana_but(&mut engine, p0, Some(quiet_creature()));
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "four Islands and nothing off the Elf"
    );
    assert!(!is_tapped(&engine, mine), "which the mana never touched");

    activate(&mut engine, p0, soratami_rainshaper(), 0);
    let Pending::ChooseTargets {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        panic!(
            "\"target creature you control\" is a target choice, got {:?}",
            engine.pending()
        )
    };
    assert_eq!(player, p0, "the activating seat aims it");
    assert_eq!((min, max), (1, 1), "one creature, no more and no fewer");
    assert!(
        options.contains(&mine),
        "the creature this seat controls is on the menu: {options:?}"
    );
    assert!(
        !options.contains(&theirs),
        "\"creature *you* control\" declines the Elf across the table: {options:?}"
    );
    // CR 601.2c names the target before CR 601.2h pays anything: while this
    // question stands, the mana is still floating and every Island is still
    // on the battlefield.
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        4,
        "the price is paid after the target, not before it"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        4,
        "and no land has left the battlefield yet"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![mine],
            },
        )
        .expect("the creature the question offered was chosen");

    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        prompt,
        ..
    } = engine.pending().clone()
    else {
        panic!("the cost asks which land, got {:?}", engine.pending())
    };
    assert_eq!(player, p0, "the activating seat answers its own cost");
    assert_eq!(
        prompt,
        crate::choice::ChoicePrompt::CostReturn,
        "a land given up to pay, and not a search and not a tap"
    );
    assert_eq!((min, max), (1, 1), "one land, no more and no fewer");
    let my_lands = lands_of(&engine, p0);
    for land in &my_lands {
        assert!(
            options.contains(land),
            "a *tapped* land you control is still a land you control — the \
             card prints no \"untapped\", so {land:?} belongs on this menu: \
             {options:?}"
        );
    }
    for offered in &options {
        assert!(
            my_lands.contains(offered),
            "\"a land you control\" is a land and not a creature or an \
             artifact: {offered:?} is none of the {my_lands:?}"
        );
    }

    let given_up = options[0];
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![given_up],
            },
        )
        .expect("the land the question offered pays the cost");
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        1,
        "the {{3}} came out of the pool the four Islands filled"
    );
    assert_eq!(
        lands_of(&engine, p0).len(),
        3,
        "exactly one land was given up: the other three are still standing"
    );
    assert!(
        in_hand(&engine, p0, island()).is_some(),
        "\"to its owner's hand\": the land is a card in the hand of the seat \
         that controlled it"
    );
    assert_eq!(
        engine.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "one card up, so a land that merely vanished would not satisfy the \
         claim above"
    );

    assert!(
        keywords(&engine, mine).contains(KeywordSet::SHROUD),
        "the creature the question offered gained shroud"
    );
    assert!(
        !keywords(&engine, shaper).contains(KeywordSet::SHROUD),
        "the Shaper grants the keyword and does not keep it"
    );
    assert!(
        !keywords(&engine, theirs).contains(KeywordSet::SHROUD),
        "and the Elf the ability declined to target never got it"
    );

    // "until end of turn": one turn cycle is enough for the grant to lapse,
    // which is what tells the printed duration from a permanent one.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !keywords(&engine, mine).contains(KeywordSet::SHROUD),
        "the shroud is gone by the caster's next main phase"
    );
}
