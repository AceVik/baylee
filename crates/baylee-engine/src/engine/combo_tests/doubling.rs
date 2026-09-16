//! Replacement effects that multiply, and whose side of the table they read: Doubling Season's two sentences — tokens created under your control, counters put on a permanent you control — Panharmonicon over an enters-trigger, and the same arithmetic reached again through a kicked Rite of Replication, a Helm of the Host's token, a Spark Double's counters and a Thief of Blood's drain. Every board here either carries the enchantment on one seat and the card it would replace on both, or is the same game played twice with the enchantment moved one seat across, because a doubling that asked the resolving effect's own controller answers yes for everybody and a board with a single enchantment on it can never say so (CR 614.12, CR 614.16). What the doubled token then *carries* is `copied_abilities`, and a doubling read through Nesting Dovehawk's populate is `token_arrivals`.

#[allow(clippy::wildcard_imports)] // this module's own vocabulary
use super::*;

/// Doubling Season ("If an effect would create one or more tokens under
/// **your** control, it creates twice that many of those tokens instead")
/// with a Maskwood Nexus on each side of the table.
///
/// The same token maker on both sides is the whole design of the test: one
/// ability, activated by two seats, so the only thing that differs between
/// the two counts below is who controls the enchantment. A replacement that
/// asked the *resolving* effect's own controller whether it controlled the
/// effect — which is a question that answers yes for everyone — would double
/// both, and a board with a single Nexus on it could never say so.
#[test]
fn doubling_season_doubles_your_tokens_and_not_the_ones_across_the_table() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(82, forest())
        .battlefield(
            0,
            &[
                doubling_season(),
                maskwood_nexus(),
                plains(),
                swamp(),
                forest(),
            ],
        )
        .battlefield(1, &[maskwood_nexus(), plains(), swamp(), forest()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let mine = on_battlefield(&engine, p0, maskwood_nexus()).expect("my nexus");
    activate(&mut engine, p0, mine);
    pass_until(&mut engine, |e| tokens_controlled(e, p0) > 0);
    assert_eq!(
        tokens_controlled(&engine, p0),
        2,
        "one Shapeshifter is created twice under my own Doubling Season"
    );

    reach_their_main_phase(&mut engine, p1);
    let theirs = on_battlefield(&engine, p1, maskwood_nexus()).expect("their nexus");
    activate(&mut engine, p1, theirs);
    pass_until(&mut engine, |e| tokens_controlled(e, p1) > 0);
    assert_eq!(
        tokens_controlled(&engine, p1),
        1,
        "their Nexus creates its token under *their* control, which is not \
         what my enchantment replaces"
    );
    assert_eq!(
        tokens_controlled(&engine, p0),
        2,
        "and nothing on their turn arrived on my side of the table"
    );
}

/// Panharmonicon ("If an artifact or creature entering causes a triggered
/// ability of a permanent **you control** to trigger, that ability triggers
/// an additional time") over Earth King's Lieutenant ("When this creature
/// enters, put a +1/+1 counter on each other Ally creature you control").
///
/// The counters are the readout: the Lieutenant's trigger places exactly one
/// per firing, so an Ondu Cleric that ends at 3/3 was given two and the
/// trigger fired twice. Both seats hold a Cleric and cast a Lieutenant, so
/// the artifact's controller is the only difference between the two numbers
/// — and the Cleric across the table is the bystander for the *trigger's*
/// own "you control" at the same time.
#[test]
fn panharmonicon_fires_your_enters_trigger_twice_and_theirs_once() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(83, forest())
        .battlefield(0, &[panharmonicon(), ondu_cleric(), forest(), plains()])
        .hand(0, &[earth_king_s_lieutenant()])
        .battlefield(1, &[ondu_cleric(), forest(), plains()])
        .hand(1, &[earth_king_s_lieutenant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("my cleric");
    let their_cleric = on_battlefield(&engine, p1, ondu_cleric()).expect("their cleric");

    cast_from_hand(&mut engine, p0, earth_king_s_lieutenant());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, earth_king_s_lieutenant()).is_some() && stack_is_empty(e)
    });
    assert_eq!(
        pt(&engine, my_cleric),
        (3, 3),
        "a 1/1 Ally under two firings of \"a +1/+1 counter on each other \
         Ally you control\""
    );
    assert_eq!(
        pt(&engine, their_cleric),
        (1, 1),
        "the Lieutenant's own trigger says \"you control\", so their Ally \
         was never in it"
    );

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, earth_king_s_lieutenant());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p1, earth_king_s_lieutenant()).is_some() && stack_is_empty(e)
    });
    assert_eq!(
        pt(&engine, their_cleric),
        (2, 2),
        "their Lieutenant is not a permanent I control, so my Panharmonicon \
         does not multiply its trigger"
    );
    assert_eq!(
        pt(&engine, my_cleric),
        (3, 3),
        "and my own Ally took nothing from their turn"
    );
}

