//! Word of Undoing — `{U}` Instant (Ice Age)
//!
//! Return target creature and all white Auras you own attached to it to their
//! owners' hands.

#[allow(clippy::wildcard_imports)]
use super::*;

fn word_of_undoing() -> CardIndex {
    card_index("2597a1ee-fe9e-44b5-a5e3-d34332c6c123")
}

fn white_scarab() -> CardIndex {
    card_index("ec9c3906-8628-48c6-96a7-8c6748cba816")
}

fn unholy_strength() -> CardIndex {
    card_index("090d88a9-7f2d-4bd1-a30a-7c48d05068be")
}

fn cast_at_object(
    engine: &mut Engine<RegistryLookup>,
    player: PlayerId,
    card: ObjectId,
    target: ObjectId,
) {
    engine
        .apply(player, PlayerAction::CastSpell { card })
        .unwrap();
    engine
        .apply(
            player,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .unwrap();
    pass_until(engine, stack_is_empty);
}

#[test]
fn word_of_undoing_returns_target_creature_and_only_white_auras_you_own_attached_to_it() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, island())
        .hand(0, &[white_scarab(), unholy_strength(), word_of_undoing()])
        .hand(1, &[white_scarab()])
        .battlefield(0, &[plains(), swamp(), island()])
        .battlefield(1, &[plains(), grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let target_bears = on_battlefield(&engine, p1, grizzly_bears()).unwrap();
    let our_white_aura = in_hand(&engine, p0, white_scarab()).unwrap();
    let our_black_aura = in_hand(&engine, p0, unholy_strength()).unwrap();
    let spell = in_hand(&engine, p0, word_of_undoing()).unwrap();

    // p0 attaches White Scarab (white Aura owned by p0) and Unholy Strength (black Aura owned by p0) to p1's Grizzly Bears.
    tap_all_mana(&mut engine, p0);
    cast_at_object(&mut engine, p0, our_white_aura, target_bears);

    cast_at_object(&mut engine, p0, our_black_aura, target_bears);

    // Advance to p1's main phase so p1 can attach their own White Scarab (white Aura owned by p1) to Grizzly Bears.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p1 && e.state().turn.phase == Phase::FirstMain
    });
    let opp_white_aura = in_hand(&engine, p1, white_scarab()).unwrap();
    tap_all_mana(&mut engine, p1);
    cast_at_object(&mut engine, p1, opp_white_aura, target_bears);

    // Advance to p0's next main phase so p0's Island untaps, then cast Word of Undoing targeting p1's Grizzly Bears.
    pass_until(&mut engine, |e| {
        e.state().turn.active == p0 && e.state().turn.phase == Phase::FirstMain
    });
    tap_all_mana(&mut engine, p0);
    cast_at_object(&mut engine, p0, spell, target_bears);

    assert_eq!(
        engine.state().object(target_bears).unwrap().zone,
        Zone::Hand,
        "target creature returned to its owner's hand"
    );
    assert_eq!(
        engine.state().object(our_white_aura).unwrap().zone,
        Zone::Hand,
        "white Aura owned by p0 returned to p0's hand"
    );
    assert_eq!(
        engine.state().object(our_black_aura).unwrap().zone,
        Zone::Graveyard,
        "non-white Aura owned by p0 was not returned to hand and went to graveyard by SBA"
    );
    assert_eq!(
        engine.state().object(opp_white_aura).unwrap().zone,
        Zone::Graveyard,
        "white Aura owned by opponent p1 was not returned to hand and went to graveyard by SBA"
    );
}
