use super::*;
use baylee_cards_dsl::effect::{Amount, Effect};
use baylee_cards_dsl::filter::Filter;
use baylee_cards_dsl::loyalty;

#[test]
fn damage_history_death_trigger_maps_to_the_death_clause() {
    use baylee_cards_dsl::{CounterKind, Trigger, triggered};
    let ability = triggered!(
        Trigger::DiesAfterDamageByThis(&Filter::CREATURE),
        &[Effect::AddCounter {
            kind: CounterKind::P1P1,
            amount: Amount::Fixed(1),
        }]
    );
    let text = "Flying\nWhenever a creature dealt damage by this creature this turn dies, put a +1/+1 counter on this creature.";
    assert_eq!(map(&[ability], text).lines, [Some(1)]);
    assert!(!content_fits(
        &ability,
        "Whenever this creature deals damage to a creature, put a +1/+1 counter on this creature."
    ));
}

#[test]
fn life_ward_maps_to_its_printed_sentence_for_the_client() {
    use baylee_cards_dsl::{PlayerRel, Trigger, triggered};
    let ability = triggered!(
        Trigger::Ward,
        &[Effect::PlayerMayPayLifeOr {
            player: PlayerRel::ControllerOfTarget,
            life: Amount::SourcePower,
            effect: &Effect::CounterTargetSpellOrAbility,
        }]
    );
    let oracle = "Menace, lifelink\nWard—Pay life equal to this creature's power.";
    assert_eq!(map(&[ability], oracle).lines, [Some(1)]);
    assert_eq!(
        map(&[AbilityDef::Ward { mana: 3 }], "Ward {3}").lines,
        [Some(0)]
    );
    assert!(!content_fits(
        &ability,
        "Whenever this creature attacks, draw a card."
    ));
}

/// A mana ability whose "add" opens the body's second sentence is still
/// a mana line — and one that targets is not, whatever it adds.
#[test]
fn a_mana_line_may_choose_before_it_adds() {
    assert_eq!(
        line_shape(
            "{2}, {T}: Choose a color. Add an amount of mana of that color \
             equal to the number of creatures you control of the chosen type."
        ),
        LineShape::Mana,
        "Three Tree City's second ability"
    );
    assert_eq!(
        line_shape("{T}: Target player loses 1 life. Add {B}."),
        LineShape::Activated,
        "a targeting ability is not a mana ability (CR 605.1a)"
    );
    assert_eq!(
        line_shape(
            "{T}: Add one mana of any color. When that mana is spent to cast an \
             instant or sorcery spell, copy that spell and you may choose new \
             targets for the copy."
        ),
        LineShape::Mana,
        "Primal Wellspring: the \"target\" is the delayed trigger's"
    );
}

#[test]
fn mana_with_draw_or_mill_is_activated_and_keeps_its_printed_sentence() {
    use baylee_cards_dsl::{Cost, PlayerRel, activated, cost, mana_ability};
    use baylee_core::mana::ManaColor;
    const DRAW: &[Effect] = &[Effect::mana_of_any_color(), Effect::draw(1)];
    const BUNDLE_DRAW: &[Effect] = &[
        Effect::mana(ManaColor::Blue, 1),
        Effect::mana(ManaColor::Black, 1),
        Effect::draw(1),
    ];
    const OPTIONAL_DRAW: &[Effect] = &[
        Effect::mana_of_any_color(),
        Effect::MayDo {
            effects: &[Effect::draw(1)],
        },
    ];
    const MILL: &[Effect] = &[
        Effect::mana(ManaColor::Blue, 1),
        Effect::Mill {
            amount: Amount::Fixed(2),
            target: PlayerRel::You,
        },
    ];
    for (ability, text) in [
        (
            activated!(cost!("{1}", TapSelf, SacrificeSelf), DRAW),
            "{1}, {T}, Sacrifice this artifact: Add one mana of any color. Draw a card. (Activate only as an instant.)",
        ),
        (
            activated!(cost!("{2}", TapSelf, SacrificeSelf), BUNDLE_DRAW),
            "{2}, {T}, Sacrifice this artifact: Add {U}{B}. Draw a card.",
        ),
        (activated!(Cost::TAP, MILL), "{T}: Add {U}. Mill two cards."),
        (
            activated!(Cost::TAP, DRAW),
            "{T}: Draw a card. Add one mana of any color.",
        ),
        (
            mana_ability!(Cost::TAP, DRAW),
            "{T}: Add one mana of any color. Its controller draws a card.",
        ),
        (
            mana_ability!(
                Cost::TAP,
                OPTIONAL_DRAW,
                condition = Some(baylee_cards_dsl::Condition::ControlCount(
                    &Filter::ARTIFACT,
                    3
                ))
            ),
            "{T}: Add one mana of any color. You may draw a card. Activate only if you control three or more artifacts.",
        ),
    ] {
        assert_eq!(line_shape(text), LineShape::Activated, "{text}");
        assert_eq!(ability_shape(&ability), LineShape::Activated, "{text}");
        let found = map(&[ability], text);
        assert_eq!(found.lines, [Some(0)], "{text}");
        assert!(found.in_order, "{text}");
        assert_eq!(found.ambiguous, 0, "{text}");
    }
}