/// Doubling Season's **second** sentence ("If an effect would put one or
/// more counters on a permanent **you control**, it puts twice that many of
/// those counters on that permanent instead") over the same Earth King's
/// Lieutenant.
///
/// Deliberately the same board and the same two numbers as
/// [`panharmonicon_fires_your_enters_trigger_twice_and_theirs_once`],
/// reached the other way round: there one counter was placed twice, here
/// two counters are placed once. Read together they say the two rules are
/// separate machines that happen to agree here — and the pair is what a
/// board with only one of the enchantments on it can never show.
#[test]
fn doubling_season_doubles_the_counters_on_your_own_permanents_only() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = Duel::new(87, forest())
        .battlefield(0, &[doubling_season(), ondu_cleric(), forest(), plains()])
        .hand(0, &[earth_king_s_lieutenant()])
        .battlefield(1, &[ondu_cleric(), forest(), plains()])
        .hand(1, &[earth_king_s_lieutenant()])
        .start();
    keep_mulligans(&mut engine);
    reach_main_phase(&mut engine, p0);

    let my_cleric = on_battlefield(&engine, p0, ondu_cleric()).expect("my cleric");
    let their_cleric = on_battlefield(&engine, p1, ondu_cleric()).expect("their cleric");

    cast_from_hand(&mut engine, p0, earth_king_s_lieutenant());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p0, earth_king_s_lieutenant()).is_some() && stack_is_empty(e)
    });
    assert_eq!(
        pt(&engine, my_cleric),
        (3, 3),
        "one +1/+1 counter, put on a permanent I control, is put twice"
    );
    assert_eq!(
        pt(&engine, their_cleric),
        (1, 1),
        "the Lieutenant's trigger never reached their Ally to begin with"
    );

    reach_their_main_phase(&mut engine, p1);
    cast_from_hand(&mut engine, p1, earth_king_s_lieutenant());
    pass_until(&mut engine, |e| {
        on_battlefield(e, p1, earth_king_s_lieutenant()).is_some() && stack_is_empty(e)
    });
    assert_eq!(
        pt(&engine, their_cleric),
        (2, 2),
        "their Ally is not a permanent I control, so my enchantment does \
         not replace the counter going onto it"
    );
    assert_eq!(
        pt(&engine, my_cleric),
        (3, 3),
        "and my own Ally took nothing from their turn"
    );
}

/// Crib Swap ("Exile target creature. **Its controller** creates a 1/1
/// colorless Shapeshifter creature token") cast under my own Doubling
/// Season.
///
/// CR 614.1 asks whose control the tokens would be created *under*, not
/// whose spell is creating them, and this spell deliberately hands them to
/// the player whose creature was just exiled. So my enchantment has nothing
/// to replace here: the Shapeshifter is theirs, and my removal must not
/// make it a pair of them.
#[test]
fn my_season_does_not_double_the_shapeshifter_my_own_removal_hands_them() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine =
        a_table_with_a_season_on_one_side(88, 0, &[plains(), plains(), plains()], crib_swap());
    aim_at_their_elf(&mut engine, crib_swap(), false);

    assert_eq!(
        tokens_controlled(&engine, p1),
        1,
        "the token is created under the exiled creature's controller, and \
         my Doubling Season is on the other side of the table from it"
    );
    assert_eq!(
        tokens_controlled(&engine, p0),
        0,
        "and nothing arrived on my own board"
    );
}

