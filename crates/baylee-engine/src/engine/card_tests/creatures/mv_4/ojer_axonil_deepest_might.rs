//! `cards/creatures/mv_4/ojer_axonil_deepest_might.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Ojer Axonil, Deepest Might` // `Temple of Power` (`Coverage::Partial`):
/// "Trample. If a red source you control would deal an amount of noncombat damage less than
/// `Ojer Axonil`'s power to an opponent, that source deals damage equal to `Ojer Axonil`'s power instead.
/// When `Ojer Axonil` dies, return it to the battlefield tapped and transformed under its owner's
/// control. // `{{T}}`: Add `{{R}}`. `{{2}}{{R}}`, `{{T}}`: Transform this land. Activate only if red sources
/// you controlled dealt 4 or more noncombat damage this turn and only as a sorcery."
///
/// Under `Coverage::Partial`, the noncombat damage replacement and transform-back are omitted,
/// leaving `KeywordSet::TRAMPLE` on a 4/4 God and the dies-trigger `Effect::ExileSelfReturnAsFace`.
/// The test verifies that `Ojer Axonil` starts with trample, is destroyed by an opponent's
/// `heroes_downfall()`, and returns to the battlefield transformed as the land `Temple of Power` on face 1.
#[test]
fn ojer_axonil_deepest_might_dies_and_returns_as_temple_of_power() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(309, swamp())
        .battlefield(0, &[ojer_axonil_deepest_might()])
        .battlefield(1, &[swamp(), swamp(), swamp()])
        .hand(1, &[heroes_downfall()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let god = on_battlefield(&engine, p0, ojer_axonil_deepest_might())
        .expect("Ojer Axonil on battlefield");
    assert_eq!(pt(&engine, god), (4, 4), "starts as a 4/4 God");
    assert!(
        keywords(&engine, god).contains(KeywordSet::TRAMPLE),
        "Ojer Axonil has trample"
    );

    // Advance to p1's main phase and cast Hero's Downfall targeting Ojer Axonil.
    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, heroes_downfall());

    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    engine
        .apply(
            p1,
            PlayerAction::ChooseTargets {
                objects: vec![god],
                players: vec![],
            },
        )
        .unwrap();

    pass_until(&mut engine, stack_is_empty);

    let temple = on_battlefield(&engine, p0, ojer_axonil_deepest_might())
        .expect("returned to battlefield transformed");
    assert_eq!(
        engine.state().object(temple).map(|o| o.face_index),
        Some(1),
        "returned as face 1 (Temple of Power)"
    );

    let t = types(&engine, temple);
    assert!(t.contains(TypeSet::LAND), "Temple of Power is a land");
    assert!(
        !t.contains(TypeSet::CREATURE),
        "Temple of Power is not a creature"
    );
    assert!(
        in_graveyard(&engine, p0, ojer_axonil_deepest_might()).is_none(),
        "the card is on the battlefield and not in the graveyard"
    );
}

use crate::engine::card_tests::partials_w1b_kit as kit;
use baylee_core::generated::index;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn aim(e: &mut Engine<RegistryLookup>, seat: PlayerId, objects: &[ObjectId], players: &[PlayerId]) {
    let Pending::ChooseTargets { player, .. } = e.pending() else {
        panic!("target question: {:?}", e.pending())
    };
    assert_eq!(*player, seat);
    e.apply(
        seat,
        PlayerAction::ChooseTargets {
            objects: objects.to_vec(),
            players: players.to_vec(),
        },
    )
    .unwrap();
}

fn cast_aim(
    e: &mut Engine<RegistryLookup>,
    seat: PlayerId,
    card: CardIndex,
    objects: &[ObjectId],
    players: &[PlayerId],
) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == seat),
    );
    cast_from_hand(e, seat, card);
    aim(e, seat, objects, players);
}

