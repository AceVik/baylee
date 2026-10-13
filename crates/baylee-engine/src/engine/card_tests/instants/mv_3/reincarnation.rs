//! `cards/instants/mv_3/reincarnation.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

// oracle_id = "d6bf5e22-8d33-43a9-8824-435068e0a87a"
fn reincarnation() -> CardIndex {
    card_index("d6bf5e22-8d33-43a9-8824-435068e0a87a")
}

/// Casts Reincarnation off floating mana, names `target` and lets the spell
/// resolve. All it does is set a delayed triggered ability up (ruling
/// 2009-10-01), so the creature is where it was.
#[track_caller]
fn cast_it_at(engine: &mut Engine<RegistryLookup>, target: ObjectId) {
    cast_with_floating(engine, kit::P0, reincarnation());
    aim_at(engine, kit::P0, target);
    pass_until(engine, stack_is_empty);
    assert!(
        kit::is_on_battlefield(engine, target),
        "resolving Reincarnation moves nothing"
    );
}

/// The creature dies, by the destruction door the harness offers, and the
/// game is given the priority pass that judges the board.
#[track_caller]
fn dies(engine: &mut Engine<RegistryLookup>, creature: ObjectId) {
    bury(engine, &[creature]);
    let Pending::Priority { player, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    engine
        .apply(player, PlayerAction::PassPriority)
        .expect("passing priority is always legal");
}

/// Waits for the question the delayed trigger asks when it resolves and hands
/// back `(who is asked, the menu, min, max)`.
#[track_caller]
fn the_question(engine: &mut Engine<RegistryLookup>) -> (PlayerId, Vec<ObjectId>, u8, u8) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        min,
        max,
        ..
    } = engine.pending().clone()
    else {
        unreachable!("pass_until stopped on the question")
    };
    (player, options, min, max)
}

/// "Choose target creature. When that creature dies this turn, return a
/// creature card from its owner's graveyard to the battlefield under the
/// control of that creature's owner."
///
/// The card is chosen when the delayed trigger resolves, by the caster
/// (ruling 2009-10-01): a creature card that reaches the graveyard *after*
/// Reincarnation resolved — the Elves, moved there while the Ogre is still
/// alive — is on the menu beside the Ogre that just died. The card that comes
/// back is the one chosen, and the other stays where it was.
#[test]
fn reincarnation_returns_a_creature_card_the_caster_picks_when_the_creature_dies() {
    let p0 = kit::P0;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), gray_ogre()])
        .hand(0, &[reincarnation(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("the Ogre");
    tap_all_mana(&mut engine, p0);

    cast_it_at(&mut engine, ogre);
    let elves = hand_to_graveyard(&mut engine, p0, llanowar_elves());

    dies(&mut engine, ogre);
    let (asked, menu, min, max) = the_question(&mut engine);

    assert_eq!(asked, p0, "the player who controlled Reincarnation chooses");
    assert_eq!((min, max), (1, 1), "a creature card, and exactly one");
    assert!(
        menu.contains(&ogre),
        "the creature that died is a creature card in its owner's graveyard: {menu:?}"
    );
    assert!(
        menu.contains(&elves),
        "chosen on resolution, so a card that arrived after the cast is on the \
         menu too: {menu:?}"
    );
    assert_eq!(menu.len(), 2, "and nothing else is: {menu:?}");

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![elves],
            },
        )
        .expect("the Elves were on the menu");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p0, llanowar_elves()).is_some(),
        "the card chosen is on the battlefield, under its owner's control"
    );
    assert!(
        in_graveyard(&engine, p0, gray_ogre()).is_some(),
        "the Ogre was not chosen and stays dead"
    );
}