#[test]
fn mana_without_library_movement_stays_mana_on_both_sides() {
    use baylee_cards_dsl::{PlayerRel, TargetSpec, mana_ability};
    use baylee_core::mana::ManaColor;
    const PURE: &[Effect] = &[Effect::mana(ManaColor::Colorless, 2)];
    const DAMAGE: &[Effect] = &[
        Effect::mana(ManaColor::Colorless, 2),
        Effect::DealDamage {
            amount: Amount::Fixed(2),
            target: TargetSpec::Player(PlayerRel::You),
        },
    ];
    const DELAYED_DRAW: &[Effect] = &[
        Effect::mana(ManaColor::Colorless, 2),
        Effect::AtNextEndStep {
            effects: &[Effect::draw(1)],
        },
    ];
    const SCRY_SHUFFLE: &[Effect] = &[
        Effect::mana(ManaColor::Colorless, 2),
        Effect::Scry {
            amount: Amount::Fixed(1),
        },
        Effect::ShuffleLibrary {
            who: PlayerRel::You,
        },
    ];
    for (effects, text) in [
        (PURE, "{T}: Add {C}{C}."),
        (DAMAGE, "{T}: Add {C}{C}. This land deals 2 damage to you."),
        (
            DELAYED_DRAW,
            "{T}: Add {C}{C}. At the beginning of the next end step, draw a card.",
        ),
        (
            SCRY_SHUFFLE,
            "{T}: Add {C}{C}. Scry 1. Shuffle your library.",
        ),
    ] {
        let ability = mana_ability!(effects);
        assert_eq!(line_shape(text), LineShape::Mana, "{text}");
        assert_eq!(ability_shape(&ability), LineShape::Mana, "{text}");
        assert_eq!(map(&[ability], text).lines, [Some(0)], "{text}");
    }
}

/// "… add {B} instead" prints two outputs and the ability makes one;
/// without "instead", every printed symbol still has to be made.
#[test]
fn an_instead_sentence_fits_either_of_its_outputs() {
    use baylee_core::mana::ManaColor;
    let blue = [Effect::mana(ManaColor::Blue, 1)];
    assert!(mana_fits(
        &blue,
        "{T}: Add {U}. If you played a land this turn, add {B} instead."
    ));
    assert!(
        !mana_fits(&blue, "{T}: Add {U} or {B}."),
        "a choice between two is not an ability that makes one of them"
    );
    let red = [Effect::mana(ManaColor::Red, 1)];
    assert!(
        !mana_fits(
            &red,
            "{T}: Add {U}. If you played a land this turn, add {B} instead."
        ),
        "\"instead\" widens the printed pair, not the whole wheel"
    );
}

