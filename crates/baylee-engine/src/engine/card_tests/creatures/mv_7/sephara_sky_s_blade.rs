//! `cards/creatures/mv_7/sephara_sky_s_blade.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn sephara() -> CardIndex {
    card_index("ac8952c9-bd30-48cd-b964-7e6271f9cbc8")
}

fn flying_men() -> CardIndex {
    card_index("b5666c68-059d-4e32-9b04-548b3058430d")
}

fn destroy(engine: &mut Engine<RegistryLookup>, id: ObjectId) {
    pass_until(engine, |e| {
        e.state().turn.active == P0 && e.state().turn.phase == Phase::FirstMain
    });
    kill(engine, id);
}

#[test]
fn sephara_alternative_cost_taps_four_distinct_untapped_fliers_and_only_white() {
    let mut engine = table7(
        &[
            flying_men(),
            flying_men(),
            flying_men(),
            flying_men(),
            flying_men(),
            grizzly_bears(),
            plains(),
        ],
        &[sephara()],
        &[flying_men()],
    );
    let fliers = all_on_battlefield(&engine, P0, flying_men());
    assert_eq!(fliers.len(), 5);
    engine
        .state
        .object_mut(fliers[4])
        .unwrap()
        .status
        .insert(Status::TAPPED);
    let ground = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let theirs = on_battlefield(&engine, P1, flying_men()).unwrap();
    tap_mana_where(&mut engine, P0, |_| true);
    let spell = in_hand(&engine, P0, sephara()).unwrap();
    engine
        .apply(P0, PlayerAction::CastSpell { card: spell })
        .unwrap();

    let mut paid = Vec::new();
    for expected in (1..=4).rev() {
        let Pending::ChooseCards {
            options, min, max, ..
        } = engine.pending().clone()
        else {
            panic!("expected one tap choice, got {:?}", engine.pending());
        };
        assert_eq!((min, max), (1, 1));
        assert_eq!(options.len(), expected);
        for id in &fliers[..4] {
            assert_eq!(options.contains(id), !paid.contains(id));
        }
        assert!(!options.contains(&fliers[4]), "a tapped flier cannot pay");
        assert!(!options.contains(&ground), "a ground creature cannot pay");
        assert!(!options.contains(&theirs), "an opponent's flier cannot pay");
        let chosen = options[0];
        paid.push(chosen);
        engine
            .apply(
                P0,
                PlayerAction::ChooseObjects {
                    objects: vec![chosen],
                },
            )
            .unwrap();
    }
    let stack = engine.state.object(spell).unwrap();
    assert!(stack.alt_cast, "the alternative cost was selected");
    assert_eq!(stack.paid.as_ref().map(|paid| paid.mana_spent), Some(1));
    assert_eq!(
        engine.state.players[0]
            .mana_pool
            .available(ManaColor::White),
        0,
        "payment: {:?}; pool: {:?}; pending: {:?}",
        stack.paid,
        engine.state.players[0].mana_pool,
        engine.pending(),
    );
    for id in &fliers[..4] {
        assert!(is_tapped(&engine, *id), "the chosen flier paid a tap");
    }
    assert!(
        is_tapped(&engine, fliers[4]),
        "the initially tapped flier stays tapped"
    );
    assert!(!is_tapped(&engine, ground));
    assert!(!is_tapped(&engine, theirs));
    assert_eq!(engine.state.object(spell).unwrap().zone, Zone::Stack);
    pass_until(&mut engine, stack_is_empty);
    let angel = on_battlefield(&engine, P0, sephara()).unwrap();
    assert_body(
        &engine,
        angel,
        (7, 7),
        KeywordSet::FLYING.union(KeywordSet::LIFELINK),
    );
}

#[test]
fn sephara_printed_cost_still_casts_without_creatures_to_tap() {
    let mut engine = table7(&lots(plains(), 7), &[sephara()], &[]);
    let angel = cast_and_resolve(&mut engine, sephara());
    assert_body(
        &engine,
        angel,
        (7, 7),
        KeywordSet::FLYING.union(KeywordSet::LIFELINK),
    );
    let turn = engine.state().turn.number;
    pass_until(&mut engine, |e| {
        e.state().turn.active == P0
            && e.state().turn.phase == Phase::FirstMain
            && e.state().turn.number > turn
    });
    fight(&mut engine, &[angel], &[]);
    assert_eq!(life_of(&engine, 0), 27, "lifelink gains the damage dealt");
    assert_eq!(life_of(&engine, 1), 13);
}

#[test]
fn sephara_protects_only_other_controlled_fliers_and_stops_when_she_leaves() {
    let mut engine = table7(
        &[sephara(), flying_men(), flying_men(), grizzly_bears()],
        &[],
        &[flying_men()],
    );
    let angel = on_battlefield(&engine, P0, sephara()).unwrap();
    let fliers = all_on_battlefield(&engine, P0, flying_men());
    let ground = on_battlefield(&engine, P0, grizzly_bears()).unwrap();
    let theirs = on_battlefield(&engine, P1, flying_men()).unwrap();
    assert!(!keywords(&engine, angel).contains(KeywordSet::INDESTRUCTIBLE));
    assert!(!keywords(&engine, ground).contains(KeywordSet::INDESTRUCTIBLE));
    assert!(!keywords(&engine, theirs).contains(KeywordSet::INDESTRUCTIBLE));
    for flier in &fliers {
        assert!(keywords(&engine, *flier).contains(KeywordSet::INDESTRUCTIBLE));
    }

    destroy(&mut engine, fliers[0]);
    assert_eq!(
        engine.state.object(fliers[0]).unwrap().zone,
        Zone::Battlefield
    );
    engine.state.object_mut(fliers[1]).unwrap().damage = 5;
    destroy(&mut engine, ground);
    assert_eq!(engine.state.object(ground).unwrap().zone, Zone::Graveyard);
    assert_eq!(
        engine.state.object(fliers[1]).unwrap().zone,
        Zone::Battlefield
    );
    destroy(&mut engine, theirs);
    assert_eq!(engine.state.object(theirs).unwrap().zone, Zone::Graveyard);
    destroy(&mut engine, angel);
    assert_eq!(engine.state.object(angel).unwrap().zone, Zone::Graveyard);
    engine.state.object_mut(fliers[1]).unwrap().damage = 5;
    destroy(&mut engine, fliers[0]);
    assert_eq!(
        engine.state.object(fliers[0]).unwrap().zone,
        Zone::Graveyard
    );
    assert_eq!(
        engine.state.object(fliers[1]).unwrap().zone,
        Zone::Graveyard
    );
}