/// Ruling 2009-10-01: "The creature that's returned to the battlefield may be
/// the same one that was put into the graveyard … If so, it returns to the
/// battlefield as a brand-new creature."
///
/// The Ogre is the only creature card in any graveyard once it dies, so the
/// card chosen is the Ogre itself — and it is a new object (CR 400.7), which
/// the version every zone change bumps says.
#[test]
fn reincarnation_may_return_the_very_creature_that_died_as_a_new_object() {
    let p0 = kit::P0;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), gray_ogre()])
        .hand(0, &[reincarnation()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("the Ogre");
    let before = identity(&engine, ogre);
    tap_all_mana(&mut engine, p0);

    cast_it_at(&mut engine, ogre);
    dies(&mut engine, ogre);
    kit::drive(
        &mut engine,
        |e| kit::zone_of(e, ogre) == Zone::Battlefield && stack_is_empty(e),
        |_, pending| kit::choose_cards(pending, &[ogre]),
    );

    assert!(
        on_battlefield(&engine, p0, gray_ogre()).is_some(),
        "the Ogre is back on the battlefield under its owner's control"
    );
    assert_ne!(
        identity(&engine, ogre),
        before,
        "but it died in between, so what stands there is a new object"
    );
}

/// "…return a creature card from its owner's graveyard to the battlefield
/// under the control of that creature's owner."
///
/// Player 0 steals player 1's Grizzly Bears with Control Magic and
/// Reincarnation names it. The Bears dies, so it goes to its *owner's*
/// graveyard, where player 1's Ogre card already lies; player 0's Elves lie in
/// player 0's graveyard and are not on the menu, because the menu is the dead
/// creature's owner's graveyard. Player 0 — the caster — chooses, and what
/// they choose enters under player 1's control (ruling 2009-10-01: "the player
/// into whose graveyard it went, not necessarily the controller of that
/// creature").
#[test]
#[allow(clippy::too_many_lines)] // one stolen creature, played end to end
fn reincarnation_gives_the_card_back_to_the_owner_of_a_creature_its_caster_had_stolen() {
    let (p0, p1) = (kit::P0, kit::P1);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                island(),
                island(),
                island(),
                island(),
                forest(),
                forest(),
                forest(),
            ],
        )
        .hand(0, &[control_magic(), reincarnation(), llanowar_elves()])
        .battlefield(1, &[grizzly_bears()])
        .hand(1, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("their Bears");
    let their_ogre = hand_to_graveyard(&mut engine, p1, gray_ogre());
    let my_elves = hand_to_graveyard(&mut engine, p0, llanowar_elves());

    // Control Magic out of the islands, Reincarnation out of the forests.
    tap_all_mana_but(&mut engine, p0, Some(forest()));
    cast_with_floating(&mut engine, p0, control_magic());
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        kit::controller_of(&engine, bears),
        p0,
        "the Bears is player 0's to control"
    );
    assert_eq!(kit::owner_of(&engine, bears), p1, "and player 1's to own");
    tap_all_mana(&mut engine, p0);
    cast_it_at(&mut engine, bears);

    dies(&mut engine, bears);
    assert_eq!(
        kit::zone_of(&engine, bears),
        Zone::Graveyard,
        "the Bears died"
    );
    assert!(
        graveyard_contains(&engine, p1, bears),
        "into its owner's graveyard, not its controller's"
    );
    let (asked, menu, min, max) = the_question(&mut engine);

    assert_eq!(
        asked, p0,
        "the caster chooses, though the card is not theirs"
    );
    assert_eq!((min, max), (1, 1), "one creature card");
    let mut menu_sorted = menu.clone();
    menu_sorted.sort_unstable();
    let mut theirs = vec![bears, their_ogre];
    theirs.sort_unstable();
    assert_eq!(
        menu_sorted, theirs,
        "the creature cards in the dead creature's owner's graveyard — and not \
         the Elves in the caster's: {menu:?}, my Elves {my_elves:?}"
    );

    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![their_ogre],
            },
        )
        .expect("player 1's Ogre was on the menu");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, gray_ogre()).is_some(),
        "the Ogre enters under the control of its owner, player 1"
    );
    assert!(
        on_battlefield(&engine, p0, gray_ogre()).is_none(),
        "not under the caster's"
    );
    assert_eq!(
        kit::zone_of(&engine, my_elves),
        Zone::Graveyard,
        "and the caster's own graveyard was never touched"
    );
}

/// Ruling 2009-10-01: "If the targeted creature isn't put into a graveyard
/// this turn, that ability never triggers." The delayed trigger lasts "this
/// turn" and no longer.
///
/// The Ogre lives to the next turn and dies there: nothing asks a question and
/// nothing comes back.
#[test]
fn reincarnation_does_nothing_for_a_creature_that_dies_after_the_turn_is_over() {
    let p0 = kit::P0;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), gray_ogre()])
        .hand(0, &[reincarnation(), llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ogre = on_battlefield(&engine, p0, gray_ogre()).expect("the Ogre");
    tap_all_mana(&mut engine, p0);
    cast_it_at(&mut engine, ogre);
    let elves = hand_to_graveyard(&mut engine, p0, llanowar_elves());

    // Through the opponent's turn and into this player's next one.
    assert!(
        walk_to_own_main(&mut engine, p0),
        "p0 reaches its next turn"
    );
    assert!(
        engine.state().turn.number >= 3,
        "a later turn: {}",
        engine.state().turn.number
    );
    assert!(kit::is_on_battlefield(&engine, ogre), "the Ogre lived");

    kill(&mut engine, ogre);

    assert_eq!(
        kit::zone_of(&engine, ogre),
        Zone::Graveyard,
        "it died and stays dead"
    );
    assert_eq!(
        kit::zone_of(&engine, elves),
        Zone::Graveyard,
        "nothing was returned"
    );
    assert!(
        all_on_battlefield(&engine, p0, llanowar_elves()).is_empty(),
        "least of all the Elves"
    );
}

/// Whether `object` is in `seat`'s graveyard.
fn graveyard_contains(engine: &Engine<RegistryLookup>, seat: PlayerId, object: ObjectId) -> bool {
    kit::graveyard_of(engine, seat).contains(&object)
}