/// Two "At the beginning" sentences are told apart by the step, not by
/// the words they share.
///
/// Mana Vault is the card that found it. It prints an upkeep sentence
/// and a draw-step one, `trigger_words` answered `["beginning"]` for
/// both, and `whose_trigger_fits` said nothing about a step at all — so
/// its draw-step trigger fitted either, took the first, and the table
/// pointed at an ability the card does not have. The reader knew it was
/// a coin toss and said so in `Mapping::ambiguous`; nothing read that
/// number.
///
/// The step is what a card cannot phrase two ways: "at the beginning of
/// your upkeep", "… of your draw step". Reading it turns the toss into
/// a decision, which is what `ambiguous == 0` here asserts — and the
/// `swapped` half is the other direction, so a reader that merely
/// preferred the later sentence would not pass.
#[test]
fn two_step_triggers_are_told_apart_by_which_step_it_is() {
    use baylee_cards_dsl::effect::PlayerRel;
    use baylee_cards_dsl::{StepKind, Trigger, triggered};
    let text = "This artifact doesn't untap during your untap step.\n\
                At the beginning of your upkeep, you may pay {4}. If you \
                do, untap this artifact.\n\
                At the beginning of your draw step, if this artifact is \
                tapped, it deals 1 damage to you.\n\
                {T}: Add {C}{C}{C}.";
    let step = |step| {
        triggered!(
            Trigger::StepBegin {
                step,
                whose: PlayerRel::You,
            },
            draw(1)
        )
    };
    let abilities = [step(StepKind::Draw)];
    let found = map(&abilities, text);
    assert_eq!(
        found.lines,
        vec![Some(2)],
        "the draw-step trigger is the draw-step sentence",
    );
    assert_eq!(
        found.ambiguous, 0,
        "the step decides it, so nothing is tossed"
    );

    let abilities = [step(StepKind::Upkeep), step(StepKind::Draw)];
    let found = map(&abilities, text);
    assert_eq!(found.lines, vec![Some(1), Some(2)]);

    let swapped = [step(StepKind::Draw), step(StepKind::Upkeep)];
    let found = map(&swapped, text);
    assert_eq!(found.lines, vec![Some(2), Some(1)]);
    assert!(!found.in_order, "the card prints them the other way round");
}

