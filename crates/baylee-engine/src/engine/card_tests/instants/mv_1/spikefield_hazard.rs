//! `cards/instants/mv_1/spikefield_hazard.rs`, played.
//!
//! Spikefield Hazard // Spikefield Cave. Front: "Spikefield Hazard deals 1
//! damage to any target. If a permanent dealt damage this way would die this
//! turn, exile it instead." Back: "This land enters tapped. {T}: Add {R}."
//!
//! The rider is the card: it is *permanents dealt damage this way* and *this
//! turn*, so it does not reach a creature the spell only aimed at (its damage
//! was prevented), a creature it never touched, or a turn after the one it
//! was cast in.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Whether `card` is in `seat`'s exile zone.
fn in_exile(engine: &Engine<RegistryLookup>, seat: PlayerId, card: CardIndex) -> bool {
    engine
        .state()
        .zones
        .list(ZoneLocation::Exile(seat))
        .iter()
        .any(|&id| {
            engine
                .state()
                .object(id)
                .is_some_and(|o| o.card.is_some_and(|c| c.index == card))
        })
}

/// Casts Spikefield Hazard's front face off the floating mana at `target`
/// and lets it resolve.
#[track_caller]
fn hazard_at_creature(engine: &mut Engine<RegistryLookup>, p0: PlayerId, target: ObjectId) {
    cast_front_face(engine, p0, spikefield_hazard());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![target],
            },
        )
        .expect("the creature is a legal target");
    pass_until(engine, stack_is_empty);
}

/// Casts Healing Salve's second mode ("prevent the next 3 damage that would
/// be dealt to any target this turn") on the creature `target`.
#[track_caller]
fn shield_creature(engine: &mut Engine<RegistryLookup>, p0: PlayerId, target: ObjectId) {
    cast_with_floating(engine, p0, healing_salve());
    let Pending::ChooseCastMode { options, .. } = engine.pending().clone() else {
        panic!("expected a mode choice, got {:?}", engine.pending())
    };
    let slot = options
        .iter()
        .position(|o| matches!(o.kind, CastModeKind::Mode(1)))
        .expect("the prevention mode is offered");
    engine.apply(p0, PlayerAction::ChooseMode(slot)).unwrap();
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .expect("the creature is any target");
    pass_until(engine, stack_is_empty);
}

/// Spikefield Hazard, front face: one damage, and a permanent it kills is
/// exiled instead of dying -- "if a permanent dealt damage this way would die
/// this turn, exile it instead" covers the very damage that lets it die.
#[test]
fn spikefield_hazard_deals_one_damage() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain()])
        .hand(0, &[spikefield_hazard()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is seated");
    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, spikefield_hazard());
    engine
        .apply(p0, PlayerAction::ChooseObjects { objects: vec![elf] })
        .expect("the Elf is a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_none(),
        "one damage on a 1/1 is lethal (CR 704.5g)"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_none(),
        "\"exile it instead\": it did not go to the graveyard"
    );
    assert!(
        in_exile(&engine, p1, llanowar_elves()),
        "it is in its owner's exile zone"
    );
}

/// "If a permanent dealt damage this way would die this turn, exile it
/// instead": the Bears survive the 1 damage, and when a Lightning Bolt kills
/// them later in the same turn they are exiled rather than put into the
/// graveyard.
#[test]
fn spikefield_hazard_exiles_a_damaged_creature_that_dies_later_this_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), mountain()])
        .hand(0, &[spikefield_hazard(), lightning_bolt()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("the Bears are seated");

    tap_all_mana(&mut engine, p0);
    hazard_at_creature(&mut engine, p0, bears);
    assert_eq!(
        engine.state().object(bears).map(|o| o.damage),
        Some(1),
        "a 2/2 survives 1 damage"
    );
    cast_with_floating(&mut engine, p0, lightning_bolt());
    engine
        .apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: vec![bears],
            },
        )
        .expect("the Bears are a legal target");
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "the Bolt killed them"
    );
    assert!(
        in_graveyard(&engine, p1, grizzly_bears()).is_none(),
        "but they were dealt damage this way, so they were exiled instead of dying"
    );
    assert!(in_exile(&engine, p1, grizzly_bears()), "they are in exile");
}

