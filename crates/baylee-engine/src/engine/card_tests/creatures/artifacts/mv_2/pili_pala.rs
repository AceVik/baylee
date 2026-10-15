//! `cards/creatures/artifacts/mv_2/pili_pala.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

fn pili_pala() -> CardIndex {
    card_index("ad97333e-733c-402f-8ca3-e836c96b843c")
}

/// Pili-Pala: a 1/1 with "Flying" and "{2}, {Q}: Add one mana of any color."
/// `{Q}` is the untap symbol: the price is two mana and untapping the
/// Pili-Pala, so it cannot be paid while the Pili-Pala is untapped; once it
/// has attacked and stands tapped, two floating mana untap it and make one
/// mana of the named color.
#[test]
fn pili_pala_untaps_itself_for_two_to_make_one_mana_of_any_color() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = table(
        &[],
        &[pili_pala(), forest(), forest(), forest(), forest()],
        &[],
    );
    let me = on_battlefield(&engine, p0, pili_pala()).expect("seated");
    assert_eq!(pt(&engine, me), (1, 1));
    assert!(keywords(&engine, me).contains(KeywordSet::FLYING));
    let forests = all_on_battlefield(&engine, p0, forest());
    let offered = |e: &Engine<RegistryLookup>| {
        let o = priority_offer(e);
        o.abilities.contains(&(me, 0)) || o.mana_abilities.contains(&me)
    };

    // Untapped, with two mana floating: {Q} cannot be paid.
    tap_mana_where(&mut engine, p0, |id| forests[..2].contains(&id));
    assert_eq!(engine.state().players[0].mana_pool.total(), 2);
    assert!(!offered(&engine), "{{Q}} wants a tapped Pili-Pala");

    // Attack with it (flying, unblocked) so it stands tapped.
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseAttackers { .. })
    });
    engine
        .apply(
            p0,
            PlayerAction::DeclareAttackers {
                attackers: vec![(me, Defender::Player(p1))],
            },
        )
        .expect("the Pili-Pala attacks");
    pass_until(&mut engine, |e| {
        matches!(e.pending(), Pending::ChooseBlockers { .. })
            || (e.state().turn.phase == Phase::SecondMain && stack_is_empty(e))
    });
    pass_until(&mut engine, |e| {
        e.state().turn.phase == Phase::SecondMain && stack_is_empty(e)
    });
    assert!(is_tapped(&engine, me), "it attacked");
    assert_eq!(life_of(&engine, 1), 19);

    // The pool empties between steps; float two again in the second main.
    tap_mana_where(&mut engine, p0, |id| forests[2..].contains(&id));
    assert!(offered(&engine), "tapped, with {{2}} floating");
    let mana_ability = priority_offer(&engine).mana_abilities.contains(&me);
    if mana_ability {
        engine
            .apply(p0, PlayerAction::ActivateManaAbility { source: me })
            .unwrap();
    } else {
        activate(&mut engine, p0, pili_pala(), 0);
    }
    name_a_color(&mut engine, ManaColor::Green);
    assert!(!is_tapped(&engine, me), "{{Q}} untapped it");
    let pool = &engine.state().players[0].mana_pool;
    assert_eq!(pool.available(ManaColor::Green), 1);
    assert_eq!(pool.total(), 1, "the {{2}} was spent");
}
