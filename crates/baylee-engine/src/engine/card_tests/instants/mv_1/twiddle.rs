//! `cards/instants/mv_1/twiddle.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Twiddle: "You may tap or untap target artifact, creature, or land."
/// Targets an untapped land and accepts the "may": it comes back tapped.
#[test]
fn twiddle_toggles_the_tapped_state_of_its_target() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[twiddle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let islands = all_on_battlefield(&engine, p0, island());
    let (payer, target) = (islands[0], islands[1]);
    assert!(!is_tapped(&engine, target), "untapped before");
    tap_mana_where(&mut engine, p0, |id| id == payer);
    cast_with_floating(&mut engine, p0, twiddle());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .expect("an untapped land is a legal target");
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        is_tapped(&engine, target),
        "\"tap or untap\" — tapped, since it started untapped"
    );
}

/// Twiddle declined: "you may" answered no leaves the target exactly as it
/// was.
#[test]
fn twiddle_declined_leaves_its_target_unchanged() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island()])
        .hand(0, &[twiddle()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let islands = all_on_battlefield(&engine, p0, island());
    let (payer, target) = (islands[0], islands[1]);
    tap_mana_where(&mut engine, p0, |id| id == payer);
    cast_with_floating(&mut engine, p0, twiddle());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![target],
                players: vec![],
            },
        )
        .expect("an untapped land is a legal target");
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(false)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, target),
        "declined \"you may\" — nothing happens"
    );
}

/// Twiddle's other branch: "tap **or untap**". Aimed at the opponent's tapped
/// Elf and accepted, it comes back untapped (the target is anyone's, and the
/// toggle goes the way the permanent is not already facing).
#[test]
fn twiddle_untaps_a_tapped_creature_of_the_opponent() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island()])
        .hand(0, &[twiddle()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("their Elf");
    engine
        .dev_state_mut(p1)
        .expect("the harness may set boards up")
        .object_mut(elf)
        .expect("seated")
        .status
        .insert(Status::TAPPED);
    engine.refresh_offer();
    assert!(is_tapped(&engine, elf), "tapped before");

    cast_from_hand(&mut engine, p0, twiddle());
    engine
        .apply(
            p0,
            PlayerAction::ChooseTargets {
                objects: vec![elf],
                players: vec![],
            },
        )
        .expect("a tapped creature is a legal target");
    pass_until(&mut engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::MayDo,
                ..
            }
        )
    });
    engine.apply(p0, PlayerAction::YesNo(true)).unwrap();
    pass_until(&mut engine, stack_is_empty);
    assert!(
        !is_tapped(&engine, elf),
        "\"tap or untap\" — untapped, since it started tapped"
    );
}

/// "Target artifact, creature, or land": an artifact, a creature and a land
/// (ours or theirs) are on the menu; an enchantment is not, and no player is.
#[test]
fn twiddle_targets_an_artifact_a_creature_or_a_land_and_nothing_else() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let bad_moon = card_index("fc5d3341-cbce-49e5-93cc-8add92479dca");
    let mut engine = Duel::new(SEED, forest())
        .battlefield(0, &[island(), island(), sol_ring(), bad_moon])
        .hand(0, &[twiddle()])
        .battlefield(1, &[llanowar_elves(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let ring = on_battlefield(&engine, p0, sol_ring()).expect("an artifact");
    let enchantment = on_battlefield(&engine, p0, bad_moon).expect("an enchantment");
    let elf = on_battlefield(&engine, p1, llanowar_elves()).expect("a creature");
    let their_land = on_battlefield(&engine, p1, forest()).expect("their land");
    let islands = all_on_battlefield(&engine, p0, island());

    cast_from_hand(&mut engine, p0, twiddle());
    let Pending::ChooseTargets {
        options,
        player_options,
        ..
    } = engine.pending().clone()
    else {
        panic!("Twiddle asks for a target, got {:?}", engine.pending())
    };
    for legal in [ring, elf, their_land, islands[0], islands[1]] {
        assert!(options.contains(&legal), "on the menu: {options:?}");
    }
    assert!(
        !options.contains(&enchantment),
        "an enchantment is neither an artifact, a creature nor a land: {options:?}"
    );
    assert!(player_options.is_empty(), "and no player is a target");
    assert_eq!(options.len(), 5, "{options:?}");
}
