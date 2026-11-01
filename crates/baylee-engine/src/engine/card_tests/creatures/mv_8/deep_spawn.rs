//! `cards/creatures/mv_8/deep_spawn.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

const P0: PlayerId = PlayerId::new(0);

/// A duel at P0's first main phase with the named hand and boards, and every
/// colour of mana floating for both seats.
fn prepared(
    hand: &[CardIndex],
    mine: &[CardIndex],
    theirs: &[CardIndex],
) -> Engine<RegistryLookup> {
    let mut e = Duel::new(SEED, forest())
        .hand(0, hand)
        .battlefield(0, mine)
        .battlefield(1, theirs)
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    mana(&mut e);
    e
}

fn mana(e: &mut Engine<RegistryLookup>) {
    for p in &mut e.state.players {
        for c in ManaColor::ALL {
            p.mana_pool.add(c, 30);
        }
    }
    e.refresh_offer();
}

fn priority(e: &mut Engine<RegistryLookup>, p: PlayerId) {
    pass_until(
        e,
        |e| matches!(e.pending(), Pending::Priority { player, .. } if *player == p),
    );
}

fn cast(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex) {
    priority(e, p);
    mana(e);
    let card = in_hand(e, p, c).expect("the spell is in hand");
    e.apply(p, PlayerAction::CastSpell { card })
        .expect("the spell is cast");
}

fn activate_by_card(e: &mut Engine<RegistryLookup>, p: PlayerId, c: CardIndex, ability_index: u32) {
    priority(e, p);
    mana(e);
    let source = on_battlefield(e, p, c).expect("the source is on the battlefield");
    e.apply(
        p,
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        },
    )
    .expect("the ability is activated");
}

fn yes_no(e: &mut Engine<RegistryLookup>, answer: bool) {
    let p = e.pending().asked().expect("a question is pending");
    e.apply(p, PlayerAction::YesNo(answer))
        .expect("the question takes a yes or a no");
}

fn deep_spawn() -> CardIndex {
    card_index("d1aba343-d85a-4829-bdfe-db31fcb3c320")
}

/// Deep Spawn cast from hand in P0's first main phase and resolved. (On the
/// battlefield from the start it would ask its upkeep question before the
/// test could stand at main.)
fn spawn_in_play() -> (Engine<RegistryLookup>, ObjectId) {
    let mut e = prepared(&[deep_spawn()], &[], &[]);
    cast(&mut e, P0, deep_spawn());
    pass_until(&mut e, stack_is_empty);
    let spawn = on_battlefield(&e, P0, deep_spawn()).expect("Deep Spawn is out");
    (e, spawn)
}

/// The upkeep question of the given turn, P0's.
fn wait_for_the_upkeep_question(e: &mut Engine<RegistryLookup>, turn: u32) {
    pass_until(e, |e| {
        e.state().turn.number == turn && matches!(e.pending(), Pending::YesNo { player: P0, .. })
    });
}

/// Deep Spawn (d1aba343-d85a-4829-bdfe-db31fcb3c320) — "Trample"
/// Deep Spawn — "At the beginning of your upkeep, sacrifice this creature
/// unless you mill two cards." (milling)
#[test]
fn deep_spawn_stays_when_its_controller_mills_two() {
    let (mut e, spawn) = spawn_in_play();
    wait_for_the_upkeep_question(&mut e, 3);
    let graveyard = e.state().zones.list(ZoneLocation::Graveyard(P0)).len();
    let library = library_size(&e, P0);
    yes_no(&mut e, true);
    pass_until(&mut e, stack_is_empty);

    assert_eq!(
        e.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        graveyard + 2,
        "two cards milled"
    );
    assert_eq!(library_size(&e, P0), library - 2);
    assert_eq!(
        e.state().object(spawn).unwrap().zone,
        Zone::Battlefield,
        "Deep Spawn stays"
    );
}

/// Deep Spawn — (declining)
#[test]
fn deep_spawn_is_sacrificed_when_its_controller_declines_to_mill() {
    let (mut e, _spawn) = spawn_in_play();
    wait_for_the_upkeep_question(&mut e, 3);
    let library = library_size(&e, P0);
    yes_no(&mut e, false);
    pass_until(&mut e, stack_is_empty);

    assert!(on_battlefield(&e, P0, deep_spawn()).is_none());
    assert!(in_graveyard(&e, P0, deep_spawn()).is_some(), "sacrificed");
    assert_eq!(
        e.state().zones.list(ZoneLocation::Graveyard(P0)).len(),
        1,
        "and nothing was milled"
    );
    assert_eq!(library_size(&e, P0), library);
}

/// Deep Spawn — "{U}: This creature gains shroud until end of turn and doesn't
/// untap during your next untap step. Tap this creature."
#[test]
fn deep_spawn_shroud_ability_taps_it_and_skips_exactly_one_untap() {
    let (mut e, spawn) = spawn_in_play();
    assert!(!is_tapped(&e, spawn));
    assert!(!keywords(&e, spawn).contains(KeywordSet::SHROUD));

    activate_by_card(&mut e, P0, deep_spawn(), 1);
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, spawn), "the ability taps it");
    assert!(keywords(&e, spawn).contains(KeywordSet::SHROUD));

    // Turn 3: the untap step is skipped; the shroud has run out.
    wait_for_the_upkeep_question(&mut e, 3);
    assert!(is_tapped(&e, spawn), "it did not untap in turn 3");
    assert!(!keywords(&e, spawn).contains(KeywordSet::SHROUD));
    yes_no(&mut e, true);
    pass_until(&mut e, stack_is_empty);

    // Turn 5: it untaps again.
    wait_for_the_upkeep_question(&mut e, 5);
    assert!(!is_tapped(&e, spawn), "only one untap step was skipped");
}

#[test]
fn deep_spawn_untaps_for_a_new_controller() {
    crate::engine::mechanics_tests::untap_scope::assert_untaps_after_control_change(deep_spawn());
}
