//! `cards/lands/check/spymaster_s_vault.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Spymaster's Vault: "This land enters tapped unless you control a Swamp." / "{T}: Add {B}." / "{B}, {T}: Target creature you control connives X..."
/// Controlling a Swamp allows Spymaster's Vault to enter untapped and immediately tap for black mana.
#[test]
fn spymaster_s_vault_enters_untapped_with_swamp_and_taps_for_black() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(208, forest())
        .battlefield(0, &[swamp()])
        .hand(0, &[spymaster_s_vault()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let land = play_land(&mut engine, p0, spymaster_s_vault());
    assert!(!entered_tapped(&engine, land));

    activate(&mut engine, p0, spymaster_s_vault(), 0);
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Black), 1);
    assert!(is_tapped(&engine, land));
}

/// p0 floats {R}, casts a Lightning Bolt at `victim` and lets it resolve.
#[track_caller]
fn bolt_creature(e: &mut Engine<RegistryLookup>, victim: ObjectId) {
    let p0 = PlayerId::new(0);
    e.dev_state_mut(p0).unwrap().players[0]
        .mana_pool
        .add(ManaColor::Red, 1);
    e.refresh_offer();
    cast_with_floating(e, p0, baylee_core::generated::index::LIGHTNING_BOLT);
    e.apply(
        p0,
        PlayerAction::ChooseTargets {
            objects: vec![victim],
            players: vec![],
        },
    )
    .unwrap();
    pass_until(e, stack_is_empty);
    assert_eq!(
        e.state().object(victim).unwrap().zone,
        Zone::Graveyard,
        "the Bolt killed its target"
    );
}

/// "…where X is the number of creatures that died this turn." The count is
/// every creature, not only ours: the deaths here are real (Lightning Bolts
/// resolving on Grizzly Bears) rather than a number written into the turn's
/// tally, and the connive that follows draws, and discards, exactly that many.
#[test]
fn vault_connives_x_for_every_creature_that_died_this_turn_an_opponents_included() {
    use baylee_core::generated::index;
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    // (opponent's creatures bolted, ours bolted)
    for (theirs, mine) in [(1usize, 0usize), (1, 1), (0, 1)] {
        let deaths = theirs + mine;
        let mut e = Duel::new(SEED, forest())
            .battlefield(
                0,
                &[
                    spymaster_s_vault(),
                    serra_angel(),
                    index::GRIZZLY_BEARS,
                    index::GRIZZLY_BEARS,
                ],
            )
            .battlefield(1, &[index::GRIZZLY_BEARS, index::GRIZZLY_BEARS])
            .hand(0, &vec![index::LIGHTNING_BOLT; deaths])
            .start();
        keep_mulligans(&mut e);
        reach_main_phase(&mut e, p0);
        let angel = on_battlefield(&e, p0, serra_angel()).unwrap();
        let victims: Vec<ObjectId> = all_on_battlefield(&e, p1, index::GRIZZLY_BEARS)
            .into_iter()
            .take(theirs)
            .chain(
                all_on_battlefield(&e, p0, index::GRIZZLY_BEARS)
                    .into_iter()
                    .take(mine),
            )
            .collect();
        assert_eq!(victims.len(), deaths);
        for victim in victims {
            bolt_creature(&mut e, victim);
        }
        assert_eq!(
            e.state().per_turn.creatures_died,
            u32::try_from(deaths).unwrap(),
            "{theirs} of theirs and {mine} of ours died"
        );
        e.dev_state_mut(p0).unwrap().players[0]
            .mana_pool
            .add(ManaColor::Black, 1);
        e.refresh_offer();
        let library = library_size(&e, p0);
        activate(&mut e, p0, spymaster_s_vault(), 1);
        aim_at(&mut e, p0, angel);
        pass_until(&mut e, |e| {
            matches!(e.pending(), Pending::ChooseCards { .. }) || stack_is_empty(e)
        });
        let Pending::ChooseCards {
            options, min, max, ..
        } = e.pending().clone()
        else {
            panic!(
                "{deaths} creature(s) died this turn, so the connive must draw and ask for \
                 a discard; got {:?}",
                e.pending()
            );
        };
        assert_eq!(
            library_size(&e, p0),
            library - deaths,
            "connives X where X counts every creature that died: {theirs} theirs + {mine} ours"
        );
        assert_eq!(
            (min, max),
            (u8::try_from(deaths).unwrap(), u8::try_from(deaths).unwrap()),
            "and then discards that many cards"
        );
        e.apply(
            p0,
            PlayerAction::ChooseObjects {
                objects: options.into_iter().take(deaths).collect(),
            },
        )
        .unwrap();
        pass_until(&mut e, stack_is_empty);
        assert_eq!(
            pt(&e, angel),
            (4, 4),
            "only lands were discarded, so no +1/+1 counter"
        );
    }
}