#[test]
fn batch57_ojer_uses_current_power_and_only_red_owned_noncombat_damage_to_opponents() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::OJER_AXONIL_DEEPEST_MIGHT,
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::FOREST,
                index::ISLAND,
                index::ISLAND,
                index::ISLAND,
            ],
        )
        .hand(
            0,
            &[
                index::LIGHTNING_BOLT,
                index::LIGHTNING_BOLT,
                index::GIANT_GROWTH,
                index::SHOCK,
                index::PSIONIC_BLAST,
            ],
        )
        .battlefield(1, &[index::MOUNTAIN])
        .hand(1, &[index::LIGHTNING_BOLT])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let god = on_battlefield(&e, P0, index::OJER_AXONIL_DEEPEST_MIGHT).unwrap();
    cast_aim(&mut e, P0, index::LIGHTNING_BOLT, &[], &[P1]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[1].life, 16);
    cast_aim(&mut e, P0, index::GIANT_GROWTH, &[god], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(pt(&e, god), (7, 7));
    cast_aim(&mut e, P0, index::SHOCK, &[], &[P1]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().players[1].life, 9);
    cast_aim(&mut e, P0, index::PSIONIC_BLAST, &[], &[P1]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().players[1].life,
        5,
        "blue source is unchanged despite power seven"
    );
    assert_eq!(e.state().players[0].life, 18);
    cast_aim(&mut e, P0, index::LIGHTNING_BOLT, &[], &[P0]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().players[0].life,
        15,
        "your own damage recipient is not an opponent"
    );
    cast_aim(&mut e, P1, index::LIGHTNING_BOLT, &[], &[P0]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().players[0].life,
        12,
        "opponent-owned red source is not modified"
    );
}