/// A quoted ability belongs to whatever the sentence grants, and the
/// sentence is the granting ability's, never the granted one's.
///
/// CR 113.10a. Chromatic Lantern is the card that found it: its own
/// mana ability claimed `Lands you control have "{T}: Add one mana of
/// any color."` — the grant, one sentence before the identical line the
/// lantern actually prints for itself — and the table said `Some(0)`
/// where the card says `Some(1)`. It was `Other` then, which no ability
/// claims; it is [`LineShape::Grant`] now, which only a grant claims
/// (#212).
///
/// The short grant is the second half and is the case the pool does not
/// print yet. Length is what kept it out of the keyword branch, not
/// punctuation: a grant ends in a quotation mark, so `!ends_with('.')`
/// is true of it, and it carries a `{` like any cost. At 26 characters
/// it would have been read as `Activated` — the same defect one branch
/// over, which is why the refusal is stated and not left to fall
/// through.
#[test]
fn a_colon_inside_quotation_marks_belongs_to_the_ability_being_granted() {
    assert_eq!(
        line_shape(r#"Lands you control have "{T}: Add one mana of any color.""#),
        LineShape::Grant,
        "the lantern's grant is a static ability, not the mana it hands out",
    );
    assert_eq!(
        line_shape(r#"Lands have "{T}: Add {C}.""#),
        LineShape::Grant,
        "a grant short enough for the keyword branch is still a grant",
    );
    // The other direction, or the assertions above are satisfied by a
    // reader that refuses every sentence carrying a quotation mark: the
    // colon a card prints outside one is still its own.
    assert_eq!(
        line_shape("{T}: Add one mana of any color."),
        LineShape::Mana,
        "the lantern's own line is unchanged",
    );
    assert_eq!(
        line_shape(r#"{2}, {T}: Target creature gains "flying" until end of turn."#),
        LineShape::Activated,
        "a quotation mark later in the line does not hide the cost's colon",
    );
}

/// "As long as …" opens a static ability, never a trigger: The World
/// Tree's conditional grant was read as `Triggered`, and once the grant
/// was built no ability could claim its sentence. "As this creature
/// enters" stays the third trigger word it is.
#[test]
fn as_long_as_opens_a_static_and_as_it_enters_a_trigger() {
    assert_eq!(
        line_shape(
            r#"As long as you control six or more lands, lands you control have "{T}: Add one mana of any color.""#
        ),
        LineShape::Grant,
    );
    assert_eq!(
        line_shape("As this creature enters, choose a creature type."),
        LineShape::Triggered,
    );
}

/// A trigger on tapping for mana that adds mana is a mana ability
/// (CR 605.1b) and is read as one on both sides, so it is not counted
/// as a stack entry. One that targets is not (CR 605.1b's first test).
#[test]
fn a_triggered_mana_ability_is_mana_on_both_sides() {
    static GREEN: [baylee_cards_dsl::Effect; 1] = [baylee_cards_dsl::Effect::mana(
        baylee_core::mana::ManaColor::Green,
        1,
    )];
    assert_eq!(
        line_shape("Whenever you tap a creature for mana, add an additional {G}."),
        LineShape::Mana,
    );
    assert_eq!(
        line_shape(
            "Whenever you tap this land for mana, target opponent creates a 1/1 colorless Spirit creature token."
        ),
        LineShape::Triggered,
    );
    let cub = baylee_cards_dsl::AbilityDef::Triggered {
        trigger: baylee_cards_dsl::Trigger::TappedForMana {
            by: baylee_cards_dsl::PlayerRel::You,
            filter: &baylee_cards_dsl::Filter::CREATURE,
        },
        effects: &GREEN,
        targets: None,
        second_targets: None,
        zone: baylee_cards_dsl::TriggerZone::Battlefield,
        once_per_turn: false,
        condition: None,
    };
    assert_eq!(ability_shape(&cub), LineShape::Mana);
}

/// A static that grants an ability is placed on the sentence that
/// quotes it, and the ability it grants is not (#212).
///
/// Chromatic Lantern's two abilities, as the card writes them: the grant,
/// then the Lantern's own `{T}`. Before [`LineShape::Grant`] the static
/// was `None`, so the land it gave a `{T}` to had no sentence to show
/// for it. The second half holds the placement to the cost inside the
/// quotation marks: a grant costing `{1}` is not the Lantern's.
#[test]
fn a_static_that_grants_an_ability_is_placed_on_the_sentence_quoting_it() {
    use baylee_cards_dsl::{Cost, Modifier, mana_ability, static_ability};
    use baylee_core::mana::ManaColor;
    const ANY: [Effect; 1] = [Effect::mana(ManaColor::Blue, 1)];
    let text = "Lands you control have \"{T}: Add one mana of any color.\"\n\
                {T}: Add one mana of any color.";
    let grant = |cost| {
        static_ability!(
            Filter::LAND,
            Modifier::GrantActivated {
                cost,
                effects: &ANY,
                mana_ability: true,
            }
        )
    };
    let own = mana_ability!(&ANY);
    let found = map(&[grant(Cost::TAP), own], text);
    assert_eq!(found.lines, vec![Some(0), Some(1)]);
    assert_eq!(ability_shape(&grant(Cost::TAP)), LineShape::Grant);
    assert!(!LineShape::Grant.stackable(), "a static is no stack entry");

    let dearer = Cost {
        mana: baylee_core::mana::ManaCost::parse("{1}"),
        ..Cost::TAP
    };
    assert_eq!(
        map(&[grant(dearer), own], text).lines,
        vec![None, Some(1)],
        "a grant of `{{1}}, {{T}}` is not the one quoted here"
    );
}

#[test]
fn a_copy_exception_keeps_its_quoted_activated_cost_provenance() {
    use baylee_cards_dsl::{CopyMod, Cost, TargetSpec, mana_ability};
    use baylee_core::mana::{ManaColor, ManaCost};
    const MANA: [Effect; 1] = [Effect::mana(ManaColor::Blue, 1)];
    const GRANT: AbilityDef = mana_ability!(&MANA);
    const DEARER: AbilityDef = mana_ability!(
        Cost {
            mana: ManaCost::parse("{1}"),
            ..Cost::TAP
        },
        &MANA
    );
    let text = "You may have this artifact enter as a copy of any creature on the battlefield, except it has \"{T}: Add {U}.\"\n{T}: Add {U}.";
    let copy = |mods| AbilityDef::CopyOnEnter {
        target: TargetSpec::Object(&Filter::CREATURE),
        mods,
    };
    let matching = copy(&[CopyMod::GrantAbility(&GRANT)]);
    assert_eq!(ability_shape(&matching), LineShape::Grant);
    assert_eq!(map(&[matching, GRANT], text).lines, vec![Some(0), Some(1)]);
    let wrong_cost = copy(&[CopyMod::GrantAbility(&DEARER)]);
    assert_eq!(map(&[wrong_cost, GRANT], text).lines, vec![None, Some(1)]);
}

/// A keyword line is a sentence too, and which *kind* of keyword it is
/// the card says with its own punctuation.
///
/// Station is the case that found this. `Station (Tap another creature
/// you control: Put charge counters equal to its power on this
/// Spacecraft. …)` is an **activated ability** spelled out in reminder
/// text — it has a cost, a colon and an effect — while `Flying (This
/// creature can't be blocked except by creatures with flying or reach.)`
/// describes a rule and has no colon anywhere in it. Read as
/// [`LineShape::Other`], Station was a card whose only activated ability
/// had no sentence at all, and the sheet drew the row as "Ability 1".
///
/// The colon is read from the **printed** line and not from the line with
/// its reminder stripped, which is the whole trick: strip the reminder
/// first and both of these are one bare word.
/// "Crew 2" with no reminder is still an activated ability (CR
/// 702.122a): Unlicensed Hearse prints it bare. A word that merely
/// starts the same way is not one.
#[test]
fn a_bare_crew_line_is_an_activated_one() {
    assert_eq!(line_shape("Crew 2"), LineShape::Activated);
    assert_eq!(line_shape("Saddle 1"), LineShape::Activated);
    assert_eq!(line_shape("Crew"), LineShape::Other, "no number, no cost");
    assert_eq!(
        line_shape("Crew Captain"),
        LineShape::Other,
        "a name is not a keyword"
    );
}

#[test]
fn a_keyword_whose_reminder_spells_out_an_ability_is_an_activated_one() {
    let station = "Station (Tap another creature you control: Put charge \
                   counters equal to its power on this Spacecraft. \
                   Station only as a sorcery.)";
    assert_eq!(
        line_shape(station),
        LineShape::Activated,
        "a cost and a colon in the reminder is an activated ability",
    );

    let flying = "Flying (This creature can't be blocked except by \
                  creatures with flying or reach.)";
    assert_eq!(
        line_shape(flying),
        LineShape::Other,
        "a keyword that is a bit describes a rule and names no cost",
    );

    // The counter-test for the reading itself: it is the colon that
    // decides, and it has to be found inside the brackets. A colon in the
    // prose *outside* a reminder is a line this already handled.
    assert!(reminder_spells_out_an_ability(station));
    assert!(!reminder_spells_out_an_ability(flying));
    assert!(
        !reminder_spells_out_an_ability("Trample"),
        "a keyword printed with no reminder at all spells out nothing",
    );
}

/// Three loyalty abilities, printed the way a walker prints them.
const WALKER: &str = "+2: Look at the top card of target player's library.\n\
                      0: Draw three cards, then put two cards back.\n\
                      −1: Return target creature to its owner's hand.";

fn draw(n: u32) -> &'static [Effect] {
    Box::leak(Box::new([Effect::DrawCards {
        amount: Amount::Fixed(n),
    }]))
}

/// The counter-test the whole content check exists for.
///
/// All three of these abilities are `LineShape::Loyalty`, so a matcher
/// reading shape alone walks the swapped pair in printed order, finds a
/// loyalty line for each, and reports the card as aligned — while the
/// table it produces points the `+2` at the `0`'s sentence. Only the
/// cost printed before the colon can tell them apart.
#[test]
fn two_loyalty_abilities_in_each_others_places_are_caught_by_their_cost() {
    let right = [
        loyalty!(2, draw(1)),
        loyalty!(0, draw(3)),
        loyalty!(-1, draw(1)),
    ];
    let found = map(&right, WALKER);
    assert_eq!(found.lines, vec![Some(0), Some(1), Some(2)]);
    assert!(found.in_order);

    let swapped = [
        loyalty!(0, draw(3)),
        loyalty!(2, draw(1)),
        loyalty!(-1, draw(1)),
    ];
    let found = map(&swapped, WALKER);
    assert_eq!(
        found.lines,
        vec![Some(1), Some(0), Some(2)],
        "each ability still finds its own sentence — backwards"
    );
    assert!(
        !found.in_order,
        "and the walk says so, which is what the report counts"
    );
}

/// One printed sentence, two compiled triggers — Sun Titan's shape.
///
/// Both abilities belong to line 1, so a bijection invariant would
/// report the DSL being more precise than the printing as a defect.
#[test]
fn one_sentence_may_be_claimed_by_two_triggers() {
    use baylee_cards_dsl::{Trigger, triggered};
    let text = "Vigilance\n\
                Whenever this creature enters or attacks, you may return \
                target permanent card from your graveyard to the battlefield.";
    let abilities = [
        triggered!(Trigger::ETB, draw(1)),
        triggered!(Trigger::Attacks(&Filter::This), draw(1)),
    ];
    assert_eq!(map(&abilities, text).lines, vec![Some(1), Some(1)]);
}

/// Evoke's sacrifice is printed as a keyword line, not as a sentence.
///
/// Mulldrifter prints two lines that both say "enters" and only one of
/// them is a sentence; a trigger that accepted either took the draw and
/// left the card looking complete with its table wrong twice over.
#[test]
fn an_ability_printed_as_a_keyword_is_honestly_lineless() {
    use baylee_cards_dsl::{Trigger, triggered};
    let text = "Flying\n\
                When this creature enters, draw two cards.\n\
                Evoke {2}{U}";
    let abilities = [
        triggered!(Trigger::ETB, draw(2)),
        triggered!(Trigger::EntersBattlefieldEvoked, &[]),
    ];
    assert_eq!(map(&abilities, text).lines, vec![Some(1), None]);
}

/// Two triggers with the same verb, told apart by their subject.
///
/// "you draw" and "an opponent draws" fit each other's sentences on the
/// word alone. Whichever the walk reached first was a coin toss the
/// report printed as a hit, which is what `Mapping::ambiguous` counts.
#[test]
fn two_triggers_sharing_a_verb_are_told_apart_by_whose_event_it_is() {
    use baylee_cards_dsl::effect::PlayerRel;
    use baylee_cards_dsl::{Trigger, triggered};
    let text = "Deathtouch\n\
                Whenever you draw a card, you gain 2 life.\n\
                Whenever an opponent draws a card, they lose 2 life.";
    let abilities = [
        triggered!(Trigger::Draws(PlayerRel::You), draw(1)),
        triggered!(Trigger::Draws(PlayerRel::Opponent), draw(1)),
    ];
    let found = map(&abilities, text);
    assert_eq!(found.lines, vec![Some(1), Some(2)]);
    assert_eq!(found.ambiguous, 0, "neither fits the other's sentence");

    let swapped = [
        triggered!(Trigger::Draws(PlayerRel::Opponent), draw(1)),
        triggered!(Trigger::Draws(PlayerRel::You), draw(1)),
    ];
    let found = map(&swapped, text);
    assert_eq!(found.lines, vec![Some(2), Some(1)]);
    assert!(!found.in_order);
}

/// One sentence naming two subjects belongs to both of them.
///
/// Orcish Bowmasters prints "When this creature enters **and** whenever
/// an opponent draws a card…", and a subject test that refused either
/// half sent a correctly written card to the misses. The subject is
/// also read from the clause before the first comma only — the rest of
/// the sentence routinely names the other player without the trigger
/// being theirs.
#[test]
fn a_clause_naming_both_subjects_is_refused_to_neither() {
    use baylee_cards_dsl::effect::PlayerRel;
    use baylee_cards_dsl::{Trigger, triggered};
    let text = "Flash\n\
                When this creature enters and whenever an opponent draws a card \
                except the first one they draw in each of their draw steps, this \
                creature deals 1 damage to any target.";
    let abilities = [
        triggered!(Trigger::ETB, draw(1)),
        triggered!(Trigger::DrawsExceptFirst(PlayerRel::Opponent), draw(1)),
    ];
    let found = map(&abilities, text);
    assert_eq!(found.lines, vec![Some(1), Some(1)]);
    assert!(found.in_order, "a shared sentence is not disorder");
}

/// One mode, one bullet — the shape three of the pool's four modal
/// cards print.
#[test]
fn a_bulleted_card_gives_each_mode_its_own_bullet() {
    let text = "Choose one —\n\
                • Each opponent sacrifices a nontoken creature of their choice.\n\
                • Each opponent sacrifices a creature token of their choice.\n\
                • Each opponent sacrifices a planeswalker of their choice.";
    let modes = [mode(None), mode(None), mode(None)];
    assert_eq!(
        map_modes(&modes, text),
        vec![Some(1), Some(2), Some(3)],
        "the header is a sentence too, and no mode is it"
    );
}

/// Spree lists its modes under a plus sign, each with its own cost, and
/// the plus sign is the bullet (CR 702.172b). Final Showdown's shape:
/// the keyword line first, and no mode is it.
#[test]
fn a_spree_card_gives_each_mode_its_plus_sign() {
    let text = "Spree (Choose one or more additional costs.)\n\
                + {1} — All creatures lose all abilities until end of turn.\n\
                + {1} — Choose a creature you control. It gains indestructible \
                until end of turn.\n\
                + {3}{W}{W} — Destroy all creatures.";
    let modes = [mode(None), mode(None), mode(None)];
    assert_eq!(map_modes(&modes, text), vec![Some(1), Some(2), Some(3)]);
}

/// A bullet count that disagrees with the mode count is refused whole.
///
/// The counter-test the honesty rule exists for: walking them in step
/// places two of the three and is confidently wrong about which, and
/// the row it draws is the card's own words on the wrong mode — worse
/// than the "Mode 2" it replaced.
#[test]
fn a_printing_with_a_bullet_too_few_is_refused_whole() {
    let text = "Choose one —\n\
                • Destroy target artifact.\n\
                • Destroy target enchantment.";
    let modes = [mode(None), mode(None), mode(None)];
    assert_eq!(map_modes(&modes, text), vec![None, None, None]);
}

/// "Choose up to one" prints one bullet fewer than it has modes.
///
/// Ertai Resurrected's shape: two things to choose and a third mode
/// that does nothing, which is how a player declines. Read as an
/// ordinary bullet count it is one short, and refusing it cost the two
/// printed sentences their rows as well.
#[test]
fn a_mode_a_player_declines_with_is_printed_nowhere_and_takes_no_bullet() {
    let text = "Flash\n\
                When this creature enters, choose up to one —\n\
                • Counter target spell, activated ability, or triggered \
                ability. Its controller draws a card.\n\
                • Destroy another target creature or planeswalker. Its \
                controller draws a card.";
    let modes = [mode(None), mode(None), decline()];
    assert_eq!(
        map_modes(&modes, text),
        vec![Some(2), Some(3), None],
        "the decline is not a bullet, and the two that are keep theirs"
    );
}

/// The counter-test for the clause above, in both directions.
///
/// The reading needs the printing to say "choose up to" *and* exactly
/// one mode that does nothing. A card one bullet short without the
/// words is the ordinary disagreement and is refused whole; a card
/// that says the words while every mode does something is read
/// bullet-for-mode as any other, with nothing to spare.
#[test]
fn a_short_bullet_list_without_the_words_is_still_refused() {
    let silent = "Choose one —\n\
                  • Destroy target artifact.\n\
                  • Destroy target enchantment.";
    assert_eq!(
        map_modes(&[mode(None), mode(None), decline()], silent),
        vec![None, None, None],
        "nothing here says a mode may be declined"
    );

    let spoken = "Choose up to one —\n\
                  • Destroy target artifact.\n\
                  • Destroy target enchantment.";
    assert_eq!(
        map_modes(&[mode(None), mode(None)], spoken),
        vec![Some(1), Some(2)],
        "the words alone invent no decline"
    );
}

/// Overload prints no bullet: a body and a keyword line.
#[test]
fn an_overloaded_card_finds_its_body_and_its_keyword_line() {
    let text = "Return target nonland permanent you don't control to its \
                owner's hand.\n\
                Overload {6}{U} (You may cast this spell for its overload \
                cost. If you do, change \"target\" in its text to \"each.\")";
    let modes = [mode(None), mode(Some(baylee_core::mana!("{6}{U}")))];
    assert_eq!(map_modes(&modes, text), vec![Some(0), Some(1)]);
}

/// A second brace-free sentence leaves the plain mode two candidates.
///
/// "Flying" over a body over an overload cost is a card nobody prints
/// today, and the point is exactly that: the reader says it cannot tell
/// which of the two is the mode instead of taking the first.
#[test]
fn a_keyword_line_beside_the_body_refuses_the_plain_mode() {
    let text = "Flying and vigilance, and this creature attacks each combat \
                if able.\n\
                Return target nonland permanent you don't control to its \
                owner's hand.\n\
                Overload {6}{U}";
    let modes = [mode(None), mode(Some(baylee_core::mana!("{6}{U}")))];
    assert_eq!(map_modes(&modes, text), vec![None, None]);
}

/// The two ways a card prints an alternative cost, and a sentence that
/// is neither.
#[test]
fn an_alternative_cost_is_found_as_a_sentence_or_as_a_keyword_line() {
    let pitch = "You may pay 1 life and exile a blue card from your hand \
                 rather than pay this spell's mana cost.\n\
                 Counter target spell.";
    assert_eq!(
        map_alternatives(&[alt(life_and_pitch(), AltCondition::Always)], pitch),
        vec![Some(0)]
    );
    let evoke = "Flash\n\
                 Lifelink\n\
                 When this creature enters, exile up to one other target \
                 creature. That creature's controller gains life equal to \
                 its power.\n\
                 Evoke—Exile a white card from your hand.";
    assert_eq!(
        map_alternatives(&[alt(pitch_only(), AltCondition::Always)], evoke),
        vec![Some(3)],
        "the trigger names an exile too, and says nothing about a hand"
    );
}

/// "Without paying its mana cost" is the cost of nothing, and "rather
/// than pay" is a cost that was substituted.
///
/// Which is what tells a free alternative from a pitch one without
/// reading either sentence, and it is a claim about the templating that
/// a card printing the wrong one of the two would break loudly.
#[test]
fn the_two_templates_separate_a_free_cost_from_a_substituted_one() {
    let free = "If you control a commander, you may cast this spell \
                without paying its mana cost.\n\
                Counter target noncreature spell.";
    assert_eq!(
        map_alternatives(
            &[alt(
                baylee_cards_dsl::Cost::FREE,
                AltCondition::CommanderControlled
            )],
            free
        ),
        vec![Some(0)]
    );
    assert_eq!(
        map_alternatives(
            &[alt(pitch_only(), AltCondition::CommanderControlled)],
            free
        ),
        vec![None],
        "a cost that pays something cannot be the sentence that pays nothing"
    );
}

/// The condition is read, so the two free commander instants cannot
/// take Force of Negation's sentence and the other way round.
#[test]
fn a_condition_the_sentence_does_not_state_is_refused() {
    let text = "If it's not your turn, you may exile a blue card from your \
                hand rather than pay this spell's mana cost.\n\
                Counter target noncreature spell.";
    assert_eq!(
        map_alternatives(&[alt(pitch_only(), AltCondition::NotYourTurn)], text),
        vec![Some(0)]
    );
    assert_eq!(
        map_alternatives(&[alt(pitch_only(), AltCondition::Always)], text),
        vec![None],
        "the sentence opens with a condition and the cost carries none"
    );
}

/// A sentence that fits two alternative costs names neither of them.
///
/// No card in the pool prints two, so this is the rule stated rather
/// than a case observed — and it is read out of a snapshot, because
/// striking the first one through in place would leave the second
/// holding the sentence alone and looking like a single hit.
#[test]
fn one_sentence_that_fits_two_alternatives_names_neither() {
    let text = "You may exile a blue card from your hand rather than pay \
                this spell's mana cost.\n\
                Counter target spell.";
    assert_eq!(
        map_alternatives(
            &[
                alt(pitch_only(), AltCondition::Always),
                alt(pitch_only(), AltCondition::Always),
            ],
            text
        ),
        vec![None, None],
        "the second one must not keep the sentence the first gave up"
    );
}

/// One mode with no `cost_override` and no sentence at all.
///
/// It carries an effect, and that is not decoration: [`map_modes`]
/// reads an **effect-less** mode as the one a player declines with,
/// so a stand-in that did nothing would be every test's card printing
/// "choose up to". [`decline`] is the mode that means it.
fn mode(cost_override: Option<baylee_core::mana::ManaCost>) -> SpellMode {
    SpellMode {
        effects: draw(1),
        targets: None,
        second_targets: None,
        cost_override,
        additional_cost: None,
    }
}

/// The mode "choose up to one" gives a player to decline with: no
/// effects, nothing to target, nothing to pay (Ertai Resurrected).
fn decline() -> SpellMode {
    SpellMode {
        effects: &[],
        targets: None,
        second_targets: None,
        cost_override: None,
        additional_cost: None,
    }
}

/// One alternative cost.
fn alt(cost: baylee_cards_dsl::Cost, condition: AltCondition) -> AlternativeCost {
    AlternativeCost { cost, condition }
}

/// Force of Will's: a life payment and a pitched card.
fn life_and_pitch() -> baylee_cards_dsl::Cost {
    baylee_cards_dsl::Cost {
        mana: baylee_core::mana::ManaCost::ZERO,
        parts: &[
            CostPart::PayLife(1),
            CostPart::ExileFromHand(&baylee_cards_dsl::Filter::Any),
        ],
    }
}

/// Misdirection's and evoke's: a pitched card and nothing else.
fn pitch_only() -> baylee_cards_dsl::Cost {
    baylee_cards_dsl::Cost {
        mana: baylee_core::mana::ManaCost::ZERO,
        parts: &[CostPart::ExileFromHand(&baylee_cards_dsl::Filter::Any)],
    }
}
