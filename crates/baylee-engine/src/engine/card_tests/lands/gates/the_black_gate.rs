//! `cards/lands/gates/the_black_gate.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::engine::card_tests::partials_w1b_kit as kit;

/// "As The Black Gate enters, you may pay 3 life. If you don't, it enters
/// tapped."
///
/// Two games prove the two arms of the entry question.  In the first, the
/// controller declines the payment: the Gate enters tapped and no life is
/// lost.  In the second, the controller accepts: the Gate enters untapped and
/// life falls by exactly 3.  Two games because one land drop per turn is one
/// arm per game.
#[test]
fn the_black_gate_enters_untapped_for_three_life_or_tapped_for_free() {
    let p0 = PlayerId::new(0);

    // ── Arm 1: decline → enters tapped, no life lost.
    {
        let mut engine = Duel::new(43, forest()).hand(0, &[the_black_gate()]).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let life_before = engine.state().players[0].life;
        let land = in_hand(&engine, p0, the_black_gate()).expect("the Gate is in hand");
        engine
            .apply(p0, PlayerAction::PlayLand { card: land })
            .unwrap();

        let Pending::YesNo {
            player,
            prompt: crate::choice::YesNoPrompt::PayLifeOrEnterTapped { amount },
            ..
        } = engine.pending().clone()
        else {
            panic!("expected PayLifeOrEnterTapped, got {:?}", engine.pending())
        };
        assert_eq!(player, p0, "the controller decides");
        assert_eq!(amount, 3, "the Gate asks for 3 life");

        engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
        pass_until(&mut engine, stack_is_empty);

        let gate =
            on_battlefield(&engine, p0, the_black_gate()).expect("the Gate is on the battlefield");
        assert!(
            is_tapped(&engine, gate),
            "declining the payment leaves the Gate tapped"
        );
        assert_eq!(
            engine.state().players[0].life,
            life_before,
            "declining costs no life"
        );
    }

    // ── Arm 2: accept → enters untapped, costs 3 life.
    {
        let mut engine = Duel::new(44, forest()).hand(0, &[the_black_gate()]).start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, p0);

        let life_before = engine.state().players[0].life;
        let land = in_hand(&engine, p0, the_black_gate()).expect("the Gate is in hand");
        engine
            .apply(p0, PlayerAction::PlayLand { card: land })
            .unwrap();

        let Pending::YesNo {
            player,
            prompt: crate::choice::YesNoPrompt::PayLifeOrEnterTapped { amount },
            ..
        } = engine.pending().clone()
        else {
            panic!("expected PayLifeOrEnterTapped, got {:?}", engine.pending())
        };
        assert_eq!(player, p0);
        assert_eq!(amount, 3);

        engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
        pass_until(&mut engine, stack_is_empty);

        let gate =
            on_battlefield(&engine, p0, the_black_gate()).expect("the Gate is on the battlefield");
        assert!(
            !is_tapped(&engine, gate),
            "paying 3 life leaves the Gate untapped"
        );
        assert_eq!(
            engine.state().players[0].life,
            life_before - 3,
            "the payment costs exactly 3 life"
        );
    }
}

/// "{T}: Add {B}."
///
/// The Gate, untapped and alone, is offered a mana ability that makes exactly
/// one black mana, and is tapped by it.
#[test]
fn the_black_gate_taps_for_one_black_mana() {
    let p0 = kit::P0;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[the_black_gate()])
        .start();
    keep_mulligans(&mut engine);
    kit::answer_the_entry_question(&mut engine, true);
    reach_main_phase(&mut engine, p0);
    let gate = on_battlefield(&engine, p0, the_black_gate()).expect("the Gate");
    let offered = kit::offered(&engine, gate);
    assert_eq!(
        offered.len(),
        1,
        "with nothing to pay `{{1}}{{B}}` with, only the mana ability is offered: {offered:?}"
    );

    kit::activate_object(&mut engine, p0, gate, offered[0]);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.total(), 1, "one mana");
    assert_eq!(pool.available(ManaColor::Black), 1, "and it is black");
    assert!(is_tapped(&engine, gate), "tapping it is the cost");
}

