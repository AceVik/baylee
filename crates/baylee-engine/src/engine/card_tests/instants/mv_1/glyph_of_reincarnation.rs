//! `cards/instants/mv_1/glyph_of_reincarnation.rs`, played.
//!
//! "Cast this spell only after combat.
//! Destroy all creatures that were blocked by target Wall this turn. They can't
//! be regenerated. For each creature that died this way, put a creature card
//! from the graveyard of the player who controlled that creature the last time
//! it became blocked by that Wall onto the battlefield under its owner's
//! control."
//!
//! Rulings (2004-10-04): "First place all the affected creatures in the
//! graveyard, then choose the appropriate number of creatures from the
//! attacker's graveyard and put them onto the battlefield. Some or all of them
//! may be the same creatures which were just destroyed." and "Yes, you can cast
//! this targeting your opponent's Walls to good effect."
//!
//! The Wall is a Wall of Stone (0/8): whatever it blocks is destroyed by the
//! Glyph and by nothing else. A Hill Giant is put into the graveyard first, so
//! that the destroyed creature is not the only creature card there and the
//! choice is a real menu.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w2c_kit as kit;
use crate::turn::Step;
use baylee_core::generated::index;
use baylee_core::mana::ManaColor;

// oracle_id = "93d9a1c6-9f6f-487f-a46c-bfd80946ccdf"
fn glyph_of_reincarnation() -> CardIndex {
    card_index("93d9a1c6-9f6f-487f-a46c-bfd80946ccdf")
}

/// A board at p0's second main phase, after combat: Savannah Lions attacked and
/// p1's Wall of Stone blocked them, and a Hill Giant lies in p0's graveyard.
/// `lands` are on p0's side too and `hand` joins the Glyph in p0's hand. Returns
/// the engine and the Lions, the Wall and the Giant.
#[track_caller]
fn after_combat(
    lands: &[CardIndex],
    hand: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId, ObjectId, ObjectId) {
    let mut table = vec![kit::savannah_lions()];
    table.extend_from_slice(lands);
    let mut cards = vec![glyph_of_reincarnation(), kit::hill_giant()];
    cards.extend_from_slice(hand);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &table)
        .battlefield(1, &[kit::wall_of_stone()])
        .hand(0, &cards)
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let lions = on_battlefield(&engine, kit::P0, kit::savannah_lions()).expect("the Lions are out");
    let wall = on_battlefield(&engine, kit::P1, kit::wall_of_stone()).expect("the Wall is out");
    let giant = hand_to_graveyard(&mut engine, kit::P0, kit::hill_giant());

    kit::attack(&mut engine, kit::P0, &[lions], kit::P1);
    kit::block(&mut engine, kit::P1, &[(wall, lions)]);
    kit::walk_to_second_main(&mut engine, &mut kit::Answers::new(), kit::P0);
    assert_eq!(
        kit::damage_on(&engine, wall),
        2,
        "the Lions and the Wall fought"
    );
    (engine, lions, wall, giant)
}

/// Casts Glyph of Reincarnation on `wall` as p0 off the mana in its lands, with
/// `answers` for the card it asks for, and lets everything resolve.
#[track_caller]
fn reincarnate(engine: &mut Engine<RegistryLookup>, answers: &mut kit::Answers) {
    tap_all_mana(engine, kit::P0);
    kit::cast(engine, kit::P0, glyph_of_reincarnation(), answers);
    kit::settle(engine, kit::P0, answers);
}