#[test]
fn batch57_ojer_tramples_without_raising_combat_damage() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::OJER_AXONIL_DEEPEST_MIGHT])
        .battlefield(1, &[index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    let god = on_battlefield(&e, P0, index::OJER_AXONIL_DEEPEST_MIGHT).unwrap();
    let bear = on_battlefield(&e, P1, index::GRIZZLY_BEARS).unwrap();
    attack_and_collect_blocks(&mut e, god, P1);
    e.apply(
        P1,
        PlayerAction::DeclareBlockers {
            blockers: vec![(bear, god)],
        },
    )
    .unwrap();
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(
        e.state().players[1].life,
        18,
        "two excess combat damage, not a floor of four"
    );
    assert!(in_graveyard(&e, P1, index::GRIZZLY_BEARS).is_some());
}

/// Read the damage log independently of the activation condition.
fn red_noncombat_damage(e: &Engine<RegistryLookup>) -> u32 {
    e.state()
        .per_turn
        .damage_log
        .iter()
        .filter(|d| {
            d.snapshot.controller == P0
                && d.snapshot
                    .characteristics()
                    .colors
                    .contains(baylee_core::color::Color::Red)
                && !d.combat
        })
        .map(|d| d.amount)
        .sum()
}

#[test]
fn batch57_ojer_temple_counts_multiple_red_sources_and_all_recipients_then_transforms_as_sorcery() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(
            0,
            &[
                index::OJER_AXONIL_DEEPEST_MIGHT,
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::MOUNTAIN,
                index::PRODIGAL_PYROMANCER,
            ],
        )
        .hand(0, &[index::LIGHTNING_BOLT])
        .battlefield(
            1,
            &[
                index::SWAMP,
                index::SWAMP,
                index::SWAMP,
                index::ROOTBREAKER_WURM,
            ],
        )
        .hand(1, &[index::HERO_S_DOWNFALL])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    reach_their_main_phase(&mut e, P1);
    let god = on_battlefield(&e, P0, index::OJER_AXONIL_DEEPEST_MIGHT).unwrap();
    cast_aim(&mut e, P1, index::HERO_S_DOWNFALL, &[god], &[]);
    pass_until(&mut e, stack_is_empty);
    let temple = on_battlefield(&e, P0, index::OJER_AXONIL_DEEPEST_MIGHT).unwrap();
    assert!(is_tapped(&e, temple));
    assert_eq!(e.state().object(temple).unwrap().face_index, 1);
    reach_their_main_phase(&mut e, P0);
    assert!(!is_tapped(&e, temple));
    assert!(!priority_offer(&e).abilities.contains(&(temple, 1)));
    let mountain = on_battlefield(&e, P0, index::MOUNTAIN).unwrap();
    let wurm = on_battlefield(&e, P1, index::ROOTBREAKER_WURM).unwrap();
    tap_mana_where(&mut e, P0, |id| id == mountain);
    cast_with_floating(&mut e, P0, index::LIGHTNING_BOLT);
    aim(&mut e, P0, &[wurm], &[]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(wurm).unwrap().damage, 3);
    assert!(
        !priority_offer(&e).abilities.contains(&(temple, 1)),
        "three actual damage is insufficient"
    );
    assert!(
        !priority_offer(&e)
            .unpaid_abilities
            .iter()
            .any(|(source, ability, _)| *source == temple && *ability == 1)
    );
    let pyro = on_battlefield(&e, P0, index::PRODIGAL_PYROMANCER).unwrap();
    kit::activate_object(&mut e, P0, pyro, 0);
    aim(&mut e, P0, &[], &[P1]);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(red_noncombat_damage(&e), 4);
    assert!(!is_tapped(&e, temple));
    assert!(
        priority_offer(&e)
            .unpaid_abilities
            .iter()
            .any(|(source, ability, _)| *source == temple && *ability == 1),
        "four actual damage offers the mana-unpaid activation with three untapped Mountains available"
    );
    pass_until(&mut e, |e| {
        e.state().turn.step == Step::CombatBegin
            && matches!(e.pending(),Pending::Priority{player,..}if *player==P0)
    });
    assert!(
        !priority_offer(&e).abilities.contains(&(temple, 1)),
        "not during combat, with three untapped Mountains available"
    );
    assert!(
        !priority_offer(&e)
            .unpaid_abilities
            .iter()
            .any(|(source, ability, _)| *source == temple && *ability == 1)
    );
    pass_until(&mut e, |e| e.state().turn.phase == Phase::SecondMain);
    assert_eq!(tap_mana_except(&mut e, P0, temple), 3);
    assert_eq!(e.state().players[0].mana_pool.available(ManaColor::Red), 3);
    kit::activate_object(&mut e, P0, temple, 1);
    pass_until(&mut e, stack_is_empty);
    assert_eq!(e.state().object(temple).unwrap().face_index, 0);
    assert_eq!(pt(&e, temple), (4, 4));
    assert!(is_tapped(&e, temple));
    assert!(keywords(&e, temple).contains(KeywordSet::TRAMPLE));
    assert_eq!(e.state().players[0].mana_pool.total(), 0);
}

#[test]
fn batch57_ojer_temple_taps_for_exactly_one_red() {
    let mut e = Duel::new(SEED, index::FOREST)
        .battlefield(0, &[index::OJER_AXONIL_DEEPEST_MIGHT])
        .battlefield(1, &[index::SWAMP; 3])
        .hand(1, &[index::HERO_S_DOWNFALL])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    reach_their_main_phase(&mut e, P1);
    let god = on_battlefield(&e, P0, index::OJER_AXONIL_DEEPEST_MIGHT).unwrap();
    cast_aim(&mut e, P1, index::HERO_S_DOWNFALL, &[god], &[]);
    pass_until(&mut e, stack_is_empty);
    reach_their_main_phase(&mut e, P0);
    let temple = on_battlefield(&e, P0, index::OJER_AXONIL_DEEPEST_MIGHT).unwrap();
    kit::activate_object(&mut e, P0, temple, 0);
    assert_eq!(e.state().players[0].mana_pool.total(), 1);
    assert_eq!(e.state().players[0].mana_pool.available(ManaColor::Red), 1);
    assert!(is_tapped(&e, temple));
}
