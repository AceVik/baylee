//! `cards/creatures/mv_7/akroma_vision_of_ixidor.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

static RED: baylee_cards_dsl::Filter =
    baylee_cards_dsl::Filter::HasColor(baylee_cards_dsl::ColorSet::from_slice(&[
        baylee_cards_dsl::Color::Red,
    ]));

fn grant(
    engine: &mut Engine<RegistryLookup>,
    target: ObjectId,
    modifier: baylee_cards_dsl::Modifier,
) {
    let filter = crate::effects::EffectFilter::object(&engine.state, target);
    let timestamp = engine.state.next_timestamp();
    engine
        .state
        .effects
        .register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: None,
            controller: P0,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp,
            duration: baylee_cards_dsl::Duration::Indefinitely,
            filter,
            modifier,
        });
    engine.state.invalidate_projections();
    engine.sync_static_effects();
    engine.refresh_offer();
}

#[test]
fn akroma_counts_each_listed_ability_at_each_combat_and_binds_the_set() {
    let counted = KeywordSet::FLYING
        .union(KeywordSet::FIRST_STRIKE)
        .union(KeywordSet::DOUBLE_STRIKE)
        .union(KeywordSet::DEATHTOUCH)
        .union(KeywordSet::HASTE)
        .union(KeywordSet::HEXPROOF)
        .union(KeywordSet::INDESTRUCTIBLE)
        .union(KeywordSet::LIFELINK)
        .union(KeywordSet::MENACE)
        .union(KeywordSet::REACH)
        .union(KeywordSet::TRAMPLE)
        .union(KeywordSet::VIGILANCE);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                index::AKROMA_VISION_OF_IXIDOR,
                index::WHITE_KNIGHT,
                index::SAKASHIMA_OF_A_THOUSAND_FACES,
            ],
        )
        .hand(0, &[serra_angel()])
        .battlefield(1, &[index::WHITE_KNIGHT])
        .start();
    keep_mulligans(&mut engine);
    assert!(matches!(engine.pending(), Pending::ChooseTargets { .. }));
    engine
        .apply(
            P0,
            PlayerAction::ChooseTargets {
                objects: vec![],
                players: vec![],
            },
        )
        .unwrap();
    reach_main_phase(&mut engine, P0);
    let akroma = on_battlefield(&engine, P0, index::AKROMA_VISION_OF_IXIDOR).unwrap();
    let knight = on_battlefield(&engine, P0, index::WHITE_KNIGHT).unwrap();
    let partner = on_battlefield(&engine, P0, index::SAKASHIMA_OF_A_THOUSAND_FACES).unwrap();
    let theirs = on_battlefield(&engine, P1, index::WHITE_KNIGHT).unwrap();
    assert!(keywords(&engine, akroma).contains(KeywordSet::PARTNER));
    assert!(keywords(&engine, partner).contains(KeywordSet::PARTNER));

    // Twelve keyword bits plus protection, not thirteen bits plus both of
    // the Knight's two protection abilities. Sakashima has only partner.
    grant(
        &mut engine,
        knight,
        baylee_cards_dsl::Modifier::AddKeyword(counted),
    );
    grant(
        &mut engine,
        knight,
        baylee_cards_dsl::Modifier::ProtectionFrom(&RED),
    );
    pass_until(&mut engine, |e| {
        e.state().turn.step == Step::CombatBegin && stack_is_empty(e)
    });
    assert_eq!(pt(&engine, knight), (15, 15));
    assert_eq!(pt(&engine, partner), (4, 2));
    assert_eq!(pt(&engine, akroma), (6, 6), "Akroma is other to nobody");
    assert_eq!(pt(&engine, theirs), (2, 2), "only my creatures");

    let late = in_hand(&engine, P0, serra_angel()).unwrap();
    engine
        .state
        .move_object(
            late,
            ZoneLocation::Battlefield,
            ZonePosition::Top,
            Cause::Effect,
        )
        .unwrap();
    engine.state.invalidate_projections();
    engine.sync_static_effects();
    engine.refresh_offer();
    assert_eq!(pt(&engine, late), (4, 4), "the set was bound on resolution");

    pass_until(&mut engine, |e| {
        e.state().turn.active == P1
            && e.state().turn.step == Step::CombatBegin
            && !stack_is_empty(e)
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, knight), (15, 15), "each combat, not only mine");
    assert_eq!(
        pt(&engine, late),
        (6, 6),
        "flying and vigilance next combat"
    );
    assert_eq!(pt(&engine, theirs), (2, 2));
    reach_their_main_phase(&mut engine, P0);
    assert_eq!(
        pt(&engine, knight),
        (2, 2),
        "the previous turn's pump ended"
    );
    assert_eq!(pt(&engine, partner), (3, 1));
    assert_eq!(pt(&engine, late), (4, 4));
}