/// "Cast this spell only after combat." With a Wall and mana in the pool, the
/// Glyph cannot be cast in the first main phase, in the declare blockers step
/// or in the combat damage step, and can be in the second main phase. The same
/// board at each moment is the control: the target exists throughout, so what
/// changes between "no" and "yes" is the timing alone.
#[test]
fn glyph_of_reincarnation_can_be_cast_only_after_combat() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kit::savannah_lions(), forest()])
        .battlefield(1, &[kit::wall_of_stone()])
        .hand(0, &[glyph_of_reincarnation()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let lions = on_battlefield(&engine, kit::P0, kit::savannah_lions()).expect("the Lions are out");
    let wall = on_battlefield(&engine, kit::P1, kit::wall_of_stone()).expect("the Wall is out");
    kit::float(&mut engine, kit::P0, ManaColor::Green, 1);
    assert!(
        !kit::castable(&engine, kit::P0, glyph_of_reincarnation()),
        "not in the first main phase either, with the Wall on the table and the mana floating"
    );
    kit::attack(&mut engine, kit::P0, &[lions], kit::P1);
    kit::block(&mut engine, kit::P1, &[(wall, lions)]);

    let mut quiet = kit::Answers::new();
    kit::walk_to_quiet_priority(&mut engine, &mut quiet, kit::P0, Step::DeclareBlockers);
    kit::float(&mut engine, kit::P0, ManaColor::Green, 1);
    assert!(
        !kit::castable(&engine, kit::P0, glyph_of_reincarnation()),
        "not while blockers are being declared: combat is not over"
    );

    kit::walk_to_quiet_priority(&mut engine, &mut quiet, kit::P0, Step::CombatDamage);
    kit::float(&mut engine, kit::P0, ManaColor::Green, 1);
    assert!(
        !kit::castable(&engine, kit::P0, glyph_of_reincarnation()),
        "nor in the combat damage step"
    );

    kit::walk_to_second_main(&mut engine, &mut quiet, kit::P0);
    kit::float(&mut engine, kit::P0, ManaColor::Green, 1);
    assert!(
        kit::castable(&engine, kit::P0, glyph_of_reincarnation()),
        "but in the second main phase, with the same Wall that blocked and the same mana"
    );
}

/// "Destroy all creatures that were blocked by target Wall this turn. … put a
/// creature card from the graveyard of the player who controlled that creature
/// the last time it became blocked by that Wall onto the battlefield under its
/// owner's control": the Lions, blocked by the Wall, are destroyed and go to
/// their owner's graveyard beside the Giant already there; the question offers
/// both, p0 chooses the Giant, and the Giant comes onto the battlefield (it was
/// p0's) while the Lions stay in the graveyard.
#[test]
fn glyph_of_reincarnation_destroys_the_blocked_creature_and_returns_a_chosen_creature_card() {
    let (mut engine, lions, wall, giant) = after_combat(&[forest()], &[]);

    let mut answers = kit::Answers::new().aim(&[wall]).pick(&[giant]);
    reincarnate(&mut engine, &mut answers);

    assert_eq!(
        kit::zone_of(&engine, lions),
        Zone::Graveyard,
        "the creature the Wall blocked is destroyed and stays dead"
    );
    assert!(
        answers
            .card_menus
            .iter()
            .any(|menu| menu.contains(&lions) && menu.contains(&giant)),
        "the destroyed creature is already in the graveyard when the card is \
         chosen (ruling): both creature cards were offered: {:?}",
        answers.card_menus
    );
    assert!(
        kit::on_the_table(&engine, giant),
        "the chosen card is on the battlefield"
    );
    assert_eq!(
        engine.state().object(giant).expect("the Giant").controller,
        kit::P0,
        "under its owner's control"
    );
}

/// Ruling (2004-10-04): "Some or all of them may be the same creatures which
/// were just destroyed." The same board, and p0 chooses the Lions: they are
/// destroyed and come straight back, and the Giant is left in the graveyard.
#[test]
fn glyph_of_reincarnation_may_return_the_creature_it_just_destroyed() {
    let (mut engine, lions, wall, giant) = after_combat(&[forest()], &[]);
    let before = identity(&engine, lions);
    assert!(
        is_tapped(&engine, lions),
        "the Lions attacked and are tapped"
    );

    let mut answers = kit::Answers::new().aim(&[wall]).pick(&[lions]);
    reincarnate(&mut engine, &mut answers);

    assert!(
        answers
            .card_menus
            .iter()
            .any(|menu| menu.contains(&lions) && menu.contains(&giant)),
        "the Lions were in the graveyard to be chosen from: {:?}",
        answers.card_menus
    );
    assert!(
        kit::on_the_table(&engine, lions),
        "the destroyed Lions were put back onto the battlefield"
    );
    assert_ne!(
        identity(&engine, lions),
        before,
        "as a new object (CR 400.7): it died and came back, it was not left alone"
    );
    assert!(
        !is_tapped(&engine, lions),
        "and the new object is untapped, where the attacker it replaced was tapped"
    );
    assert_eq!(
        kit::zone_of(&engine, giant),
        Zone::Graveyard,
        "and the Giant, not chosen, stays where it was"
    );
}

/// "For each creature that died this way": a creature that did not die gives
/// nothing. Heroic Intervention makes the Lions indestructible before the
/// Glyph is cast, the destroy fails, and no card comes back: the Giant is still
/// in the graveyard and no card question was asked. The tests above are the
/// control: the same board without Intervention destroys the Lions and returns
/// a card.
#[test]
fn glyph_of_reincarnation_returns_nothing_for_a_creature_that_did_not_die() {
    let (mut engine, lions, wall, giant) = after_combat(
        &[forest(), forest(), forest()],
        &[index::HEROIC_INTERVENTION],
    );

    tap_all_mana(&mut engine, kit::P0);
    let mut answers = kit::Answers::new().aim(&[wall]).pick(&[giant]);
    kit::cast(
        &mut engine,
        kit::P0,
        index::HEROIC_INTERVENTION,
        &mut answers,
    );
    kit::settle(&mut engine, kit::P0, &mut answers);
    kit::cast(&mut engine, kit::P0, glyph_of_reincarnation(), &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);

    assert!(
        in_graveyard(&engine, kit::P0, glyph_of_reincarnation()).is_some()
            && in_graveyard(&engine, kit::P0, index::HEROIC_INTERVENTION).is_some(),
        "both spells resolved"
    );
    assert!(
        kit::on_the_table(&engine, lions),
        "an indestructible creature is not destroyed"
    );
    assert_eq!(
        kit::zone_of(&engine, giant),
        Zone::Graveyard,
        "nothing died, so nothing came back"
    );
    assert!(
        answers.card_menus.is_empty(),
        "and no card was asked for: {:?}",
        answers.card_menus
    );
}

/// "They can't be regenerated.": a Lotleth Troll with a `{B}` regeneration
/// shield is blocked by the Wall. After combat the shield goes up and the Glyph
/// is cast: the Troll is destroyed anyway and goes to the graveyard, and the
/// card p0 chooses (the Giant) comes back. Had the shield worked nothing would
/// have died and the Giant would still be in the graveyard.
#[test]
fn glyph_of_reincarnation_destroys_through_a_regeneration_shield() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kit::troll(), forest(), swamp()])
        .battlefield(1, &[kit::wall_of_stone()])
        .hand(0, &[glyph_of_reincarnation(), kit::hill_giant()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    let troll = on_battlefield(&engine, kit::P0, kit::troll()).expect("the Troll is out");
    let wall = on_battlefield(&engine, kit::P1, kit::wall_of_stone()).expect("the Wall is out");
    let giant = hand_to_graveyard(&mut engine, kit::P0, kit::hill_giant());

    kit::attack(&mut engine, kit::P0, &[troll], kit::P1);
    kit::block(&mut engine, kit::P1, &[(wall, troll)]);
    let mut answers = kit::Answers::new().aim(&[wall]).pick(&[giant]).shield(true);
    kit::walk_to_second_main(&mut engine, &mut answers, kit::P0);

    tap_all_mana(&mut engine, kit::P0);
    raise_a_shield(&mut engine, kit::P0, troll, kit::TROLL_REGENERATES);
    kit::cast(&mut engine, kit::P0, glyph_of_reincarnation(), &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);

    assert_eq!(
        kit::zone_of(&engine, troll),
        Zone::Graveyard,
        "the shield did not save it: \"they can't be regenerated\""
    );
    assert!(
        kit::on_the_table(&engine, giant),
        "and since it died, a creature card came back"
    );
}

/// The board of the two tests below: p1 steals p0's Grizzly Bears with
/// Song-Mad Treachery in p1's turn, attacks with them, and p0's own Wall of
/// Stone blocks them. The Bears are p0's card and were controlled by p1 when
/// they became blocked. After combat, in p1's second main phase, p0 holds
/// Glyph of Reincarnation with a Forest. `p1_graveyard` are the cards p1 has in
/// its graveyard (moved there from its hand first) and `p0_graveyard` p0's.
/// Returns the engine and the Bears and the Wall.
#[track_caller]
fn stolen_bears_blocked_by_p0s_wall(
    p0_graveyard: &[CardIndex],
    p1_graveyard: &[CardIndex],
) -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut p0_hand = vec![glyph_of_reincarnation()];
    p0_hand.extend_from_slice(p0_graveyard);
    let mut p1_hand = vec![index::SONG_MAD_TREACHERY];
    p1_hand.extend_from_slice(p1_graveyard);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[kit::bears(), kit::wall_of_stone(), forest()])
        .battlefield(1, &kit::lands(mountain(), 5))
        .hand(0, &p0_hand)
        .hand(1, &p1_hand)
        .start();
    keep_mulligans(&mut engine);
    for card in p0_graveyard {
        hand_to_graveyard(&mut engine, kit::P0, *card);
    }
    for card in p1_graveyard {
        hand_to_graveyard(&mut engine, kit::P1, *card);
    }
    kit::walk_to_next_own_main(&mut engine, kit::P1);

    let bears = on_battlefield(&engine, kit::P0, kit::bears()).expect("the Bears are out");
    let wall = on_battlefield(&engine, kit::P0, kit::wall_of_stone()).expect("p0's Wall is out");

    tap_all_mana(&mut engine, kit::P1);
    let mut steal = kit::Answers::new()
        .mode(CastModeKind::Face(0))
        .aim(&[bears]);
    kit::cast(&mut engine, kit::P1, index::SONG_MAD_TREACHERY, &mut steal);
    kit::settle(&mut engine, kit::P1, &mut steal);
    assert_eq!(
        engine.state().object(bears).expect("the Bears").controller,
        kit::P1,
        "p1 controls p0's Bears"
    );

    kit::attack(&mut engine, kit::P1, &[bears], kit::P0);
    kit::block(&mut engine, kit::P0, &[(wall, bears)]);
    kit::walk_to_second_main(&mut engine, &mut kit::Answers::new(), kit::P0);
    assert_eq!(
        kit::damage_on(&engine, wall),
        2,
        "the stolen Bears and the Wall fought"
    );
    (engine, bears, wall)
}

