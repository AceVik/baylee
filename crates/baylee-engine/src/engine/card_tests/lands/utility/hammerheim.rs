//! `cards/lands/utility/hammerheim.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Hammerheim: "{T}: Add {R}." / "{T}: Target creature loses all landwalk abilities until end of turn."
/// The legendary land taps to add {R} to the mana pool and cannot activate another tap ability while tapped.
#[test]
fn hammerheim_taps_for_red_mana_and_cannot_activate_again_while_tapped() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(126, forest())
        .battlefield(0, &[hammerheim()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let hammer = on_battlefield(&engine, p0, hammerheim()).expect("Hammerheim deployed");

    activate(&mut engine, p0, hammerheim(), 0);

    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&engine, hammer));

    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending());
    };
    assert!(
        !legal
            .abilities
            .iter()
            .any(|(s, ai)| *s == hammer && *ai == 1),
        "a tapped source cannot pay a second tap cost"
    );
}

use baylee_core::generated::index;
const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);
fn q_prepared(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, index::FOREST)
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    q_mana(&mut e);
    e
}
fn q_mana(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}
fn q_obj(e: &Engine<RegistryLookup>, p: PlayerId, c: CardIndex) -> ObjectId {
    on_battlefield(e, p, c).unwrap()
}
fn q_priority(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(),Pending::Priority{player,..} if *player==p),
    );
}
fn q_cast(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex) {
    q_priority(e, p);
    q_mana(e);
    let card = in_hand(e, p, c).unwrap();
    e.apply(p, PlayerAction::CastSpell { card }).unwrap();
}
fn q_targets(e: &mut Engine<RegistryLookup>, objects: Vec<ObjectId>, players: Vec<PlayerId>) {
    let p = e.pending().asked().unwrap();
    e.apply(p, PlayerAction::ChooseTargets { objects, players })
        .unwrap();
}
fn q_cast_target(e: &mut Engine<RegistryLookup>, c: CardIndex, target: ObjectId) {
    q_cast(e, P0, c);
    q_targets(e, vec![target], vec![]);
    pass_until(e, stack_is_empty);
}
fn q_activate(e: &mut Engine<RegistryLookup>, c: CardIndex, i: u32) {
    q_priority(e, P0);
    q_mana(e);
    let source = q_obj(e, P0, c);
    e.apply(
        P0,
        PlayerAction::ActivateAbility {
            source,
            ability_index: i,
        },
    )
    .unwrap();
}
fn q_next_main(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    let turn = e.state.turn.number;
    pass_until(e, |e| {
        e.state.turn.number > turn
            && e.state.turn.active == p
            && e.state.turn.phase == crate::turn::Phase::FirstMain
            && matches!(e.pending(),Pending::Priority{player,..} if *player==p)
    });
}

#[test]
fn hammerheim_removes_nonbasic_desertwalk_and_allows_actual_block() {
    let mut e = q_prepared(
        &[],
        &[index::HAMMERHEIM, index::DESERT, index::GRIZZLY_BEARS],
        &[index::DESERT_NOMADS],
    );
    let nomads = q_obj(&e, P1, index::DESERT_NOMADS);
    let bear = q_obj(&e, P0, index::GRIZZLY_BEARS);
    q_next_main(&mut e, P1);
    q_activate(&mut e, index::HAMMERHEIM, 1);
    q_targets(&mut e, vec![nomads], vec![]);
    pass_until(&mut e, stack_is_empty);
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::ChooseAttackers{player,..} if *player==P1),
    );
    e.apply(
        P1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(nomads, Defender::Player(P0))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    e.apply(
        P0,
        PlayerAction::DeclareBlockers {
            blockers: vec![(bear, nomads)],
        },
    )
    .unwrap();
}

#[test]
fn hammerheim_removal_does_not_forbid_a_later_landwalk_grant() {
    let mut e = q_prepared(
        &[index::FISHLIVER_OIL, index::FISHLIVER_OIL],
        &[index::HAMMERHEIM, index::GRIZZLY_BEARS],
        &[index::ISLAND, index::GRIZZLY_BEARS],
    );
    let bear = q_obj(&e, P0, index::GRIZZLY_BEARS);
    let blocker = q_obj(&e, P1, index::GRIZZLY_BEARS);
    q_cast_target(&mut e, index::FISHLIVER_OIL, bear);
    assert!(keywords(&e, bear).contains(KeywordSet::ISLANDWALK));
    q_activate(&mut e, index::HAMMERHEIM, 1);
    q_targets(&mut e, vec![bear], vec![]);
    pass_until(&mut e, stack_is_empty);
    assert!(!keywords(&e, bear).contains(KeywordSet::ISLANDWALK));
    q_cast_target(&mut e, index::FISHLIVER_OIL, bear);
    assert!(keywords(&e, bear).contains(KeywordSet::ISLANDWALK));
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::ChooseAttackers{player,..}if *player==P0),
    );
    e.apply(
        P0,
        PlayerAction::DeclareAttackers {
            attackers: vec![(bear, Defender::Player(P1))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let hash = e.snapshot_hash();
    assert!(
        e.apply(
            P1,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, bear)]
            }
        )
        .is_err()
    );
    assert_eq!(e.snapshot_hash(), hash);
}

// Append to lands/utility/hammerheim.rs (existing q_* helpers).
#[test]
fn hammerheim_allows_later_nonbasic_landwalk_matching_grant() {
    let mut e = q_prepared(
        &[],
        &[index::HAMMERHEIM, index::DESERT, index::GRIZZLY_BEARS],
        &[index::DESERT_NOMADS],
    );
    let nomads = q_obj(&e, P1, index::DESERT_NOMADS);
    let blocker = q_obj(&e, P0, index::GRIZZLY_BEARS);
    q_next_main(&mut e, P1);
    q_activate(&mut e, index::HAMMERHEIM, 1);
    q_targets(&mut e, vec![nomads], vec![]);
    pass_until(&mut e, stack_is_empty);
    // Transparent later grant exercises the nonbasic LandwalkMatching family,
    // for which this card set has no Implemented granting spell.
    let timestamp = e.state.next_timestamp();
    let filter = crate::effects::EffectFilter::object(&e.state, nomads);
    e.state.effects.register(crate::effects::ContinuousEffect {
        id: baylee_core::ids::EffectId::new(0),
        source: None,
        controller: P1,
        origin: crate::effects::EffectOrigin::Resolution,
        layer: baylee_cards_dsl::Layer::Ability,
        timestamp,
        duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
        filter,
        modifier: baylee_cards_dsl::Modifier::LandwalkMatching(
            &baylee_cards_dsl::Filter::HasSubtype(baylee_core::generated::subtypes::land::DESERT),
        ),
    });
    e.state.invalidate_projections();
    e.sync_static_effects();
    e.refresh_offer();
    pass_until(
        &mut e,
        |e| matches!(e.pending(),Pending::ChooseAttackers{player,..}if *player==P1),
    );
    e.apply(
        P1,
        PlayerAction::DeclareAttackers {
            attackers: vec![(nomads, Defender::Player(P0))],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
    });
    let hash = e.snapshot_hash();
    assert!(
        e.apply(
            P0,
            PlayerAction::DeclareBlockers {
                blockers: vec![(blocker, nomads)]
            }
        )
        .is_err()
    );
    assert_eq!(
        e.snapshot_hash(),
        hash,
        "later nonbasic walk grant is effective, illegal block stays atomic"
    );
}