/// The same spell, the same board, the enchantment moved one seat: their
/// Doubling Season doubles the token my removal hands them.
///
/// The mirror of the test above and the half that has to fail before the
/// fix, because a branch that consults no replacement at all passes the
/// first one for the wrong reason.
#[test]
fn their_season_doubles_the_shapeshifter_my_removal_hands_them() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine =
        a_table_with_a_season_on_one_side(89, 1, &[plains(), plains(), plains()], crib_swap());
    aim_at_their_elf(&mut engine, crib_swap(), false);

    assert_eq!(
        tokens_controlled(&engine, p1),
        2,
        "the tokens are created under their control, which is exactly what \
         their own Doubling Season replaces"
    );
    assert_eq!(
        tokens_controlled(&engine, p0),
        0,
        "and my side of the table gained nothing from their enchantment"
    );
}

/// Rite of Replication ("Create a token that's a copy of target creature")
/// under my own Doubling Season.
///
/// A token copy is a token, so the same rule applies to it — and this is
/// the direction the previous pair cannot reach: the copy is created under
/// *my* control however far away the creature it copies is, so my
/// enchantment doubles it and their creature is untouched.
#[test]
fn my_season_doubles_the_copy_i_make_of_their_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_table_with_a_season_on_one_side(
        90,
        0,
        &[island(), island(), island(), island()],
        rite_of_replication(),
    );
    let victim = aim_at_their_elf(&mut engine, rite_of_replication(), false);

    assert_eq!(
        cardless_permanents(&engine, p0),
        2,
        "one token copy created twice, under my control and my own \
         Doubling Season"
    );
    assert!(
        on_battlefield(&engine, p1, llanowar_elves()).is_some(),
        "the creature that was copied is still theirs and still there"
    );
    assert_eq!(
        cardless_permanents(&engine, p1),
        0,
        "and copying their creature put nothing on their side of the table"
    );
    assert_eq!(
        engine
            .state()
            .object(victim)
            .map(|o| o.controller)
            .expect("the original is still an object"),
        p1,
        "copying a permanent does not take it"
    );
}

/// The same cast with the Doubling Season across the table: one copy.
///
/// Their enchantment reads "under **your** control", and the copy is
/// created under mine, so it has nothing to say about a spell of mine that
/// happens to point at a creature of theirs. Without this half, a branch
/// that doubled unconditionally would look right.
#[test]
fn their_season_does_not_double_the_copy_i_make_of_their_creature() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_table_with_a_season_on_one_side(
        91,
        1,
        &[island(), island(), island(), island()],
        rite_of_replication(),
    );
    aim_at_their_elf(&mut engine, rite_of_replication(), false);

    assert_eq!(
        cardless_permanents(&engine, p0),
        1,
        "the copy is created under my control, and their Doubling Season \
         replaces nothing that happens on my side of the table"
    );
    assert_eq!(
        cardless_permanents(&engine, p1),
        0,
        "and their own board gained nothing from copying their creature"
    );
}

/// The same Rite of Replication, kicked, under my own Doubling Season: ten
/// copies.
///
/// Kicker turns "create a token that's a copy" into five of them, and the
/// replacement multiplies the total rather than the printed one, so this is
/// the arithmetic the doubled branch has to get right and the first test to
/// send a kicked spell through it at all. Nine Islands is exactly
/// {7}{U}{U}, which is also what says the wizard charged for the kicker.
#[test]
fn a_kicked_rite_under_my_season_makes_ten_copies() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let mut engine = a_table_with_a_season_on_one_side(
        92,
        0,
        &[
            island(),
            island(),
            island(),
            island(),
            island(),
            island(),
            island(),
            island(),
            island(),
        ],
        rite_of_replication(),
    );
    aim_at_their_elf(&mut engine, rite_of_replication(), true);

    assert_eq!(
        cardless_permanents(&engine, p0),
        10,
        "five copies from the kicker, doubled by my own Doubling Season"
    );
    assert_eq!(
        cardless_permanents(&engine, p1),
        0,
        "and none of them arrived on the side of the table the original is on"
    );
}