/// A 3-seat table: player 0 controls The Black Gate, two Swamps to pay
/// `{1}{B}` with, and two Ogres that may attack; players 1 and 2 each control
/// one Ogre that may block. `lives` sets the three life totals once the Gate's
/// entry question (pay 3 life to enter untapped) has been answered.
///
/// Returns the engine, the Gate, player 0's two Ogres and the other two
/// players' Ogres (`[player 1's, player 2's]`).
#[track_caller]
fn the_gate_table(
    lives: [i32; 3],
) -> (
    Engine<RegistryLookup>,
    ObjectId,
    [ObjectId; 2],
    [ObjectId; 2],
) {
    let p0 = kit::P0;
    let mut engine = Duel::table(SEED, forest(), 3)
        .life(0, lives[0] + 3)
        .life(1, lives[1])
        .life(2, lives[2])
        .battlefield(
            0,
            &[the_black_gate(), swamp(), swamp(), gray_ogre(), gray_ogre()],
        )
        .battlefield(1, &[gray_ogre()])
        .battlefield(2, &[gray_ogre()])
        .start();
    keep_mulligans(&mut engine);
    // The placed Gate still asks its entry question; paying the 3 life
    // leaves it untapped (and player 0 on exactly `lives[0]`).
    kit::answer_the_entry_question(&mut engine, true);
    reach_main_phase(&mut engine, p0);
    let gate = on_battlefield(&engine, p0, the_black_gate()).expect("the Gate");
    let mine = all_on_battlefield(&engine, p0, gray_ogre());
    assert_eq!(mine.len(), 2, "two Ogres on the Gate's controller's side");
    let first = on_battlefield(&engine, kit::P1, gray_ogre()).expect("player 1's Ogre");
    let second = on_battlefield(&engine, kit::P2, gray_ogre()).expect("player 2's Ogre");
    (engine, gate, [mine[0], mine[1]], [first, second])
}

/// Pays `{1}{B}` and `{T}`, activates the Gate's third ability on `target`
/// and answers the question of which player with `pick`. Returns who was asked
/// and the menu they were offered.
#[track_caller]
fn use_the_gate(
    engine: &mut Engine<RegistryLookup>,
    gate: ObjectId,
    target: ObjectId,
    pick: PlayerId,
) -> (PlayerId, Vec<PlayerId>) {
    let p0 = kit::P0;
    tap_mana_except(engine, p0, gate);
    let offered = kit::offered(engine, gate);
    assert!(
        offered.len() >= 2,
        "the mana ability and the `{{1}}{{B}}, {{T}}` ability are both offered: {offered:?}"
    );
    let third = *offered.last().expect("a third ability");
    kit::activate_object(engine, p0, gate, third);
    let mut asked = None;
    kit::settle(engine, p0, |_, pending| match pending {
        Pending::ChoosePlayer { player, options } => {
            asked = Some((*player, options.clone()));
            Some(PlayerAction::ChoosePlayer(pick))
        }
        other => kit::choose_targets(other, &[target], &[]),
    });
    asked.expect("the controller is asked which player is meant")
}

/// Which `(blocker, attacker)` pairs `defender` may declare when `attackers`
/// attack them. Empty when the engine has nothing to ask, because no creature
/// of theirs may block anything.
#[track_caller]
fn what_may_block(
    engine: &mut Engine<RegistryLookup>,
    attackers: &[ObjectId],
    defender: PlayerId,
) -> Vec<(ObjectId, ObjectId)> {
    kit::declare_attack(engine, attackers, defender);
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
            || matches!(e.state().turn.phase, Phase::SecondMain)
    });
    let Pending::ChooseBlockers {
        player, blockers, ..
    } = engine.pending().clone()
    else {
        return Vec::new();
    };
    assert_eq!(player, defender, "the defending player declares the blocks");
    blockers
        .iter()
        .flat_map(|option| {
            option
                .attackers
                .iter()
                .map(move |attacker| (option.blocker, *attacker))
        })
        .collect()
}

/// "{1}{B}, {T}: Choose a player with the most life or tied for most life."
///
/// Player 0 is on 15 and players 1 and 2 are both on 20: the two of them tie
/// for the most life, so the Gate's controller is asked to choose among
/// exactly those two (and not themselves, who is behind). The cost is paid:
/// the Gate is tapped and the two mana are gone.
#[test]
fn the_black_gate_offers_every_player_tied_for_the_most_life() {
    let (p0, p1, p2) = (kit::P0, kit::P1, kit::P2);
    let (mut engine, gate, [target, _], _) = the_gate_table([15, 20, 20]);

    let (asked, menu) = use_the_gate(&mut engine, gate, target, p1);

    assert_eq!(asked, p0, "the Gate's controller chooses");
    assert_eq!(
        menu.len(),
        2,
        "the two tied players and nobody else: {menu:?}"
    );
    assert!(
        menu.contains(&p1) && menu.contains(&p2),
        "both players on 20 are offered: {menu:?}"
    );
    assert!(is_tapped(&engine, gate), "`{{T}}` was part of the cost");
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "`{{1}}{{B}}` was paid"
    );
}

