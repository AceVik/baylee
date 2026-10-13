//! `cards/enchantments/mv_3/jihad.rs`, played.
//!
//! Jihad: "As this enchantment enters, choose a color and an opponent. /
//! White creatures get +2/+1 as long as the chosen player controls a nontoken
//! permanent of the chosen color. / When the chosen player controls no
//! nontoken permanents of the chosen color, sacrifice this enchantment."
//!
//! The choices are made as it enters (CR 614.12a), before it is on the
//! battlefield. White creatures are told by the pump on an Ondu Cleric, whose
//! base size is read off the board rather than assumed; the chosen colour is
//! told by what the chosen player controls: a red Gray Ogre, a green Llanowar
//! Elves. Permanents are removed with a Lightning Bolt and read once the
//! stack has settled.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

fn jihad() -> CardIndex {
    card_index("b18b9869-8490-4875-a5bb-484c3299f2c5")
}

fn sprout() -> CardIndex {
    card_index("424c6f5e-b386-47e9-b3fe-25b263097d40")
}

/// Answers the two choices Jihad asks as it enters: `colour` for the colour
/// (which must be on offer) and `opponent` for the player (likewise). Every
/// other question is left to the default.
fn choosing(
    colour: ManaColor,
    opponent: PlayerId,
) -> impl FnMut(&Engine<RegistryLookup>, &Pending) -> Option<PlayerAction> {
    move |_, pending| match pending {
        Pending::ChooseColor { options, .. } => {
            assert!(
                options.contains(&colour),
                "{colour:?} is on offer: {options:?}"
            );
            Some(PlayerAction::ChooseColor(colour))
        }
        Pending::ChoosePlayer { options, .. } => {
            assert!(
                options.contains(&opponent),
                "{opponent:?} is on offer: {options:?}"
            );
            Some(PlayerAction::ChoosePlayer(opponent))
        }
        _ => None,
    }
}

/// Casts Jihad off Plains the caster controls and settles the stack,
/// choosing `colour` and `opponent`.
#[track_caller]
fn cast_jihad(engine: &mut Engine<RegistryLookup>, colour: ManaColor, opponent: PlayerId) {
    cast_from_hand(engine, kit::P0, jihad());
    kit::resolve_the_stack(engine, choosing(colour, opponent));
}

/// Casts the Lightning Bolt in player 0's hand at `target`, off mana that is
/// already floating or untapped, and lets it resolve with whatever it sets
/// off.
#[track_caller]
fn bolt(engine: &mut Engine<RegistryLookup>, target: ObjectId) {
    cast_from_hand(engine, kit::P0, lightning_bolt());
    aim_at(engine, kit::P0, target);
    kit::resolve_the_stack(engine, |_, _| None);
}

/// Player 0 on the first main phase with three Plains, a Mountain for the
/// Bolt, an Ondu Cleric (white) and Jihad and a Bolt in hand; `theirs` is
/// player 1's board and `hand` their hand.
#[track_caller]
fn the_duel(theirs: &[CardIndex], hand: &[CardIndex]) -> (Engine<RegistryLookup>, ObjectId) {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[plains(), plains(), plains(), mountain(), ondu_cleric()],
        )
        .hand(0, &[jihad(), lightning_bolt()])
        .battlefield(1, theirs)
        .hand(1, hand)
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let cleric = on_battlefield(&engine, kit::P0, ondu_cleric()).expect("the Cleric");
    (engine, cleric)
}

fn grown(before: (i16, i16), after: (i16, i16)) -> (i16, i16) {
    (after.0 - before.0, after.1 - before.1)
}

/// "As this enchantment enters, choose a color and an opponent": at a table
/// of three both are asked of the caster, before Jihad is on the battlefield
/// (CR 614.12a: it is still on the stack), the colour among the colours and
/// the player among the two opponents and no one else.
#[test]
fn jihad_asks_for_a_colour_and_an_opponent_as_it_enters() {
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[plains(), plains(), plains()])
        .hand(0, &[jihad()])
        .battlefield(1, &[gray_ogre()])
        .battlefield(2, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    cast_from_hand(&mut engine, kit::P0, jihad());

    let mut asked = Vec::new();
    kit::resolve_the_stack(&mut engine, |e, pending| match pending {
        Pending::ChooseColor { player, options } => {
            assert_eq!(*player, kit::P0, "the caster chooses");
            assert!(
                on_stack(e, jihad()).is_some(),
                "Jihad has not entered while the choice is made"
            );
            asked.push(format!("colour {}", options.len()));
            assert!(options.contains(&ManaColor::Red), "{options:?}");
            Some(PlayerAction::ChooseColor(ManaColor::Red))
        }
        Pending::ChoosePlayer { player, options } => {
            assert_eq!(*player, kit::P0, "the caster chooses");
            assert!(
                on_stack(e, jihad()).is_some(),
                "Jihad has not entered while the choice is made"
            );
            assert_eq!(
                options,
                &vec![kit::P1, kit::P2],
                "an opponent: both of them and not the caster"
            );
            asked.push("opponent".to_owned());
            Some(PlayerAction::ChoosePlayer(kit::P1))
        }
        _ => None,
    });

    assert_eq!(asked.len(), 2, "one colour and one opponent: {asked:?}");
    assert!(asked.iter().any(|a| a.starts_with("colour")), "{asked:?}");
    assert!(asked.iter().any(|a| a == "opponent"), "{asked:?}");
    assert!(
        on_battlefield(&engine, kit::P0, jihad()).is_some(),
        "player 1 controls the red Ogre, so Jihad stays"
    );
}

