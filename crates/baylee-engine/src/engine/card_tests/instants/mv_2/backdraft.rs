//! `cards/instants/mv_2/backdraft.rs`, played.
//!
//! "Choose a player who cast one or more sorcery spells this turn. Backdraft
//! deals damage to that player equal to half the damage dealt by one of those
//! sorcery spells this turn, rounded down."
//!
//! The sorceries are Blaze (`{X}{R}`: X damage to any target), which makes the
//! damage a number the test chooses, and Stream of Life (`{X}{G}`: a player
//! gains X life), which makes a sorcery that dealt none. Everything happens in
//! p0's first main phase, with the mana floating from one tap of every land,
//! because unspent mana empties only as a step or phase ends (CR 500.5).

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w2c_kit as kit;
use baylee_core::generated::index;

// oracle_id = "80734bb0-5032-4355-9304-49f4a4557aba"
fn backdraft() -> CardIndex {
    card_index("80734bb0-5032-4355-9304-49f4a4557aba")
}

/// Casts Blaze for `x` at `at` as `seat` off the mana already floating and lets
/// it resolve; `seat` holds priority afterwards.
#[track_caller]
fn blaze(engine: &mut Engine<RegistryLookup>, seat: PlayerId, x: u32, at: PlayerId) {
    let mut answers = kit::Answers::new().x(x).aim_players(&[at]);
    kit::cast(engine, seat, index::BLAZE, &mut answers);
    kit::settle(engine, seat, &mut answers);
}

/// Casts Backdraft as `seat` off floating mana, with `answers` scripted for
/// the one question it may ask.
#[track_caller]
fn cast_backdraft(engine: &mut Engine<RegistryLookup>, seat: PlayerId, answers: &mut kit::Answers) {
    kit::cast(engine, seat, backdraft(), answers);
    kit::settle(engine, seat, answers);
    assert!(
        in_graveyard(engine, seat, backdraft()).is_some(),
        "an instant that resolved is in its owner's graveyard"
    );
}

/// One sorcery, five damage: "half … rounded down" is 2, not 3 (and not 2.5).
/// With a single candidate there is nothing to choose, so Backdraft asks
/// nothing; and the caster of Backdraft is the chosen player here, because the
/// only one who cast a sorcery this turn is the seat holding Backdraft. p0
/// ends at 20 - 2, and p1 keeps what Blaze left it.
#[test]
fn backdraft_deals_half_of_five_rounded_down_to_the_player_who_cast_the_sorcery() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &kit::lands(mountain(), 8))
        .hand(0, &[index::BLAZE, backdraft()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    tap_all_mana(&mut engine, kit::P0);
    blaze(&mut engine, kit::P0, 5, kit::P1);
    assert_eq!(kit::life(&engine, kit::P1), 15, "Blaze dealt 5 to p1");

    cast_backdraft(
        &mut engine,
        kit::P0,
        &mut kit::Answers::new().person(kit::P0),
    );
    assert_eq!(
        kit::life(&engine, kit::P0),
        18,
        "half of 5 rounded down is 2, dealt to p0, who cast the sorcery"
    );
    assert_eq!(
        kit::life(&engine, kit::P1),
        15,
        "p1 cast nothing and takes nothing from Backdraft"
    );
}

/// "Sorcery spells": an instant that dealt damage this turn is not one of them.
/// p0 casts Lightning Bolt (3) and then Blaze (5). If the Bolt counted there
/// would be two spells to choose between and the unscripted question would
/// fail the test; with only the Blaze, Backdraft asks nothing and deals half of
/// 5, which is 2 — and half of the Bolt's 3 would have been 1.
#[test]
fn backdraft_does_not_count_an_instant_that_dealt_damage() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &kit::lands(mountain(), 9))
        .hand(0, &[kit::bolt(), index::BLAZE, backdraft()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    tap_all_mana(&mut engine, kit::P0);
    let mut answers = kit::Answers::new().aim_players(&[kit::P1]);
    kit::cast(&mut engine, kit::P0, kit::bolt(), &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);
    blaze(&mut engine, kit::P0, 5, kit::P1);
    assert_eq!(
        kit::life(&engine, kit::P1),
        12,
        "3 from the Bolt and 5 from Blaze"
    );

    cast_backdraft(
        &mut engine,
        kit::P0,
        &mut kit::Answers::new().person(kit::P0),
    );
    assert_eq!(
        kit::life(&engine, kit::P0),
        18,
        "the only sorcery was Blaze: half of 5 is 2 — the Bolt's 3 would have made it 1"
    );
}

/// With no sorcery cast this turn there is no player to choose and Backdraft
/// deals nothing, whatever damage instants dealt: p0 Bolts p1 and casts
/// Backdraft, and neither life total moves beyond the Bolt. Backdraft is not a
/// sorcery either, so it does not name itself.
#[test]
fn backdraft_with_no_sorcery_cast_this_turn_deals_nothing() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &kit::lands(mountain(), 3))
        .hand(0, &[kit::bolt(), backdraft()])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    tap_all_mana(&mut engine, kit::P0);
    let mut answers = kit::Answers::new().aim_players(&[kit::P1]);
    kit::cast(&mut engine, kit::P0, kit::bolt(), &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);
    assert_eq!(kit::life(&engine, kit::P1), 17, "the Bolt dealt 3");

    cast_backdraft(&mut engine, kit::P0, &mut kit::Answers::new());
    assert_eq!(
        kit::life(&engine, kit::P0),
        20,
        "no sorcery: nothing for p0"
    );
    assert_eq!(kit::life(&engine, kit::P1), 17, "and nothing for p1 either");
}

