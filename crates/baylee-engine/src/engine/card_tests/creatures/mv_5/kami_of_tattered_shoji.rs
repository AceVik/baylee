//! `cards/creatures/mv_5/kami_of_tattered_shoji.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// Kami of Tattered Shoji — {4}{W} 2/5 Spirit: "Whenever you cast a Spirit or
/// Arcane spell, this creature gains flying until end of turn."
///
/// The trigger is fired with the card's own second copy, because a Spirit spell
/// is exactly what the sentence asks for and a second Kami is a Spirit nobody
/// has to be trusted about. Five Plains are tapped before anything is claimed
/// (`can_afford` reads the pool, not the lands), the spell is left standing on
/// the stack so the flying is read as a *trigger* that has not resolved yet,
/// and the copy that enters afterwards is the control on `Filter::This`: it is
/// a Spirit as well, and a pump that had swept every Spirit would have armed
/// it. The printed +0/+0 is checked on the body, and a turn is walked because
/// "until end of turn" is part of the card.
#[test]
fn kami_of_tattered_shoji_gains_flying_when_a_spirit_spell_is_cast_under_it() {
    let p0 = PlayerId::new(0);
    let p1 = PlayerId::new(1);
    let mut engine = Duel::new(SEED, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                kami_of_tattered_shoji(),
            ],
        )
        .hand(0, &[kami_of_tattered_shoji()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let shrine =
        on_battlefield(&engine, p0, kami_of_tattered_shoji()).expect("the first Kami is out");
    assert!(
        !keywords(&engine, shrine).contains(KeywordSet::FLYING),
        "a seeded permanent is no cast, so nothing has granted it anything yet"
    );
    assert_eq!(pt(&engine, shrine), (2, 5), "the body the card prints");

    // Mana first: the offer is read off the pool and not off the five untapped
    // Plains, so the claim below would stay green on a board with no mana at all.
    tap_all_mana(&mut engine, p0);
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        5,
        "five Plains, five white, and the Kami makes no mana of its own"
    );
    let card = in_hand(&engine, p0, kami_of_tattered_shoji()).expect("the second Kami is in hand");
    let Pending::Priority { legal, .. } = engine.pending().clone() else {
        panic!("expected priority, got {:?}", engine.pending())
    };
    assert!(
        legal.castable.contains(&card),
        "{{4}}{{W}} is floating, so the second copy is castable: {:?}",
        legal.castable
    );

    cast_with_floating(&mut engine, p0, kami_of_tattered_shoji());
    assert_eq!(
        engine.state().players[0].mana_pool.total(),
        0,
        "the {{4}}{{W}} came out of the pool"
    );
    assert!(
        !stack_is_empty(&engine),
        "the spell is on the stack, and the trigger it set off is above it"
    );
    assert!(
        !keywords(&engine, shrine).contains(KeywordSet::FLYING),
        "CR 603.3b: the ability is a trigger, so the flying arrives when it \
         resolves and not while the spell is being cast"
    );

    pass_until(&mut engine, stack_is_empty);

    assert!(
        keywords(&engine, shrine).contains(KeywordSet::FLYING),
        "\"Whenever you cast a Spirit or Arcane spell, this creature gains \
         flying until end of turn\""
    );
    assert_eq!(
        pt(&engine, shrine),
        (2, 5),
        "the pump is +0/+0 with flying: the printed body is untouched"
    );
    let kamis = mine(&engine, p0, kami_of_tattered_shoji(), Zone::Battlefield);
    assert_eq!(kamis.len(), 2, "the second copy resolved onto the table");
    let arrived = kamis
        .into_iter()
        .find(|id| *id != shrine)
        .expect("one of the two is the copy that just resolved");
    assert!(
        !keywords(&engine, arrived).contains(KeywordSet::FLYING),
        "the copy that arrived is a Spirit too, so a pump that had lost \
         `Filter::This` would have armed it as well"
    );

    // "until end of turn": one turn later the creature is still standing and
    // the keyword is not — a static or permanent grant would still be on it.
    reach_their_main_phase(&mut engine, p1);
    reach_their_main_phase(&mut engine, p0);
    assert!(
        on_battlefield(&engine, p0, kami_of_tattered_shoji()).is_some(),
        "the Kami survived the turn it was pumped in"
    );
    assert!(
        !keywords(&engine, shrine).contains(KeywordSet::FLYING),
        "the grant lasted the turn it was made in and no longer"
    );
}
