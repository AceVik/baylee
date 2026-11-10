//! `cards/sorceries/mv_7/emeria_s_call.rs`, played.

#[allow(clippy::wildcard_imports)] // the parent's vocabulary and helpers
use super::*;

/// `Emeria's Call` // `Emeria, Shattered Skyclave` (`Coverage::Partial`): "Create two 4/4 white
/// Angel Warrior creature tokens with flying. Non-Angel creatures you control gain indestructible
/// until your next turn. // As this land enters, you may pay 3 life. If you don't, it enters tapped.
/// {T}: Add {W}."
///
/// Under `Coverage::Partial`, the tokens are created as 4/4 white Angels with flying (omitting
/// the Warrior subtype), and `Modifier::AddKeyword(KeywordSet::INDESTRUCTIBLE)` is applied to
/// all controlled non-Angel creatures. The test casts the front face, verifies that two 4/4
/// flying tokens are created, confirms that the caster's non-Angel creature gains indestructible,
/// and verifies that neither the Angel tokens nor the opponent's creature gain indestructible.
#[test]
fn emerias_call_creates_two_angel_tokens_and_grants_indestructible_to_non_angels() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(471, plains())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .battlefield(1, &[llanowar_elves()])
        .hand(0, &[emeria_s_call()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_elf = on_battlefield(&engine, p0, llanowar_elves()).expect("p0 controls an elf");
    let their_elf = on_battlefield(&engine, p1, llanowar_elves()).expect("p1 controls an elf");

    assert!(
        !keywords(&engine, my_elf).contains(KeywordSet::INDESTRUCTIBLE),
        "elf does not have indestructible before the spell"
    );

    tap_all_mana_but(&mut engine, p0, Some(llanowar_elves()));
    cast_front_face(&mut engine, p0, emeria_s_call());

    pass_until(&mut engine, stack_is_empty);

    let created_tokens = tokens_of(&engine, p0);
    assert_eq!(created_tokens.len(), 2, "creates exactly two tokens");
    for token in &created_tokens {
        assert_eq!(pt(&engine, *token), (4, 4), "each Angel token is 4/4");
        assert!(
            keywords(&engine, *token).contains(KeywordSet::FLYING),
            "each Angel token has flying"
        );
        assert!(
            !keywords(&engine, *token).contains(KeywordSet::INDESTRUCTIBLE),
            "Angel tokens do not gain indestructible (non-Angel restriction)"
        );
    }

    assert!(
        keywords(&engine, my_elf).contains(KeywordSet::INDESTRUCTIBLE),
        "controlled non-Angel creature gains indestructible"
    );
    assert!(
        !keywords(&engine, their_elf).contains(KeywordSet::INDESTRUCTIBLE),
        "opponent's creature does not gain indestructible"
    );
    assert!(
        in_graveyard(&engine, p0, emeria_s_call()).is_some(),
        "the sorcery resolved to the graveyard"
    );
}

/// Emeria's Call: "Create two 4/4 white Angel Warrior creature tokens with
/// flying. Non-Angel creatures you control gain indestructible until your
/// next turn." The two halves read each other, which is why the token's
/// **Warrior** half of the type line is not decoration: the rider spares
/// Angels, so a pair made from the pool's plain 4/4 white flying Angel
/// would look identical on the board and be the same card — the assertion
/// that separates them is that the Angels do *not* gain indestructible
/// while the Elves beside them do.
#[test]
fn emeria_s_call_makes_two_angel_warriors_its_own_rider_then_spares() {
    let p0 = PlayerId::new(0);
    let mut engine = Duel::new(9106, forest())
        .battlefield(
            0,
            &[
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                plains(),
                llanowar_elves(),
            ],
        )
        .hand(0, &[emeria_s_call()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let elves = on_battlefield(&engine, p0, llanowar_elves()).expect("the Elves are seated");
    assert!(
        !keywords(&engine, elves).contains(KeywordSet::INDESTRUCTIBLE),
        "nothing has been cast yet"
    );

    cast_from_hand(&mut engine, p0, emeria_s_call());
    pass_until(&mut engine, stack_is_empty);

    let made = tokens_of(&engine, p0);
    assert_eq!(made.len(), 2, "\"Create two … tokens\"");
    for angel in &made {
        assert_eq!(pt(&engine, *angel), (4, 4), "the printed 4/4");
        let wings = keywords(&engine, *angel);
        assert!(
            wings.contains(KeywordSet::FLYING),
            "\"with flying\": {wings:?}"
        );
        assert!(
            !wings.contains(KeywordSet::INDESTRUCTIBLE),
            "the rider reads \"non-Angel creatures\", and these are Angels"
        );
        let printed = engine
            .state()
            .object(*angel)
            .expect("the Angel is on the battlefield")
            .token
            .expect("it knows which token it is");
        assert_eq!(
            printed.name, "Angel Warrior",
            "not the pool's plain Angel, which the rider would have caught"
        );
    }
    assert!(
        keywords(&engine, elves).contains(KeywordSet::INDESTRUCTIBLE),
        "\"Non-Angel creatures you control gain indestructible\""
    );
}