/// "…a player with the most life": a single leader is the only choice. Player
/// 1 is on 20 against 12 and 8, so the menu is player 1 alone.
#[test]
fn the_black_gate_offers_only_the_player_with_the_most_life() {
    let (p1, _) = (kit::P1, kit::P2);
    let (mut engine, gate, [target, _], _) = the_gate_table([12, 20, 8]);

    let (_, menu) = use_the_gate(&mut engine, gate, target, p1);

    assert_eq!(menu, vec![p1], "only the strictly highest life total");
}

/// "Choose a player with the most life…" does not exclude the Gate's own
/// controller: on 20 against 10 and 5 they are the only player on the menu.
#[test]
fn the_black_gate_offers_its_controller_when_they_have_the_most_life() {
    let p0 = kit::P0;
    let (mut engine, gate, [target, _], _) = the_gate_table([20, 10, 5]);

    let (asked, menu) = use_the_gate(&mut engine, gate, target, p0);

    assert_eq!(asked, p0);
    assert_eq!(menu, vec![p0], "the controller leads and is the one choice");
}

/// "Target creature can't be blocked by creatures that player controls this
/// turn."
///
/// Both players on 20 are tied; player 2 is chosen and the Gate's first Ogre
/// is the target. Both of player 0's Ogres attack player 2: player 2's Ogre
/// may block the second one — nothing was said about it — and may not block
/// the first.
#[test]
fn the_black_gate_stops_the_chosen_players_creatures_blocking_the_target() {
    let (p2, p0) = (kit::P2, kit::P0);
    let (mut engine, gate, [target, other], [_, their_ogre]) = the_gate_table([15, 20, 20]);

    let (asked, _) = use_the_gate(&mut engine, gate, target, p2);
    assert_eq!(asked, p0);
    let pairs = what_may_block(&mut engine, &[target, other], p2);

    assert!(
        pairs.contains(&(their_ogre, other)),
        "the restriction names one creature: the other attacker may be blocked \
         ({pairs:?})"
    );
    assert!(
        !pairs.contains(&(their_ogre, target)),
        "player 2 was chosen, so player 2's Ogre can't block the target \
         ({pairs:?})"
    );
}

/// The restriction is on the creatures *that player* controls, nobody else's.
///
/// Player 1 is the chosen player, but the Ogre attacks player 2, and player 2's
/// Ogre may block it all the same. (At a table of three only the defending
/// player's creatures may block, so this is the only way to tell a restriction
/// on one player from one on every player.)
#[test]
fn the_black_gate_does_not_stop_another_players_creatures_blocking() {
    let (p0, p1, p2) = (kit::P0, kit::P1, kit::P2);
    let (mut engine, gate, [target, _], [_, their_ogre]) = the_gate_table([15, 20, 20]);

    let (asked, _) = use_the_gate(&mut engine, gate, target, p1);
    assert_eq!(asked, p0);
    let pairs = what_may_block(&mut engine, &[target], p2);

    assert!(
        pairs.contains(&(their_ogre, target)),
        "player 1 was chosen, not player 2: player 2's Ogre may block ({pairs:?})"
    );
}

/// "…this turn": the restriction ends with the turn. On the Gate's
/// controller's next turn the same Ogre attacks the same player and that
/// player's Ogre may block it again.
#[test]
fn the_black_gate_restriction_ends_at_end_of_turn() {
    let (p0, p2) = (kit::P0, kit::P2);
    let (mut engine, gate, [target, _], [_, their_ogre]) = the_gate_table([15, 20, 20]);
    let _ = use_the_gate(&mut engine, gate, target, p2);
    let pairs = what_may_block(&mut engine, &[target], p2);
    assert!(
        !pairs.contains(&(their_ogre, target)),
        "this turn it can't be blocked ({pairs:?})"
    );

    assert!(walk_to_own_main(&mut engine, p0), "back to player 0's turn");
    let pairs = what_may_block(&mut engine, &[target], p2);

    assert!(
        pairs.contains(&(their_ogre, target)),
        "next turn the restriction is gone ({pairs:?})"
    );
}
