//! `cards/lands/restricted/inventors_fair.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Inventors' Fair — a legendary land printing "{T}: Add {C}", an upkeep
/// trigger that gains 1 life whenever you control three or more artifacts,
/// and "{4}, {T}, Sacrifice this land: search your library for an artifact
/// card … Activate only if you control three or more artifacts."
///
/// Both artifact-gated halves are read off two boards that differ only in the
/// count: two artifacts against three. The smaller board reaches its own main
/// phase on twenty life and floats the tutor's whole {4} across two tapped
/// Sol Rings without being offered it, so the missing third artifact is the
/// only thing that can be withholding either line; the larger board is one
/// life up on the same walk, is offered the tutor, and pays for it with the
/// land — which is in its owner's graveyard afterwards. The land's own {T} is
/// pressed on the smaller board too, so all three printed lines are played and
/// neither refusal can be blamed on a tapped land or a missing mana ability.
#[test]
#[allow(clippy::too_many_lines)] // two boards, because the gate has two answers
fn inventors_fair_gates_its_lifegain_and_its_tutor_on_three_artifacts() {
    let p0 = PlayerId::new(0);

    // Two artifacts: the printed count is one short, and the tutor's {4} is
    // floating before the offer is read.
    let mut poor = Duel::new(31, quiet_artifact())
        .battlefield(0, &[inventors_fair(), quiet_artifact(), quiet_artifact()])
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut poor);
    assert!(walk_to_own_main(&mut poor, p0), "p0 reaches its own main");
    assert_eq!(
        poor.state().players[0].life,
        20,
        "two artifacts is not three, so the upkeep trigger gained nothing",
    );

    let held = on_battlefield(&poor, p0, inventors_fair()).expect("the Fair is on the table");
    for _ in 0..2 {
        activate(&mut poor, p0, quiet_artifact(), 0);
    }
    assert_eq!(
        poor.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        4,
        "two Sol Rings pay {{C}}{{C}} apiece",
    );
    let Pending::Priority { legal, .. } = poor.pending().clone() else {
        panic!("expected priority, got {:?}", poor.pending())
    };
    assert!(
        !legal.abilities.contains(&(held, 2)),
        "the tutor's whole {{4}} is in the pool and the land is untapped, so \
         only the two-artifact board withholds it: {:?}",
        legal.abilities,
    );

    // Ability 1 is the printed "{T}: Add {C}".
    activate(&mut poor, p0, inventors_fair(), 1);
    assert_eq!(
        poor.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        5,
        "the Fair's own tap added the single colorless it prints",
    );
    assert!(is_tapped(&poor, held), "which tapped the land");
    assert!(
        stack_is_empty(&poor),
        "CR 605.3b: a mana ability never uses the stack",
    );

    // Three artifacts: the same two readings, both the other way round.
    let mut rich = Duel::new(31, quiet_artifact())
        .battlefield(
            0,
            &[
                inventors_fair(),
                quiet_artifact(),
                quiet_artifact(),
                quiet_artifact(),
            ],
        )
        .life(0, 20)
        .life(1, 20)
        .start();
    keep_mulligans(&mut rich);
    assert!(walk_to_own_main(&mut rich, p0), "p0 reaches its own main");
    assert_eq!(
        rich.state().players[0].life,
        21,
        "three artifacts at the beginning of your upkeep is one life",
    );

    let fair = on_battlefield(&rich, p0, inventors_fair()).expect("the Fair is on the table");
    for _ in 0..3 {
        activate(&mut rich, p0, quiet_artifact(), 0);
    }
    let Pending::Priority { legal, .. } = rich.pending().clone() else {
        panic!("expected priority, got {:?}", rich.pending())
    };
    assert_eq!(
        rich.state().players[0]
            .mana_pool
            .available(ManaColor::Colorless),
        6,
        "three Sol Rings, which is the tutor's {{4}} and then some",
    );
    assert!(
        legal.abilities.contains(&(fair, 2)),
        "the same count that fired the trigger now opens the tutor: {:?}",
        legal.abilities,
    );

    let library_before = library_size(&rich, p0);
    let hand_before = rich.state().zones.list(ZoneLocation::Hand(p0)).len();

    // Ability 2 is the tutor, whose cost is {4}, {T} and the sacrifice. It is
    // an ordinary activated ability, so the costs are paid now and the search
    // is a question the *resolution* asks: the stack is in between.
    activate(&mut rich, p0, inventors_fair(), 2);
    pass_until(&mut rich, |e| {
        matches!(e.pending(), Pending::ChooseCards { .. })
    });
    let Pending::ChooseCards {
        player,
        options,
        prompt,
        ..
    } = rich.pending().clone()
    else {
        panic!("the search asks which artifact, got {:?}", rich.pending())
    };
    assert_eq!(player, p0, "the seat that paid is the seat that searches");
    assert_eq!(
        prompt,
        ChoicePrompt::SearchLibrary,
        "a search of the library, not a cost being paid",
    );
    assert!(
        !options.is_empty(),
        "the deck this duel was dealt is made of artifacts",
    );
    let found = options[0];
    rich.apply(
        p0,
        PlayerAction::ChooseObjects {
            objects: vec![found],
        },
    )
    .expect("a card the search offered");
    pass_until(&mut rich, |e| at_rest(e, p0));

    assert_eq!(
        library_size(&rich, p0),
        library_before - 1,
        "\"search your library for an artifact card\" — one card left it",
    );
    assert_eq!(
        rich.state().zones.list(ZoneLocation::Hand(p0)).len(),
        hand_before + 1,
        "and arrived in hand",
    );
    assert!(
        on_battlefield(&rich, p0, inventors_fair()).is_none(),
        "the sacrifice is paid from the battlefield",
    );
    assert!(
        in_graveyard(&rich, p0, inventors_fair()).is_some(),
        "so the land is in its owner's graveyard",
    );
}
