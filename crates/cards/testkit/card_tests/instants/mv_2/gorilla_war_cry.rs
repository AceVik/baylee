//! `cards/instants/mv_2/gorilla_war_cry.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn gorilla_war_cry() -> CardIndex {
    card_index("60a7f61c-41ad-4dce-b556-a99bf38c3d73")
}

fn hill_giant() -> CardIndex {
    card_index("342199e0-15b6-4824-83da-25caef2592b3")
}

/// Player 0 attacks player 1 with a Grizzly Bears; player 1 has a Hill Giant and a Gray Ogre.
/// Gorilla War Cry is in player 0's hand, two Mountains untapped. Returns in the declare attackers
/// step, with priority.
fn bears_attack() -> (Engine<RegistryLookup>, ObjectId) {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), grizzly_bears()])
        .battlefield(1, &[hill_giant(), gray_ogre()])
        .hand(0, &[gorilla_war_cry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p0, grizzly_bears()).unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.step == Step::DeclareAttackers
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(bears, Defender::Player(p1))],
            },
        )
        .unwrap();
    (engine, bears)
}

/// "Cast this spell only during combat before blockers are declared": with the mana floating, a
/// main phase refuses it (a test without mana would be refused for the wrong reason).
#[test]
fn gorilla_war_cry_cannot_be_cast_in_a_main_phase() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain(), grizzly_bears()])
        .hand(0, &[gorilla_war_cry()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let spell = in_hand(&engine, p0, gorilla_war_cry()).unwrap();
    tap_all_mana(&mut engine, p0);
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: spell })
            .is_err(),
        "paid for, but a main phase is not combat"
    );
    assert!(in_hand(&engine, p0, gorilla_war_cry()).is_some());
}

/// ... and once blockers are declared it is too late, again with the mana floating.
#[test]
fn gorilla_war_cry_cannot_be_cast_after_blockers_are_declared() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, _bears) = bears_attack();
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    let spell = in_hand(&engine, p0, gorilla_war_cry()).unwrap();
    tap_all_mana(&mut engine, p0);
    assert!(
        engine
            .apply(p0, PlayerAction::CastSpell { card: spell })
            .is_err(),
        "blockers are declared"
    );
}

/// "All creatures gain menace": the attacker, and the other player's creatures too. A single
/// blocker is then refused and two are accepted.
#[test]
fn gorilla_war_cry_gives_every_creature_menace() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, bears) = bears_attack();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let ogre = on_battlefield(&engine, p1, gray_ogre()).unwrap();

    cast_from_hand(&mut engine, p0, gorilla_war_cry());
    pass_until(&mut engine, stack_is_empty);
    for creature in [bears, giant, ogre] {
        assert!(
            keywords(&engine, creature).contains(KeywordSet::MENACE),
            "{creature:?} has menace"
        );
    }

    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    assert!(
        engine
            .apply(
                p1,
                PlayerAction::DeclareBlockers {
                    blockers: vec![(giant, bears)],
                },
            )
            .is_err(),
        "menace: one blocker is not enough"
    );
    engine
        .apply(
            p1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(giant, bears), (ogre, bears)],
            },
        )
        .expect("two blockers are enough");
}

/// "Until end of turn" and "draw a card at the beginning of the next turn's upkeep": the menace is
/// gone in the opponent's turn, the card comes in its upkeep (replacing the spell in hand), and it
/// is one card only: the turn after brings just the ordinary draw.
#[test]
fn gorilla_war_cry_menace_ends_and_it_draws_exactly_one_card_next_upkeep() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, bears) = bears_attack();
    let giant = on_battlefield(&engine, p1, hill_giant()).unwrap();
    let hand_size = |e: &Engine<RegistryLookup>| e.state().zones.list(ZoneLocation::Hand(p0)).len();
    let before = hand_size(&engine);

    cast_from_hand(&mut engine, p0, gorilla_war_cry());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(hand_size(&engine), before - 1, "the spell left the hand");
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::DeclareBlockers
    });
    engine
        .apply(p1, PlayerAction::DeclareBlockers { blockers: vec![] })
        .unwrap();

    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.step == Step::Upkeep
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        hand_size(&engine),
        before,
        "the drawn card replaced the spell"
    );

    reach_their_main_phase(&mut engine, p1);
    assert!(!keywords(&engine, bears).contains(KeywordSet::MENACE));
    assert!(!keywords(&engine, giant).contains(KeywordSet::MENACE));
    assert_eq!(hand_size(&engine), before, "the opponent's draw is its own");

    reach_their_main_phase(&mut engine, p0);
    assert_eq!(
        hand_size(&engine),
        before + 1,
        "only the ordinary draw this time"
    );
}