/// "… from the graveyard of the player who controlled that creature the last
/// time it became blocked by that Wall … under its owner's control": the Bears
/// are p0's, p1 controlled them when the Wall blocked them. Destroyed, they go
/// to p0's graveyard (their owner's); the card comes from p1's graveyard,
/// where a Hill Giant and Savannah Lions lie. p0 chooses the Giant and it
/// enters under its owner's control, p1's. The menu is p1's two cards and not
/// the Bears in p0's graveyard; the Lions stay in p1's graveyard.
#[test]
fn glyph_of_reincarnation_takes_the_card_from_the_graveyard_of_the_creatures_controller() {
    let (mut engine, bears, wall) =
        stolen_bears_blocked_by_p0s_wall(&[], &[kit::hill_giant(), kit::savannah_lions()]);
    let giant = kit::graveyard_objects(&engine, kit::P1, kit::hill_giant())[0];
    let lions = kit::graveyard_objects(&engine, kit::P1, kit::savannah_lions())[0];

    let mut answers = kit::Answers::new().aim(&[wall]).pick(&[giant]);
    reincarnate(&mut engine, &mut answers);

    assert_eq!(
        kit::zone_of(&engine, bears),
        Zone::Graveyard,
        "the stolen Bears were destroyed"
    );
    assert!(
        kit::graveyard_objects(&engine, kit::P0, kit::bears()).contains(&bears),
        "and are in their owner's graveyard"
    );
    assert!(
        answers
            .card_menus
            .iter()
            .any(|menu| menu.contains(&giant) && menu.contains(&lions) && !menu.contains(&bears)),
        "the choice was p1's graveyard (its controller when blocked), not p0's: {:?}",
        answers.card_menus
    );
    assert!(
        !answers.askers.is_empty() && answers.askers.iter().all(|asked| *asked == kit::P0),
        "every question, the card choice included, was put to the caster, p0, though \
         the graveyard is p1's: {:?}",
        answers.askers
    );
    assert!(
        kit::on_the_table(&engine, giant),
        "the chosen Giant is on the battlefield"
    );
    assert_eq!(
        engine.state().object(giant).expect("the Giant").controller,
        kit::P1,
        "under its owner's control: p1's, not the caster's"
    );
    assert_eq!(
        kit::zone_of(&engine, lions),
        Zone::Graveyard,
        "the card not chosen stays in p1's graveyard"
    );
}

