//! `cards/creatures/mv_5/homarid_warrior.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;
use crate::turn::Step;

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

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

fn homarid_warrior() -> CardIndex {
    card_index("6facb0f6-fa49-4ce6-9f95-15bb3ce4bcf2")
}

/// Homarid Warrior (6facb0f6-fa49-4ce6-9f95-15bb3ce4bcf2) — "{U}: This
/// creature gains shroud until end of turn and doesn't untap during your next
/// untap step. Tap it."
#[test]
fn homarid_warrior_gains_shroud_taps_and_skips_exactly_one_untap() {
    let mut e = prepared(&[], &[homarid_warrior()], &[]);
    let warrior = on_battlefield(&e, P0, homarid_warrior()).expect("Homarid Warrior is out");
    assert_eq!(pt(&e, warrior), (3, 3));
    assert!(!is_tapped(&e, warrior), "untapped before the ability");
    assert!(!keywords(&e, warrior).contains(KeywordSet::SHROUD));

    activate_by_card(&mut e, P0, homarid_warrior(), 0);
    pass_until(&mut e, stack_is_empty);
    assert!(is_tapped(&e, warrior), "the ability taps it");
    assert!(keywords(&e, warrior).contains(KeywordSet::SHROUD));

    // P0's next untap step is turn 3: it is skipped, and the shroud is gone.
    pass_until(&mut e, |e| {
        e.state().turn.number == 3 && e.state().turn.step == Step::Upkeep
    });
    assert!(is_tapped(&e, warrior), "it did not untap in turn 3");
    assert!(
        !keywords(&e, warrior).contains(KeywordSet::SHROUD),
        "shroud lasts until end of turn only"
    );

    // Only one untap step is skipped: turn 5 untaps it.
    pass_until(&mut e, |e| {
        e.state().turn.number == 5 && e.state().turn.step == Step::Upkeep
    });
    assert!(!is_tapped(&e, warrior), "the turn-5 untap step untaps it");
}

/// Homarid Warrior — shroud means an opponent's spell cannot name it.
#[test]
fn homarid_warrior_with_shroud_cannot_be_targeted() {
    let bolt = card_index("4457ed35-7c10-48c8-9776-456485fdf070");
    let bears = card_index("14c8f55d-d177-4c25-a931-ebeb9e6062a0");
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[homarid_warrior(), bears])
        .hand(1, &[bolt])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, P0);
    mana(&mut e);
    let warrior = on_battlefield(&e, P0, homarid_warrior()).expect("Homarid Warrior is out");
    let bears_id = on_battlefield(&e, P0, bears).expect("Grizzly Bears is out");

    activate_by_card(&mut e, P0, homarid_warrior(), 0);
    pass_until(&mut e, stack_is_empty);
    assert!(keywords(&e, warrior).contains(KeywordSet::SHROUD));

    cast(&mut e, P1, bolt);
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        panic!("Lightning Bolt asks for a target, got {:?}", e.pending())
    };
    assert!(
        options.contains(&bears_id),
        "an unshrouded creature is a legal target"
    );
    assert!(
        !options.contains(&warrior),
        "the shrouded Homarid Warrior is not: {options:?}"
    );
}

#[test]
fn homarid_warrior_untaps_for_a_new_controller() {
    crate::engine::mechanics_tests::untap_scope::assert_untaps_after_control_change(
        homarid_warrior(),
    );
}