/// Helm of the Host ("At the beginning of combat on your turn, create a
/// token that's a copy of equipped creature, except the token isn't
/// legendary") on a legendary creature, under its controller's Doubling
/// Season.
///
/// Two rules meet on one trigger and each would hide the other. The copy is
/// a **token**, so Doubling Season doubles it — and the token is **not
/// legendary**, so the state-based action that keeps one of each legend
/// (CR 704.5j) takes neither of them, nor the original. Drop the supertype
/// mod and this board collapses to a single permanent with the player asked
/// which one to keep; drop the token-ness and the Season has nothing to
/// double.
#[test]
fn a_helm_on_a_legend_makes_two_copies_the_legend_rule_lets_stand() {
    let (p0, p1) = (PlayerId::new(0), PlayerId::new(1));
    let engine = a_helm_on_a_legend(103, Some(0));

    assert_eq!(
        cardless_permanents(&engine, p0),
        2,
        "one token from the Helm, doubled by the Season"
    );
    assert!(
        on_battlefield(&engine, p0, loran_of_the_third_path()).is_some(),
        "and the legend the Helm is on is still standing"
    );
    for id in engine
        .state()
        .zones
        .list(crate::zone::ZoneLocation::Battlefield)
    {
        let Some(obj) = engine.state().object(*id) else {
            continue;
        };
        if obj.controller == p0 && obj.card.is_none() {
            let c = obj.characteristics();
            assert!(
                !c.supertypes
                    .contains(baylee_core::types::SupertypeSet::LEGENDARY),
                "the printing says the token isn't legendary"
            );
            assert!(
                c.keywords.contains(baylee_cards_dsl::KeywordSet::HASTE),
                "and that it gains haste"
            );
        }
    }
    assert_eq!(
        cardless_permanents(&engine, p1),
        0,
        "the tokens are the Helm controller's"
    );
}

/// The same Helm, with the Doubling Season across the table.
///
/// A doubling that read "a token is created" rather than "*you* create a
/// token" (CR 614.16) would double this too, and the test above could not
/// tell the difference: two tokens is two tokens whichever enchantment
/// caused them.
#[test]
fn their_doubling_season_does_not_double_the_helms_token() {
    let p0 = PlayerId::new(0);
    let engine = a_helm_on_a_legend(104, Some(1));
    assert_eq!(
        cardless_permanents(&engine, p0),
        1,
        "their Season doubles their tokens, and this one is mine"
    );
}

/// "This creature enters with a +1/+1 counter on it for each counter removed
/// this way" is the effect of a resolving ability putting counters on a
/// permanent, which is the first case CR 614.16 names — so a Doubling Season
/// doubles what lands. The drain above it is removal, which no replacement
/// here touches.
///
/// The enchantment moves across the table rather than off it, because both
/// numbers this test reads are ones it changes: under my own Season a Karn
/// enters with ten loyalty and the Thief takes twice that, and the same
/// board with the Season on the other seat gives five and five. A board with
/// one enchantment on it and no second reading cannot tell the two apart.
#[test]
fn a_thief_of_blood_takes_twice_what_it_drained_under_your_own_season() {
    let (loyalty, size) = a_thief_over_a_karn(88, 0);
    assert_eq!(loyalty, 10, "Karn's printed five, doubled on the way in");
    assert_eq!(
        size,
        (21, 21),
        "and the ten it drained doubled again on the way onto the Thief"
    );

    let (loyalty, size) = a_thief_over_a_karn(89, 1);
    assert_eq!(
        loyalty, 5,
        "their enchantment does not double my walker's loyalty"
    );
    assert_eq!(
        size,
        (6, 6),
        "nor what lands on my Thief: five drained, five placed"
    );
}

/// "…except it enters with an additional +1/+1 counter on it" is a
/// replacement effect (CR 614.1c), and a counter-doubling replacement
/// applies to what another replacement effect places even when the event it
/// modified was not itself an effect (CR 614.16).
///
/// What it copies is a 1/1, so every point above that is a counter this test
/// is about. The loyalty counter is asserted beside the P/T because the card
/// puts one on whatever it copied, and a door that doubled one kind and not
/// the other would read as working from the creature alone.
#[test]
fn a_spark_double_under_a_doubling_season_enters_with_two_of_each_counter() {
    let (bare, bare_loyalty) = a_spark_double_copying_an_elf(90, false);
    assert_eq!(bare, (2, 2), "one +1/+1 counter on a copied 1/1");
    assert_eq!(bare_loyalty, 1, "and one loyalty counter beside it");

    let (doubled, doubled_loyalty) = a_spark_double_copying_an_elf(91, true);
    assert_eq!(doubled, (3, 3), "two +1/+1 counters under the Season");
    assert_eq!(doubled_loyalty, 2, "and two loyalty counters");
}