/// "White creatures get +2/+1 as long as the chosen player controls a
/// nontoken permanent of the chosen color": every player's white creature
/// grows, the caster's, the chosen opponent's and the other opponent's, and a
/// creature that is not white (the red Ogre, a green Elves) does not.
#[test]
fn jihad_pumps_every_white_creature_while_the_chosen_player_has_the_colour() {
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                ondu_cleric(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[jihad()])
        .battlefield(1, &[gray_ogre(), ondu_cleric()])
        .battlefield(2, &[ondu_cleric()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let mine = on_battlefield(&engine, kit::P0, ondu_cleric()).expect("my Cleric");
    let chosen = on_battlefield(&engine, kit::P1, ondu_cleric()).expect("player 1's Cleric");
    let other = on_battlefield(&engine, kit::P2, ondu_cleric()).expect("player 2's Cleric");
    let elves = on_battlefield(&engine, kit::P0, llanowar_elves()).expect("the Elves");
    let ogre = on_battlefield(&engine, kit::P1, gray_ogre()).expect("the Ogre");
    let sizes = |e: &Engine<RegistryLookup>| [mine, chosen, other, elves, ogre].map(|id| pt(e, id));
    let before = sizes(&engine);

    cast_jihad(&mut engine, ManaColor::Red, kit::P1);
    assert!(on_battlefield(&engine, kit::P0, jihad()).is_some());
    let after = sizes(&engine);

    for (name, index) in [
        ("mine", 0),
        ("the chosen player's", 1),
        ("the other opponent's", 2),
    ] {
        assert_eq!(
            grown(before[index], after[index]),
            (2, 1),
            "{name} white Cleric gets +2/+1"
        );
    }
    for (name, index) in [("the green Elves", 3), ("the red Ogre", 4)] {
        assert_eq!(before[index], after[index], "{name} is not white");
    }
}

/// "When the chosen player controls no nontoken permanents of the chosen
/// color, sacrifice this enchantment": player 1's only red permanent dies to a
/// Bolt and Jihad is sacrificed, its bonus going with it. Before the Bolt the
/// bonus is on and Jihad stands, which is what makes the sacrifice the
/// answer to the Bolt and not a failure to ever stand.
#[test]
fn jihad_is_sacrificed_when_the_chosen_player_has_nothing_of_the_colour() {
    let (mut engine, cleric) = the_duel(&[gray_ogre()], &[]);
    let ogre = on_battlefield(&engine, kit::P1, gray_ogre()).expect("the Ogre");
    let plain = pt(&engine, cleric);

    cast_jihad(&mut engine, ManaColor::Red, kit::P1);
    assert!(
        on_battlefield(&engine, kit::P0, jihad()).is_some(),
        "it entered"
    );
    assert_eq!(grown(plain, pt(&engine, cleric)), (2, 1), "and pumps");

    bolt(&mut engine, ogre);
    assert!(
        in_graveyard(&engine, kit::P1, gray_ogre()).is_some(),
        "the red Ogre is dead"
    );
    assert!(
        on_battlefield(&engine, kit::P0, jihad()).is_none(),
        "Jihad did not stay"
    );
    assert!(
        in_graveyard(&engine, kit::P0, jihad()).is_some(),
        "it was sacrificed, not exiled or returned"
    );
    assert_eq!(pt(&engine, cleric), plain, "and its bonus is gone");
}

/// "…controls a **nontoken** permanent of the chosen color": a token of the
/// colour does not keep Jihad. Green is chosen, player 1 controls a Llanowar
/// Elves and (made on their turn by a Sprout) a green Saproling token. Jihad
/// stands while the Elves does; when a Bolt kills the Elves it is sacrificed
/// although the green Saproling is still on the battlefield.
#[test]
fn jihad_does_not_count_a_token_of_the_chosen_colour() {
    let (mut engine, cleric) = the_duel(&[llanowar_elves(), forest()], &[sprout()]);
    let plain = pt(&engine, cleric);
    cast_jihad(&mut engine, ManaColor::Green, kit::P1);
    assert_eq!(
        grown(plain, pt(&engine, cleric)),
        (2, 1),
        "Jihad stands on the Elves"
    );

    reach_their_main_phase(&mut engine, kit::P1);
    cast_from_hand(&mut engine, kit::P1, sprout());
    kit::resolve_the_stack(&mut engine, |_, _| None);
    assert_eq!(
        tokens_of(&engine, kit::P1).len(),
        1,
        "player 1 has a green Saproling token"
    );
    assert!(
        on_battlefield(&engine, kit::P0, jihad()).is_some(),
        "and Jihad is untroubled by it"
    );

    kit::walk_to_next_own_main(&mut engine, kit::P0);
    let elves = on_battlefield(&engine, kit::P1, llanowar_elves()).expect("the Elves");
    bolt(&mut engine, elves);

    assert!(
        in_graveyard(&engine, kit::P1, llanowar_elves()).is_some(),
        "the Elves died"
    );
    assert_eq!(
        tokens_of(&engine, kit::P1).len(),
        1,
        "the green token is still there"
    );
    assert!(
        in_graveyard(&engine, kit::P0, jihad()).is_some(),
        "and Jihad was sacrificed anyway: a token of the colour does not count"
    );
}

/// The colour chosen decides which permanents count. Player 1 controls only
/// a green Llanowar Elves. Choosing green, Jihad stands and pumps; choosing
/// red, there is no red nontoken permanent from the start, so it is
/// sacrificed at once and never pumps.
#[test]
fn jihad_follows_the_colour_it_chose_and_no_other() {
    for (colour, stays) in [(ManaColor::Green, true), (ManaColor::Red, false)] {
        let (mut engine, cleric) = the_duel(&[llanowar_elves()], &[]);
        let plain = pt(&engine, cleric);

        cast_jihad(&mut engine, colour, kit::P1);

        assert_eq!(
            on_battlefield(&engine, kit::P0, jihad()).is_some(),
            stays,
            "chose {colour:?} against a green creature"
        );
        assert_eq!(
            in_graveyard(&engine, kit::P0, jihad()).is_some(),
            !stays,
            "chose {colour:?}: sacrificed or standing"
        );
        assert_eq!(
            grown(plain, pt(&engine, cleric)),
            if stays { (2, 1) } else { (0, 0) },
            "chose {colour:?}"
        );
    }
}

/// A three-seat table: Jihad (red, player 1 chosen) over player 1's and
/// player 2's red Gray Ogres. Returns the engine, player 0's Cleric and the
/// two Ogres.
#[track_caller]
fn red_ogres_on_both_sides() -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(
            0,
            &[plains(), plains(), plains(), mountain(), ondu_cleric()],
        )
        .hand(0, &[jihad(), lightning_bolt()])
        .battlefield(1, &[gray_ogre()])
        .battlefield(2, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, kit::P0);
    let cleric = on_battlefield(&engine, kit::P0, ondu_cleric()).expect("the Cleric");
    let first = on_battlefield(&engine, kit::P1, gray_ogre()).expect("player 1's Ogre");
    let second = on_battlefield(&engine, kit::P2, gray_ogre()).expect("player 2's Ogre");
    cast_jihad(&mut engine, ManaColor::Red, kit::P1);
    assert!(on_battlefield(&engine, kit::P0, jihad()).is_some());
    (engine, cleric, first, second)
}

/// Ruling (2004-10-04): "In a multi-player game, it tracks only the chosen
/// player." The other opponent loses their only red permanent and Jihad
/// stands, its bonus on.
#[test]
fn jihad_does_not_care_when_another_opponent_loses_the_colour() {
    let (mut engine, cleric, _, second) = red_ogres_on_both_sides();
    let pumped = pt(&engine, cleric);

    bolt(&mut engine, second);

    assert!(in_graveyard(&engine, kit::P2, gray_ogre()).is_some());
    assert!(
        on_battlefield(&engine, kit::P0, jihad()).is_some(),
        "player 1, the chosen player, still has a red permanent"
    );
    assert_eq!(pt(&engine, cleric), pumped, "and the bonus is on");
}

/// The other half of the same ruling: the chosen player loses their red
/// permanent while another opponent still has one, and Jihad is sacrificed.
#[test]
fn jihad_is_not_kept_by_another_opponents_permanents_of_the_colour() {
    let (mut engine, cleric, first, _) = red_ogres_on_both_sides();
    let pumped = pt(&engine, cleric);

    bolt(&mut engine, first);

    assert!(in_graveyard(&engine, kit::P1, gray_ogre()).is_some());
    assert!(
        on_battlefield(&engine, kit::P2, gray_ogre()).is_some(),
        "player 2 still has a red permanent"
    );
    assert!(
        in_graveyard(&engine, kit::P0, jihad()).is_some(),
        "which does not matter: the chosen player has none"
    );
    assert_ne!(pt(&engine, cleric), pumped, "the bonus went with it");
}