/// "Put a creature card from the graveyard of the player who controlled that
/// creature …": an empty graveyard gives nothing. The same stolen Bears, but
/// p1's graveyard holds no creature card, while p0's (the owner's, the caster's)
/// holds a Hill Giant. Nothing comes back: the Giant is still in p0's
/// graveyard, the Bears are in it too, and no card question was asked. The test
/// above is the control: the same destruction with creature cards in p1's
/// graveyard returns one.
#[test]
fn glyph_of_reincarnation_returns_nothing_when_that_players_graveyard_has_no_creature() {
    let (mut engine, bears, wall) = stolen_bears_blocked_by_p0s_wall(&[kit::hill_giant()], &[]);
    let giant = kit::graveyard_objects(&engine, kit::P0, kit::hill_giant())[0];

    let mut answers = kit::Answers::new().aim(&[wall]).pick(&[giant]);
    reincarnate(&mut engine, &mut answers);

    assert_eq!(
        kit::zone_of(&engine, bears),
        Zone::Graveyard,
        "the stolen Bears were destroyed"
    );
    assert_eq!(
        kit::zone_of(&engine, giant),
        Zone::Graveyard,
        "p0's own graveyard is not where the card comes from: the Giant stays"
    );
    assert!(
        answers.card_menus.is_empty(),
        "and no card was asked for: {:?}",
        answers.card_menus
    );
}