/// A board on which both players have cast a damaging sorcery in p0's turn,
/// stopped with p0 holding priority and `Backdraft` in hand with two Mountains
/// floating. p0 cast Blaze for 5 at p1; p1 cast Borne Upon a Wind ("you may cast
/// spells this turn as though they had flash") and then Blaze for 3 at p0.
/// Returns the engine, p0's Blaze and p1's Blaze (both now in graveyards).
#[track_caller]
fn both_players_cast_a_sorcery() -> (Engine<RegistryLookup>, ObjectId, ObjectId) {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &kit::lands(mountain(), 8))
        .battlefield(
            1,
            &[
                island(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                mountain(),
            ],
        )
        .hand(0, &[index::BLAZE, backdraft()])
        .hand(1, &[borne_upon_a_wind(), index::BLAZE])
        .start();
    keep_mulligans(&mut engine);
    assert!(
        walk_to_own_main(&mut engine, kit::P0),
        "p0 reaches its own main"
    );

    // p0's sorcery: 5 at p1. Two Mountains stay floating for Backdraft.
    tap_all_mana(&mut engine, kit::P0);
    blaze(&mut engine, kit::P0, 5, kit::P1);

    // p0 passes; p1, with priority in p0's main phase, gives itself flash.
    engine
        .apply(kit::P0, PlayerAction::PassPriority)
        .expect("p0 passes with an empty stack");
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == kit::P1),
        "p1 holds priority in p0's main phase, got {:?}",
        engine.pending()
    );
    tap_all_mana(&mut engine, kit::P1);
    let mut answers = kit::Answers::new();
    kit::cast(&mut engine, kit::P1, borne_upon_a_wind(), &mut answers);
    kit::settle(&mut engine, kit::P1, &mut answers);

    // p0 passes again; p1 casts a sorcery on someone else's turn.
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == kit::P1),
        "p1 has priority again after Borne resolved, got {:?}",
        engine.pending()
    );
    assert!(
        kit::castable(&engine, kit::P1, index::BLAZE),
        "Borne Upon a Wind lets p1 cast a sorcery in p0's main phase"
    );
    let mut answers = kit::Answers::new().x(3).aim_players(&[kit::P0]);
    kit::cast(&mut engine, kit::P1, index::BLAZE, &mut answers);
    kit::settle(&mut engine, kit::P0, &mut answers);
    assert_eq!(kit::life(&engine, kit::P0), 17, "p1's Blaze dealt 3 to p0");
    assert_eq!(kit::life(&engine, kit::P1), 15, "p0's Blaze dealt 5 to p1");

    let mine = kit::graveyard_objects(&engine, kit::P0, index::BLAZE);
    let theirs = kit::graveyard_objects(&engine, kit::P1, index::BLAZE);
    assert_eq!(
        (mine.len(), theirs.len()),
        (1, 1),
        "one Blaze in each graveyard"
    );
    (engine, mine[0], theirs[0])
}

/// What Backdraft asked, in whichever shape it asked it: a spell out of the
/// two, or one of the two players. `pick_player` and `pick_spell` name the same
/// choice, so the test answers whichever question arrives.
#[track_caller]
fn assert_backdraft_asked_p0_to_choose_between_both_players(
    answers: &kit::Answers,
    spell_of_p0: ObjectId,
    spell_of_p1: ObjectId,
) {
    assert!(
        !answers.askers.is_empty() && answers.askers.iter().all(|asked| *asked == kit::P0),
        "Backdraft asked its caster, and only its caster, to choose: {:?}",
        answers.askers
    );
    let spells = answers
        .card_menus
        .iter()
        .any(|menu| menu.contains(&spell_of_p0) && menu.contains(&spell_of_p1));
    let players = answers
        .player_menus
        .iter()
        .any(|menu| menu.contains(&kit::P0) && menu.contains(&kit::P1));
    assert!(
        spells || players,
        "the question offered both players' sorceries (or both players): {:?} / {:?}",
        answers.card_menus,
        answers.player_menus
    );
}

