//! `cards/sorceries/mv_5/hellfire.rs`, played.
//!
//! "Destroy all nonblack creatures. Hellfire deals X plus 3 damage to you,
//! where X is the number of creatures that died this way." The count is the
//! half the rulings are about (2009-10-01): it is the creatures *put into
//! graveyards* as the result of the destruction, so a creature that
//! regenerated, or that a replacement effect exiled instead, does not count,
//! and a token that was destroyed does.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn hellfire() -> CardIndex {
    card_index("e7b1975d-9574-4333-8530-33167948078a")
}

fn black_knight() -> CardIndex {
    card_index("9456c5b6-946d-403a-8ed0-dff9f921d98c")
}

fn drudge_skeletons() -> CardIndex {
    card_index("c180ed02-07f4-4538-9bea-8c249234a8e2")
}

fn grizzly_bears() -> CardIndex {
    card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0")
}

fn wall_of_brambles() -> CardIndex {
    card_index("f8d82a00-c10e-4b9e-9642-d05706900a97")
}

fn raise_the_alarm() -> CardIndex {
    card_index("5b2364d7-a811-4595-a1b4-224c70555ffa")
}

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

/// Passes priority until the stack is empty, answering the one question a
/// regeneration shield can ask on the way ("use it?") with yes.
#[track_caller]
fn resolve_using_shields(engine: &mut Engine<RegistryLookup>) {
    for _ in 0..40 {
        if stack_is_empty(engine) {
            return;
        }
        match engine.pending().clone() {
            Pending::Priority { player, .. } => {
                engine.apply(player, PlayerAction::PassPriority).unwrap();
            }
            Pending::YesNo {
                player,
                prompt: crate::choice::YesNoPrompt::UseRegenerationShield { .. },
                ..
            } => {
                engine.apply(player, PlayerAction::YesNo(true)).unwrap();
            }
            other => panic!("unexpected while Hellfire resolves: {other:?}"),
        }
    }
    panic!("Hellfire never finished resolving");
}

/// "Destroy all nonblack creatures. Hellfire deals X plus 3 damage to you,
/// where X is the number of creatures that died this way." Three nonblack
/// creatures across both sides die, so the caster takes 3 + 3. The Black
/// Knight on the other side is black, is not destroyed and is not counted,
/// and the damage goes to the caster -- the opponent loses nothing.
#[test]
fn hellfire_destroys_every_nonblack_creature_and_deals_x_plus_three_to_its_caster() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
                grizzly_bears(),
            ],
        )
        .hand(0, &[hellfire()])
        .battlefield(1, &[serra_angel(), black_knight()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, hellfire());
    pass_until(&mut engine, stack_is_empty);

    for (seat, card) in [
        (p0, llanowar_elves()),
        (p0, grizzly_bears()),
        (p1, serra_angel()),
    ] {
        assert!(
            on_battlefield(&engine, seat, card).is_none(),
            "a nonblack creature is destroyed, whoever controls it"
        );
        assert!(
            in_graveyard(&engine, seat, card).is_some(),
            "and it is put into its owner's graveyard"
        );
    }
    assert!(
        on_battlefield(&engine, p1, black_knight()).is_some(),
        "\"nonblack\": the Black Knight is not destroyed"
    );
    assert_eq!(
        life_of(&engine, p0),
        20 - (3 + 3),
        "X = 3 creatures died, so X plus 3 is 6 damage to the caster"
    );
    assert_eq!(
        life_of(&engine, p1),
        20,
        "to you -- the opponent is not hit"
    );
}