/// "For each creature that died this way, put a creature card from the
/// graveyard …": a token is not a card (CR 108.2b), so the Soldier token the
/// Wall blocked is destroyed (a token that dies goes to the graveyard first, CR
/// 111.7), and the choice is between the creature *cards* in its controller's
/// graveyard, never the token. Raise the Alarm makes two Soldiers in turn 1;
/// in turn 3 one attacks and p1's Wall of Stone blocks it, and a Hill Giant
/// and a Grizzly Bears lie in p0's graveyard. After combat the Glyph destroys
/// that Soldier, the other Soldier (not blocked) lives, the menu is exactly the
/// Giant and the Bears, and the chosen Giant enters under p0's control.
#[test]
fn glyph_of_reincarnation_offers_the_creature_cards_and_never_the_token_it_destroyed() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), forest()])
        .battlefield(1, &[kit::wall_of_stone()])
        .hand(
            0,
            &[
                raise_the_alarm(),
                glyph_of_reincarnation(),
                kit::hill_giant(),
                kit::bears(),
            ],
        )
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );
    let wall = on_battlefield(&engine, kit::P1, kit::wall_of_stone()).expect("the Wall is out");
    let giant = hand_to_graveyard(&mut engine, kit::P0, kit::hill_giant());
    let bears = hand_to_graveyard(&mut engine, kit::P0, kit::bears());

    let mut none = kit::Answers::new();
    tap_all_mana(&mut engine, kit::P0);
    kit::cast(&mut engine, kit::P0, raise_the_alarm(), &mut none);
    kit::settle(&mut engine, kit::P0, &mut none);
    let soldiers = tokens_of(&engine, kit::P0);
    assert_eq!(soldiers.len(), 2, "two Soldier tokens: {soldiers:?}");
    let (blocked, spared) = (soldiers[0], soldiers[1]);

    kit::walk_to_next_own_main(&mut engine, kit::P0);
    kit::attack(&mut engine, kit::P0, &[blocked], kit::P1);
    kit::block(&mut engine, kit::P1, &[(wall, blocked)]);
    kit::walk_to_second_main(&mut engine, &mut none, kit::P0);
    assert_eq!(
        kit::damage_on(&engine, wall),
        1,
        "the Soldier and the Wall fought"
    );

    let mut answers = kit::Answers::new().aim(&[wall]).pick(&[giant]);
    reincarnate(&mut engine, &mut answers);

    assert!(
        !engine
            .state()
            .object(blocked)
            .is_some_and(|o| o.zone == Zone::Battlefield),
        "the Soldier the Wall blocked was destroyed"
    );
    assert!(
        kit::on_the_table(&engine, spared),
        "the Soldier it did not block is untouched"
    );
    assert_eq!(
        answers.card_menus.len(),
        1,
        "the destroyed token earned one card choice: {:?}",
        answers.card_menus
    );
    let menu = &answers.card_menus[0];
    assert!(
        menu.contains(&giant) && menu.contains(&bears),
        "both creature cards were offered: {menu:?}"
    );
    assert!(
        !menu.contains(&blocked) && menu.len() == 2,
        "and the token, which is not a card, was not: {menu:?}"
    );
    assert!(
        kit::on_the_table(&engine, giant),
        "the chosen creature card entered the battlefield"
    );
    assert_eq!(
        engine.state().object(giant).expect("the Giant").controller,
        kit::P0,
        "under its owner's control"
    );
    assert_eq!(
        kit::zone_of(&engine, bears),
        Zone::Graveyard,
        "and the card not chosen stays in the graveyard"
    );
}
