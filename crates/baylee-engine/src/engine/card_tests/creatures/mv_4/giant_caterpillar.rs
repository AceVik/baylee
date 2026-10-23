//! `cards/creatures/mv_4/giant_caterpillar.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn giant_caterpillar() -> CardIndex {
    card_index("43a515f2-b53a-4bb4-bb6e-0d79c9162359")
}

/// The creatures `seat` controls on the battlefield.
fn creatures_of(engine: &Engine<RegistryLookup>, seat: PlayerId) -> Vec<ObjectId> {
    engine
        .state()
        .zones
        .list(ZoneLocation::Battlefield)
        .iter()
        .copied()
        .filter(|id| {
            engine.state().object(*id).is_some_and(|o| {
                o.controller == seat && o.characteristics().types.contains(TypeSet::CREATURE)
            })
        })
        .collect()
}

/// `Giant Caterpillar` ({3}{G}, 3/3 Insect, `Coverage::Implemented`):
/// `{G}, Sacrifice this creature: Create a 1/1 green Insect creature token with
/// flying named Butterfly at the beginning of the next end step.`
#[test]
fn giant_caterpillar_sacrifices_to_create_butterfly_token_at_next_end_step() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[forest(), forest(), forest(), forest(), forest()])
        .hand(0, &[giant_caterpillar()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, giant_caterpillar());
    pass_until(&mut engine, stack_is_empty);
    let caterpillar = on_battlefield(&engine, p0, giant_caterpillar()).unwrap();

    activate(&mut engine, p0, giant_caterpillar(), 0);
    pass_until(&mut engine, stack_is_empty);

    assert_eq!(
        engine.state().object(caterpillar).unwrap().zone,
        Zone::Graveyard
    );
    assert!(
        creatures_of(&engine, p0).is_empty(),
        "no token yet: it comes at the beginning of the next end step"
    );

    // Advance past P0's end step into P1's main phase; the Butterfly token is on the battlefield.
    reach_their_main_phase(&mut engine, p1);
    let creatures = creatures_of(&engine, p0);
    assert_eq!(creatures.len(), 1, "exactly one Butterfly");
    let butterfly = creatures[0];
    let (name, _, colors) = name_value_colors(&engine, butterfly);
    assert_eq!(name, "Butterfly");
    assert_eq!(
        colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::Green])
    );
    assert!(
        engine
            .state()
            .object(butterfly)
            .unwrap()
            .characteristics()
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::INSECT)
    );
    assert_eq!(pt(&engine, butterfly), (1, 1));
    assert!(keywords(&engine, butterfly).contains(KeywordSet::FLYING));
    assert!(
        creatures_of(&engine, p1).is_empty(),
        "the opponent gets no token"
    );
}
