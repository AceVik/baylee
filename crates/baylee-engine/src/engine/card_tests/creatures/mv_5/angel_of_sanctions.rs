//! `cards/creatures/mv_5/angel_of_sanctions.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;

fn angel_of_sanctions() -> CardIndex {
    card_index("3c49e98d-d33b-47fa-8b07-b38b367de2c7")
}

#[test]
fn angel_of_sanctions_exiles_an_opponents_nonland_and_returns_it_on_leaving() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                gray_ogre(),
            ],
        )
        .hand(0, &[angel_of_sanctions()])
        .battlefield(1, &[sol_ring(), forest()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let ring = on_battlefield(&e, p1, sol_ring()).expect("opponent nonland");
    let land = on_battlefield(&e, p1, forest()).expect("land control");
    let own = on_battlefield(&e, p0, gray_ogre()).expect("own-permanent control");
    cast_from_hand(&mut e, p0, angel_of_sanctions());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    let Pending::ChooseTargets { options, .. } = e.pending().clone() else {
        unreachable!("the arrival trigger targets")
    };
    assert!(options.contains(&ring));
    assert!(!options.contains(&land));
    assert!(!options.contains(&own));
    aim_at(&mut e, p0, ring);
    pass_until(&mut e, |e| matches!(e.pending(), Pending::YesNo { .. }));
    e.apply(p0, PlayerAction::YesNo(true))
        .expect("the optional exile may be used");
    pass_until(&mut e, stack_is_empty);
    assert_eq!(
        e.state().object(ring).expect("exiled Ring").zone,
        Zone::Exile
    );
    let angel = on_battlefield(&e, p0, angel_of_sanctions()).expect("the Angel");
    assert!(keywords(&e, angel).contains(KeywordSet::FLYING));
    kill(&mut e, angel);
    assert_eq!(on_battlefield(&e, p1, sol_ring()), Some(ring));
}

/// Embalm is a graveyard ability with a real mana-and-exile cost, sorcery
/// timing, and a white Zombie Angel token without a mana cost.
#[test]
fn angel_of_sanctions_embalms_only_as_a_sorcery_into_the_printed_token() {
    let p0 = PlayerId::new(0);
    let mut e = Duel::new(SEED, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                forest(),
            ],
        )
        .hand(0, &[angel_of_sanctions(), llanowar_elves()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let card = hand_to_graveyard(&mut e, p0, angel_of_sanctions());
    let embalm = (card, 1);
    assert!(!priority_offer(&e).abilities.contains(&embalm));
    tap_all_mana(&mut e, p0);
    cast_with_floating(&mut e, p0, llanowar_elves());
    assert!(
        !priority_offer(&e).abilities.contains(&embalm),
        "not while another spell is on the stack"
    );
    pass_until(&mut e, stack_is_empty);
    assert!(priority_offer(&e).abilities.contains(&embalm));
    activate(&mut e, p0, angel_of_sanctions(), 1);
    assert_eq!(
        e.state().object(card).expect("embalmed card").zone,
        Zone::Exile
    );
    pass_until(&mut e, stack_is_empty);

    let tokens = tokens_of(&e, p0);
    assert_eq!(tokens.len(), 1);
    let token = tokens[0];
    let object = e.state().object(token).expect("the token");
    let chars = object.characteristics();
    assert!(object.card.is_none());
    assert_eq!(e.state().names.get(chars.name), "Angel of Sanctions");
    assert_eq!(pt(&e, token), (3, 4));
    assert_eq!(
        chars.colors,
        baylee_core::color::ColorSet::from_slice(&[baylee_core::color::Color::White])
    );
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::ANGEL)
    );
    assert!(
        chars
            .subtypes
            .contains(baylee_core::generated::subtypes::creature::ZOMBIE)
    );
    assert_eq!(chars.mana_cost.cmc(), 0, "no mana cost");
    assert!(keywords(&e, token).contains(KeywordSet::FLYING));
}

/// "Until this creature leaves the battlefield" (CR 610.3): an Angel that is
/// gone before its trigger resolves exiles nothing (CR 610.3a). Written as an
/// exile plus a separate leaves trigger, the trigger found nothing to return
/// and the Ring went into exile for good.
#[test]
fn angel_of_sanctions_gone_before_its_trigger_resolves_exiles_nothing() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut e = Duel::new(SEED, forest())
        .battlefield(0, &[plains(), plains(), plains(), plains(), plains()])
        .hand(0, &[angel_of_sanctions()])
        .battlefield(1, &[sol_ring()])
        .start();
    keep_mulligans(&mut e);
    reach_main_phase(&mut e, p0);
    let ring = on_battlefield(&e, p1, sol_ring()).expect("opponent nonland");
    cast_from_hand(&mut e, p0, angel_of_sanctions());
    pass_until(&mut e, |e| {
        matches!(e.pending(), Pending::ChooseTargets { .. })
    });
    aim_at(&mut e, p0, ring);
    let angel = on_battlefield(&e, p0, angel_of_sanctions()).expect("the Angel");
    // The trigger is on the stack; the Angel dies in response.
    crate::sba::destroy(e.dev_state_mut(p0).expect("dev board"), angel);
    pass_until(&mut e, stack_is_empty);
    assert!(on_battlefield(&e, p0, angel_of_sanctions()).is_none());
    assert_eq!(on_battlefield(&e, p1, sol_ring()), Some(ring));
}