/// Two players each cast a damaging sorcery this turn: the caster of Backdraft
/// chooses which. Choosing p1's (3 damage) deals half of it, 1, to p1 — who
/// cast it — and nothing to p0.
#[test]
fn backdraft_lets_its_caster_choose_the_other_players_sorcery() {
    let (mut engine, mine, theirs) = both_players_cast_a_sorcery();
    let mut answers = kit::Answers::new().pick(&[theirs]).person(kit::P1);
    cast_backdraft(&mut engine, kit::P0, &mut answers);

    assert_backdraft_asked_p0_to_choose_between_both_players(&answers, mine, theirs);
    assert_eq!(
        kit::life(&engine, kit::P1),
        14,
        "p1 was at 15 after p0's Blaze; half of its own Blaze's 3, rounded down, is 1"
    );
    assert_eq!(
        kit::life(&engine, kit::P0),
        17,
        "p0 chose the other player: nothing for p0"
    );
}

/// The same board, and this time the caster of Backdraft chooses its own
/// sorcery (5 damage): half is 2, dealt to p0 — "the caster of Backdraft can be
/// the chosen player" — and p1 is not touched.
#[test]
fn backdraft_lets_its_caster_choose_its_own_sorcery() {
    let (mut engine, mine, theirs) = both_players_cast_a_sorcery();
    let mut answers = kit::Answers::new().pick(&[mine]).person(kit::P0);
    cast_backdraft(&mut engine, kit::P0, &mut answers);

    assert_backdraft_asked_p0_to_choose_between_both_players(&answers, mine, theirs);
    assert_eq!(
        kit::life(&engine, kit::P0),
        15,
        "p0 was at 17 after p1's Blaze; half of its own Blaze's 5 is 2"
    );
    assert_eq!(
        kit::life(&engine, kit::P1),
        15,
        "p1 was not chosen: nothing for p1"
    );
}

/// One player, two sorceries: a Blaze that dealt 4 and a Stream of Life that
/// dealt none. The choice is the sorcery, and "half the damage dealt by one of
/// those sorcery spells" is read off the one picked — 2 for the Blaze, 0 for
/// the Stream, which is a sorcery that dealt no damage. Two boards, one per
/// answer, and the Blaze board is the control that a damage amount reaches p0
/// at all.
#[test]
fn backdraft_reads_the_damage_of_the_sorcery_it_is_pointed_at() {
    let board = || {
        let mut engine = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[
                    forest(),
                    mountain(),
                    mountain(),
                    mountain(),
                    mountain(),
                    mountain(),
                    mountain(),
                    mountain(),
                    mountain(),
                ],
            )
            .hand(0, &[index::STREAM_OF_LIFE, index::BLAZE, backdraft()])
            .start();
        keep_mulligans(&mut engine);
        assert!(
            walk_to_own_main(&mut engine, kit::P0),
            "p0 reaches its own main"
        );
        tap_all_mana(&mut engine, kit::P0);

        // Stream of Life for 1 on p0 itself: a sorcery that dealt no damage.
        let mut answers = kit::Answers::new().x(1).person(kit::P0);
        kit::cast(&mut engine, kit::P0, index::STREAM_OF_LIFE, &mut answers);
        kit::settle(&mut engine, kit::P0, &mut answers);
        assert_eq!(kit::life(&engine, kit::P0), 21, "Stream of Life gained 1");
        // Blaze for 4 at p1.
        blaze(&mut engine, kit::P0, 4, kit::P1);
        assert_eq!(kit::life(&engine, kit::P1), 16, "Blaze dealt 4");
        let stream = kit::graveyard_objects(&engine, kit::P0, index::STREAM_OF_LIFE)[0];
        let fire = kit::graveyard_objects(&engine, kit::P0, index::BLAZE)[0];
        (engine, stream, fire)
    };

    let (mut engine, stream, fire) = board();
    let mut answers = kit::Answers::new().pick(&[fire]).person(kit::P0);
    cast_backdraft(&mut engine, kit::P0, &mut answers);
    assert!(
        answers
            .card_menus
            .iter()
            .any(|menu| menu.contains(&stream) && menu.contains(&fire)),
        "one player cast two sorceries, so the caster chose between them: {:?}",
        answers.card_menus
    );
    assert_eq!(
        kit::life(&engine, kit::P0),
        19,
        "pointed at the Blaze: half of 4 is 2, off the 21"
    );

    let (mut engine, stream, fire) = board();
    let mut answers = kit::Answers::new().pick(&[stream]).person(kit::P0);
    cast_backdraft(&mut engine, kit::P0, &mut answers);
    assert!(
        answers
            .card_menus
            .iter()
            .any(|menu| menu.contains(&stream) && menu.contains(&fire)),
        "one player cast two sorceries, so the caster chose between them: {:?}",
        answers.card_menus
    );
    assert_eq!(
        kit::life(&engine, kit::P0),
        21,
        "pointed at the Stream of Life, which dealt no damage: half of 0 is 0"
    );
}
