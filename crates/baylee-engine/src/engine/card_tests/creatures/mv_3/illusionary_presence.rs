//! `cards/creatures/mv_3/illusionary_presence.rs`, played.

#[allow(clippy::wildcard_imports)]
use super::*;
use crate::engine::card_tests::legends_kit as lk;
use baylee_core::generated::{index, subtypes};

const P0: PlayerId = PlayerId::new(0);
const P1: PlayerId = PlayerId::new(1);

fn illusionary_presence() -> CardIndex {
    card_index("8f319b31-efb0-4f48-a2a3-1f308e7b1dda")
}

fn answer_upkeep(
    engine: &mut Engine<RegistryLookup>,
    presence: ObjectId,
    land_type: baylee_core::ids::SubtypeId,
    expected_age: u16,
    pay: Option<u32>,
) {
    pass_until(engine, |e| {
        matches!(e.pending(), Pending::ChooseSubtype { .. })
    });
    engine
        .apply(P0, PlayerAction::ChooseSubtype(land_type))
        .expect("choose land type for landwalk");
    pass_until(engine, |e| {
        matches!(
            e.pending(),
            Pending::YesNo {
                prompt: YesNoPrompt::PayMana { .. },
                ..
            }
        )
    });
    assert_eq!(
        engine
            .state()
            .object(presence)
            .unwrap()
            .counters
            .get(baylee_cards_dsl::counters::AGE),
        expected_age
    );
    if let Some(blue) = pay {
        lk::float(engine, P0, &[(ManaColor::Blue, blue)]);
        lk::yes_no(engine, true);
        pass_until(engine, |e| {
            e.state().turn.phase == Phase::FirstMain && stack_is_empty(e)
        });
    } else {
        lk::yes_no(engine, false);
        lk::settle(engine);
    }
}

/// Illusionary Presence (8f319b31-efb0-4f48-a2a3-1f308e7b1dda) — {1}{U}{U} 2/2 Creature — Illusion:
/// - "Cumulative upkeep {U}"
/// - "At the beginning of your upkeep, choose a land type. This creature gains landwalk of the chosen type until end of turn."
#[test]
fn illusionary_presence_gains_chosen_landwalk_until_eot_and_has_cumulative_upkeep() {
    let mut engine = Duel::new(SEED, island())
        .battlefield(0, &[illusionary_presence(), island(), island()])
        .battlefield(1, &[forest(), index::GRIZZLY_BEARS])
        .start();
    keep_mulligans(&mut engine);

    let presence = on_battlefield(&engine, P0, illusionary_presence()).expect("Presence seated");
    let bears = on_battlefield(&engine, P1, index::GRIZZLY_BEARS).expect("Bears seated");

    // First upkeep: choose Forest, 1 age counter, pay {U}.
    answer_upkeep(&mut engine, presence, subtypes::land::FOREST, 1, Some(1));
    assert!(
        keywords(&engine, presence).contains(KeywordSet::FORESTWALK),
        "choosing Forest grants forestwalk until end of turn"
    );

    // P1 controls a Forest, so Grizzly Bears cannot block Illusionary Presence.
    let offer = lk::attack(&mut engine, &[presence]);
    assert!(
        !lk::may_block(&offer, bears, presence),
        "forestwalk makes Illusionary Presence unblockable while defending player controls a Forest"
    );
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);
    assert_eq!(engine.state().players[1].life, 18);

    // Forestwalk wears off at end of turn.
    reach_their_main_phase(&mut engine, P1);
    assert!(
        !keywords(&engine, presence).contains(KeywordSet::FORESTWALK),
        "landwalk wears off at end of turn"
    );

    // Second upkeep: 2 age counters, pay {U}{U}, choose Swamp -> no forestwalk.
    answer_upkeep(&mut engine, presence, subtypes::land::SWAMP, 2, Some(2));
    assert!(
        keywords(&engine, presence).contains(KeywordSet::SWAMPWALK),
        "second upkeep grants swampwalk"
    );
    // P1 controls a Forest, not a Swamp, so Grizzly Bears CAN block now.
    let offer2 = lk::attack(&mut engine, &[presence]);
    assert!(
        lk::may_block(&offer2, bears, presence),
        "without a Swamp on defending player's side, Grizzly Bears can block"
    );
    lk::block(&mut engine, &[]);
    lk::through_combat(&mut engine);

    // Third upkeep: decline cumulative upkeep -> sacrificed.
    answer_upkeep(&mut engine, presence, subtypes::land::ISLAND, 3, None);
    assert!(
        in_graveyard(&engine, P0, illusionary_presence()).is_some(),
        "declining cumulative upkeep sacrifices Illusionary Presence"
    );
}
