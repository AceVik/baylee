//! Independent real-card self-reference review: CR 400.7, 400.7j,
//! 113.7a, 611.2c, and 603.7c. No returned object inherits a waiting pump.

#[allow(clippy::wildcard_imports)] // Shared behavioral card-test vocabulary.
use super::*;
use baylee_core::generated::index;

const OWNER: PlayerId = PlayerId::new(0);
const OTHER: PlayerId = PlayerId::new(1);

fn priority(engine: &mut Engine<RegistryLookup>, player: PlayerId) {
    pass_until(
        engine,
        |e| matches!(e.pending(), Pending::Priority { player: holder, .. } if *holder == player),
    );
}

fn version(engine: &Engine<RegistryLookup>, object: ObjectId) -> u32 {
    engine.state().object(object).unwrap().version
}

fn blink(engine: &mut Engine<RegistryLookup>, object: ObjectId) {
    let old = version(engine, object);
    priority(engine, OWNER);
    cast_from_hand(engine, OWNER, ephemerate());
    aim(engine, vec![object], vec![]);
    pass_until(engine, |e| {
        version(e, object) != old && matches!(e.pending(), Pending::Priority { .. })
    });
    assert_eq!(
        engine.state().object(object).unwrap().zone,
        Zone::Battlefield
    );
}

fn mana_from(engine: &mut Engine<RegistryLookup>, player: PlayerId, card: CardIndex) {
    for source in all_on_battlefield(engine, player, card) {
        if !is_tapped(engine, source) {
            engine
                .apply(player, PlayerAction::ActivateManaAbility { source })
                .unwrap();
        }
    }
}

fn end_step_delays(engine: &Engine<RegistryLookup>) -> usize {
    engine
        .state()
        .delayed
        .iter()
        .filter(|trigger| trigger.when == crate::state::DelayedWhen::NextEndStep)
        .count()
}

#[test]
fn self_review_four_alpha_pumpers_do_not_pump_the_returned_creature() {
    for (card, land, base, boosted) in [
        (index::SHIVAN_DRAGON, mountain(), (5, 5), (6, 5)),
        (index::FROZEN_SHADE, swamp(), (0, 1), (1, 2)),
        (index::GRANITE_GARGOYLE, mountain(), (2, 2), (2, 3)),
        (index::DRAGON_WHELP, mountain(), (2, 3), (3, 3)),
    ] {
        for do_blink in [false, true] {
            let mut engine = Duel::new(SEED, forest())
                .battlefield(0, &[card, land, plains()])
                .hand(0, &[ephemerate()])
                .start();
            keep_mulligans(&mut engine);
            reach_main_phase(&mut engine, OWNER);
            let creature = on_battlefield(&engine, OWNER, card).unwrap();
            mana_from(&mut engine, OWNER, land);
            activate(&mut engine, OWNER, card, 0);
            if do_blink {
                blink(&mut engine, creature);
            }
            pass_until(&mut engine, stack_is_empty);
            assert_eq!(
                pt(&engine, creature),
                if do_blink { base } else { boosted },
                "card {card:?}, blink={do_blink}"
            );
            pass_until(&mut engine, |e| e.state().turn.number >= 2);
            assert_eq!(
                pt(&engine, creature),
                base,
                "cleanup removes the ordinary pump"
            );
            assert_eq!(engine.state().players[0].life, 20);
        }
    }
}

#[test]
fn self_review_shivan_multiple_old_activations_and_one_fresh_activation() {
    let card = index::SHIVAN_DRAGON;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[card, mountain(), mountain(), mountain(), plains()])
        .hand(0, &[ephemerate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, OWNER);
    let dragon = on_battlefield(&engine, OWNER, card).unwrap();
    mana_from(&mut engine, OWNER, mountain());
    activate(&mut engine, OWNER, card, 0);
    activate(&mut engine, OWNER, card, 0);
    blink(&mut engine, dragon);
    priority(&mut engine, OWNER);
    activate(&mut engine, OWNER, card, 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        pt(&engine, dragon),
        (6, 5),
        "only the fresh activation pumps"
    );
    assert!(keywords(&engine, dragon).contains(KeywordSet::FLYING));
    pass_until(&mut engine, |e| e.state().turn.number >= 2);
    assert_eq!(pt(&engine, dragon), (5, 5));
}

#[test]
fn self_review_resolved_shivan_pump_is_lost_on_blink() {
    let card = index::SHIVAN_DRAGON;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[card, mountain(), plains()])
        .hand(0, &[ephemerate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, OWNER);
    let dragon = on_battlefield(&engine, OWNER, card).unwrap();
    mana_from(&mut engine, OWNER, mountain());
    activate(&mut engine, OWNER, card, 0);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, dragon), (6, 5));
    blink(&mut engine, dragon);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, dragon), (5, 5));
}