/// "...a permanent dealt damage this way": a creature the spell targeted but
/// whose damage was prevented was not dealt damage (CR 615.1). A Healing
/// Salve shield soaks up the 1, and when the Bears die later in the turn
/// they go to the graveyard like any creature.
#[test]
fn spikefield_hazard_does_not_exile_a_creature_whose_damage_was_prevented() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain(), plains()])
        .hand(0, &[spikefield_hazard(), healing_salve()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("the Bears are seated");

    tap_all_mana(&mut engine, p0);
    shield_creature(&mut engine, p0, bears);
    hazard_at_creature(&mut engine, p0, bears);
    assert_eq!(
        engine.state().object(bears).map(|o| o.damage),
        Some(0),
        "the shield prevented the 1 damage: none was dealt"
    );
    kill(&mut engine, bears);

    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "they died"
    );
    assert!(
        in_graveyard(&engine, p1, grizzly_bears()).is_some(),
        "into the graveyard: the Hazard dealt them no damage, so its exile rider \
         does not reach them"
    );
    assert!(!in_exile(&engine, p1, grizzly_bears()));
}

/// "Any target" takes a player: 1 damage to the opponent, and nothing the
/// Hazard did reaches a creature it never touched -- an Elf that dies later
/// the same turn goes to the graveyard.
#[test]
fn spikefield_hazard_hits_a_player_and_leaves_other_creatures_to_die_normally() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain()])
        .hand(0, &[spikefield_hazard()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("the Elf is seated");

    tap_all_mana(&mut engine, p0);
    cast_front_face(&mut engine, p0, spikefield_hazard());
    let Pending::ChooseTargets { player_options, .. } = engine.pending().clone() else {
        panic!("expected a target choice, got {:?}", engine.pending())
    };
    assert!(
        player_options.contains(&p1),
        "\"any target\" includes the opponent: {player_options:?}"
    );
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![p1],
            },
        )
        .expect("a player is a legal target");
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(life_of(&engine, p1), 19, "1 damage to the player");

    kill(&mut engine, elf);
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the Elf was not dealt damage this way: it dies into the graveyard"
    );
    assert!(!in_exile(&engine, p1, llanowar_elves()));
}

/// "...would die **this turn**": the rider ends with the turn. The Bears take
/// 1 damage on p0's turn and survive it; when they are destroyed on p1's
/// turn they go to the graveyard.
#[test]
fn spikefield_hazard_exile_rider_ends_with_the_turn() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, mountain())
        .battlefield(0, &[mountain()])
        .hand(0, &[spikefield_hazard()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("the Bears are seated");

    tap_all_mana(&mut engine, p0);
    hazard_at_creature(&mut engine, p0, bears);
    reach_their_main_phase(&mut engine, p1);
    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_some(),
        "the Bears are still there on the next turn"
    );
    kill(&mut engine, bears);

    assert!(
        in_graveyard(&engine, p1, grizzly_bears()).is_some(),
        "a turn later they die normally: the rider was only for the turn it was cast"
    );
    assert!(!in_exile(&engine, p1, grizzly_bears()));
}

/// Spikefield Cave, the back face: "This land enters tapped. {T}: Add {R}."
/// Played as a land it comes in tapped, stands up in its controller's next
/// turn, and taps for one red mana.
#[test]
fn spikefield_cave_enters_tapped_and_taps_for_red() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let (mut engine, cave) =
        play_land_face(spikefield_hazard(), 1).expect("plays as Spikefield Cave");
    assert!(is_tapped(&engine, cave), "\"This land enters tapped\"");

    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        !is_tapped(&engine, cave),
        "it untaps in the next untap step"
    );
    let red_before = engine.state().players[0]
        .mana_pool
        .available(ManaColor::Red);

    // Ability 0 of the back face is its mana ability, as on the other back
    // faces that are lands: `activate` refuses when the face offers none.
    activate(&mut engine, p0, spikefield_hazard(), 0);

    assert!(is_tapped(&engine, cave), "{{T}} taps it");
    assert_eq!(
        engine.state().players[0]
            .mana_pool
            .available(ManaColor::Red),
        red_before + 1,
        "{{T}}: Add {{R}}"
    );
}
