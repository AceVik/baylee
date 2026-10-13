//! `cards/sorceries/mv_3/syphon_soul.rs`, played.
//!
//! "Syphon Soul deals 2 damage to each other player. You gain life equal to
//! the damage dealt this way." Two readings are at stake: *each other
//! player* is every seat but the caster's -- a teammate included, because it
//! is not "each opponent" -- and the gain is the damage actually *dealt*, so a
//! player whose share was prevented adds nothing to it.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn syphon_soul() -> CardIndex {
    card_index("d47e55a6-7a16-4ed9-a94b-f1575588ee9e")
}

fn healing_salve() -> CardIndex {
    card_index("8da8644c-75a1-4fe9-8e94-900d948d631c")
}

/// Casts Syphon Soul from `p0`'s three Swamps and lets it resolve.
#[track_caller]
fn cast_and_resolve(engine: &mut Engine<RegistryLookup>, p0: PlayerId) {
    cast_from_hand(engine, p0, syphon_soul());
    pass_until(engine, stack_is_empty);
}

/// A duel: the one other player is dealt 2 and the caster gains 2.
#[test]
fn syphon_soul_deals_two_to_the_opponent_and_gains_that_much() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[syphon_soul()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_and_resolve(&mut engine, p0);

    assert_eq!(life_of(&engine, p1), 18, "2 damage to the other player");
    assert_eq!(life_of(&engine, p0), 22, "and the caster gains 2");
}

/// "...to each other player": at a three-player table both of the others are
/// dealt 2, and "life equal to the damage dealt this way" is the *total*,
/// 4 -- not 2, which is what a single-target reading would gain.
#[test]
fn syphon_soul_deals_two_to_every_other_player_and_gains_the_total() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[syphon_soul()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_and_resolve(&mut engine, p0);

    assert_eq!(
        life_of(&engine, p1),
        18,
        "the first other player is dealt 2"
    );
    assert_eq!(life_of(&engine, p2), 18, "so is the second");
    assert_eq!(
        life_of(&engine, p0),
        24,
        "the caster gains the total damage dealt: 2 + 2"
    );
}

/// "Each *other* player", not each opponent: a teammate of the caster is
/// dealt 2 as well, and the caster gains for both.
#[test]
fn syphon_soul_also_hits_a_teammate_of_its_caster() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(SEED, forest(), 3)
        .team(0, 1)
        .team(1, 1)
        .team(2, 2)
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[syphon_soul()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_and_resolve(&mut engine, p0);

    assert_eq!(
        life_of(&engine, p1),
        18,
        "the caster's teammate is another player, so they are dealt 2 too"
    );
    assert_eq!(life_of(&engine, p2), 18, "and so is the opponent");
    assert_eq!(life_of(&engine, p0), 24, "the caster gains for both: 2 + 2");
}

/// "You gain life equal to the damage dealt this way": damage that a
/// prevention effect stopped was not dealt. A Healing Salve cast by `p1`
/// (prevent the next 3 damage to them) absorbs their whole share, so the
/// caster gains only what `p2` took.
#[test]
fn syphon_soul_gains_only_the_damage_that_was_actually_dealt() {
    let (p0, p1, p2) = (PlayerId::new(0), PlayerId::new(1), PlayerId::new(2));
    let mut engine = Duel::table(SEED, forest(), 3)
        .battlefield(0, &[swamp(), swamp(), swamp()])
        .hand(0, &[syphon_soul()])
        .battlefield(1, &[plains()])
        .hand(1, &[healing_salve()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    // p1 answers in p0's main phase, before the sorcery is cast.
    engine.apply(p0, PlayerAction::PassPriority).unwrap();
    assert!(
        matches!(engine.pending(), Pending::Priority { player, .. } if *player == p1),
        "p1 holds priority after p0 passes: {:?}",
        engine.pending()
    );
    tap_all_mana(&mut engine, p1);
    cast_with_floating(&mut engine, p1, healing_salve());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected a mode choice, got {:?}", engine.pending())
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, crate::choice::CastModeKind::Mode(1)))
        .expect("the prevention mode is offered");
    engine.apply(p1, PlayerAction::ChooseMode(slot)).unwrap();
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("p1 is any target");
    pass_until(&mut engine, |e| {
        stack_is_empty(e)
            && matches!(e.pending(), Pending::Priority { player, .. } if *player == p0)
    });

    cast_and_resolve(&mut engine, p0);

    assert_eq!(life_of(&engine, p1), 20, "p1's 2 damage was prevented");
    assert_eq!(life_of(&engine, p2), 18, "p2 took the full 2");
    assert_eq!(
        life_of(&engine, p0),
        22,
        "the caster gains 2 -- only what p2 was actually dealt"
    );
}