/// Black creatures are not destroyed and do not count: with nothing but black
/// creatures on the table X is 0, and the spell still costs its caster the 3.
#[test]
fn hellfire_spares_black_creatures_and_still_deals_three() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[swamp(), swamp(), swamp(), swamp(), swamp(), black_knight()],
        )
        .hand(0, &[hellfire()])
        .battlefield(1, &[drudge_skeletons()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    cast_from_hand(&mut engine, p0, hellfire());
    pass_until(&mut engine, stack_is_empty);

    assert!(on_battlefield(&engine, p0, black_knight()).is_some());
    assert!(on_battlefield(&engine, p1, drudge_skeletons()).is_some());
    assert!(in_graveyard(&engine, p0, black_knight()).is_none());
    assert_eq!(
        life_of(&engine, p0),
        20 - 3,
        "no creature died: X is 0 and the damage is just the 3"
    );
}

/// "A creature that regenerates instead of being destroyed doesn't leave the
/// battlefield, so it's not put into a graveyard." (ruling, 2009-10-01.) The
/// Wall of Brambles buys a regeneration shield, Hellfire's destruction is
/// replaced, and only the Elves are counted: X = 1.
#[test]
fn hellfire_does_not_count_a_creature_that_regenerated() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                forest(),
                wall_of_brambles(),
            ],
        )
        .hand(0, &[hellfire()])
        .battlefield(1, &[llanowar_elves()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let wall = on_battlefield(&engine, p0, wall_of_brambles()).expect("the Wall is seated");

    // `{G}: Regenerate` is bought off the card, then Hellfire is cast out of
    // the black mana that is left floating.
    tap_all_mana(&mut engine, p0);
    raise_a_shield(&mut engine, p0, wall, 0);
    cast_with_floating(&mut engine, p0, hellfire());
    resolve_using_shields(&mut engine);

    assert!(
        on_battlefield(&engine, p0, wall_of_brambles()).is_some(),
        "the shield replaced the destruction: the Wall is still there"
    );
    assert!(
        in_graveyard(&engine, p0, wall_of_brambles()).is_none(),
        "and was never put into a graveyard"
    );
    assert!(
        is_tapped(&engine, wall),
        "regenerating taps the creature (CR 701.19c)"
    );
    assert!(
        in_graveyard(&engine, p1, llanowar_elves()).is_some(),
        "the unshielded Elves died"
    );
    assert_eq!(
        life_of(&engine, p0),
        20 - (1 + 3),
        "only the Elves died this way: X = 1, not 2"
    );
}

/// "...a replacement effect that says 'If a creature would be put into a
/// graveyard from the battlefield, exile it instead' won't affect whether a
/// creature is destroyed or not, but will affect whether it's put into a
/// graveyard or not." (ruling, 2009-10-01.) Disintegrate leaves the
/// opponent's Bears alive with 1 damage and "if it would die this turn,
/// exile it instead" on them; Hellfire then destroys them, they are exiled,
/// and so they are not counted.
#[test]
fn hellfire_does_not_count_a_creature_exiled_instead_of_dying() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                mountain(),
                mountain(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[disintegrate(), hellfire()])
        .battlefield(1, &[grizzly_bears()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);
    let bears = on_battlefield(&engine, p1, grizzly_bears()).expect("the Bears are seated");

    let mountains = all_on_battlefield(&engine, p0, mountain());
    tap_mana_where(&mut engine, p0, |id| mountains.contains(&id));
    cast_with_floating(&mut engine, p0, disintegrate());
    engine.apply(p0, PlayerAction::ChooseNumber(1)).unwrap();
    aim_at(&mut engine, p0, bears);
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        engine.state().object(bears).map(|o| o.damage),
        Some(1),
        "1 damage on a 2/2 leaves it alive, with the exile rider on it"
    );

    let swamps = all_on_battlefield(&engine, p0, swamp());
    tap_mana_where(&mut engine, p0, |id| swamps.contains(&id));
    cast_with_floating(&mut engine, p0, hellfire());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        on_battlefield(&engine, p1, grizzly_bears()).is_none(),
        "Hellfire destroyed the Bears"
    );
    assert!(
        in_graveyard(&engine, p1, grizzly_bears()).is_none(),
        "but the rider exiled them instead of letting them die"
    );
    assert!(in_exile(&engine, p1, grizzly_bears()), "they are in exile");
    assert!(
        in_graveyard(&engine, p0, llanowar_elves()).is_some(),
        "the Elves died normally"
    );
    assert_eq!(
        life_of(&engine, p0),
        20 - (1 + 3),
        "only the Elves were put into a graveyard: X = 1, not 2"
    );
}

/// "A token creature that's destroyed is put into its owner's graveyard, and
/// will be counted by Hellfire's effect." (ruling, 2009-10-01.) Two Soldier
/// tokens are the only creatures: they die, the tokens are gone, and X = 2.
#[test]
fn hellfire_counts_a_destroyed_token() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                swamp(),
                plains(),
                plains(),
            ],
        )
        .hand(0, &[raise_the_alarm(), hellfire()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let plains_lands = all_on_battlefield(&engine, p0, plains());
    tap_mana_where(&mut engine, p0, |id| plains_lands.contains(&id));
    cast_with_floating(&mut engine, p0, raise_the_alarm());
    pass_until(&mut engine, stack_is_empty);
    assert_eq!(
        tokens_of(&engine, p0).len(),
        2,
        "Raise the Alarm made two Soldier tokens"
    );

    tap_all_mana(&mut engine, p0);
    cast_with_floating(&mut engine, p0, hellfire());
    pass_until(&mut engine, stack_is_empty);

    assert!(
        tokens_of(&engine, p0).is_empty(),
        "the white Soldier tokens were destroyed"
    );
    assert_eq!(
        life_of(&engine, p0),
        20 - (2 + 3),
        "each token died this way and counts: X = 2"
    );
}