#[test]
fn self_review_whelp_old_and_new_activation_counts_are_separate() {
    for fresh_count in [1_u32, 4] {
        let card = index::DRAGON_WHELP;
        let mut board = vec![card, plains()];
        board.extend([mountain(); 8]);
        let mut engine = Duel::new(SEED, forest())
            .battlefield(0, &board)
            .hand(0, &[ephemerate()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, OWNER);
        let whelp = on_battlefield(&engine, OWNER, card).unwrap();
        mana_from(&mut engine, OWNER, mountain());
        for _ in 0..4 {
            activate(&mut engine, OWNER, card, 0);
        }
        blink(&mut engine, whelp);
        priority(&mut engine, OWNER);
        for _ in 0..fresh_count {
            activate(&mut engine, OWNER, card, 0);
        }
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            pt(&engine, whelp),
            (2 + i16::try_from(fresh_count).unwrap(), 3),
            "old pumps cannot affect the returned Whelp"
        );
        assert_eq!(
            engine
                .state()
                .ability_fires
                .get(&(engine.state().source_identity(whelp).unwrap(), 0))
                .copied(),
            Some(fresh_count),
            "old resolutions cannot erase the new incarnation's activation count"
        );
        assert_eq!(
            end_step_delays(&engine),
            if fresh_count == 4 { 8 } else { 4 },
            "old resolutions read the old incarnation's four activations; its delays cannot sacrifice the returned Whelp"
        );
        pass_until(&mut engine, |e| {
            e.state().turn.step == crate::turn::Step::End
        });
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine.state().object(whelp).unwrap().zone,
            if fresh_count == 4 {
                Zone::Graveyard
            } else {
                Zone::Battlefield
            },
            "only four fresh activations sacrifice the returned Whelp"
        );
    }
}

#[test]
fn self_review_whelp_delayed_sacrifice_cannot_follow_a_later_blink() {
    let card = index::DRAGON_WHELP;
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                card,
                mountain(),
                mountain(),
                mountain(),
                mountain(),
                plains(),
            ],
        )
        .hand(0, &[ephemerate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, OWNER);
    let whelp = on_battlefield(&engine, OWNER, card).unwrap();
    mana_from(&mut engine, OWNER, mountain());
    for _ in 0..4 {
        activate(&mut engine, OWNER, card, 0);
    }
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, whelp), (6, 3));
    assert_eq!(
        end_step_delays(&engine),
        4,
        "each stacked activation sees four activations at resolution and creates its own delayed trigger"
    );
    blink(&mut engine, whelp);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(pt(&engine, whelp), (2, 3));
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(whelp).unwrap().zone,
        Zone::Battlefield
    );
}

#[test]
fn self_review_whelp_five_sequential_activations_create_two_delays() {
    // Official DMU release notes' five-activation example. Resolve each
    // activation before announcing the next; compare with stacked four above.
    let card = index::DRAGON_WHELP;
    let mut board = vec![card];
    board.extend([mountain(); 5]);
    let mut engine = Duel::new(SEED, forest()).battlefield(0, &board).start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, OWNER);
    let whelp = on_battlefield(&engine, OWNER, card).unwrap();
    mana_from(&mut engine, OWNER, mountain());
    for _ in 0..5 {
        priority(&mut engine, OWNER);
        activate(&mut engine, OWNER, card, 0);
        pass_until(&mut engine, stack_is_empty);
    }
    assert_eq!(pt(&engine, whelp), (7, 3));
    assert_eq!(end_step_delays(&engine), 2);
    pass_until(&mut engine, |e| {
        e.state().turn.step == crate::turn::Step::End
    });
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(engine.state().object(whelp).unwrap().zone, Zone::Graveyard);
}

#[test]
fn self_review_drudge_and_sedge_regeneration_protects_only_the_original_incarnation() {
    for (card, ability) in [(index::DRUDGE_SKELETONS, 0), (index::SEDGE_TROLL, 1)] {
        for do_blink in [false, true] {
            let mut engine = Duel::new(SEED, forest())
                .battlefield(0, &[card, swamp(), plains()])
                .hand(0, &[ephemerate()])
                .battlefield(1, &[mountain()])
                .hand(1, &[lightning_bolt()])
                .start();
            keep_mulligans(&mut engine);
            reach_main_phase(&mut engine, OWNER);
            let creature = on_battlefield(&engine, OWNER, card).unwrap();
            mana_from(&mut engine, OWNER, swamp());
            activate(&mut engine, OWNER, card, ability);
            if do_blink {
                blink(&mut engine, creature);
            }
            pass_until(&mut engine, stack_is_empty);
            assert_eq!(
                engine
                    .state()
                    .object(creature)
                    .unwrap()
                    .regeneration_shields,
                u8::from(!do_blink)
            );
            priority(&mut engine, OTHER);
            cast_from_hand(&mut engine, OTHER, lightning_bolt());
            aim(&mut engine, vec![creature], vec![]);
            pass_until(&mut engine, stack_is_empty);
            assert_eq!(
                engine.state().object(creature).unwrap().zone,
                if do_blink {
                    Zone::Graveyard
                } else {
                    Zone::Battlefield
                }
            );
            if !do_blink {
                assert!(is_tapped(&engine, creature));
                assert_eq!(engine.state().object(creature).unwrap().damage, 0);
            }
        }
    }
}

#[test]
fn self_review_rock_hydra_waiting_shield_does_not_protect_the_returned_hydra() {
    for do_blink in [false, true] {
        // Anthem keeps a zero-counter returned Hydra alive as a 1/1.
        let mut engine = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[
                    index::ROCK_HYDRA,
                    index::GLORIOUS_ANTHEM,
                    mountain(),
                    plains(),
                ],
            )
            .hand(0, &[ephemerate()])
            .battlefield(1, &[index::ROD_OF_RUIN, forest(), forest(), forest()])
            .start();
        keep_mulligans(&mut engine);
        reach_main_phase(&mut engine, OWNER);
        let hydra = on_battlefield(&engine, OWNER, index::ROCK_HYDRA).unwrap();
        assert_eq!(pt(&engine, hydra), (1, 1));
        mana_from(&mut engine, OWNER, mountain());
        activate(&mut engine, OWNER, index::ROCK_HYDRA, 0);
        if do_blink {
            blink(&mut engine, hydra);
        }
        pass_until(&mut engine, stack_is_empty);
        priority(&mut engine, OTHER);
        mana_from(&mut engine, OTHER, forest());
        activate(&mut engine, OTHER, index::ROD_OF_RUIN, 0);
        aim(&mut engine, vec![hydra], vec![]);
        pass_until(&mut engine, stack_is_empty);
        assert_eq!(
            engine.state().object(hydra).unwrap().zone,
            if do_blink {
                Zone::Graveyard
            } else {
                Zone::Battlefield
            }
        );
        if !do_blink {
            assert_eq!(engine.state().object(hydra).unwrap().damage, 0);
        }
        assert_eq!(engine.state().players[0].life, 20);
    }
}

#[test]
fn self_review_basalt_old_untap_cannot_untap_a_returned_retapped_monolith() {
    let mut board = vec![index::BASALT_MONOLITH, plains()];
    board.extend([island(); 7]);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &board)
        .hand(0, &[index::ANIMATE_ARTIFACT, ephemerate()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, OWNER);
    let basalt = on_battlefield(&engine, OWNER, index::BASALT_MONOLITH).unwrap();
    mana_from(&mut engine, OWNER, island());
    // Tap it while it is still a noncreature artifact; animating an artifact
    // seated this turn makes its tap ability subject to summoning sickness.
    activate(&mut engine, OWNER, index::BASALT_MONOLITH, 1);
    cast_with_floating(&mut engine, OWNER, index::ANIMATE_ARTIFACT);
    aim(&mut engine, vec![basalt], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert!(types(&engine, basalt).contains(TypeSet::CREATURE));
    priority(&mut engine, OWNER);
    activate(&mut engine, OWNER, index::BASALT_MONOLITH, 2);
    blink(&mut engine, basalt);
    priority(&mut engine, OWNER);
    activate(&mut engine, OWNER, index::BASALT_MONOLITH, 1);
    assert!(is_tapped(&engine, basalt));
    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_tapped(&engine, basalt),
        "old untap cannot untap the returned artifact"
    );
    priority(&mut engine, OWNER);
    activate(&mut engine, OWNER, index::BASALT_MONOLITH, 2);
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, basalt),
        "the returned artifact's own activation does untap it"
    );
}

#[test]
fn self_review_enduring_vitality_follows_its_own_return_and_changes_the_new_object() {
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[index::ENDURING_VITALITY])
        .battlefield(1, &[mountain()])
        .hand(1, &[lightning_bolt()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, OWNER);
    let vitality = on_battlefield(&engine, OWNER, index::ENDURING_VITALITY).unwrap();
    let old = version(&engine, vitality);
    priority(&mut engine, OTHER);
    cast_from_hand(&mut engine, OTHER, lightning_bolt());
    aim(&mut engine, vec![vitality], vec![]);
    pass_until(&mut engine, stack_is_empty);
    assert_ne!(version(&engine, vitality), old);
    assert_eq!(
        engine.state().object(vitality).unwrap().zone,
        Zone::Battlefield
    );
    let returned_types = types(&engine, vitality);
    assert!(returned_types.contains(TypeSet::ENCHANTMENT));
    assert!(
        !returned_types.contains(TypeSet::CREATURE),
        "the same effect may modify the object it returned (400.7j)"
    );
    assert_eq!(engine.state().object(vitality).unwrap().damage, 0);
}
