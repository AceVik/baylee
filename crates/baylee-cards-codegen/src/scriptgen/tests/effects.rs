//! Reading mana, animation, searches, pumps and statics.

use super::*;

#[test]
fn revealed_hand_discard_distinguishes_permanents_from_other_nonlands() {
    for (valid, permanent_only) in [("Permanent.nonLand", true), ("Card.nonLand", false)] {
        let body = read(&format!(
            "Name:Private inspection\nManaCost:2 B\nTypes:Sorcery\n\
             A:SP$ Discard | ValidTgts$ Opponent | Mode$ RevealYouChoose | DiscardValid$ {valid}"
        ));
        let effect = &body.abilities[0];
        assert!(effect.contains("Filter::NONLAND"), "{effect}");
        assert_eq!(
            effect.contains("TypeSet::PLANESWALKER"),
            permanent_only,
            "{effect}"
        );
    }
}

#[test]
fn a_sweep_exempts_its_source_from_each_printed_type() {
    for (instruction, exceptions) in [
        (
            "Destroy all artifacts and creatures other than Private furnace.",
            2,
        ),
        (
            "Destroy all artifacts and creatures other than this creature.",
            2,
        ),
        ("Destroy all other creatures and all artifacts.", 1),
    ] {
        let body = read(&format!(
            "Name:Private furnace\nManaCost:4 R\nTypes:Artifact\n\
             A:AB$ DestroyAll | Cost$ T | ValidCards$ Creature.Other,Artifact\n\
             Oracle:{{T}}: {instruction}"
        ));
        let effect = &body.abilities[0];
        assert_eq!(
            effect.matches("Filter::Another").count(),
            exceptions,
            "{effect}"
        );
    }
}

#[test]
fn control_and_untap_resolve_in_printed_order() {
    for (oracle, control_first) in [
        (
            "Gain control of target artifact until end of turn. Untap it.",
            true,
        ),
        (
            "Untap target artifact and gain control of it until end of turn.",
            false,
        ),
    ] {
        let body = read(&format!(
            "Name:Private exchange\nManaCost:3 R\nTypes:Instant\n\
             A:SP$ GainControl | ValidTgts$ Artifact | LoseControl$ EOT | Untap$ True\nOracle:{oracle}"
        ));
        let effects = &body.abilities[0];
        assert_eq!(
            effects.find("Modifier::GainControl").unwrap()
                < effects.find("Effect::UntapTarget").unwrap(),
            control_first,
            "{effects}"
        );
    }
}

#[test]
fn repeated_target_declarations_keep_independent_pump_slots() {
    // Synthetic grammar fixture, not a corpus card: distinct numbers make
    // accidentally moving an instruction to the other slot observable.
    for second in ["Creature", "Creature.YouCtrl"] {
        let text = format!(
            "Name:Two separate gifts\nTypes:Instant\nManaCost:1 G\n\
             A:SP$ Pump | ValidTgts$ Creature | NumAtt$ 2 | SubAbility$ Next\n\
             SVar:Next:DB$ Pump | ValidTgts$ {second} | NumDef$ 5\n"
        );
        let body = read(&text);
        let ability = &body.abilities[0];
        assert!(
            ability.contains("Effect::PumpTarget { power: Amount::Fixed(2)"),
            "{ability}"
        );
        assert!(
            ability.contains("Effect::PumpTargetSlot { slot: TargetSlot::Second,"),
            "{ability}"
        );
        assert!(ability.contains("toughness: Amount::Fixed(5)"), "{ability}");
        assert!(
            ability.contains("second_targets = Some(TargetReq::one("),
            "{ability}"
        );
    }
}

#[test]
fn repeated_targets_without_slot_support_are_refused_even_when_types_match() {
    for next in [
        "DB$ Pump | ValidTgts$ Creature | NumDef$ 1 | SubAbility$ Third",
        "DB$ Pump | ValidTgts$ Creature | NumDef$ X",
        "DB$ Pump | ValidTgts$ Creature | NumDef$ 1 | UnknownRule$ True",
        "DB$ Pump | ValidTgts$ Creature | NumDef$ 1 | Duration$ Permanent",
        "DB$ Pump | ValidTgts$ Creature | NumDef$ 1 | TgtNum$ 2",
    ] {
        let text = format!(
            "Name:Independent target boundary\nTypes:Instant\nManaCost:2 G\n\
             A:SP$ Pump | ValidTgts$ Creature | NumAtt$ 4 | SubAbility$ Next\n\
             SVar:Next:{next}\nSVar:Third:DB$ Pump | ValidTgts$ Creature | NumDef$ 2\n"
        );
        assert!(refused(&text), "independent target collapsed: {text}");
    }
}

#[test]
fn a_defined_target_continuation_does_not_ask_for_another_target() {
    let body = read(
        "Name:One gift in two parts\nTypes:Instant\nManaCost:1 G\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ 2 | SubAbility$ Next\n\
         SVar:Next:DB$ Pump | Defined$ Targeted | NumDef$ 5\n",
    );
    assert!(!body.abilities[0].contains("second_targets"));
    assert_eq!(body.abilities[0].matches("Effect::PumpTarget {").count(), 2);
}

#[test]
fn independent_counter_targets_are_refused_until_counter_slots_are_supported() {
    let head = "Name:Separate counters\nTypes:Instant\nManaCost:2 G\n\
                A:SP$ PutCounter | ValidTgts$ Creature | CounterType$ P1P1 | CounterNum$ 2";
    let single = read(head);
    assert!(single.abilities[0].contains("Effect::AddCounter"));
    let pair = format!(
        "{head} | SubAbility$ Next\n\
         SVar:Next:DB$ PutCounter | ValidTgts$ Creature | CounterType$ P1P1 | CounterNum$ 4\n"
    );
    let body = read(&pair);
    assert!(
        body.abilities[0].contains("Effect::WithSecondTarget(&[Effect::AddCounter { kind: CounterKind::P1P1, amount: Amount::Fixed(4) }])")
            && body.abilities[0].contains("second_targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))"),
        "two independent PutCounter clauses now lower via WithSecondTarget: {}",
        body.abilities[0]
    );
    let triple = format!(
        "{head} | SubAbility$ Next\n\
         SVar:Next:DB$ PutCounter | ValidTgts$ Creature | CounterType$ P1P1 | CounterNum$ 4 | SubAbility$ Third\n\
         SVar:Third:DB$ PutCounter | ValidTgts$ Creature | CounterType$ P1P1 | CounterNum$ 6\n"
    );
    assert_eq!(
        refusal_reason(&parse(&triple), &cats(), None).as_deref(),
        Some("unsupported independent target in one chain")
    );
}

/// `Produced$ W U` is "add {W}{U}" — two mana at once. `Combo W U` is
/// the choice between them, and reading one as the other would hand a
/// bounce land twice the mana or half of it.
#[test]
fn produced_lists_two_mana_and_combo_offers_a_choice() {
    let both = read("Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ W U");
    assert_eq!(
        both.abilities,
        ["mana_ability!(&[Effect::mana(ManaColor::White, 1), Effect::mana(ManaColor::Blue, 1)])"]
    );
    let either = read("Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ Combo W U");
    assert_eq!(
        either.abilities,
        ["mana_ability!(&[Effect::mana_choice(&[ManaColor::White, ManaColor::Blue])])"]
    );
}

/// "Spend this mana only to cast a creature spell" is a rider on the
/// mana, and only on spells: `Activated.Hero` restricts an *ability*,
/// which `ManaRestriction` cannot say, so a card printing both stays a
/// stub rather than becoming the half of itself we can express.
#[test]
fn restricted_mana_reads_a_spell_filter_and_only_that() {
    let body = read(
        "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ Any | RestrictValid$ Spell.Creature",
    );
    assert_eq!(
        body.abilities,
        [
            "mana_ability!(&[Effect::mana_of_any_color().restricted(&Filter::CREATURE, SpendRider::None)])"
        ]
    );

    let script = parse(
        "Name:X\nTypes:Land\nA:AB$ Mana | Cost$ T | Produced$ Any | RestrictValid$ Spell.Hero,Activated.Hero",
    );
    assert_eq!(
        refusal_reason(&script, &cats(), None).as_deref(),
        Some("`Mana.RestrictValid` beyond a spell")
    );
}

/// A manland: one printed sentence, four layers. "It's still a land" is
/// why the types are *added*, and CR 613.1 is why each layer is its own
/// effect rather than one lump.
#[test]
fn animate_becomes_one_continuous_effect_per_layer() {
    let body = read(
        "Name:X\nTypes:Land\n\
         A:AB$ Animate | Cost$ 1 G | Defined$ Self | Power$ 3 | Toughness$ 3 | Types$ Creature,Goblin | Colors$ Green | OverwriteColors$ True | Keywords$ Trample",
    );
    let a = body.abilities.join("");
    // No layer is written: `Effect::continuous` derives it from the
    // modifier (CR 613.1), so the modifier *is* the layer claim here.
    for expected in [
        "Effect::continuous(&Filter::This, Modifier::AddType(TypeSet::CREATURE), Duration::UntilEndOfTurn)",
        "Modifier::AddSubtype(subtypes::creature::GOBLIN)",
        "Effect::continuous(&Filter::This, Modifier::SetColor(ColorSet::from_slice(&[Color::Green])), Duration::UntilEndOfTurn)",
        "Effect::continuous(&Filter::This, Modifier::AddKeyword(KeywordSet::TRAMPLE), Duration::UntilEndOfTurn)",
        "Effect::continuous(&Filter::This, Modifier::SetPT(3, 3), Duration::UntilEndOfTurn)",
    ] {
        assert!(a.contains(expected), "missing `{expected}` in {a}");
    }
    assert!(!a.contains("RemoveType"), "it's still a land");

    // `Filter::This` binds to the first target when the chain has one,
    // so an animate that also targets would animate the wrong
    // permanent. Refuse rather than guess which was meant.
    let script = parse(
        "Name:X\nTypes:Instant\nA:SP$ Animate | ValidTgts$ Land | Defined$ Self | Power$ 3 | Toughness$ 3 | Types$ Creature",
    );
    assert_eq!(
        refusal_reason(&script, &cats(), None).as_deref(),
        Some("`Animate` of something other than the source")
    );

    // The Laces: the target is what `Filter::This` binds to, the stack
    // is a zone it may be in, and "becomes" lasts the game.
    let lace = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ Animate | Colors$ Red | OverwriteColors$ True | ValidTgts$ Card \
         | TgtZone$ Stack,Battlefield | Duration$ Permanent",
    );
    let a = lace.abilities.join("");
    assert!(
        a.contains(
            "Effect::continuous(&Filter::This, Modifier::SetColor(ColorSet::from_slice(\
             &[Color::Red])), Duration::Indefinitely)"
        ),
        "{a}"
    );
    assert!(
        a.contains("TargetSpec::StackOrBattlefield(&Filter::Any)"),
        "{a}"
    );
    let exiled = parse(
        "Name:X\nTypes:Instant\n\
         A:SP$ Animate | Colors$ Red | ValidTgts$ Card | TgtZone$ Exile",
    );
    assert_eq!(
        refusal_reason(&exiled, &cats(), None).as_deref(),
        Some("a target in `TgtZone$ Exile`")
    );
}

/// The bounce land's sentence: nobody is targeted, the ability's
/// controller picks one of their own lands, and it goes to its owner's
/// hand.
///
/// Each half is struck on its own below, because a reader that emitted
/// this shape for *any* untargeted `Battlefield` → `Hand` line would
/// pass the first assertion and be wrong about four other cards.
#[test]
fn a_hidden_battlefield_to_hand_is_a_choice_among_your_own() {
    let body = read(
        "Name:X\nTypes:Land\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
         | Execute$ TrigReturn | TriggerDescription$ return a land you control.\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand \
         | Hidden$ True | Mandatory$ True | ChangeType$ Land.YouCtrl \
         | AILogic$ NeverBounceItself | SpellDescription$ Return a land you control.",
    );
    assert_eq!(
        body.abilities,
        [
            "triggered!(Trigger::ETB, &[Effect::ReturnChosenToHand { who: PlayerRel::You, \
             filter: &Filter::YOUR_LAND }])"
        ]
    );

    // `Mandatory$ True` is what separates "return a land you control"
    // from "you may return a land you control", and the effect can only
    // say the first. Without the word it is refused rather than read as
    // either — the same bargain a library search makes about its count.
    let silent = parse(
        "Name:X\nTypes:Land\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
         | Execute$ TrigReturn | TriggerDescription$ return a land you control.\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand \
         | Hidden$ True | ChangeType$ Land.YouCtrl",
    );
    assert_eq!(
        refusal_reason(&silent, &cats(), None).as_deref(),
        Some("`ChangeZone` chosen without `Mandatory$`")
    );

    // `Hidden$` is the discriminator, and without it the line falls
    // through to the report it had before this rule existed.
    let open = parse(
        "Name:X\nTypes:Land\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
         | Execute$ TrigReturn | TriggerDescription$ return a land you control.\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand \
         | Mandatory$ True | ChangeType$ Land.YouCtrl",
    );
    assert_eq!(
        refusal_reason(&open, &cats(), None).as_deref(),
        Some("`ChangeZone` with neither a target nor `Defined$`")
    );

    // Arid Archway: the return is this rule, and the surveil hanging
    // off it reads the permanent that came back. Nothing claims
    // `RememberLKI$`, so the card is refused whole rather than written
    // as a bounce land that forgot half its sentence.
    let remembered = parse(
        "Name:X\nTypes:Land\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
         | Execute$ TrigReturn | TriggerDescription$ return a land you control.\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand \
         | Hidden$ True | Mandatory$ True | ChangeType$ Land.YouCtrl | RememberLKI$ True",
    );
    assert_eq!(
        refusal_reason(&remembered, &cats(), None).as_deref(),
        Some("unclaimed parameter `ChangeZone.RememberLKI`")
    );

    // A second permanent is a different rule, and it says so rather
    // than writing a card that returns one of them.
    let two = parse(
        "Name:X\nTypes:Land\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
         | Execute$ TrigReturn | TriggerDescription$ return two lands you control.\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand \
         | Hidden$ True | Mandatory$ True | ChangeNum$ 2 | ChangeType$ Land.YouCtrl",
    );
    assert_eq!(
        refusal_reason(&two, &cats(), None).as_deref(),
        Some("`ChangeZone` returning 2 chosen permanents")
    );
}

/// The two enters-tapped sentences that count players, and the four
/// ways a line is refused instead.
///
/// Both directions are struck on their own, because the whole risk in
/// this rule is reading one of them with the other's sign: the
/// comparator says when the land comes down *tapped* and the card prints
/// when it does not.
#[test]
fn an_enters_tapped_condition_may_count_players() {
    let head = "Name:X\nTypes:Land\n\
         R:Event$ Moved | ValidCard$ Card.Self | Destination$ Battlefield \
         | ReplaceWith$ LandTapped | ReplacementResult$ Updated | Description$ enters tapped.\n";

    // Luxury Suite: taps while you have fewer than two opponents, which
    // is "unless you have two or more opponents".
    let crowd = read(&format!(
        "{head}SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True \
         | ConditionCheckSVar$ Y | ConditionSVarCompare$ LT2\n\
         SVar:Y:PlayerCountOpponents$Amount"
    ));
    assert_eq!(
        crowd.enter_modifiers,
        ["EnterModifier::TappedUnlessOpponents { at_least: 2 }"]
    );

    // Razortrap Gorge: taps while the lowest life total is above
    // thirteen, which is "unless a player has 13 or less life".
    let unlucky = read(&format!(
        "{head}SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True \
         | ConditionCheckSVar$ X | ConditionSVarCompare$ GT13\n\
         SVar:X:PlayerCountPlayers$LowestLifeTotal"
    ));
    assert_eq!(
        unlucky.enter_modifiers,
        ["EnterModifier::TappedUnlessSomeoneAtOrBelow { life: 13 }"]
    );

    // `LE`/`GE` are the same sentences off by one, and the arithmetic is
    // asserted rather than assumed.
    let off_by_one = read(&format!(
        "{head}SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True \
         | ConditionCheckSVar$ Y | ConditionSVarCompare$ LE1\n\
         SVar:Y:PlayerCountOpponents$Amount"
    ));
    assert_eq!(
        off_by_one.enter_modifiers,
        ["EnterModifier::TappedUnlessOpponents { at_least: 2 }"]
    );

    for (tail, svar, why) in [
        // The opponent count with the life total's comparator. Read with
        // a flipped sign this would be a land that enters untapped in
        // every duel; refused, it is a stub.
        (
            "| ConditionCheckSVar$ Y | ConditionSVarCompare$ GT1",
            "SVar:Y:PlayerCountOpponents$Amount",
            "an enter-tapped condition counting `PlayerCountOpponents$Amount` `GT1`",
        ),
        // And the life total with the count's comparator.
        (
            "| ConditionCheckSVar$ X | ConditionSVarCompare$ LT13",
            "SVar:X:PlayerCountPlayers$LowestLifeTotal",
            "an enter-tapped condition counting `PlayerCountPlayers$LowestLifeTotal` `LT13`",
        ),
        // A count this rule has never seen: named by what it counts, not
        // by the letter the corpus wrote.
        (
            "| ConditionCheckSVar$ Z | ConditionSVarCompare$ EQ0",
            "SVar:Z:Count$Valid Creature.YouCtrl",
            "an enter-tapped condition counting `Count$Valid Creature.YouCtrl` `EQ0`",
        ),
        // The comparison missing altogether.
        (
            "| ConditionCheckSVar$ Y",
            "SVar:Y:PlayerCountOpponents$Amount",
            "replacement `Moved` counting an SVar with no comparison",
        ),
    ] {
        let script = parse(&format!(
            "{head}SVar:LandTapped:DB$ Tap | Defined$ Self | ETB$ True {tail}\n{svar}"
        ));
        assert_eq!(
            refusal_reason(&script, &cats(), None).as_deref(),
            Some(why),
            "{tail}"
        );
    }
}

/// A fetchland: `Origin$ Library` is a *search*, not a zone change with
/// a hidden target — a card in a library cannot be targeted at all.
#[test]
fn a_library_change_zone_is_a_search() {
    let body = read(
        "Name:X\nTypes:Land\n\
         A:AB$ ChangeZone | Cost$ T Sac<1/CARDNAME> | Origin$ Library | Destination$ Battlefield | ChangeType$ Forest",
    );
    assert_eq!(
        body.abilities,
        [
            "activated!(cost!(TapSelf, SacrificeSelf), &[Effect::SearchLibrary { filter: &SEARCH1, finds: &[Find::BATTLEFIELD], optional: false }])"
        ]
    );

    // `finds` is positional and its length is the count, so two cards
    // are the same `Find` twice — and `Tapped$ True` is a different
    // `Find`, not a flag beside it.
    let two = read(
        "Name:X\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Battlefield | Tapped$ True | ChangeNum$ 2 | Optional$ True | ChangeType$ Forest",
    );
    assert!(
        two.abilities.join("").contains(
            "finds: &[Find::BATTLEFIELD_TAPPED, Find::BATTLEFIELD_TAPPED], optional: true"
        ),
        "{:?}",
        two.abilities
    );

    // The same line with neither word. `ChangeNum$` alone does not say
    // whether two is a maximum or a requirement, and the two are
    // different cards, so it is refused rather than guessed at.
    let ambiguous = parse(
        "Name:X\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeNum$ 2 | ChangeType$ Forest",
    );
    assert_eq!(
        refusal_reason(&ambiguous, &cats(), None).as_deref(),
        Some("`ChangeZone` finding several cards without saying whether that is a maximum")
    );

    // `Mandatory$ True` is the other word, and it says the opposite of
    // `Optional$ True` rather than merely failing to say it.
    let must = read(
        "Name:X\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeNum$ 2 | Mandatory$ True | ChangeType$ Card",
    );
    assert!(
        must.abilities
            .join("")
            .contains("finds: &[Find::HAND, Find::HAND], optional: false"),
        "{:?}",
        must.abilities
    );

    // A count of one needs no word at all, which is what keeps every
    // fetchland in the pool readable.
    let one = read(
        "Name:X\nTypes:Land\n\
         A:AB$ ChangeZone | Cost$ T Sac<1/CARDNAME> | Origin$ Library | Destination$ Battlefield | ChangeType$ Land.Basic | ChangeTypeDesc$ basic land",
    );
    assert!(
        one.abilities.join("").contains("optional: false"),
        "{:?}",
        one.abilities
    );
}

#[test]
fn threshold_and_stated_quality_searches_honor_optional_rules() {
    let threshold = read(
        "Name:ThresholdSearch\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Battlefield | Tapped$ True | \
         ChangeType$ Land.Basic | ChangeNum$ NumLands\n\
         SVar:NumLands:Count$Threshold.3.1\n\
         Oracle:Search your library for a basic land card, put that card onto the battlefield tapped, then shuffle.\\nThreshold — If there are seven or more cards in your graveyard, instead search your library for up to three basic land cards, put them onto the battlefield tapped, then shuffle.",
    );
    let rendered = threshold.abilities.join("");
    assert!(
        rendered.contains(
            "then: &[Effect::SearchLibrary { filter: &SEARCH1, finds: &[Find::BATTLEFIELD_TAPPED, Find::BATTLEFIELD_TAPPED, Find::BATTLEFIELD_TAPPED], optional: true }]"
        ),
        "{rendered}"
    );
    assert!(
        rendered.contains(
            "otherwise: &[Effect::SearchLibrary { filter: &SEARCH1, finds: &[Find::BATTLEFIELD_TAPPED], optional: false }]"
        ),
        "{rendered}"
    );

    let stated_mandatory = parse(
        "Name:StrictQualitySearch\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Battlefield | \
         ChangeNum$ 2 | Mandatory$ True | ChangeType$ Land.Basic",
    );
    assert_eq!(
        refusal_reason(&stated_mandatory, &cats(), None).as_deref(),
        Some("`ChangeZone` requiring multiple cards with a stated quality")
    );
}

/// `ChangeTypeDesc$` restates the filter beside it for a human, and is
/// claimed only while that filter is there to carry the meaning.
///
/// Not a `PROSE_KEYS` entry, and this is the difference: alone on a
/// line it is the only thing said about what is being found, and a
/// reader that dropped it would be inventing a filter.
#[test]
fn a_search_label_is_claimed_only_beside_the_filter_it_restates() {
    let labelled = read(
        "Name:X\nTypes:Land\n\
         A:AB$ ChangeZone | Cost$ T Sac<1/CARDNAME> | Origin$ Library | Destination$ Battlefield | Tapped$ True | ChangeType$ Land.Basic | ChangeTypeDesc$ basic land",
    );
    assert!(
        labelled
            .abilities
            .join("")
            .contains("finds: &[Find::BATTLEFIELD_TAPPED]"),
        "{:?}",
        labelled.abilities
    );

    let bare = parse(
        "Name:X\nTypes:Land\n\
         A:AB$ ChangeZone | Cost$ T Sac<1/CARDNAME> | Origin$ Library | Destination$ Battlefield | ChangeTypeDesc$ basic land",
    );
    assert_eq!(
        refusal_reason(&bare, &cats(), None).as_deref(),
        Some("unreadable value in `ChangeZone`"),
        "a label with no filter beside it says nothing a card can be built from"
    );
}

/// The load-bearing rule: an unread parameter, an unread effect, an
/// unread line kind or an unread keyword all refuse the whole card. Each
/// Giant Growth: the commonest shape in the whole script corpus.
#[test]
fn a_pump_binds_to_the_target_and_keeps_its_sign() {
    let body = read(
        "Name:Giant Growth\nManaCost:G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +3 | NumDef$ +3 | \
         SpellDescription$ gets +3/+3.\n",
    );
    let text = body.abilities.join("\n");
    assert!(text.contains("Effect::PumpTarget"), "{text}");
    assert!(text.contains("power: Amount::Fixed(3)"), "{text}");
    assert!(text.contains("keywords: KeywordSet::EMPTY"), "{text}");

    // A shrink is the same effect with the sign in the variant,
    // because `Amount::Fixed` cannot hold one.
    let body = read(
        "Name:Weakness\nManaCost:B\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -2 | NumDef$ -1\n",
    );
    let text = body.abilities.join("\n");
    assert!(text.contains("power: Amount::NegXFixed(2)"), "{text}");
    assert!(text.contains("toughness: Amount::NegXFixed(1)"), "{text}");
}

/// A pump that **counts**, both ways round.
///
/// The letter is not the number: `NumAtt$ -X` with `SVar:X:Count$xPaid`
/// is the X a player announced and reads as `Amount::NegX`, while
/// `Count$Valid Artifact.YouCtrl` is a board count and has nothing to do
/// with an announced number at all. Both were refused, because a pump
/// asked [`amount`] and [`amount`] reads no count; a mana line has asked
/// [`Tx::counted_amount`] the whole time.
///
/// The positive side is the larger half — 148 reference scripts against
/// 26 — and comes from the same fallthrough, so refusing it would have
/// been a guard written for no reason a card could state.
#[test]
fn a_pump_counts_in_either_direction() {
    // Irradiate: "-1/-1 until end of turn for each artifact you control".
    let body = read(
        "Name:Irradiate\nManaCost:3 B\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -X | NumDef$ -X | IsCurse$ True\n\
         SVar:X:Count$Valid Artifact.YouCtrl\n",
    );
    let text = body.abilities.join("\n");
    assert!(text.contains("Effect::PumpTarget"), "{text}");
    assert_eq!(
        text.matches("Amount::Negated(&Amount::CountOf {").count(),
        2,
        "both sides of the pump negate the same count: {text}"
    );

    // Wirewood Pride's shape — "+X/+X, where X is the number of Elves" —
    // over a subtype this fixture's catalog knows. The same reading with
    // no sign in front of it.
    let body = read(
        "Name:Goblin Pride\nManaCost:G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ X | NumDef$ X\n\
         SVar:X:Count$Valid Goblin\n",
    );
    let text = body.abilities.join("\n");
    assert!(text.contains("power: Amount::CountOf {"), "{text}");
    assert!(
        !text.contains("Negated"),
        "nothing here counts downwards: {text}"
    );

    // And a count this reader still cannot say is still a refusal, named
    // by what it resolves *through*. Blood Lust is the card; the
    // honest-stub rule does not care which side of the sign it is on.
    assert_eq!(
        refusal_reason(
            &parse(
                "Name:Blood Lust\nManaCost:B\nTypes:Instant\n\
                 A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -X | NumDef$ -X\n\
                 SVar:X:Count$Compare T GE4.4.T\n"
            ),
            &cats(),
            None
        )
        .as_deref(),
        Some("pump amount `-X` = `Count$Compare T GE4.4.T`"),
        "a refused pump says what its letter resolves through"
    );
}

/// "Doesn't untap during your untap step" is an `R:` line in the
/// reference and a **static ability** here, because CR 613.11 makes it a
/// continuous effect modifying a game rule rather than a replacement of
/// any event. Basalt Monolith is the card, and the whole of it is read:
/// the rule, the mana ability and the way out.
#[test]
fn a_cant_happen_untap_replacement_is_a_static_ability() {
    let body = read(
        "Name:Basalt Monolith\nManaCost:3\nTypes:Artifact\n\
         R:Event$ Untap | ValidCard$ Card.Self | ValidStepTurnToController$ You | \
         Layer$ CantHappen | Description$ This artifact doesn't untap.\n\
         A:AB$ Mana | Cost$ T | Produced$ C | Amount$ 3\n\
         A:AB$ Untap | Cost$ 3\n",
    );
    assert_eq!(
        body.abilities,
        [
            "static_ability!(Filter::This, Modifier::DoesNotUntap)",
            "mana_ability!(&[Effect::mana(ManaColor::Colorless, 3)])",
            "activated!(cost!(\"{3}\"), &[Effect::UntapSelf])",
        ]
    );
    assert!(
        body.statics.is_empty(),
        "`Card.Self` is `Filter::This` and needs no `static`: {}",
        body.statics
    );
}

/// `Defined$ Self` is the source, not the target, even inside an
/// ability that has one.
#[test]
fn a_pump_on_itself_is_not_a_pump_on_the_target() {
    let body = read(
        "Name:X\nManaCost:R\nTypes:Creature Goblin\nPT:1/1\n\
         A:AB$ Pump | Cost$ R | Defined$ Self | NumAtt$ +1 | NumDef$ +0\n",
    );
    let text = body.abilities.join("\n");
    assert!(text.contains("Effect::PumpFilter"), "{text}");
    assert!(text.contains("filter: &Filter::This"), "{text}");
}

/// A static's `IsPresent$` is the condition it exists under, and
/// `GainControl$ You` is layer 2 to the static's controller.
#[test]
fn a_static_holds_while_its_clause_does_and_can_give_control() {
    // Sedge Troll, with a land the test catalog knows.
    let body = read(
        "Name:X\nManaCost:2 R\nTypes:Creature Goblin\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddPower$ 1 | AddToughness$ 1 | \
         IsPresent$ Mountain.YouCtrl | Description$ gets +1/+1.\n",
    );
    let text = body.abilities.join("\n");
    assert!(
        text.contains("condition = Some(Condition::ControlCount(&CHECK"),
        "{text}"
    );
    assert!(text.trim_end().ends_with("))"), "{text}");

    // Control Magic: layer 2, to the static's controller.
    let body = read(
        "Name:X\nManaCost:2 U U\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ Continuous | Affected$ Card.EnchantedBy | GainControl$ You | \
         Description$ You control enchanted creature.\n",
    );
    let text = body.abilities.join("\n");
    assert!(
        text.contains("static_ability!(Filter::AttachedToBySource, Modifier::GainControl)"),
        "{text}"
    );

    // Control given to anyone else is not a sentence it can write.
    let script = parse(
        "Name:X\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ Continuous | Affected$ Card.EnchantedBy | GainControl$ Opponent\n",
    );
    assert!(transcode(&script, &cats(), None).is_none());
}

/// Disrupting Scepter: the target player chooses the card, and the
/// ability is activated only during its controller's turn.
#[test]
fn a_chosen_discard_on_your_turn_only() {
    let body = read(
        "Name:X\nManaCost:3\nTypes:Artifact\n\
         A:AB$ Discard | Cost$ 3 T | ValidTgts$ Player | NumCards$ 1 | Mode$ TgtChoose | \
         PlayerTurn$ True | SpellDescription$ Target player discards a card.\n",
    );
    let text = body.abilities.join("\n");
    assert!(
        text.contains("Effect::DiscardForPlayers { who: PlayerRel::Chosen, count: 1 }"),
        "{text}"
    );
    assert!(
        text.contains("condition = Some(Condition::YourTurn)"),
        "{text}"
    );
}

/// Psionic Blast: `DamageMap$` gathers, `DamageResolve` deals.
#[test]
fn gathered_damage_is_dealt_in_one_resolution() {
    let body = read(
        "Name:X\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 4 | DamageMap$ True | \
         SubAbility$ DBDealDamage | SpellDescription$ 4 to any target and 2 to you.\n\
         SVar:DBDealDamage:DB$ DealDamage | Defined$ You | NumDmg$ 2 | \
         SubAbility$ DBDamageResolve\n\
         SVar:DBDamageResolve:DB$ DamageResolve\n",
    );
    let text = body.abilities.join("\n");
    assert_eq!(text.matches("Effect::DealDamage").count(), 2, "{text}");
    assert!(
        text.contains("TargetSpec::Player(PlayerRel::You)"),
        "{text}"
    );
}

/// Zombie Master / Viridian Longbow: a plain granted activated ability uses
/// `Modifier::GrantActivated`; a granted ability that targets hoists a
/// `static GRANTED_ACTIVATED_<N>: AbilityDef` via `Modifier::GrantActivatedDefinition`,
/// while unsupported granted forms (e.g. `ActivationZone$ Graveyard`) stay refused.
#[test]
fn a_granted_ability_is_the_holders_own() {
    let body = read(
        "Name:X\nManaCost:1 B B\nTypes:Creature Goblin\nPT:2/3\n\
         S:Mode$ Continuous | Affected$ Card.Goblin+Other | AddAbility$ Regenerate | \
         Description$ Other Goblins have regenerate.\n\
         SVar:Regenerate:AB$ Regenerate | Cost$ B | SpellDescription$ Regenerate this permanent.\n",
    );
    let text = body.abilities.join("\n");
    assert!(text.contains("Modifier::GrantActivated {"), "{text}");
    assert!(text.contains("cost: cost!(\"{B}\")"), "{text}");
    assert!(text.contains("TargetSpec::ThisObject"), "{text}");
    assert!(text.contains("mana_ability: false"), "{text}");

    let targeted = read(
        "Name:X\nTypes:Creature Goblin\nPT:2/3\n\
         S:Mode$ Continuous | Affected$ Creature.Goblin | AddAbility$ Ping\n\
         SVar:Ping:AB$ DealDamage | Cost$ T | ValidTgts$ Any | NumDmg$ 1\n",
    );
    assert!(
        targeted
            .statics
            .contains("static GRANTED_ACTIVATED_1: AbilityDef = activated!(Cost::TAP, &[Effect::DealDamage { amount: Amount::Fixed(1), target: TargetSpec::AnyTarget }], target = Some(TargetSpec::AnyTarget));"),
        "{}",
        targeted.statics
    );
    assert_eq!(
        targeted.abilities,
        [
            "static_ability!(Filter::And(&[Filter::CREATURE, Filter::HasSubtype(subtypes::creature::GOBLIN)]), Modifier::GrantActivatedDefinition(&GRANTED_ACTIVATED_1))"
        ]
    );

    assert!(refused(
        "Name:X\nTypes:Creature Goblin\nPT:2/3\n\
         S:Mode$ Continuous | Affected$ Creature.Goblin | AddAbility$ GravePing\n\
         SVar:GravePing:AB$ DealDamage | Cost$ 1 | ActivationZone$ Graveyard | ValidTgts$ Any | NumDmg$ 1\n"
    ));
}

/// Lifetap and Psychic Venom: a `Taps` trigger on another permanent, and
/// "that land's controller" read off it.
#[test]
fn a_tapped_trigger_reads_any_permanent_and_its_controller() {
    let venom = read(
        "Name:X\nTypes:Enchantment Aura\nK:Enchant:Land\n\
         T:Mode$ Taps | ValidCard$ Card.AttachedBy | TriggerZones$ Battlefield | Execute$ D\n\
         SVar:D:DB$ DealDamage | Defined$ TriggeredCardController | NumDmg$ 2",
    );
    let a = venom.abilities.join("");
    assert!(
        a.contains("Trigger::BecomesTapped(&Filter::AttachedToBySource)"),
        "{a}"
    );
    assert!(
        a.contains("TargetSpec::Player(PlayerRel::ControllerOfEvent)"),
        "{a}"
    );
    let lifetap = read(
        "Name:X\nTypes:Enchantment\n\
         T:Mode$ Taps | ValidCard$ Forest.OppCtrl | TriggerZones$ Battlefield | Execute$ G\n\
         SVar:G:DB$ GainLife | LifeAmount$ 1",
    );
    assert!(
        lifetap
            .abilities
            .join("")
            .contains("Trigger::BecomesTapped(&TRIGGER"),
        "{:?}",
        lifetap.abilities
    );
}

/// Disintegrate and Magma Spray: "if it's a creature, it can't be
/// regenerated this turn, and if it would die this turn, exile it
/// instead", on an any-target and on a creature target. "A creature
/// dealt damage this way" uses actual recipient memory instead; an unread
/// condition on the older target-based rider and a player target are refused.
#[test]
fn exile_if_it_dies_and_no_regeneration_read_the_lines_target() {
    let disintegrate = read(
        "Name:X\nTypes:Sorcery\nManaCost:X R\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ X | SubAbility$ E | \
         ReplaceDyingDefined$ ThisTargetedCard.Creature\n\
         SVar:E:DB$ Effect | RememberObjects$ ParentTarget | ForgetOnMoved$ Battlefield | \
         StaticAbilities$ NoRegen | IsCurse$ True | ConditionDefined$ ParentTarget | \
         ConditionPresent$ Creature | AILogic$ CantRegenerate\n\
         SVar:NoRegen:Mode$ CantRegenerate | ValidCard$ Card.IsRemembered | \
         Description$ It can't be regenerated.\n\
         SVar:X:Count$xPaid",
    );
    let a = disintegrate.abilities.join("");
    assert!(
        a.contains(
            "Effect::IfTargetMatches { filter: &Filter::CREATURE, then: \
             &[Effect::ExileIfDiesThisTurn { target: TargetSpec::AnyTarget }] }"
        ),
        "{a}"
    );
    assert!(
        a.contains(
            "Effect::IfTargetMatches { filter: &Filter::CREATURE, then: \
             &[Effect::CantBeRegeneratedThisTurn { target: TargetSpec::AnyTarget }] }"
        ),
        "{a}"
    );
    let spray = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 2 | ReplaceDyingDefined$ Targeted",
    );
    let a = spray.abilities.join("");
    assert!(
        a.contains("Effect::ExileIfDiesThisTurn { target: TargetSpec::Object("),
        "{a}"
    );
    assert!(!a.contains("IfTargetMatches"), "{a}");
    for refused_line in [
        "A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | ReplaceDyingDefined$ Remembered.Creature",
        "A:SP$ DealDamage | ValidTgts$ Player | NumDmg$ 3 | ReplaceDyingDefined$ Targeted",
        "A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 2 | \
         ReplaceDyingDefined$ ThisTargetedCard.Creature | ReplaceDyingCondition$ Kicked",
    ] {
        assert!(
            refused(&format!("Name:X\nTypes:Instant\n{refused_line}")),
            "{refused_line}"
        );
    }
    // Actual recipients differ from original targets when damage redirects.
    let recipient_restriction = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3 | SubAbility$ E | RememberDamaged$ True\n\
         SVar:E:DB$ Effect | RememberObjects$ Remembered.Creature | ForgetOnMoved$ Battlefield | \
         StaticAbilities$ NoRegen | IsCurse$ True\n\
         SVar:NoRegen:Mode$ CantRegenerate | ValidCard$ Card.IsRemembered",
    );
    let effects = recipient_restriction.abilities.join("");
    assert!(effects.contains("Effect::DamagedThisWayCantBeRegenerated"));
    assert!(!effects.contains("Effect::CantBeRegeneratedThisTurn"));
}

/// Wheel of Fortune, Timetwister and Natural Selection: whose hand,
/// whose graveyard, whose library. A library position, a random pick
/// and a count the player announced are refused.
#[test]
fn whole_hands_graveyards_and_another_players_library_read_whose() {
    let wheel = read(
        "Name:X\nTypes:Sorcery\n\
         A:SP$ Discard | Mode$ Hand | Defined$ Player | SubAbility$ D\n\
         SVar:D:DB$ Draw | Defined$ Player | NumCards$ 7",
    );
    let a = wheel.abilities.join("");
    assert!(
        a.contains("Effect::DiscardHand { who: PlayerRel::EachPlayer }"),
        "{a}"
    );
    let twister = read(
        "Name:X\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ChangeType$ Card | Origin$ Hand,Graveyard | \
         Destination$ Library | Shuffle$ True | UseAllOriginZones$ True",
    );
    let a = twister.abilities.join("");
    assert!(
        a.contains(
            "Effect::ShuffleIntoLibrary { who: PlayerRel::EachPlayer, hand: true, \
             graveyard: true }"
        ),
        "{a}"
    );
    let feldon = read(
        "Name:X\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ValidTgts$ Player | ChangeType$ Card | Origin$ Graveyard | \
         Destination$ Library | Shuffle$ True",
    );
    let a = feldon.abilities.join("");
    assert!(
        a.contains(
            "Effect::ShuffleIntoLibrary { who: PlayerRel::Chosen, hand: false, \
             graveyard: true }"
        ),
        "{a}"
    );
    let selection = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ RearrangeTopOfLibrary | ValidTgts$ Player | NumCards$ 3 | MayShuffle$ True",
    );
    let a = selection.abilities.join("");
    assert!(
        a.contains("Effect::ReorderTopLibraryOf { who: PlayerRel::Chosen, count: 3 }"),
        "{a}"
    );
    assert!(
        a.contains(
            "Effect::MayDo { effects: &[Effect::ShuffleLibrary { who: PlayerRel::Chosen }] }"
        ),
        "{a}"
    );
    let mine = read(
        "Name:X\nTypes:Artifact\n\
         A:AB$ RearrangeTopOfLibrary | Cost$ 1 | Defined$ You | NumCards$ 3",
    );
    assert!(
        mine.abilities
            .join("")
            .contains("Effect::ReorderTopLibrary { count: 3 }"),
        "{:?}",
        mine.abilities
    );
    for refused_line in [
        "A:SP$ ChangeZoneAll | ChangeType$ Card | Origin$ Hand,Graveyard | \
         Destination$ Library | Shuffle$ True | Random$ True",
        "A:SP$ ChangeZoneAll | ChangeType$ Creature | Origin$ Battlefield | \
         Destination$ Library | LibraryPosition$ -1",
        "A:SP$ RearrangeTopOfLibrary | Defined$ You | NumCards$ X",
    ] {
        assert!(
            refused(&format!("Name:X\nTypes:Sorcery\n{refused_line}")),
            "{refused_line}"
        );
    }
}

/// Stone Giant: "toughness less than this creature's power" only where
/// `X` is the source's power, and "destroy that creature at the
/// beginning of the next end step" only on a targeted pump.
#[test]
fn a_comparison_with_the_sources_power_and_a_destroy_at_the_next_end_step() {
    let giant = read(
        "Name:X\nTypes:Creature\nPT:3/4\n\
         A:AB$ Pump | Cost$ T | ValidTgts$ Creature.YouCtrl+toughnessLTX | KW$ Flying | \
         AtEOT$ Destroy\n\
         SVar:X:Count$CardPower",
    );
    let a = giant.abilities.join("");
    assert!(
        a.contains(
            "Effect::AtNextEndStep { effects: &[Effect::destroy(TargetSpec::EventObject)] }"
        ),
        "{a}"
    );
    assert!(
        giant
            .statics
            .contains("Filter::ToughnessLessThanSourcePower"),
        "{}",
        giant.statics
    );
    for refused_card in [
        // `X` is not the source's power.
        "A:AB$ Pump | Cost$ T | ValidTgts$ Creature.toughnessLTX | KW$ Flying\n\
         SVar:X:Count$Valid Island.YouCtrl",
        // Another delayed sentence, and a delayed one about no target.
        "A:AB$ Pump | Cost$ T | ValidTgts$ Creature | KW$ Haste | AtEOT$ Sacrifice",
        "A:AB$ Pump | Cost$ R | Defined$ Self | NumAtt$ +1 | AtEOT$ Destroy",
    ] {
        assert!(
            refused(&format!("Name:X\nTypes:Creature\nPT:3/4\n{refused_card}")),
            "{refused_card}"
        );
    }
}

/// Stasis: "Players skip their untap steps" is every player's untap step,
/// from the battlefield; a plane's skip from the command zone, another
/// step, and a skip for one player refuse.
#[test]
fn a_skipped_untap_step_is_every_players() {
    let body = read(
        "Name:X\nTypes:Enchantment\n\
         R:Event$ BeginPhase | ActiveZones$ Battlefield | Phase$ Untap | Skip$ True | \
         Description$ Players skip their untap steps.",
    );
    assert_eq!(
        body.abilities,
        vec![
            "static_ability!(Filter::Any, Modifier::SkipUntapStep { who: PlayerRel::EachPlayer })"
        ]
    );
    for refused_line in [
        "R:Event$ BeginPhase | ActiveZones$ Command | Phase$ Untap | Skip$ True",
        "R:Event$ BeginPhase | Phase$ Draw | Skip$ True",
        "R:Event$ BeginPhase | Phase$ Untap | Skip$ True | ValidPlayer$ You",
    ] {
        assert!(
            refused(&format!("Name:X\nTypes:Enchantment\n{refused_line}")),
            "{refused_line}"
        );
    }
}

/// "Can attack as though it didn't have defender" and "…as though it
/// had haste", on the creatures the line names; naming what may be
/// attacked instead is another sentence and refuses.
#[test]
fn an_attack_as_though_is_a_permission_on_the_named_creatures() {
    let body = read(
        "Name:X\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ CanAttackIfHaste | ValidCard$ Creature.EnchantedBy | Description$ …",
    );
    assert!(
        body.abilities.iter().any(|a| a
            == "static_ability!(Filter::And(&[Filter::CREATURE, \
                Filter::AttachedToBySource]), Modifier::AttacksAsThoughHaste)"),
        "{:?}",
        body.abilities
    );
    let body = read(
        "Name:X\nTypes:Creature\nPT:0/4\nK:Defender\n\
         S:Mode$ CanAttackDefender | ValidCard$ Card.Self | Description$ …",
    );
    assert!(
        body.abilities
            .iter()
            .any(|a| a == "static_ability!(Filter::This, Modifier::AttacksDespiteDefender)"),
        "{:?}",
        body.abilities
    );
    assert!(refused(
        "Name:X\nTypes:Creature\nPT:1/1\n\
         S:Mode$ CanAttackIfHaste | ValidTarget$ Opponent | Description$ …"
    ));
}

/// "Enchanted land is a Swamp" and "target land becomes a Forest": CR
/// 305.7's setting of a land's subtype, from a static ability and from
/// an `Animate`, and nothing but one basic land type is read that way.
#[test]
fn a_land_set_to_a_basic_land_type_is_read_as_one() {
    let body = read(
        "Name:X\nTypes:Enchantment Aura\nK:Enchant:Land\n\
         S:Mode$ Continuous | Affected$ Card.EnchantedBy | AddType$ Mountain | \
         RemoveLandTypes$ True | Description$ …",
    );
    assert!(
        body.abilities.iter().any(|a| a
            == "static_ability!(Filter::AttachedToBySource, \
                Modifier::SetLandType(subtypes::land::MOUNTAIN))"),
        "{:?}",
        body.abilities
    );
    let body = read(
        "Name:X\nTypes:Creature\nPT:1/1\n\
         A:AB$ Animate | Cost$ T | ValidTgts$ Land | Types$ Forest | \
         RemoveLandTypes$ True | Duration$ UntilHostLeavesPlay | SpellDescription$ …",
    );
    assert!(
        body.abilities.iter().any(|a| a.contains(
            "Effect::continuous(&Filter::This, Modifier::SetLandType(subtypes::land::FOREST), \
             Duration::WhileSourceOnBattlefield)"
        )),
        "{:?}",
        body.abilities
    );
    let body = read(
        "Name:X\nTypes:Enchantment Aura\nK:Enchant:Land\n\
         K:ETBReplacement:Other:DBChooseBasic\n\
         SVar:DBChooseBasic:DB$ ChooseType | Type$ Basic Land | SpellDescription$ …\n\
         S:Mode$ Continuous | Affected$ Card.EnchantedBy | AddType$ ChosenType | \
         RemoveLandTypes$ True | Description$ …",
    );
    assert_eq!(body.enter_modifiers, ["EnterModifier::ChooseBasicLandType"]);
    assert!(
        body.abilities
            .iter()
            .any(|a| a
                == "static_ability!(Filter::AttachedToBySource, Modifier::SetLandTypeToChosen)"),
        "{:?}",
        body.abilities
    );
    // An unsupported as-enters type family stays refused, and choosing a
    // creature type cannot feed a basic-land-type `RemoveLandTypes$ True` rule.
    assert!(refused(
        "Name:X\nTypes:Enchantment\n\
         K:ETBReplacement:Other:DBChoose\n\
         SVar:DBChoose:DB$ ChooseType | Type$ NonBasic Land | SpellDescription$ …"
    ));
    assert!(refused(
        "Name:X\nTypes:Enchantment Aura\nK:Enchant:Land\n\
         K:ETBReplacement:Other:DBChoose\n\
         SVar:DBChoose:DB$ ChooseType | Type$ Creature | SpellDescription$ …\n\
         S:Mode$ Continuous | Affected$ Card.EnchantedBy | AddType$ ChosenType | \
         RemoveLandTypes$ True | Description$ …"
    ));
    // A chosen type nothing asked for as the card entered.
    for refused_line in [
        "S:Mode$ Continuous | Affected$ Card.EnchantedBy | AddType$ ChosenType | \
         RemoveLandTypes$ True | Description$ …",
        "S:Mode$ Continuous | Affected$ Card.EnchantedBy | AddType$ Desert | \
         RemoveLandTypes$ True | Description$ …",
        "S:Mode$ Continuous | Affected$ Card.EnchantedBy | AddType$ Island Swamp | \
         RemoveLandTypes$ True | Description$ …",
    ] {
        assert!(
            refused(&format!(
                "Name:X\nTypes:Enchantment Aura\nK:Enchant:Land\n{refused_line}"
            )),
            "{refused_line}"
        );
    }
}

/// Smoke's and Winter Orb's "players can't untap more than one …
/// during their untap steps": a limit on the players the line affects,
/// with the Orb's "as long as this is untapped" as the static's
/// condition. Another keyword beside it, another player, or a key the
/// reader does not claim refuses.
#[test]
fn an_untap_limit_is_read_for_the_players_it_names() {
    let body = read(
        "Name:X\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Player | AddKeyword$ UntapAdjust:Creature:1 | \
         Description$ Players can't untap more than one creature during their untap steps.",
    );
    assert_eq!(
        body.abilities,
        vec![
            "static_ability!(Filter::Any, Modifier::UntapAtMost { who: PlayerRel::EachPlayer, \
             of: &Filter::CREATURE, count: 1 })"
        ]
    );
    let body = read(
        "Name:X\nTypes:Artifact\n\
         S:Mode$ Continuous | Affected$ Player.Opponent | AddKeyword$ UntapAdjust:Land:2 | \
         IsPresent$ Card.Self+untapped | Description$ …",
    );
    assert_eq!(body.abilities.len(), 1);
    assert!(
        body.abilities[0].contains("who: PlayerRel::EachOpponent")
            && body.abilities[0].contains("count: 2")
            && body.abilities[0].contains("condition = Some(Condition::SourceMatches(&CHECK"),
        "{}",
        body.abilities[0]
    );
    for refused_line in [
        "S:Mode$ Continuous | Affected$ Creature | AddKeyword$ UntapAdjust:Land:1",
        "S:Mode$ Continuous | Affected$ Player | AddKeyword$ UntapAdjust:Land:one",
        "S:Mode$ Continuous | Affected$ Player | AddKeyword$ UntapAdjust:Land:1 | \
         AddHiddenKeyword$ Shroud",
    ] {
        assert!(
            refused(&format!("Name:X\nTypes:Enchantment\n{refused_line}")),
            "{refused_line}"
        );
    }
}

#[test]
fn bronze_horse_damage_reader_requires_exact_source_and_other_creature_predicates() {
    let script = "Name:X\nTypes:Artifact Creature Goblin\nPT:4/4\nR:Event$ DamageDone | Prevent$ True | ValidCause$ Spell.IsTargeting Self | ValidTarget$ Creature.Self | CauseIsSource$ True | IsPresent$ Creature.YouCtrl+Other";
    let body = read(script);
    assert!(body.abilities[0].contains("SpellTargetingAffected"));
    assert!(body.abilities[0].contains("Condition::ControlCount(&Filter::CREATURE, 2)"));
    assert!(!body.abilities[0].contains("Filter::Another"));
    for (from, to) in [
        ("Spell.IsTargeting Self", "Ability.IsTargeting Self"),
        ("CauseIsSource$ True", "CauseIsSource$ False"),
        ("Creature.YouCtrl+Other", "Creature.YouCtrl"),
        ("Creature.Self", "Creature.YouCtrl"),
    ] {
        assert!(refused(&script.replace(from, to)), "{to}");
    }
    assert!(refused(&format!("{script} | IsCombat$ True")));
}
#[test]
fn johan_reader_keeps_whole_optional_source_bound_combat_chain() {
    let script = "Name:X\nTypes:Creature Goblin\nPT:5/4\nT:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | Execute$ Stop | TriggerZones$ Battlefield | OptionalDecider$ You\nSVar:Stop:DB$ Pump | Defined$ Self | KW$ CARDNAME can't attack. | Duration$ UntilEndOfCombat | SubAbility$ Rule\nSVar:Rule:DB$ Effect | StaticAbilities$ Untapped | Duration$ UntilEndOfCombat\nSVar:Untapped:Mode$ AttackVigilance | ValidCard$ Creature.YouCtrl | IsPresent$ Card.EffectSource+untapped";
    let body = read(script);
    let ability = &body.abilities[0];
    assert!(ability.contains("Effect::MayDo"));
    assert!(ability.contains("KeywordSet::CANT_ATTACK"));
    assert!(ability.contains("AttackWithoutTappingWhileSourceUntappedThisCombat"));
    assert!(!ability.contains("KeywordSet::VIGILANCE"));
    for (from, to) in [
        ("ValidPlayer$ You", "ValidPlayer$ Opponent"),
        ("OptionalDecider$ You", "OptionalDecider$ Opponent"),
        ("UntilEndOfCombat", "UntilEndOfTurn"),
        ("Card.EffectSource+untapped", "Card.EffectSource+tapped"),
        ("Creature.YouCtrl", "Creature"),
    ] {
        assert!(refused(&script.replace(from, to)), "{to}");
    }
    assert!(refused(&format!("{script} | Unknown$ True")));
}

// Append to scriptgen/tests/effects.rs: synthetic structural fixtures, not corpus text.
#[test]
fn player_protection_life_lock_and_simultaneous_phase_chain_are_read_exactly() {
    let text = "Name:Boundary veil\nTypes:Instant\nManaCost:2 W\nA:SP$ Effect | Duration$ UntilYourNextTurn | StaticAbilities$ Lock | SubAbility$ Shield\nSVar:Lock:Mode$ CantChangeLife | ValidPlayer$ You\nSVar:Shield:DB$ Pump | Defined$ You | Duration$ UntilYourNextTurn | KW$ Protection from everything | SubAbility$ Out\nSVar:Out:DB$ Phases | Defined$ Valid Permanent.YouCtrl | SubAbility$ Leave\nSVar:Leave:DB$ ChangeZone | Defined$ Self | Origin$ Stack | Destination$ Exile\n";
    let body = read(text);
    let emitted = body.abilities.join("\n");
    for expected in [
        "Modifier::LifeTotalCantChange",
        "Modifier::PlayerProtectionFromEverything",
        "Effect::PhaseOutFilter(&Filter::ControlledByYou)",
        "Effect::ExileSource",
    ] {
        assert!(emitted.contains(expected), "missing {expected}: {emitted}");
    }
    let opp_lock = read(&text.replace("ValidPlayer$ You", "ValidPlayer$ Opponent"));
    assert!(
        opp_lock
            .abilities
            .join("\n")
            .contains("Modifier::LifeTotalCantChange { who: PlayerRel::EachOpponent }"),
        "{:?}",
        opp_lock.abilities
    );
    assert!(refused(
        &text.replace("ValidPlayer$ You", "ValidPlayer$ TriggeredPlayer")
    ));
    assert!(refused(
        &text.replace("Valid Permanent.YouCtrl", "Valid Permanent")
    ));
    assert!(refused(&text.replace(
        "KW$ Protection from everything",
        "KW$ Protection from everything | UnreadRule$ True"
    )));
    assert!(refused(
        &text.replace("UntilYourNextTurn", "UntilEndOfTurn")
    ));
}

#[test]
fn cast_self_entry_protection_and_source_counter_amount_do_not_mean_announced_x() {
    let text = "Name:Boundary ring\nTypes:Artifact\nManaCost:4\nT:Mode$ ChangesZone | ValidCard$ Card.wasCastByYou+Self | Destination$ Battlefield | Execute$ Shield\nSVar:Shield:DB$ Pump | Defined$ You | Duration$ UntilYourNextTurn | KW$ Protection from everything\nT:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ Loss | TriggerZones$ Battlefield\nSVar:Loss:DB$ LoseLife | LifeAmount$ X\nA:AB$ PutCounter | Cost$ T | Defined$ Self | CounterType$ BURDEN | CounterNum$ 1 | SubAbility$ Draw\nSVar:Draw:DB$ Draw | Defined$ You | NumCards$ X\nSVar:X:Count$CardCounters.BURDEN\n";
    let body = read(text);
    let emitted = body.abilities.join("\n");
    assert!(emitted.contains("Condition::SourceWasCastByYou"));
    assert!(emitted.contains("Amount::CountersOnSource"));
    assert!(!emitted.contains("Amount::X"));
    assert!(refused(&text.replace(
        "Card.wasCastByYou+Self",
        "Card.wasCastByYou+Self+Creature"
    )));
    assert!(refused(&text.replace(
        "CardCounters.BURDEN",
        "CardCounters.UNKNOWN_UNASSIGNED"
    )));
}

// Append to scriptgen/tests/effects.rs; original synthetic grammar fixtures.
fn gond_read(text: &str) -> Option<CardBody> {
    let mut catalog = cats();
    catalog.land.push("Gate".into());
    catalog.normalize();
    transcode(&parse(text), &catalog, None)
}
#[test]
fn gond_entry_reader_accepts_only_own_gate_battlefield_untap() {
    let text = "Name:Entry boundary\nTypes:Land Gate\nR:Event$ Moved | ValidCard$ Gate.YouCtrl | Destination$ Battlefield | ReplacementResult$ Updated | ActiveZones$ Battlefield | ReplaceWith$ Child\nSVar:Child:DB$ Untap | ETB$ True | Defined$ ReplacedCard\n";
    let body = gond_read(text).unwrap();
    assert!(body.abilities[0].contains("ReplacementRule::EnterUntapped"));
    for (from, to) in [
        ("Gate.YouCtrl", "Gate.OppCtrl"),
        ("ActiveZones$ Battlefield", "ActiveZones$ Graveyard"),
        ("ETB$ True", "ETB$ False"),
        ("ReplacedCard", "Self"),
        ("ReplaceWith$ Child", "ReplaceWith$ Child | Unknown$ True"),
    ] {
        assert!(gond_read(&text.replace(from, to)).is_none(), "{to}");
    }
}
#[test]
fn gond_reflection_is_color_only_own_gate_and_keeps_unknown_guards() {
    let text = "Name:Reflection boundary\nTypes:Land Gate\nA:AB$ ManaReflected | Cost$ T | ColorOrType$ Color | Valid$ Gate.YouCtrl | ReflectProperty$ Produce\n";
    let body = gond_read(text).unwrap();
    assert!(
        body.abilities[0]
            .contains("mana_from_lands(&Filter::HasSubtype(subtypes::land::GATE), true, false)")
    );
    let pool = gond_read(
        &text
            .replace("ColorOrType$ Color", "ColorOrType$ Type")
            .replace("Gate.YouCtrl", "Land.YouCtrl"),
    )
    .unwrap();
    assert!(pool.abilities[0].contains("mana_from_lands(&Filter::ControlledByYou, true, true)"));
    for (from, to) in [
        ("ColorOrType$ Color", "ColorOrType$ Any"),
        ("Gate.YouCtrl", "Creature.YouCtrl"),
        ("Gate.YouCtrl", "Gate.OppCtrl"),
        ("ReflectProperty$ Produce", "ReflectProperty$ Color"),
        ("Cost$ T", "Cost$ 1 T"),
        (
            "ReflectProperty$ Produce",
            "ReflectProperty$ Produce | Amount$ 2",
        ),
        (
            "ReflectProperty$ Produce",
            "ReflectProperty$ Produce | Unknown$ True",
        ),
    ] {
        assert!(gond_read(&text.replace(from, to)).is_none(), "{to}");
    }
}

// Append to scriptgen/tests/effects.rs. Synthetic grammar, no corpus script copied.
#[test]
fn graveyard_land_grant_has_exact_source_zone_and_refuses_unknowns() {
    let text = "Name:Buried boundary\nTypes:Land\nS:Mode$ Continuous | EffectZone$ Graveyard | Affected$ Land.YouCtrl | AddAbility$ Gift\nSVar:Gift:AB$ Mana | Cost$ T | Produced$ Combo G W\n";
    let body = read(text);
    assert!(body.abilities[0].contains("zone = ZoneRef::Graveyard"));
    assert!(body.abilities[0].contains("Modifier::GrantActivated"));
    for (from, to) in [
        ("EffectZone$ Graveyard", "EffectZone$ Exile"),
        ("EffectZone$ Graveyard", "EffectZone$ Hand"),
        ("EffectZone$ Graveyard", "EffectZone$ All"),
        ("Land.YouCtrl", "Land.OppCtrl"),
        ("AddAbility$ Gift", "AddAbility$ Gift | Unknown$ True"),
        ("Cost$ T", "Cost$ 1 T"),
        ("Combo G W", "Combo G W | Amount$ 2"),
        ("Combo G W", "Combo G W | Unknown$ True"),
    ] {
        assert!(refused(&text.replace(from, to)), "{to}");
    }
}

fn portals_body(text: &str) -> Option<CardBody> {
    let mut catalog = cats();
    catalog
        .land
        .extend(["Gate".into(), "Plains".into(), "Swamp".into()]);
    catalog.normalize();
    transcode(&parse(text), &catalog, None)
}
#[test]
fn remembered_white_mana_replacement_preserves_indefinite_incarnation_binding() {
    let script = "Name:X\nTypes:Creature Gnome\nManaCost:3 R\nPT:1/1\nA:AB$ Effect | Cost$ T | ValidTgts$ Plains | IsCurse$ True | RememberObjects$ Targeted | ForgetOnMoved$ Battlefield | Duration$ Permanent | ReplacementEffects$ Rewrite\nSVar:Rewrite:Event$ ProduceMana | ValidCard$ Card.IsRemembered | ReplaceWith$ Mana\nSVar:Mana:DB$ ReplaceMana | ReplaceColor$ C | ReplaceOnly$ W";
    // The shared tiny test catalog does not contain Gnome; this grammar does
    // not depend on the source's creature subtype.
    let script = script.replace("Types:Creature Gnome", "Types:Creature Goblin");
    let body = portals_body(&script).unwrap();
    let ability = body.abilities.join("");
    assert!(ability.contains("Modifier::ReplaceTappedMana"));
    assert!(ability.contains("Duration::Indefinitely"));
    for (from, to) in [
        ("ReplaceOnly$ W", "ReplaceOnly$ U"),
        ("Card.IsRemembered", "Card"),
        ("Duration$ Permanent", "Duration$ UntilEndOfTurn"),
        ("ForgetOnMoved$ Battlefield", "ForgetOnMoved$ Graveyard"),
    ] {
        assert!(portals_body(&script.replace(from, to)).is_none(), "{to}");
    }
}
#[test]
fn self_mana_life_tax_refuses_general_abilities_and_unproven_all_zone_semantics() {
    let script = "Name:X\nTypes:Land\nS:Mode$ RaiseCost | Type$ Ability | ValidSpell$ Activated.ManaAbility | ValidCard$ Card.Self | Cost$ PayLife<1>";
    assert!(portals_body(script).unwrap().abilities[0].contains("ManaAbilitiesCostLife(1)"));
    assert!(portals_body(&script.replace("Activated.ManaAbility", "Activated")).is_none());
    assert!(portals_body(&format!("{script} | EffectZone$ All")).is_none());
}

#[test]
fn thran_additive_chosen_type_requires_its_own_basic_land_choice() {
    let script = "Name:X\nTypes:Land Gate\nK:ETBReplacement:Other:Pick\nSVar:Pick:DB$ ChooseType | Type$ Basic Land\nS:Mode$ Continuous | Affected$ Card.Self | AddType$ ChosenType";
    let body = portals_body(script).unwrap();
    assert_eq!(body.enter_modifiers, ["EnterModifier::ChooseBasicLandType"]);
    assert_eq!(
        body.abilities,
        ["static_ability!(Filter::This, Modifier::AddChosenBasicLandType)"]
    );
    assert!(portals_body(&script.replace("Card.Self", "Land.YouCtrl")).is_none());
    assert!(portals_body(&script.replace("K:ETBReplacement:Other:Pick\n", "")).is_none());
    assert!(
        portals_body(&script.replace("AddType$ ChosenType", "AddType$ ChosenType | Unknown$ True"))
            .is_none()
    );
}
#[test]
fn wall_reader_keeps_blocked_source_and_wall_only_permission_distinct() {
    let damage = "Name:X\nTypes:Creature Goblin\nPT:0/1\nR:Event$ DamageDone | Prevent$ True | ValidSource$ Creature.blockedBySource | ValidTarget$ Creature.Self";
    assert!(read(damage).abilities[0].contains("PreventDamageFrom(&Filter::BlockedBySource)"));
    assert!(refused(&format!("{damage} | IsCombat$ True")));
    let permission = "Name:X\nTypes:Creature Goblin\nPT:0/1\nS:Mode$ CantTarget | ValidTarget$ Card.Self | SourceCanOnlyTarget$ Wall";
    assert!(read(permission).abilities[0].contains("CantBeTargetedByWallOnly"));
    assert!(refused(&permission.replace(
        "SourceCanOnlyTarget$ Wall",
        "SourceCanOnlyTarget$ Creature"
    )));
    assert!(refused(&format!("{permission} | ValidSource$ Spell")));
}
#[test]
fn guardian_reader_does_not_drop_hidden_control_or_new_attachment_predicate() {
    let rule = "Name:X\nTypes:Creature Goblin\nPT:2/4\nS:Mode$ Continuous | Affected$ Artifact.nonCreature+YouCtrl | AddKeyword$ Indestructible | AddHiddenKeyword$ Other players can't gain control of CARDNAME. | IsPresent$ Card.Self+untapped";
    let body = read(rule);
    assert_eq!(body.abilities.len(), 2);
    assert!(
        body.abilities
            .iter()
            .any(|a| a.contains("OtherPlayersCantGainControl"))
    );
    assert!(
        body.abilities
            .iter()
            .all(|a| a.contains("Condition::SourceMatches(&Filter::Untapped)"))
    );
    assert!(refused(
        &rule.replace("Card.Self+untapped", "Card.Self+tapped")
    ));
    let attach = "Name:X\nTypes:Creature Goblin\nPT:2/4\nS:Mode$ CantAttach | ValidCardToTarget$ Aura.!Attached | Target$ Artifact.nonCreature+YouCtrl | IsPresent$ Card.Self+untapped";
    assert!(read(attach).abilities[0].contains("Modifier::CantBecomeEnchanted"));
    for (from, to) in [
        ("Aura.!Attached", "Aura"),
        ("Artifact.nonCreature+YouCtrl", "Artifact.YouCtrl"),
        ("Card.Self+untapped", "Card.Self"),
    ] {
        assert!(refused(&attach.replace(from, to)), "{to}");
    }
}

#[test]
fn transform48_source_mana_trigger_preserves_event_mana_value_and_refuses_other_sources() {
    let script = "Name:X\nTypes:Land\nT:Mode$ SpellCast | ValidCard$ Permanent | ValidSA$ Spell.ManaFromCard.StrictlySelf | ValidActivatingPlayer$ You | Execute$ D\nSVar:D:DB$ Discover | Num$ TriggeredSpellAbility$CardManaCostLKI";
    let body = transcode(&parse(script), &cats(), None).unwrap();
    assert!(body.abilities[0].contains("SpellCastUsingSourceMana"));
    assert!(body.abilities[0].contains("Amount::EventManaValue"));
    for (from, to) in [
        ("StrictlySelf", "Other"),
        (
            "ValidActivatingPlayer$ You",
            "ValidActivatingPlayer$ Opponent",
        ),
        ("CardManaCostLKI", "CardManaCost"),
        (
            "Num$ TriggeredSpellAbility$CardManaCostLKI",
            "Num$ TriggeredSpellAbility$CardManaCostLKI | Unknown$ True",
        ),
    ] {
        assert!(
            transcode(&parse(&script.replace(from, to)), &cats(), None).is_none(),
            "{to}"
        );
    }
}
#[test]
fn transform48_first_main_requires_exact_optional_union_five_cost() {
    let script = "Name:X\nTypes:Artifact\nT:Mode$ Phase | Phase$ Main1 | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ F\nSVar:F:AB$ SetState | Cost$ tapXType<5/Artifact;Creature/artifact or creature> | Defined$ Self | Mode$ Transform";
    let body = transcode(&parse(script), &cats(), None).unwrap();
    assert!(body.abilities[0].contains("StepKind::FirstMain"));
    assert!(body.abilities[0].contains("CostPart::TapN"));
    for (from, to) in [
        ("tapXType<5/", "tapXType<4/"),
        ("Phase$ Main1", "Phase$ Main2"),
        ("Defined$ Self", "Defined$ Targeted"),
        ("Mode$ Transform", "Mode$ Transform | Unknown$ True"),
    ] {
        assert!(
            transcode(&parse(&script.replace(from, to)), &cats(), None).is_none(),
            "{to}"
        );
    }
}
#[test]
fn transform48_optional_discard_draw_chain_is_read_as_a_complete_sentence() {
    let script = "Name:X\nTypes:Artifact\nT:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ D\nSVar:D:DB$ Discard | AnyNumber$ True | Optional$ True | Mode$ TgtChoose | RememberDiscarded$ True | SubAbility$ N\nSVar:N:DB$ Draw | Defined$ You | NumCards$ Remembered$Amount/Plus.1 | SubAbility$ C\nSVar:C:DB$ Cleanup | ClearRemembered$ True";
    let body = transcode(&parse(script), &cats(), None).unwrap();
    assert!(body.abilities[0].contains("DiscardAnyThenDraw, Effect::draw(1)"));
    for (from, to) in [
        ("Optional$ True", "Optional$ False"),
        ("Plus.1", "Plus.2"),
        ("ClearRemembered$ True", "ClearRemembered$ False"),
        (
            "RememberDiscarded$ True",
            "RememberDiscarded$ True | Unknown$ True",
        ),
    ] {
        assert!(
            transcode(&parse(&script.replace(from, to)), &cats(), None).is_none(),
            "{to}"
        );
    }
}

#[test]
fn transform48_clue_sacrifice_condition_keeps_subtype_count_player_and_timing() {
    // Synthetic names around the verified property grammar; no corpus body copied.
    let script = "Name:Boundary laboratory\nTypes:Land\n\
        T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ Flip | CheckSVar$ Clues | SVarCompare$ GE3\n\
        SVar:Clues:PlayerCountPropertyYou$SacrificedThisTurn Clue\n\
        SVar:Flip:DB$ SetState | Defined$ Self | Mode$ Transform";
    let body = read(script);
    assert_eq!(body.abilities.len(), 1);
    assert!(body.abilities[0].contains("StepKind::End, whose: PlayerRel::You"));
    assert!(body.abilities[0].contains(
        "Condition::SacrificedSubtypeThisTurn { subtype: subtypes::artifact::CLUE, count: 3 }"
    ));
    for (from, to) in [
        ("SacrificedThisTurn Clue", "SacrificedThisTurn Food"),
        ("PropertyYou$", "PropertyOpponent$"),
        ("SVarCompare$ GE3", "SVarCompare$ GE2"),
        ("SVarCompare$ GE3", "SVarCompare$ EQ3"),
        ("ValidPlayer$ You", "ValidPlayer$ Opponent"),
        ("Phase$ End of Turn", "Phase$ Upkeep"),
        ("TriggerZones$ Battlefield", "TriggerZones$ Graveyard"),
        ("Defined$ Self", "Defined$ Targeted"),
        ("Mode$ Transform", "Mode$ Transform | Unknown$ True"),
        ("CheckSVar$ Clues", "CheckSVar$ Clues | Unknown$ True"),
    ] {
        assert!(refused(&script.replace(from, to)), "{to}");
    }
}

#[test]
fn deathtouch_counter_is_a_keyword_counter_not_an_assigned_custom_counter() {
    // Synthetic grammar fixture: verify the complete reader, not just its lookup.
    let script = "Name:Boundary fang\nTypes:Artifact\n\
        A:AB$ PutCounter | Cost$ T | Defined$ Self | CounterType$ Deathtouch | CounterNum$ 2";
    let body = read(script);
    assert_eq!(body.abilities.len(), 1);
    assert!(body.abilities[0].contains(
        "Effect::AddCounter { kind: CounterKind::Deathtouch, amount: Amount::Fixed(2) }"
    ));
    for unknown in ["DEATHTOUCH", "deathtouch", "Deathtouchs"] {
        assert!(refused(&script.replace("Deathtouch", unknown)), "{unknown}");
    }
    assert!(refused(&format!("{script} | UnknownRule$ True")));
}

#[test]
fn bore_counter_generic_reader_preserves_the_verified_exact_counter_word() {
    use super::super::constants::counter_kind;
    assert_eq!(counter_kind("BORE").as_deref(), Some("CounterKind::Bore"));
    for unknown in ["Bore", "bore", "BORES", "BORE.EXTRA"] {
        assert!(counter_kind(unknown).is_none(), "{unknown}");
    }
    // Generic activated-counter grammar rather than the transform reader's
    // specialized PutCounter/Branch chain.
    let script = "Name:Boundary drill\nTypes:Artifact\n\
        A:AB$ PutCounter | Cost$ T | Defined$ Self | CounterType$ BORE | CounterNum$ 1";
    let body = read(script);
    assert!(body.abilities[0].contains("CounterKind::Bore"));
    for wrong in ["Bore", "bore", "BORES"] {
        assert!(refused(&script.replace("BORE", wrong)), "{wrong}");
    }
}

#[test]
fn extravagant_spirit_food_chain_flailing_and_haunted_crossroads_shapes_read() {
    let spirit = "Name:Extravagant Spirit\nManaCost:3 U\nTypes:Creature Spirit\nPT:4/4\nK:Flying\n\
        T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigUpkeep\n\
        SVar:TrigUpkeep:DB$ Sacrifice | UnlessPayer$ You | UnlessCost$ Y\n\
        SVar:Y:Count$ValidHand Card.YouOwn";
    let body = read(spirit);
    assert!(
        body.abilities[0].contains("Amount::HandSize(PlayerRel::You)"),
        "{}",
        body.abilities[0]
    );

    let flailing = "Name:Flailing Soldier\nManaCost:R\nTypes:Creature Human Soldier\nPT:2/2\n\
        A:AB$ Pump | Cost$ 1 | Defined$ Self | NumAtt$ +1 | NumDef$ +1 | Activator$ Player";
    let body = read(flailing);
    assert!(
        body.abilities[0].contains("zone = ActivationZone::BattlefieldAnyPlayer"),
        "{}",
        body.abilities[0]
    );

    let food_chain = "Name:Food Chain\nManaCost:2 G\nTypes:Enchantment\n\
        A:AB$ Mana | Cost$ Exile<1/Creature> | Produced$ Any | Amount$ X | RestrictValid$ Spell.Creature\n\
        SVar:X:Exiled$CardManaCost/Plus.1";
    let body = read(food_chain);
    assert!(
        body.abilities[0].contains("ExilePermanent(&Filter::YOUR_CREATURE)")
            && body.abilities[0]
                .contains("Amount::Plus { base: &Amount::SacrificedManaValue, offset: 1 }"),
        "{}",
        body.abilities[0]
    );

    let crossroads = "Name:Haunted Crossroads\nManaCost:2 B\nTypes:Enchantment\n\
        A:AB$ ChangeZone | Cost$ B | ValidTgts$ Creature.YouCtrl | Origin$ Graveyard | Destination$ Library";
    let body = read(crossroads);
    assert!(
        body.abilities[0].contains("Effect::GraveyardToTop"),
        "{}",
        body.abilities[0]
    );

    let furious = "Name:Furious Assault\nManaCost:2 R\nTypes:Enchantment\n\
        T:Mode$ SpellCast | ValidCard$ Creature | ValidActivatingPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigDealDamage\n\
        SVar:TrigDealDamage:DB$ DealDamage | ValidTgts$ Player,Planeswalker | NumDmg$ 1";
    let body = read(furious);
    assert!(
        body.abilities[0].contains("TargetSpec::PlayerOrObject(&Filter::PLANESWALKER)"),
        "{}",
        body.abilities[0]
    );

    let high_seas = "Name:High Seas\nManaCost:2 U\nTypes:Enchantment\n\
        S:Mode$ RaiseCost | ValidCard$ Creature.Red,Creature.Green | Type$ Spell | Amount$ 1";
    let body = read(high_seas);
    assert!(
        body.abilities[0].contains("Modifier::SpellsCostMore(1)"),
        "{}",
        body.abilities[0]
    );

    assert!(refused(
        "Name:Hammer Mage\nManaCost:1 R\nTypes:Creature Human Spellshaper\nPT:1/1\n\
         A:AB$ DestroyAll | Cost$ X R T Discard<1/Card> | ValidCards$ Artifact.cmcLEX\n\
         SVar:X:Count$xPaid"
    ));
}

#[test]
fn jeweled_torque_karns_touch_cant_block_liability_muzzle_and_pangosaur_shapes_read() {
    let torque = "Name:Jeweled Torque\nManaCost:2\nTypes:Artifact\n\
        K:ETBReplacement:Other:ChooseColor\n\
        SVar:ChooseColor:DB$ ChooseColor | Defined$ You\n\
        T:Mode$ SpellCast | ValidCard$ Card.ChosenColor | TriggerZones$ Battlefield | Execute$ TrigGainLife\n\
        SVar:TrigGainLife:AB$ GainLife | Cost$ 2 | Defined$ You | LifeAmount$ 2";
    let body = read(torque);
    assert!(
        body.abilities[0].contains("PlayerMayPayThen")
            && body.abilities[0].contains("Amount::Fixed(2)"),
        "{}",
        body.abilities[0]
    );

    let touch = "Name:Karn's Touch\nManaCost:U U\nTypes:Instant\n\
        A:SP$ Animate | ValidTgts$ Artifact.nonCreature | Power$ X | Toughness$ X | Types$ Artifact,Creature\n\
        SVar:X:Targeted$CardManaCost";
    let body = read(touch);
    assert!(
        body.abilities[0].contains("Effect::SetPTFilter")
            && body.abilities[0].contains("Amount::TargetCmc"),
        "{}",
        body.abilities[0]
    );

    let veto = "Name:Magistrate's Veto\nManaCost:2 R\nTypes:Enchantment\n\
        S:Mode$ CantBlock | ValidCard$ Creature.White,Creature.Blue";
    let body = read(veto);
    assert!(
        body.abilities[0].contains("Modifier::AddKeyword(KeywordSet::CANT_BLOCK)"),
        "{}",
        body.abilities[0]
    );

    let liability = "Name:Liability\nManaCost:1 B B\nTypes:Enchantment\n\
        T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Permanent.nonToken | TriggerZones$ Battlefield | Execute$ TrigLoseLife\n\
        SVar:TrigLoseLife:DB$ LoseLife | Defined$ TriggeredCardOwner | LifeAmount$ 1";
    let body = read(liability);
    assert!(
        body.abilities[0].contains("PlayerRel::OwnerOfEvent"),
        "{}",
        body.abilities[0]
    );

    let muzzle = "Name:Muzzle\nManaCost:1 W\nTypes:Enchantment Aura\n\
        K:Enchant:Creature\n\
        R:Event$ DamageDone | ActiveZones$ Battlefield | Prevent$ True | ValidSource$ Creature.EnchantedBy";
    let body = read(muzzle);
    assert!(
        body.abilities[1].contains("Modifier::PreventAllDamageFromIt"),
        "{}",
        body.abilities[1]
    );

    let pangosaur = "Name:Pangosaur\nManaCost:2 G G\nTypes:Creature Dinosaur\nPT:6/6\n\
        T:Mode$ LandPlayed | ValidCard$ Land | TriggerZones$ Battlefield | Execute$ TrigBounce\n\
        SVar:TrigBounce:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand";
    let body = read(pangosaur);
    assert!(
        body.abilities[0].contains("Trigger::PlaysLand(PlayerRel::EachPlayer)"),
        "{}",
        body.abilities[0]
    );
}

#[test]
fn inviolability_natural_affinity_noble_purpose_pious_warrior_revered_elder_and_rishadan_pirate_shapes_read()
 {
    let inviolability = "Name:Inviolability\nManaCost:1 W\nTypes:Enchantment Aura\n\
        K:Enchant:Creature\n\
        R:Event$ DamageDone | ActiveZones$ Battlefield | Prevent$ True | ValidTarget$ Creature.EnchantedBy";
    let body = read(inviolability);
    assert!(
        body.abilities[1].contains("Modifier::PreventDamageFrom(&Filter::Any)"),
        "{}",
        body.abilities[1]
    );

    let natural_affinity = "Name:Natural Affinity\nManaCost:2 G\nTypes:Instant\n\
        A:SP$ AnimateAll | ValidCards$ Land | Types$ Creature | Power$ 2 | Toughness$ 2";
    let body = read(natural_affinity);
    assert!(
        body.abilities[0].contains("Modifier::AddType(TypeSet::CREATURE)")
            && body.abilities[0].contains("Modifier::SetPT(2, 2)"),
        "{}",
        body.abilities[0]
    );

    let noble_purpose = "Name:Noble Purpose\nManaCost:3 W W\nTypes:Enchantment\n\
        T:Mode$ DamageDealtOnce | CombatDamage$ True | ValidSource$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ TrigGain\n\
        SVar:TrigGain:DB$ GainLife | Defined$ You | LifeAmount$ X\n\
        SVar:X:TriggerCount$DamageAmount";
    let body = read(noble_purpose);
    assert!(
        body.abilities[0].contains("Trigger::DealsCombatDamage(&Filter::YOUR_CREATURE)")
            && body.abilities[0].contains("Effect::GainLife { amount: Amount::EventAmount }"),
        "{}",
        body.abilities[0]
    );

    let pious_warrior = "Name:Pious Warrior\nManaCost:3 W\nTypes:Creature Human Rebel Warrior\nPT:2/3\n\
        T:Mode$ DamageDoneOnce | CombatDamage$ True | ValidTarget$ Card.Self | Execute$ TrigGain\n\
        SVar:TrigGain:DB$ GainLife | Defined$ You | LifeAmount$ X\n\
        SVar:X:TriggerCount$DamageAmount";
    let body = read(pious_warrior);
    assert!(
        body.abilities[0].contains("Trigger::DealtCombatDamage(&Filter::This)")
            && body.abilities[0].contains("Effect::GainLife { amount: Amount::EventAmount }"),
        "{}",
        body.abilities[0]
    );

    let revered_elder = "Name:Revered Elder\nManaCost:2 W\nTypes:Creature Human Cleric\nPT:1/2\n\
        A:AB$ PreventDamage | Cost$ 1 | Defined$ Self | Amount$ 1";
    let body = read(revered_elder);
    assert!(
        body.abilities[0].contains("TargetSpec::ThisObject")
            && body.abilities[0].contains("Amount::Fixed(1)"),
        "{}",
        body.abilities[0]
    );

    let cutpurse = "Name:Rishadan Cutpurse\nManaCost:2 U\nTypes:Creature Human Pirate\nPT:1/1\n\
        T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigSac\n\
        SVar:TrigSac:DB$ RepeatEach | RepeatPlayers$ Player.Opponent | RepeatSubAbility$ DBSac\n\
        SVar:DBSac:DB$ Sacrifice | Defined$ Remembered | SacValid$ Permanent | UnlessCost$ 1 | UnlessPayer$ Remembered";
    let body = read(cutpurse);
    assert!(
        body.abilities[0].contains("PlayerRel::EachOpponent")
            && body.abilities[0].contains("PlayerRel::EventPlayer"),
        "{}",
        body.abilities[0]
    );
}

#[test]
fn pawnshop_heir_outrigger_scandalmonger_sever_soul_silent_assassin_and_fugitive_shapes_read() {
    let pawnshop = "Name:Rishadan Pawnshop\nManaCost:2\nTypes:Artifact\n\
        A:AB$ ChangeZone | Cost$ 2 T | Origin$ Battlefield | Destination$ Library | Shuffle$ True | ValidTgts$ Permanent.nonToken+YouCtrl";
    let body = read(pawnshop);
    assert!(
        body.abilities[0].contains("Effect::ShuffleTargetsIntoOwnersLibraries"),
        "{}",
        body.abilities[0]
    );

    let heir = "Name:Saprazzan Heir\nManaCost:1 U\nTypes:Creature Merfolk\nPT:1/1\n\
        T:Mode$ AttackerBlocked | ValidCard$ Card.Self | Execute$ TrigDraw\n\
        SVar:TrigDraw:DB$ Draw | NumCards$ 3 | OptionalDecider$ You";
    let body = read(heir);
    assert!(
        body.abilities[0].contains("Effect::MayDo { effects: &[Effect::draw(3)] }"),
        "{}",
        body.abilities[0]
    );

    let outrigger = "Name:Saprazzan Outrigger\nManaCost:3 U\nTypes:Creature Merfolk\nPT:5/5\n\
        T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigDelayed\n\
        T:Mode$ Blocks | ValidCard$ Card.Self | Secondary$ True | Execute$ TrigDelayed\n\
        SVar:TrigDelayed:DB$ DelayedTrigger | Mode$ Phase | Phase$ EndCombat | Execute$ TrigTop\n\
        SVar:TrigTop:DB$ ChangeZone | Origin$ Battlefield | Destination$ Library | LibraryPosition$ 0";
    let body = read(outrigger);
    assert_eq!(body.abilities.len(), 2);
    assert!(
        body.abilities[0].contains("Trigger::Attacks(&Filter::This)")
            && body.abilities[0].contains("Effect::PutSourceOnTopOfLibrary")
            && body.abilities[1].contains("Trigger::Blocks(&Filter::This)"),
        "{:?}",
        body.abilities
    );

    let scandalmonger = "Name:Scandalmonger\nManaCost:3 B\nTypes:Creature Boar Monger\nPT:3/3\n\
        A:AB$ Discard | Cost$ 2 | ValidTgts$ Player | NumCards$ 1 | Mode$ TgtChoose | IsCurse$ True | SorcerySpeed$ True | Activator$ Player";
    let body = read(scandalmonger);
    assert!(
        body.abilities[0].contains("timing = ActivationTiming::SorcerySpeed")
            && body.abilities[0].contains("zone = ActivationZone::BattlefieldAnyPlayer"),
        "{}",
        body.abilities[0]
    );

    let sever_soul = "Name:Sever Soul\nManaCost:3 B B\nTypes:Sorcery\n\
        A:SP$ Destroy | ValidTgts$ Creature.nonBlack | NoRegen$ True | SubAbility$ DBGain\n\
        SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ X\n\
        SVar:X:Targeted$CardToughness";
    let body = read(sever_soul);
    assert!(
        body.abilities[0].contains("Amount::TargetToughness"),
        "{}",
        body.abilities[0]
    );

    let silent_assassin = "Name:Silent Assassin\nManaCost:B B\nTypes:Creature Human Mercenary Assassin\nPT:2/1\n\
        A:AB$ Effect | Cost$ 3 B | ValidTgts$ Creature.blocking | RememberObjects$ Targeted | Triggers$ DelTrig\n\
        SVar:DelTrig:Mode$ Phase | Phase$ EndCombat | Execute$ TrigDestroy\n\
        SVar:TrigDestroy:DB$ Destroy | Defined$ Remembered";
    let body = read(silent_assassin);
    assert!(
        body.abilities[0].contains(
            "Effect::AtEndOfCombat { about: TargetSpec::Object(&TARGET1), effects: &[Effect::destroy(TargetSpec::EventObject)] }"
        ),
        "{}",
        body.abilities[0]
    );

    let fugitive = "Name:Skulking Fugitive\nManaCost:2 B\nTypes:Creature Horror Mercenary\nPT:3/4\n\
        T:Mode$ BecomesTarget | ValidTarget$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigSac\n\
        SVar:TrigSac:DB$ Sacrifice";
    let body = read(fugitive);
    assert!(
        body.abilities[0].contains("Trigger::BecomesTarget")
            && body.abilities[0].contains("Effect::SacrificeSelf"),
        "{}",
        body.abilities[0]
    );
}

#[test]
fn overtaker_squee_statecraft_and_territorial_dispute_shapes_read() {
    let overtaker = "Name:Overtaker\nManaCost:1 U\nTypes:Creature Merfolk Spellshaper\nPT:1/1\n\
        A:AB$ Untap | Cost$ 3 U T Discard<1/Card> | ValidTgts$ Creature | SubAbility$ DBChange\n\
        SVar:DBChange:DB$ GainControl | Defined$ Targeted | AddKWs$ Haste | LoseControl$ EOT";
    let body = read(overtaker);
    assert!(
        body.abilities[0].contains("Effect::UntapTarget")
            && body.abilities[0].contains(
                "Effect::continuous(&Filter::This, Modifier::GainControl, Duration::UntilEndOfTurn)"
            )
            && body.abilities[0].contains("keywords: KeywordSet::HASTE"),
        "{}",
        body.abilities[0]
    );

    let squee = "Name:Squee, Goblin Nabob\nManaCost:2 R\nTypes:Legendary Creature Goblin\nPT:1/1\n\
        T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Graveyard | OptionalDecider$ You | Execute$ TrigChange\n\
        SVar:TrigChange:DB$ ChangeZone | Origin$ Graveyard | Destination$ Hand | Defined$ Self";
    let body = read(squee);
    assert!(
        body.abilities[0].contains("zone = TriggerZone::Graveyard")
            && body.abilities[0].contains("Effect::bounce(TargetSpec::ThisObject)"),
        "{}",
        body.abilities[0]
    );

    let statecraft = "Name:Statecraft\nManaCost:3 U\nTypes:Enchantment\n\
        R:Event$ DamageDone | Prevent$ True | IsCombat$ True | ValidTarget$ Creature.YouCtrl | ActiveZones$ Battlefield\n\
        R:Event$ DamageDone | Prevent$ True | IsCombat$ True | ValidSource$ Creature.YouCtrl | ActiveZones$ Battlefield | Secondary$ True";
    let body = read(statecraft);
    assert_eq!(body.abilities.len(), 2);
    assert!(
        body.abilities[0].contains("Modifier::PreventDamageToIt")
            && body.abilities[1].contains("Modifier::PreventDamageFromIt"),
        "{:?}",
        body.abilities
    );

    let dispute = "Name:Territorial Dispute\nManaCost:4 R R\nTypes:Enchantment\n\
        T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigUpkeep\n\
        SVar:TrigUpkeep:DB$ Sacrifice | UnlessPayer$ You | UnlessCost$ Sac<1/Land>\n\
        S:Mode$ CantPlayLand | Description$ Players can't play lands.";
    let body = read(dispute);
    assert!(
        body.abilities
            .iter()
            .any(|a| a.contains("Modifier::PlayersCantPlay(&Filter::LAND)")),
        "{:?}",
        body.abilities
    );

    let warpath = "Name:Warpath\nManaCost:3 R\nTypes:Instant\n\
        A:SP$ DamageAll | NumDmg$ 3 | ValidCards$ Creature.blocked,Creature.blocking";
    let body = read(warpath);
    assert!(
        body.statics.contains("Filter::Blocked") && body.statics.contains("Filter::Blocking"),
        "{}",
        body.statics
    );
}

#[test]
fn prophecy_batch_3_patterns_generate() {
    let fiend = "Name:Plague Fiend\nManaCost:1 B\nTypes:Creature Insect\nPT:1/1\n\
        T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Creature | CombatDamage$ True | TriggerZones$ Battlefield | Execute$ TrigDestroy\n\
        SVar:TrigDestroy:DB$ Destroy | Defined$ TriggeredTargetLKICopy | UnlessCost$ 2 | UnlessPayer$ TriggeredTargetController";
    let body = read(fiend);
    assert!(
        body.abilities[0].contains("Trigger::DealsCombatDamageToCreature(&Filter::This)")
            && body.abilities[0].contains("PlayerRel::ControllerOfEvent")
            && body.abilities[0].contains("Effect::destroy(TargetSpec::EventObject)"),
        "{}",
        body.abilities[0]
    );

    let informer = "Name:Rebel Informer\nManaCost:2 B\nTypes:Creature Human Mercenary Rebel\nPT:1/2\n\
        S:Mode$ CantTarget | ValidTarget$ Card.Self | ValidSource$ Card.White\n\
        A:AB$ ChangeZone | Cost$ 3 | ValidTgts$ Goblin.!token | IsCurse$ True | Origin$ Battlefield | Destination$ Library | LibraryPosition$ -1";
    let body = read(informer);
    assert_eq!(body.abilities.len(), 2);
    assert!(
        body.abilities[0].contains("Modifier::CantBeTargetedBy(&SOURCE1)")
            && body.abilities[1].contains("Effect::PutTargetOnBottomOfLibrary"),
        "{:?}",
        body.abilities
    );

    let rethink = "Name:Rethink\nManaCost:2 U\nTypes:Instant\n\
        A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | UnlessCost$ X | UnlessPayer$ TargetedController\n\
        SVar:X:Targeted$CardManaCost";
    let body = read(rethink);
    assert!(
        body.abilities[0].contains("Amount::TargetCmc")
            && body.abilities[0].contains("Effect::CounterTargetSpell"),
        "{}",
        body.abilities[0]
    );

    let syphon = "Name:Rhystic Syphon\nManaCost:3 B B\nTypes:Sorcery\n\
        A:SP$ LoseLife | LifeAmount$ 5 | ValidTgts$ Player | SubAbility$ DBGainLife | UnlessCost$ 3 | UnlessPayer$ Targeted | UnlessResolveSubs$ WhenNotPaid\n\
        SVar:DBGainLife:DB$ GainLife | Defined$ You | LifeAmount$ 5";
    let body = read(syphon);
    assert!(
        body.abilities[0].contains("Effect::PlayerMayPayOr")
            && body.abilities[0].contains("Effect::Sequence(&[Effect::LoseLife")
            && body.abilities[0].contains("Effect::gain_life(5)"),
        "{}",
        body.abilities[0]
    );

    let snake = "Name:Ribbon Snake\nManaCost:1 U U\nTypes:Creature Snake\nPT:2/3\nK:Flying\n\
        A:AB$ Debuff | Cost$ 2 | Defined$ Self | Keywords$ Flying | Activator$ Player";
    let body = read(snake);
    assert!(
        body.abilities[0].contains("Modifier::RemoveKeyword(KeywordSet::FLYING)")
            && body.abilities[0].contains("zone = ActivationZone::BattlefieldAnyPlayer"),
        "{}",
        body.abilities[0]
    );

    let serpent = "Name:Shrouded Serpent\nManaCost:4 U U U\nTypes:Creature Serpent\nPT:4/4\n\
        T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigUnblockable\n\
        SVar:TrigUnblockable:DB$ Effect | UnlessCost$ 4 | UnlessPayer$ TriggeredDefendingPlayer | RememberObjects$ Self | ExileOnMoved$ Battlefield | StaticAbilities$ Unblockable\n\
        SVar:Unblockable:Mode$ CantBlockBy | ValidAttacker$ Card.IsRemembered";
    let body = read(serpent);
    assert!(
        body.abilities[0].contains("PlayerRel::DefendingPlayer")
            && body.abilities[0].contains("KeywordSet::UNBLOCKABLE"),
        "{}",
        body.abilities[0]
    );
}

#[test]
fn sacrifice_effect_accepts_change_num_one() {
    let beast = "Name:Thresher Beast\nManaCost:3 G G\nTypes:Creature Beast\nPT:4/4\n\
        T:Mode$ AttackerBlocked | ValidCard$ Card.Self | Execute$ TrigSac\n\
        SVar:TrigSac:DB$ Sacrifice | SacValid$ Land | Defined$ TriggeredDefendingPlayer | ChangeNum$ 1";
    let body = read(beast);
    assert_eq!(
        body.abilities,
        [
            "triggered!(Trigger::BecomesBlocked, &[Effect::SacrificeFilter { \
             who: PlayerRel::DefendingPlayer, filter: &Filter::LAND }])"
        ]
    );

    let two = parse(
        "Name:X\nManaCost:3 G G\nTypes:Creature Beast\nPT:4/4\n\
        T:Mode$ AttackerBlocked | ValidCard$ Card.Self | Execute$ TrigSac\n\
        SVar:TrigSac:DB$ Sacrifice | SacValid$ Land | Defined$ TriggeredDefendingPlayer | ChangeNum$ 2",
    );
    assert!(refusal_reason(&two, &cats(), None).is_some());
}

#[test]
fn invasion_batch_one_reader_patterns() {
    let leech = "Name:Alabaster Leech\nManaCost:W\nTypes:Creature Leech\nPT:1/3\n\
        S:Mode$ RaiseCost | ValidCard$ Card.White | Activator$ You | Type$ Spell | Cost$ W";
    let body = read(leech);
    assert_eq!(
        body.abilities,
        [
            "static_ability!(Filter::And(&[Filter::HasColor(ColorSet::from_slice(&[Color::White])), \
             Filter::ControlledByYou]), Modifier::SpellsCostMoreMana(mana!(\"{W}\")))"
        ]
    );

    let kavu = "Name:Ancient Kavu\nManaCost:3 R\nTypes:Creature Kavu\nPT:3/3\n\
        A:AB$ Animate | Cost$ 2 | Defined$ Self | Colors$ Colorless | OverwriteColors$ True";
    let body = read(kavu);
    assert_eq!(
        body.abilities,
        [
            "activated!(cost!(\"{2}\"), &[Effect::continuous(&Filter::This, \
             Modifier::SetColor(ColorSet::EMPTY), Duration::UntilEndOfTurn)])"
        ]
    );

    let guardian = "Name:Armored Guardian\nManaCost:3 W U\nTypes:Creature Cat Soldier\nPT:2/5\n\
        A:AB$ Protection | Cost$ 1 W W | ValidTgts$ Creature.YouCtrl | Gains$ Choice | Choices$ AnyColor";
    let body = read(guardian);
    assert_eq!(
        body.abilities,
        [
            "activated!(cost!(\"{1}{W}{W}\"), &[Effect::ProtectionFromChosenColor { \
             duration: Duration::UntilEndOfTurn }], target = Some(TargetSpec::Object(&Filter::YOUR_CREATURE)))"
        ]
    );

    let bind = "Name:Bind\nManaCost:1 G\nTypes:Instant\n\
        A:SP$ Counter | ValidTgts$ Card | TargetType$ Activated | SubAbility$ DBDraw\n\
        SVar:DBDraw:DB$ Draw";
    let body = read(bind);
    assert_eq!(
        body.abilities,
        ["spell!(&[Effect::CounterTargetAbility, Effect::draw(1)], \
             targets = Some(TargetReq::one(TargetSpec::AbilityOnStack(&TARGET1))))"]
    );
    assert!(
        body.statics
            .contains("static TARGET1: Filter = Filter::ActivatedAbilityFrom(&Filter::Any);")
    );

    let mongoose = "Name:Blurred Mongoose\nManaCost:1 G\nTypes:Creature Mongoose\nPT:2/1\n\
        R:Event$ Counter | ValidCard$ Card.Self | ValidSA$ Spell | Layer$ CantHappen\n\
        K:Shroud";
    let body = read(mongoose);
    assert_eq!(
        body.keywords,
        ["KeywordSet::SHROUD", "KeywordSet::UNCOUNTERABLE"]
    );
}

#[test]
fn invasion_batch_two_reader_patterns() {
    let shade = "Name:Cinder Shade\nManaCost:1 B R\nTypes:Creature Shade\nPT:1/1\n\
        A:AB$ DealDamage | Cost$ R Sac<1/CARDNAME> | ValidTgts$ Creature | NumDmg$ X\n\
        SVar:X:Sacrificed$CardPower";
    let body = read(shade);
    assert_eq!(
        body.abilities,
        [
            "activated!(cost!(\"{R}\", SacrificeSelf), &[Effect::DealDamage { amount: Amount::SourcePower, \
             target: TargetSpec::Object(&Filter::CREATURE) }], target = Some(TargetSpec::Object(&Filter::CREATURE)))"
        ]
    );

    let tears = "Name:Defiling Tears\nManaCost:2 B\nTypes:Instant\n\
        A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +1 | NumDef$ -1 | SubAbility$ DBAnimate\n\
        SVar:DBAnimate:DB$ Animate | Colors$ Black | OverwriteColors$ True | Abilities$ ABRegenerate | Defined$ Targeted\n\
        SVar:ABRegenerate:AB$ Regenerate | Cost$ B";
    let body = read(tears);
    assert_eq!(
        body.abilities,
        [
            "spell!(&[Effect::PumpTarget { power: Amount::Fixed(1), toughness: Amount::NegXFixed(1), \
             keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }, \
             Effect::continuous(&Filter::This, Modifier::SetColor(ColorSet::from_slice(&[Color::Black])), Duration::UntilEndOfTurn), \
             Effect::continuous(&Filter::This, Modifier::GrantActivated { cost: cost!(\"{B}\"), effects: &[Effect::regenerate(TargetSpec::ThisObject)], mana_ability: false }, Duration::UntilEndOfTurn)], \
             targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))"
        ]
    );

    let sisay = "Name:Captain Sisay\nManaCost:2 G W\nTypes:Legendary Creature Human Soldier\nPT:2/2\n\
        A:AB$ ChangeZone | Cost$ T | Origin$ Library | Destination$ Hand | ChangeType$ Legendary | ChangeNum$ 1";
    assert_eq!(
        read(sisay).abilities,
        [
            "activated!(Cost::TAP, &[Effect::SearchLibrary { filter: &SEARCH1, finds: &[Find::HAND], optional: false }])"
        ]
    );

    let cremate = "Name:Cremate\nManaCost:B\nTypes:Instant\n\
        A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Card | SubAbility$ DBDraw\n\
        SVar:DBDraw:DB$ Draw";
    assert_eq!(
        read(cremate).abilities,
        [
            "spell!(&[Effect::exile(TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::EachPlayer)), Effect::draw(1)], \
             targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::EachPlayer))))"
        ]
    );

    let acolyte = "Name:Crimson Acolyte\nManaCost:1 W\nTypes:Creature Human Cleric\nPT:1/1\n\
        K:Protection from red\n\
        A:AB$ Protection | Cost$ W | ValidTgts$ Creature | Gains$ red";
    assert_eq!(
        read(acolyte).abilities,
        [
            "static_ability!(Filter::This, Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[Color::Red]))))",
            "activated!(cost!(\"{W}\"), &[Effect::continuous(&Filter::This, Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[Color::Red]))), Duration::UntilEndOfTurn)], \
             target = Some(TargetSpec::Object(&Filter::CREATURE)))"
        ]
    );

    let knight = "Name:Crusading Knight\nManaCost:2 W W\nTypes:Creature Human Knight\nPT:2/2\n\
        K:Protection from black\n\
        S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ X | AddToughness$ X\n\
        SVar:X:Count$Valid Forest.OppCtrl";
    assert_eq!(
        read(knight).abilities,
        [
            "static_ability!(Filter::This, Modifier::ProtectionFrom(&Filter::HasColor(ColorSet::from_slice(&[Color::Black]))))",
            "static_ability!(Filter::This, Modifier::ModifyPTByCount(PtCount::OnBattlefield(&COUNTED1)))"
        ]
    );

    let wake = "Name:Distorting Wake\nManaCost:X U U U\nTypes:Sorcery\n\
        A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Hand | TargetMin$ X | TargetMax$ X | ValidTgts$ Permanent.nonLand\n\
        SVar:X:Count$xPaid";
    assert_eq!(
        read(wake).abilities,
        [
            "spell!(&[Effect::bounce(TargetSpec::Object(&Filter::NONLAND))], \
             targets = Some(TargetReq::x_targets(TargetSpec::Object(&Filter::NONLAND))))"
        ]
    );
}

#[test]
fn invasion_batch_three_reader_patterns() {
    let ranger = "Name:Firebrand Ranger\nManaCost:1 R\nTypes:Creature Human Soldier\nPT:2/1\n\
        A:AB$ ChangeZone | Cost$ G T | Origin$ Hand | Destination$ Battlefield | ChangeType$ Land.Basic | Optional$ You";
    assert_eq!(
        read(ranger).abilities,
        [
            "activated!(cost!(\"{G}\", TapSelf), &[Effect::PutFromHandOntoBattlefield { tapped: false, filter: &PUT_FROM_HAND1, mana_value: None, optional: true }])"
        ]
    );

    let tilling = "Name:Frenzied Tilling\nManaCost:3 R G\nTypes:Sorcery\n\
        A:SP$ Destroy | ValidTgts$ Land | SubAbility$ DBSearch\n\
        SVar:DBSearch:DB$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeType$ Land.Basic | Tapped$ True";
    assert_eq!(
        read(tilling).abilities,
        [
            "spell!(&[Effect::destroy(TargetSpec::Object(&Filter::LAND)), Effect::SearchLibrary { filter: &SEARCH1, finds: &[Find::BATTLEFIELD_TAPPED], optional: false }], \
             targets = Some(TargetReq::one(TargetSpec::Object(&Filter::LAND))))"
        ]
    );

    let kavu = "Name:Hunting Kavu\nManaCost:1 R G\nTypes:Creature\nPT:2/3\n\
        A:AB$ ChangeZone | Cost$ 1 R G T | ValidTgts$ Creature.withoutFlying+attackingYou | ThisDefinedAndTgts$ Self | Origin$ Battlefield | Destination$ Exile";
    assert_eq!(
        read(kavu).abilities,
        [
            "activated!(cost!(\"{1}{R}{G}\", TapSelf), &[Effect::ExileSource, Effect::exile(TargetSpec::Object(&TARGET1))], \
             target = Some(TargetSpec::Object(&TARGET1)))"
        ]
    );

    let chameleon = "Name:Kavu Chameleon\nManaCost:3 G G\nTypes:Creature\nPT:4/4\n\
        A:AB$ ChooseColor | Cost$ G | Defined$ You | SubAbility$ Animate\n\
        SVar:Animate:DB$ Animate | Defined$ Self | Colors$ ChosenColor | OverwriteColors$ True";
    assert_eq!(
        read(chameleon).abilities,
        [
            "activated!(cost!(\"{G}\"), &[Effect::ChooseColors { subject: ColorSubject::Source, max: 1, duration: Duration::UntilEndOfTurn }])"
        ]
    );
}

#[test]
fn invasion_batch_four_reader_patterns() {
    let delver = "Name:Phyrexian Delver\nManaCost:3 B B\nTypes:Creature Phyrexian Zombie\nPT:3/2\n\
        T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigChangeZone\n\
        SVar:TrigChangeZone:DB$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature.YouCtrl | GainControl$ True | ChangeNum$ 1 | SubAbility$ DBLoseLifeYou\n\
        SVar:DBLoseLifeYou:DB$ LoseLife | Defined$ You | LifeAmount$ X\n\
        SVar:X:Targeted$CardManaCost";
    assert_eq!(
        read(delver).abilities,
        [
            "triggered!(Trigger::ETB, &[Effect::GraveyardToBattlefield { target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You), owner_control: false, counters: None }, Effect::LoseLife { amount: Amount::TargetCmc, target: PlayerRel::You }], targets = Some(TargetReq::one(TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You))))"
        ]
    );
}

#[test]
fn invasion_batch_five_reader_patterns() {
    let rewards = "Name:Rewards of Diversity\nManaCost:2 W\nTypes:Enchantment\n\
        T:Mode$ SpellCast | ValidCard$ Card.MultiColor | ValidActivatingPlayer$ Opponent | TriggerZones$ Battlefield | Execute$ TrigGainLife\n\
        SVar:TrigGainLife:DB$ GainLife | Defined$ You | LifeAmount$ 4";
    assert_eq!(
        read(rewards).abilities,
        ["triggered!(Trigger::SpellCast(&TRIGGER1), &[Effect::gain_life(4)])"]
    );

    let rogue = "Name:Rogue Kavu\nManaCost:1 R\nTypes:Creature\nPT:1/1\n\
        T:Mode$ Attacks | ValidCard$ Card.Self | Alone$ True | Execute$ TrigPump\n\
        SVar:TrigPump:DB$ Pump | Defined$ Self | NumAtt$ +2";
    assert_eq!(
        read(rogue).abilities,
        [
            "triggered!(Trigger::AttacksAlone(&Filter::This), &[Effect::PumpFilter { filter: &Filter::This, controlled_by: None, power: Amount::Fixed(2), toughness: Amount::Fixed(0), keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }])"
        ]
    );
}

#[test]
fn invasion_batch_six_reader_patterns() {
    let skittish = "Name:Skittish Kavu\nManaCost:1 R\nTypes:Creature\nPT:1/1\n\
        S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ 1 | AddToughness$ 1 | CheckSVar$ X | SVarCompare$ EQ0\n\
        SVar:X:Count$Valid Creature.White+OppCtrl/Plus.Y\n\
        SVar:Y:Count$Valid Creature.Blue+OppCtrl";
    assert_eq!(
        read(skittish).abilities,
        [
            "static_ability!(Filter::This, Modifier::ModifyPT(1, 1), condition = Some(Condition::BattlefieldCountAtMost(&CHECK1, 0)))"
        ]
    );

    let sparring = "Name:Sparring Golem\nManaCost:3\nTypes:Artifact Creature\nPT:2/2\n\
        T:Mode$ AttackerBlocked | ValidCard$ Card.Self | Execute$ TrigPump\n\
        SVar:TrigPump:DB$ Pump | Defined$ Self | NumAtt$ +X | NumDef$ +X\n\
        SVar:X:Count$Valid Creature.blockingTriggeredAttacker";
    assert_eq!(
        read(sparring).abilities,
        [
            "triggered!(Trigger::BecomesBlocked, &[Effect::PumpFilter { filter: &Filter::This, controlled_by: None, power: Amount::CountOf { filter: &COUNT1, zone: ZoneSel::Battlefield }, toughness: Amount::CountOf { filter: &COUNT1, zone: ZoneSel::Battlefield }, keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }])"
        ]
    );
}

#[test]
fn invasion_batch_seven_reader_patterns() {
    let moat = "Name:Teferi's Moat\nManaCost:3 W U\nTypes:Enchantment\n\
        K:ETBReplacement:Other:ChooseColor\n\
        SVar:ChooseColor:DB$ ChooseColor | Defined$ You\n\
        S:Mode$ CantAttack | ValidCard$ Creature.withoutFlying+ChosenColor | Target$ You";
    let moat_body = read(moat);
    assert_eq!(moat_body.enter_modifiers, ["EnterModifier::ChooseColor"]);
    assert_eq!(
        moat_body.abilities,
        [
            "static_ability!(Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::HasKeyword(KeywordSet::FLYING)), Filter::OfSourceChosenColor]), Modifier::CantAttackYou)"
        ]
    );

    let undermine = "Name:Undermine\nManaCost:U U B\nTypes:Instant\n\
        A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | SubAbility$ DBDrain | Destination$ Graveyard\n\
        SVar:DBDrain:DB$ LoseLife | Defined$ TargetedController | LifeAmount$ 3";
    assert_eq!(
        read(undermine).abilities,
        [
            "spell!(&[Effect::CounterTargetSpell, Effect::LoseLife { amount: Amount::Fixed(3), target: PlayerRel::ControllerOfTarget }], targets = Some(TargetReq::one(TargetSpec::Spell(&Filter::Any))))"
        ]
    );
}

#[test]
fn planeshift_batch_one_reader_patterns() {
    let destructive_flow = "Name:Destructive Flow\nManaCost:B R G\nTypes:Enchantment\n\
        T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | TriggerZones$ Battlefield | Execute$ TrigSac\n\
        SVar:TrigSac:DB$ Sacrifice | SacValid$ Land.nonBasic | Defined$ TriggeredPlayer | SacMessage$ nonbasic land";
    assert_eq!(
        read(destructive_flow).abilities,
        [
            "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::EachPlayer }, &[Effect::SacrificeFilter { who: PlayerRel::ActivePlayer, filter: &Filter::NONBASIC_LAND }])"
        ]
    );

    let ertai = "Name:Ertai, the Corrupted\nManaCost:2 W U B\nTypes:Legendary Creature Phyrexian Human Wizard\nPT:3/4\n\
        A:AB$ Counter | Cost$ U T Sac<1/Creature;Enchantment/creature or enchantment> | TargetType$ Spell | TgtPrompt$ Select target spell | ValidTgts$ Card";
    assert_eq!(
        read(ertai).abilities,
        [
            "activated!(cost!(\"{U}\", TapSelf, Sacrifice(&COST1)), &[Effect::CounterTargetSpell], target = Some(TargetSpec::Spell(&Filter::Any)))"
        ]
    );
}

#[test]
fn planeshift_batch_two_reader_patterns() {
    let hobble = "Name:Hobble\nManaCost:2 W\nTypes:Enchantment Aura\n\
        K:Enchant:Creature\n\
        T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw\n\
        SVar:TrigDraw:DB$ Draw | Defined$ You | NumCards$ 1\n\
        S:Mode$ CantAttack | ValidCard$ Creature.EnchantedBy\n\
        S:Mode$ CantBlock | ValidCard$ Creature.EnchantedBy+Black";
    assert_eq!(
        read(hobble).abilities,
        [
            "spell!(&[Effect::AttachSelf { target: TargetSpec::Object(&Filter::CREATURE) }], targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))",
            "triggered!(Trigger::ETB, &[Effect::draw(1)])",
            "static_ability!(Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]), Modifier::AddKeyword(KeywordSet::CANT_ATTACK))",
            "static_ability!(Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource, Filter::HasColor(ColorSet::from_slice(&[Color::Black]))]), Modifier::AddKeyword(KeywordSet::CANT_BLOCK))",
        ]
    );

    let mogg_jailer = "Name:Mogg Jailer\nManaCost:1 R\nTypes:Creature Goblin\nPT:2/2\n\
        S:Mode$ CantAttack | ValidCard$ Card.Self | UnlessDefender$ !controlsCreature.untapped+powerLE2";
    assert_eq!(
        read(mogg_jailer).abilities,
        [
            "static_ability!(Filter::This, Modifier::CantAttackIfDefenderControls(&DEFENDER_CONTROLS1))"
        ]
    );

    let bloodstock = "Name:Phyrexian Bloodstock\nManaCost:4 B\nTypes:Creature Phyrexian Zombie\nPT:3/3\n\
        T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | ValidCard$ Card.Self | Execute$ TrigDestroy\n\
        SVar:TrigDestroy:DB$ Destroy | ValidTgts$ Creature.White | NoRegen$ True";
    assert_eq!(
        read(bloodstock).abilities,
        [
            "triggered!(Trigger::LeavesBattlefield(&Filter::This), &[Effect::destroy_no_regen(TargetSpec::Object(&TARGET1))], targets = Some(TargetReq::one(TargetSpec::Object(&TARGET1))))"
        ]
    );

    let tyranny = "Name:Phyrexian Tyranny\nManaCost:U B R\nTypes:Enchantment\n\
        T:Mode$ Drawn | ValidCard$ Card | TriggerZones$ Battlefield | Execute$ TrigLose\n\
        SVar:TrigLose:DB$ LoseLife | Defined$ TriggeredCardController | LifeAmount$ 2 | UnlessCost$ 2 | UnlessPayer$ TriggeredCardController";
    assert_eq!(
        read(tyranny).abilities,
        [
            "triggered!(Trigger::Draws(PlayerRel::EachPlayer), &[Effect::PlayerMayPayOr { player: PlayerRel::EventPlayer, mana: Amount::Fixed(2), effect: &Effect::LoseLife { amount: Amount::Fixed(2), target: PlayerRel::EventPlayer } }])"
        ]
    );
}

#[test]
fn planeshift_batch_three_and_four_reader_patterns() {
    let pilgrim = "Name:Samite Pilgrim\nManaCost:1 W\nTypes:Creature Human Cleric\nPT:1/1\n\
        A:AB$ PreventDamage | Cost$ T | ValidTgts$ Creature | Amount$ X | PrecostDesc$ Domain —\n\
        SVar:X:Count$Domain";
    assert_eq!(
        read(pilgrim).abilities,
        [
            "activated!(Cost::TAP, &[Effect::PreventNextDamage { target: TargetSpec::Object(&Filter::CREATURE), amount: Amount::BasicLandTypesAmong(&Filter::YOUR_LAND) }], target = Some(TargetSpec::Object(&Filter::CREATURE)))"
        ]
    );

    let explorer = "Name:Quirion Explorer\nManaCost:1 G\nTypes:Creature Elf Druid Scout\nPT:1/1\n\
        A:AB$ ManaReflected | Cost$ T | ColorOrType$ Color | Valid$ Land.OppCtrl | ReflectProperty$ Produce";
    assert_eq!(
        read(explorer).abilities,
        ["mana_ability!(&[Effect::mana_from_lands(&Filter::ControlledByOpponent, false, false)])"]
    );

    let compass = "Name:Star Compass\nManaCost:2\nTypes:Artifact\n\
        A:AB$ ManaReflected | Cost$ T | ColorOrType$ Color | Valid$ Basic.YouCtrl | ReflectProperty$ Produce";
    assert_eq!(
        read(compass).abilities,
        ["mana_ability!(&[Effect::mana_from_lands(&Filter::BASIC_LAND, true, false)])"]
    );

    let radiant_kavu = "Name:Radiant Kavu\nManaCost:R G W\nTypes:Creature Kavu\nPT:3/3\n\
        A:AB$ Effect | Cost$ R G W | ReplacementEffects$ RPrevent\n\
        SVar:RPrevent:Event$ DamageDone | Prevent$ True | IsCombat$ True | ActiveZones$ Command | ValidSource$ Creature.Blue,Creature.Black";
    assert_eq!(
        read(radiant_kavu).abilities,
        [
            "activated!(cost!(\"{R}{G}{W}\"), &[Effect::continuous(&EFFECT1, Modifier::PreventDamageFromIt, Duration::UntilEndOfTurn)])"
        ]
    );

    let familiar = "Name:Stormscape Familiar\nManaCost:1 U\nTypes:Creature Bird\nPT:1/1\n\
        S:Mode$ ReduceCost | ValidCard$ Card.White,Card.Black | Type$ Spell | Activator$ You | Amount$ 1";
    assert_eq!(
        read(familiar).abilities,
        [
            "static_ability!(Filter::Or(&[Filter::And(&[Filter::HasColor(ColorSet::from_slice(&[Color::White])), Filter::ControlledByYou]), Filter::And(&[Filter::HasColor(ColorSet::from_slice(&[Color::Black])), Filter::ControlledByYou])]), Modifier::SpellsCostLess(1))"
        ]
    );

    let sunken_hope = "Name:Sunken Hope\nManaCost:3 U U\nTypes:Enchantment\n\
        T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | TriggerZones$ Battlefield | Execute$ TrigChangeZoneYou\n\
        SVar:TrigChangeZoneYou:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand | Hidden$ True | Mandatory$ True | ChangeType$ Creature | DefinedPlayer$ TriggeredPlayer | ChangeNum$ 1";
    assert_eq!(
        read(sunken_hope).abilities,
        [
            "triggered!(Trigger::StepBegin { step: StepKind::Upkeep, whose: PlayerRel::EachPlayer }, &[Effect::ReturnChosenToHand { who: PlayerRel::ActivePlayer, filter: &Filter::CREATURE }])"
        ]
    );
}

#[test]
fn apocalypse_reader_patterns() {
    let divine_light = "Name:Divine Light\nManaCost:W\nTypes:Sorcery\n\
        A:SP$ Effect | ReplacementEffects$ RPrevent\n\
        SVar:RPrevent:Event$ DamageDone | Prevent$ True | ActiveZones$ Command | ValidTarget$ Creature.YouCtrl";
    assert_eq!(
        read(divine_light).abilities,
        [
            "spell!(&[Effect::continuous(&Filter::YOUR_CREATURE, Modifier::PreventDamageFrom(&Filter::Any), Duration::UntilEndOfTurn)])"
        ]
    );

    let consume_strength = "Name:Consume Strength\nManaCost:1 B G\nTypes:Instant\n\
        A:SP$ Pump | ValidTgts$ Creature | TargetUnique$ True | NumAtt$ +2 | NumDef$ +2 | SubAbility$ DBPumpNeg\n\
        SVar:DBPumpNeg:DB$ Pump | ValidTgts$ Creature | TargetUnique$ True | NumAtt$ -2 | NumDef$ -2 | IsCurse$ True";
    let body = read(consume_strength);
    assert!(
        body.statics
            .contains("Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::Reference)])"),
        "{}",
        body.statics
    );
    assert!(
        body.abilities[0].contains("second_targets = Some(TargetReq::one("),
        "{}",
        body.abilities[0]
    );

    let bloodfire_infusion = "Name:Bloodfire Infusion\nManaCost:2 R\nTypes:Enchantment Aura\n\
        K:Enchant:Creature.YouCtrl\n\
        A:AB$ DamageAll | Cost$ R Sac<1/Enchanted> | ValidCards$ Creature | NumDmg$ X | ActivationZone$ Battlefield\n\
        SVar:X:Sacrificed$CardPower";
    assert!(read(bloodfire_infusion).abilities[1].contains(
        "Effect::DealDamageEach { amount: Amount::SacrificedPower, filter: &Filter::CREATURE }"
    ),);

    let planar_despair = "Name:Planar Despair\nManaCost:3 B B\nTypes:Sorcery\n\
        A:SP$ PumpAll | ValidCards$ Creature | NumAtt$ -X | NumDef$ -X | IsCurse$ True\n\
        SVar:X:Count$Domain";
    assert_eq!(
        read(planar_despair).abilities,
        [
            "spell!(&[Effect::PumpFilter { filter: &Filter::CREATURE, controlled_by: None, power: Amount::Negated(&Amount::BasicLandTypesAmong(&Filter::YOUR_LAND)), toughness: Amount::Negated(&Amount::BasicLandTypesAmong(&Filter::YOUR_LAND)), keywords: KeywordSet::EMPTY, duration: Duration::UntilEndOfTurn }])"
        ]
    );

    let urborg_uprising = "Name:Urborg Uprising\nManaCost:4 B\nTypes:Sorcery\n\
        A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | TargetMin$ 0 | TargetMax$ 2 | ValidTgts$ Creature.YouCtrl | SubAbility$ DBDraw\n\
        SVar:DBDraw:DB$ Draw | NumCards$ 1";
    assert_eq!(
        read(urborg_uprising).abilities,
        [
            "spell!(&[Effect::GraveyardToHand { target: TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You) }, Effect::draw(1)], targets = Some(TargetReq::up_to(TargetSpec::CardInGraveyard(&Filter::CREATURE, PlayerRel::You), 2)))"
        ]
    );
}

#[test]
fn odyssey_reader_patterns() {
    let horror = "Name:Childhood Horror\nManaCost:3 B\nTypes:Creature\nPT:2/2\n\
        S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ 2 | AddToughness$ 2 | Condition$ Threshold\n\
        S:Mode$ CantBlock | ValidCard$ Card.Self | Condition$ Threshold | Secondary$ True";
    assert_eq!(
        read(horror).abilities,
        [
            "static_ability!(Filter::This, Modifier::ModifyPT(2, 2), condition = Some(Condition::GraveyardCountAtLeast(7)))",
            "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::CANT_BLOCK), condition = Some(Condition::GraveyardCountAtLeast(7)))",
        ]
    );

    let basilisk = "Name:Stone-Tongue Basilisk\nManaCost:4 G G G\nTypes:Creature\nPT:4/5\n\
        S:Mode$ Continuous | AffectedDefined$ Self | AddHiddenKeyword$ All creatures able to block CARDNAME do so. | Condition$ Threshold\n\
        T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Creature | CombatDamage$ True | TriggerZones$ Battlefield | Execute$ DelTrig\n\
        SVar:DelTrig:DB$ DelayedTrigger | Mode$ Phase | Phase$ EndCombat | ValidPlayer$ Player | Execute$ TrigDestroy | RememberObjects$ TriggeredTargetLKICopy\n\
        SVar:TrigDestroy:DB$ Destroy | Defined$ DelayTriggerRememberedLKI";
    assert_eq!(
        read(basilisk).abilities,
        [
            "static_ability!(Filter::This, Modifier::MustBeBlockedByAllAble, condition = Some(Condition::GraveyardCountAtLeast(7)))",
            "triggered!(Trigger::DealsCombatDamageToCreature(&Filter::This), &[Effect::AtEndOfCombat { about: TargetSpec::EventObject, effects: &[Effect::destroy(TargetSpec::EventObject)] }])",
        ]
    );

    let dragon = "Name:Vampiric Dragon\nManaCost:6 B R\nTypes:Creature\nPT:5/5\n\
        T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Creature.DamagedBy | TriggerZones$ Battlefield | Execute$ TrigPutCounter\n\
        SVar:TrigPutCounter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1";
    assert_eq!(
        read(dragon).abilities,
        [
            "triggered!(Trigger::DiesAfterDamageByThis(&Filter::CREATURE), &[Effect::AddCounter { kind: CounterKind::P1P1, amount: Amount::Fixed(1) }])",
        ]
    );

    let epicenter = "Name:Epicenter\nManaCost:4 R\nTypes:Sorcery\n\
        A:SP$ Branch | ValidTgts$ Player | BranchConditionSVar$ X | BranchConditionSVarCompare$ GE7 | TrueSubAbility$ ThreshWaste | FalseSubAbility$ WasteLand\n\
        SVar:WasteLand:DB$ Sacrifice | Defined$ Targeted | SacValid$ Land\n\
        SVar:ThreshWaste:DB$ SacrificeAll | ValidCards$ Land\n\
        SVar:X:Count$ValidGraveyard Card.YouOwn";
    assert_eq!(
        read(epicenter).abilities,
        [
            "spell!(&[Effect::IfCondition { condition: Condition::GraveyardCountAtLeast(7), then: &[Effect::SacrificeAll { filter: &Filter::LAND }], otherwise: &[Effect::SacrificeFilter { who: PlayerRel::Chosen, filter: &Filter::LAND }] }], targets = Some(TargetReq::one(TargetSpec::AnyPlayer)))",
        ]
    );
}

#[test]
fn mirrodin_reader_patterns() {
    let frogmite =
        "Name:Frogmite\nManaCost:4\nTypes:Artifact Creature\nPT:2/2\nK:Affinity:Artifact";
    assert_eq!(
        read(frogmite).cost_reduction.as_deref(),
        Some(
            "Some(CostReduction::PerCount { amount: Amount::CountOf { filter: &Filter::YOUR_ARTIFACT, zone: ZoneSel::Battlefield }, each: 1 })"
        )
    );

    let bosh = "Name:Bosh, Iron Golem\nManaCost:8\nTypes:Legendary Artifact Creature\nPT:6/7\n\
        A:AB$ DealDamage | Cost$ 3 R Sac<1/Artifact> | ValidTgts$ Any | NumDmg$ X\n\
        SVar:X:Sacrificed$CardManaCost";
    assert_eq!(
        read(bosh).abilities,
        [
            "activated!(cost!(\"{3}{R}\", Sacrifice(&Filter::YOUR_ARTIFACT)), &[Effect::DealDamage { amount: Amount::SacrificedManaValue, target: TargetSpec::AnyTarget }], target = Some(TargetSpec::AnyTarget))",
        ]
    );

    let beetle = "Name:Clockwork Beetle\nManaCost:1\nTypes:Artifact Creature\nPT:0/0\n\
        K:etbCounter:P1P1:2\n\
        T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigDelayed\n\
        T:Mode$ Blocks | ValidCard$ Card.Self | Secondary$ True | Execute$ TrigDelayed\n\
        SVar:TrigDelayed:DB$ DelayedTrigger | Mode$ Phase | Phase$ EndCombat | Execute$ TrigRemoveCounter\n\
        SVar:TrigRemoveCounter:DB$ RemoveCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1";
    assert_eq!(
        read(beetle).abilities,
        [
            "triggered!(Trigger::Attacks(&Filter::This), &[Effect::AtEndOfCombat { about: TargetSpec::EventObject, effects: &[Effect::RemoveCounterSelf { kind: CounterKind::P1P1, n: 1 }] }])",
            "triggered!(Trigger::Blocks(&Filter::This), &[Effect::AtEndOfCombat { about: TargetSpec::EventObject, effects: &[Effect::RemoveCounterSelf { kind: CounterKind::P1P1, n: 1 }] }])",
        ]
    );
}

#[test]
fn optional_reveal_dig_takes_one_and_refuses_unread_variants() {
    let body = read(
        "Name:Reveal a permanent\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 4 | Reveal$ True | ChangeValid$ Permanent | DestinationZone2$ Graveyard | Optional$ True",
    );
    let ability = &body.abilities[0];
    for needle in [
        "Effect::RevealTopFind",
        "found: Find::HAND",
        "count: 4",
        "filter: &DIG1",
        "otherwise: SearchDest::Graveyard",
    ] {
        assert!(ability.contains(needle), "missing {needle}: {ability}");
    }
    for card_type in [
        "ARTIFACT",
        "BATTLE",
        "CREATURE",
        "ENCHANTMENT",
        "LAND",
        "PLANESWALKER",
    ] {
        assert!(
            body.statics.contains(&format!("TypeSet::{card_type}")),
            "`Permanent` in a library must mean a permanent card: {}",
            body.statics
        );
    }
    assert!(!body.statics.contains("Filter::Any"));

    for (count, reveal, rest) in [
        (4, "True", "Optional$ False | DestinationZone2$ Graveyard"),
        (4, "False", "Optional$ True | DestinationZone2$ Graveyard"),
        (
            4,
            "True",
            "Optional$ True | DestinationZone$ Exile | DestinationZone2$ Graveyard",
        ),
        (
            4,
            "True",
            "Optional$ True | DestinationZone2$ Graveyard | UnknownRule$ True",
        ),
        (
            4,
            "True",
            "Optional$ True | DestinationZone2$ Graveyard | ValidTgts$ Creature",
        ),
    ] {
        let text = format!(
            "Name:Unread dig\nManaCost:1 G\nTypes:Sorcery\n\
             A:SP$ Dig | DigNum$ {count} | Reveal$ {reveal} | ChangeValid$ Permanent | {rest}"
        );
        assert!(refused(&text), "silently accepted a different dig: {text}");
    }
    assert!(refused(
        "Name:Dynamic dig\nManaCost:X G\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ X | Reveal$ True | ChangeValid$ Permanent | DestinationZone2$ Graveyard | Optional$ True"
    ));
}

// Table-style regression: all batch-two reader patterns share one assertion helper.
#[allow(clippy::too_many_lines)]
#[test]
fn mirrodin_batch_two_reader_patterns() {
    let assert_reads = |script: &str, expected: &[&str]| {
        let parsed = parse(script);
        let body = transcode(&parsed, &cats(), None).unwrap_or_else(|| {
            panic!(
                "should be read in full: {:?}\n{script}",
                refusal_reason(&parsed, &cats(), None)
            )
        });
        let rendered = format!(
            "{} {}\n{}",
            body.statics,
            body.entwine.as_deref().unwrap_or(""),
            body.abilities.join("\n")
        );
        for needle in expected {
            assert!(
                rendered.contains(needle),
                "missing `{needle}` in {rendered}"
            );
        }
    };

    assert_reads(
        "Name:Two activation costs\nTypes:Artifact\n\
         A:AB$ DealDamage | Cost$ 3 T | AlternateCost$ R T | ValidTgts$ Any | NumDmg$ 1",
        &["cost!(\"{3}\", TapSelf)", "cost!(\"{R}\", TapSelf)"],
    );
    assert_reads(
        "Name:Two modes\nManaCost:2 G\nTypes:Sorcery\nK:Entwine:2 G\n\
         A:SP$ Charm | Choices$ Search,Grow\n\
         SVar:Search:DB$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Land.Basic | ChangeTypeDesc$ basic land | ChangeNum$ 2 | Optional$ True\n\
         SVar:Grow:DB$ Effect | StaticAbilities$ Lands\n\
         SVar:Lands:Mode$ Continuous | Affected$ You | AdjustLandPlays$ 2",
        &[
            "Some(cost!(\"{2}{G}\"))",
            "AbilityDef::ModalSpell",
            "Modifier::ExtraLandDrops(2)",
        ],
    );
    assert_reads(
        "Name:No creatures\nTypes:Artifact Creature\nPT:4/6\n\
         S:Mode$ CantBeCast | ValidCard$ Creature | Caster$ You",
        &[
            "Modifier::PlayersCantCast",
            "who: PlayerRel::You",
            "filter: &Filter::CREATURE",
        ],
    );
    assert_reads(
        "Name:Combat loot\nTypes:Artifact Equipment\nK:Equip:1\n\
         T:Mode$ DamageDone | ValidSource$ Creature.AttachedBy | ValidTarget$ Player | CombatDamage$ True | Execute$ Loot | TriggerZones$ Battlefield\n\
         SVar:Loot:AB$ Discard | Defined$ You | Mode$ TgtChoose | NumCards$ 1 | Cost$ Draw<2/You>",
        &[
            "Trigger::DealsCombatDamageToPlayer",
            "Effect::MayDo",
            "Effect::draw(2)",
            "Effect::DiscardForPlayers",
        ],
    );
    assert_reads(
        "Name:Draw with payment\nTypes:Artifact\n\
         T:Mode$ Drawn | ValidCard$ Card.OppOwn | TriggerZones$ Battlefield | Execute$ Draw\n\
         SVar:Draw:AB$ Draw | Cost$ 1 | Defined$ You | NumCards$ 1",
        &[
            "Trigger::Draws(PlayerRel::EachOpponent)",
            "Effect::PlayerMayPayThen",
            "Effect::draw(1)",
        ],
    );
    assert_reads(
        "Name:Nonbasic land destruction\nManaCost:1 R R\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Land | SubAbility$ Damage\n\
         SVar:Damage:DB$ DealDamage | Defined$ TargetedController | NumDmg$ 2 | ConditionDefined$ Targeted | ConditionPresent$ Land.Basic | ConditionCompare$ EQ0",
        &[
            "Effect::IfTargetMatches",
            "filter: &Filter::NONBASIC_LAND",
            "TargetSpec::Player(PlayerRel::ControllerOfTarget)",
        ],
    );
    assert_reads(
        "Name:Shuffle activation\nTypes:Artifact Creature\nPT:1/1\n\
         A:AB$ Shuffle | Cost$ 2 T | Defined$ You",
        &["Effect::ShuffleLibrary", "who: PlayerRel::You"],
    );
    assert_reads(
        "Name:Reveal and sort\nManaCost:1 U\nTypes:Creature\nPT:1/1\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Any | Destination$ Battlefield | Execute$ Dig\n\
         SVar:Dig:DB$ Dig | DigNum$ 1 | Reveal$ True | ChangeNum$ All | ChangeValid$ Artifact | DestinationZone2$ Graveyard",
        &[
            "Effect::RevealTopAndSort",
            "matched: SearchDest::Hand",
            "otherwise: SearchDest::Graveyard",
        ],
    );
    assert_reads(
        "Name:Artifact evasion\nManaCost:2 U\nTypes:Creature\nPT:2/2\n\
         S:Mode$ CantBlockBy | ValidAttacker$ Card.Self | IsPresent$ Artifact.DefenderCtrl",
        &[
            "Modifier::AddKeyword(KeywordSet::UNBLOCKABLE)",
            "Condition::SomeDefendingPlayerControlsNone",
        ],
    );
    assert_reads(
        "Name:Artifact blocker\nManaCost:4 R\nTypes:Creature\nPT:3/3\n\
         T:Mode$ AttackerBlockedByCreature | ValidCard$ Card.Self | ValidBlocker$ Creature.Artifact | TriggerZones$ Battlefield | Execute$ Destroy\n\
         SVar:Destroy:DB$ Destroy | Defined$ TriggeredBlockerLKICopy",
        &[
            "Trigger::BecomesBlockedBy",
            "Effect::destroy(TargetSpec::EventObject)",
        ],
    );
    assert_reads(
        "Name:Counted unless cost\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card | UnlessCost$ Y\n\
         SVar:Y:Count$Valid Artifact.YouCtrl",
        &[
            "Effect::PlayerMayPayOr",
            "player: PlayerRel::ControllerOfTarget",
            "filter: &Filter::YOUR_ARTIFACT",
        ],
    );
    assert_reads(
        "Name:Empty library\nTypes:Artifact Creature\nPT:10/10\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Exile\n\
         SVar:Exile:DB$ ChangeZoneAll | ChangeType$ Card.YouOwn | Origin$ Library | Destination$ Exile",
        &[
            "Effect::ExileTop",
            "count: Amount::LibrarySize(PlayerRel::You)",
        ],
    );
    assert_reads(
        "Name:Animate artifacts\nManaCost:3 U\nTypes:Enchantment\n\
         S:Mode$ Continuous | Affected$ Artifact.nonCreature | SetPower$ AffectedX | SetToughness$ AffectedX | AddType$ Creature\n\
         SVar:AffectedX:Count$CardManaCost",
        &[
            "Filter::And(&[Filter::ARTIFACT, Filter::NONCREATURE])",
            "Modifier::AnimateNoncreatureArtifact",
        ],
    );
    assert_reads(
        "Name:Counter combat costs\nManaCost:5\nTypes:Artifact Creature\nPT:2/2\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ Counter\n\
         SVar:Counter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1\n\
         S:Mode$ CantAttackUnless | ValidCard$ Card.Self | Cost$ Y\n\
         S:Mode$ CantBlockUnless | ValidCard$ Card.Self | Cost$ Y\n\
         SVar:Y:Count$CardCounters.P1P1",
        &[
            "Modifier::CantAttackUnlessPayManaPerCounter(CounterKind::P1P1)",
            "Modifier::CantBlockUnlessPayManaPerCounter(CounterKind::P1P1)",
        ],
    );
    assert_reads(
        "Name:Printed tap correction\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ 6 Sac<1/CARDNAME> | Defined$ You | NumCards$ 1\n\
         Oracle:{6}, {T}, Sacrifice this artifact: Draw a card.",
        &["cost!(\"{6}\", TapSelf, SacrificeSelf)"],
    );
    assert_reads(
        "Name:Printed tap correction after sacrifice\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ 6 Sac<1/CARDNAME> | Defined$ You | NumCards$ 1\n\
         Oracle:{6}, Sacrifice this artifact, {T}: Draw a card.",
        &["cost!(\"{6}\", SacrificeSelf, TapSelf)"],
    );
    assert_reads(
        "Name:Linked instant\nTypes:Artifact\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Any | Destination$ Battlefield | OptionalDecider$ You | Execute$ Exile\n\
         SVar:Exile:DB$ ChangeZone | Imprint$ True | Origin$ Hand | Destination$ Exile | ChangeType$ Instant.cmcLE2 | ChangeNum$ 1\n\
         A:AB$ Play | Cost$ 2 T | Valid$ Card.IsImprinted+ExiledWithSource | ValidZone$ Exile | Amount$ All | WithoutManaCost$ True | ValidSA$ Spell | Optional$ True | CopyCard$ True",
        &[
            "Effect::ExileChosenFromHandWithSource",
            "Filter::CmcAtMost(2)",
            "Effect::MayCopyCastLinkedExile",
        ],
    );
    assert_reads(
        "Name:Coin replacement\nTypes:Artifact\n\
         S:Mode$ FlipCoinDoubler | ValidPlayer$ You",
        &["Modifier::FlipTwoCoinsIgnoreOne"],
    );
    assert_reads(
        "Name:Next turn control\nTypes:Artifact\n\
         A:AB$ ControlPlayer | Cost$ 4 T Sac<1/CARDNAME> | ValidTgts$ Player",
        &[
            "Effect::ControlPlayerNextTurn",
            "player: PlayerRel::Chosen",
            "TargetSpec::AnyPlayer",
        ],
    );
    assert_reads(
        "Name:Named cast history\nManaCost:2 B\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Library | ChangeType$ Card.nonLand | ChangeNum$ 1 | Imprint$ True | Reveal$ True | Shuffle$ False | StackDescription$ SpellDescription | SubAbility$ Repeat | SpellDescription$ Search and reveal, punish matching casts, then shuffle.\n\
         SVar:Repeat:DB$ RepeatEach | RepeatPlayers$ Opponent | RepeatSubAbility$ Lose | SubAbility$ Shuffle\n\
         SVar:Lose:DB$ LoseLife | Defined$ Player.IsRemembered | LifeAmount$ 6 | ConditionCheckSVar$ LoseLifeCheck | ConditionSVarCompare$ GE1\n\
         SVar:Shuffle:DB$ Shuffle | Defined$ You | SubAbility$ Cleanup\n\
         SVar:Cleanup:DB$ Cleanup | ClearImprinted$ True\n\
         SVar:LoseLifeCheck:Count$ThisTurnCast_Card.RememberedPlayerCtrl+sharesNameWith Imprinted",
        &[
            "Effect::RevealLibrarySearch",
            "Effect::PlayersWhoCastRevealedNameThisTurnLoseLife",
            "Effect::ShuffleLibrary",
        ],
    );
    assert_reads(
        "Name:Guess a hand\nTypes:Artifact\n\
         A:AB$ NameCard | Cost$ 2 T | Defined$ You | SubAbility$ Guess\n\
         SVar:Guess:DB$ GenericChoice | ValidTgts$ Opponent | Choices$ GuessInHand,GuessNotInHand | ShowChoice$ True | Guess$ True\n\
         SVar:GuessInHand:DB$ RevealHand | Defined$ You | Optional$ True | RememberRevealedPlayer$ True | SubAbility$ DBInHandDraw\n\
         SVar:GuessNotInHand:DB$ RevealHand | Defined$ You | Optional$ True | RememberRevealedPlayer$ True | SubAbility$ DBNotInHandDraw\n\
         SVar:DBInHandDraw:DB$ Draw | Defined$ You | ConditionPlayerDefined$ Remembered | ConditionPlayerContains$ You | ConditionCheckSVar$ X | ConditionSVarCompare$ EQ0 | SubAbility$ Cleanup\n\
         SVar:DBNotInHandDraw:DB$ Draw | Defined$ You | ConditionPlayerDefined$ Remembered | ConditionPlayerContains$ You | ConditionCheckSVar$ X | SubAbility$ Cleanup\n\
         SVar:Cleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$ValidHand Card.NamedCard",
        &[
            "Effect::ChooseCardName",
            "Effect::GuessChosenNameInHand",
            "Effect::RevealHandForGuess",
            "Effect::DrawIfGuessWrong",
        ],
    );
    assert_reads(
        "Name:Artifact tax\nTypes:Enchantment\n\
         S:Mode$ RaiseCost | ValidCard$ Artifact | Activator$ Player | Type$ Spell | Amount$ X | Relative$ True\n\
         SVar:X:Count$Valid Artifact.YouCtrl",
        &["Modifier::SpellsCostMorePerControlled(&Filter::ARTIFACT)"],
    );
    assert_reads(
        "Name:No game loss\nTypes:Artifact Creature\nPT:4/4\n\
         R:Event$ GameLoss | ActiveZones$ Battlefield | ValidPlayer$ You | Layer$ CantHappen\n\
         R:Event$ GameWin | ActiveZones$ Battlefield | ValidPlayer$ Opponent | Layer$ CantHappen | Secondary$ True",
        &["Modifier::NamedPlayersCantLose", "Modifier::PlayersCantWin"],
    );
}

#[test]
fn affinity_scopes_every_alternative_and_refuses_extra_fields() {
    let body = read("Name:X\nManaCost:4\nTypes:Artifact\nK:Affinity:Artifact,Creature");
    assert_eq!(
        body.statics.matches("Filter::ControlledByYou").count(),
        2,
        "each alternative is something you control: {}",
        body.statics
    );
    assert!(body.statics.contains("Filter::ARTIFACT"));
    assert!(body.statics.contains("Filter::CREATURE"));
    assert!(refused(
        "Name:X\nManaCost:4\nTypes:Artifact\nK:Affinity:Artifact:UnreadRule"
    ));
}

#[test]
fn exile_sweeps_link_only_when_a_companion_reads_the_link() {
    let entry = "T:Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Exile\n\
        SVar:Exile:DB$ ChangeZoneAll | Origin$ Battlefield | Destination$ Exile | ChangeType$ Creature";
    let body = read(&format!("Name:X\nTypes:Creature\n{entry}"));
    assert!(body.abilities[0].contains("Effect::ExileAll"));
    assert!(body.abilities[0].contains("linked: false"));

    let body = read(&format!(
        "Name:X\nTypes:Creature\n{entry}\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | ValidCard$ Card.Self | Execute$ Return\n\
         SVar:Return:DB$ ChangeZoneAll | Origin$ Exile | Destination$ Battlefield | ChangeType$ Card.ExiledWithSource"
    ));
    assert!(body.abilities[0].contains("linked: true"));
    assert!(body.abilities[1].contains("Effect::ReturnLinkedToBattlefield"));
}

/// "Whenever this deals combat damage to a player, you may have it deal 2
/// damage to target creature that player controls": the script marks no
/// `OptionalDecider$`, the printed "you may have it" is the choice (CR 603.5),
/// asked on resolution through `Effect::MayDo`. Without the may, `validate`
/// refused two morph cards the Morph reader finished.
#[test]
fn a_printed_you_may_have_it_trigger_asks_on_resolution() {
    let script = "Name:X\nTypes:Creature Goblin\nPT:2/1\nT:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ Hit | TriggerZones$ Battlefield | TriggerDescription$ Whenever CARDNAME deals combat damage to a player, you may have it deal 2 damage to target creature that player controls.\nSVar:Hit:DB$ DealDamage | ValidTgts$ Creature.ControlledBy TriggeredTarget | NumDmg$ 2";
    let ability = &read(script).abilities[0];
    assert!(ability.contains("Effect::MayDo"), "{ability}");
    let plain = script.replace(", you may have it deal 2 damage", ", it deals 2 damage");
    assert!(!read(&plain).abilities[0].contains("Effect::MayDo"));
    // The executed line asking too (`OptionalDecider$ You`) is the same
    // decision: one `MayDo`, never two.
    let both = script.replace("NumDmg$ 2", "NumDmg$ 2 | OptionalDecider$ You");
    assert_eq!(read(&both).abilities[0].matches("Effect::MayDo").count(), 1);
}

/// The printed "you may have …" is read as the choice only where it opens
/// the effect. After a mandatory clause ("…, draw a card, then you may have
/// it …") it would wrap the mandatory half in the same `Effect::MayDo`, so it
/// is not read there (the card is then held to its printed may elsewhere).
#[test]
fn a_you_may_have_after_a_mandatory_clause_does_not_make_the_chain_optional() {
    let opening = "Name:X\nTypes:Creature Goblin\nPT:2/1\nT:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ Hit | TriggerZones$ Battlefield | TriggerDescription$ Whenever CARDNAME deals combat damage to a player, you may have it deal 2 damage to target creature that player controls.\nSVar:Hit:DB$ DealDamage | ValidTgts$ Creature.ControlledBy TriggeredTarget | NumDmg$ 2";
    assert!(read(opening).abilities[0].contains("Effect::MayDo"));
    let after_a_clause = opening.replace(
        ", you may have it deal 2 damage",
        ", draw a card, then you may have it deal 2 damage",
    );
    assert!(
        !read(&after_a_clause).abilities[0].contains("Effect::MayDo"),
        "a may after a mandatory clause is not the whole chain's choice"
    );
}

#[test]
fn oracle_gates_stale_trigger_description_you_may() {
    let mandatory = "Name:Mandatory crab\nTypes:Creature Goblin\nPT:1/1\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ Draw | TriggerDescription$ When CARDNAME dies, you may draw a card.\n\
         SVar:Draw:DB$ Draw | Defined$ You | NumCards$ 1\n\
         Oracle:When this creature dies, draw a card.\n";
    let body = read(mandatory);
    assert!(
        !body.abilities[0].contains("Effect::MayDo"),
        "mandatory oracle must suppress stale TriggerDescription$ you may: {}",
        body.abilities[0]
    );

    let optional = mandatory.replace(
        "Oracle:When this creature dies, draw a card.",
        "Oracle:When this creature dies, you may draw a card.",
    );
    assert!(
        read(&optional).abilities[0].contains("Effect::MayDo"),
        "oracle containing you may preserves Effect::MayDo"
    );

    let no_oracle = mandatory.replace("Oracle:When this creature dies, draw a card.\n", "");
    assert!(
        read(&no_oracle).abilities[0].contains("Effect::MayDo"),
        "minimal synthetic scripts without Oracle keep TriggerDescription$ behavior"
    );
}

#[test]
fn becomes_blocked_self_trigger_sets_trigger_is_self() {
    let self_blocked = "Name:Spiny runner\nTypes:Creature Goblin\nPT:2/2\n\
         T:Mode$ AttackerBlocked | ValidCard$ Card.Self | OptionalDecider$ TriggeredCardController | Execute$ Ping\n\
         SVar:Ping:DB$ GainLife | Defined$ You | LifeAmount$ 1\n\
         Oracle:Whenever this creature becomes blocked, you may gain 1 life.\n";
    let body = read(self_blocked);
    assert!(
        body.abilities[0].contains("Trigger::BecomesBlocked")
            && body.abilities[0].contains("Effect::MayDo"),
        "{}",
        body.abilities[0]
    );

    let team_blocked = "Name:Banner guard\nTypes:Creature Goblin\nPT:2/2\n\
         T:Mode$ AttackerBlocked | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ Ping\n\
         SVar:Ping:DB$ GainLife | Defined$ You | LifeAmount$ 1\n";
    assert!(
        read(team_blocked).abilities[0].contains("Trigger::ControlledBecomesBlocked"),
        "{}",
        read(team_blocked).abilities[0]
    );
    // Non-self ControlledBecomesBlocked must NOT treat TriggeredCardController as You.
    assert!(refused(&self_blocked.replace(
        "ValidCard$ Card.Self",
        "ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield"
    )));
}

#[test]
fn immediate_trigger_reflexive_cost_and_remembered_when_rules() {
    let sac_power = "Name:Fling beetle\nTypes:Creature Goblin\nPT:2/2\n\
         A:AB$ ImmediateTrigger | Cost$ Sac<1/Creature.Other> | Execute$ Hit\n\
         SVar:Hit:DB$ DealDamage | ValidTgts$ Any | NumDmg$ X\n\
         SVar:X:Spawner>Sacrificed$CardPower\n";
    let a = &read(sac_power).abilities[0];
    assert!(a.contains("Effect::ReflexiveTrigger"), "{a}");
    assert!(a.contains("when: ReflexiveWhen::Always"), "{a}");
    assert!(
        a.contains("remember: TriggerRemember::SacrificedObject"),
        "{a}"
    );
    assert!(a.contains("TargetReq::one(TargetSpec::AnyTarget)"), "{a}");
    assert!(a.contains("Amount::EventPower"), "{a}");

    let mill_once = "Name:Dredge bell\nTypes:Sorcery\nManaCost:1 B\n\
         A:SP$ Mill | Defined$ You | NumCards$ 3 | RememberMilled$ True | SubAbility$ Ref\n\
         SVar:Ref:DB$ ImmediateTrigger | ConditionDefined$ Remembered | ConditionPresent$ Creature | Execute$ Gain | TriggerDescription$ When a creature card is milled this way, gain 2 life.\n\
         SVar:Gain:DB$ GainLife | Defined$ You | LifeAmount$ 2\n";
    let a = &read(mill_once).abilities[0];
    assert!(a.contains("ReflexiveWhen::ThisWay(ThisWay::Milled("), "{a}");
    assert!(a.contains("once: true"), "{a}");

    let mill_each = mill_once.replace("When a creature card", "Whenever a creature card");
    assert!(read(&mill_each).abilities[0].contains("once: false"));

    let attach_ref = "Name:Battle clasp\nTypes:Artifact Equipment\n\
         T:Mode$ Attacks | ValidCard$ Creature.YouCtrl | TriggerZones$ Battlefield | Execute$ Ref\n\
         SVar:Ref:DB$ ImmediateTrigger | RememberObjects$ TriggeredAttackerLKICopy | Execute$ Snap | OptionalDecider$ You\n\
         SVar:Snap:DB$ Attach | Object$ Self | Defined$ DelayTriggerRemembered\n";
    let a = &read(attach_ref).abilities[0];
    assert!(a.contains("remember: TriggerRemember::EventObject"), "{a}");
    assert!(
        a.contains("Effect::AttachSelf { target: TargetSpec::EventObject }"),
        "{a}"
    );

    assert!(refused(&format!("{sac_power} | Announce$ True")));
    assert!(refused(&mill_once.replace(
        "ConditionPresent$ Creature",
        "ConditionPresent$ Creature | ConditionCompare$ EQ4"
    )));
}

#[test]
fn generalized_delayed_trigger_phase_event_and_observation() {
    let phase_sac = "Name:Borrowed fury\nTypes:Sorcery\nManaCost:2 R\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ 3 | SubAbility$ Del\n\
         SVar:Del:DB$ DelayedTrigger | Mode$ Phase | Phase$ End of Turn | RememberObjects$ Targeted | Execute$ Sac\n\
         SVar:Sac:DB$ Sacrifice | Defined$ DelayTriggerRemembered\n";
    let a = &read(phase_sac).abilities[0];
    assert!(a.contains("Effect::RegisterDelayedTrigger"), "{a}");
    assert!(a.contains("DelayedStep::EndStep"), "{a}");
    assert!(a.contains("remember: TriggerRemember::FirstTarget"), "{a}");
    assert!(a.contains("Effect::SacrificeEvent"), "{a}");

    let dies_return = "Name:Graceful spark\nTypes:Instant\nManaCost:1 W\n\
         A:SP$ DelayedTrigger | ValidTgts$ Creature | Mode$ ChangesZone | RememberObjects$ Targeted | ValidCard$ Card.IsTriggerRemembered | Origin$ Battlefield | Destination$ Graveyard | ThisTurn$ True | Execute$ Ret\n\
         SVar:Ret:DB$ ChangeZone | Defined$ TriggeredNewCardLKICopy | Origin$ Graveyard | Destination$ Battlefield\n";
    let a = &read(dies_return).abilities[0];
    assert!(a.contains("DelayedTriggerWhen::OnTrigger"), "{a}");
    assert!(a.contains("watch: TriggerRemember::FirstTarget"), "{a}");
    assert!(
        a.contains("duration: DelayedDuration::OneShotThisTurn"),
        "{a}"
    );
    assert!(
        a.contains("Effect::GraveyardToBattlefield { target: TargetSpec::EventObject, owner_control: true, counters: None }"),
        "{a}"
    );

    let copy_next = "Name:Twin echo\nTypes:Instant\nManaCost:2 R\n\
         A:SP$ DelayedTrigger | Mode$ SpellCast | ValidCard$ Instant,Sorcery | ValidActivatingPlayer$ You | ThisTurn$ True | Execute$ Copy\n\
         SVar:Copy:DB$ CopySpellAbility | Defined$ TriggeredSpellAbility | MayChooseTarget$ True | Amount$ 2\n";
    let a = &read(copy_next).abilities[0];
    assert_eq!(
        a.matches("Effect::CopyEventSpellOrAbility").count(),
        2,
        "{a}"
    );

    let lose_ctrl = "Name:Loyal leash\nTypes:Artifact\n\
         T:Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Watch\n\
         SVar:Watch:DB$ DelayedTrigger | Mode$ ChangesController | ValidCard$ Card.Self | Execute$ Draw\n\
         SVar:Draw:DB$ Draw | Defined$ You | NumCards$ 1\n";
    let a = &read(lose_ctrl).abilities[0];
    assert!(a.contains("DelayedTriggerWhen::OnObservation"), "{a}");
    assert!(a.contains("DelayedObservation::YouLoseControl"), "{a}");

    assert!(refused(
        "Name:Bad upkeep\nTypes:Instant\n\
         A:SP$ DelayedTrigger | Mode$ Phase | Phase$ Upkeep | Execute$ Draw\n\
         SVar:Draw:DB$ Draw | Defined$ You | NumCards$ 1\n"
    ));
    assert!(refused(
        &copy_next.replace("ThisTurn$ True", "Static$ True")
    ));
}

#[test]
fn wave5_effect_choose_one_phase_conditions_and_event_zones() {
    let choose = "Name:Bimodal choice\nTypes:Sorcery\nManaCost:4 W W\n\
         A:SP$ GenericChoice | Choices$ WipeLand,WipeCreat\n\
         SVar:WipeLand:DB$ DestroyAll | ValidCards$ Land\n\
         SVar:WipeCreat:DB$ DestroyAll | ValidCards$ Creature\n";
    let a = &read(choose).abilities[0];
    assert!(a.contains("Effect::ChooseOne { choices: &["), "{a}");

    let multi_static = "Name:Silence flash\nTypes:Instant\nManaCost:1 W\n\
         A:SP$ Effect | StaticAbilities$ Lock,Flash | Duration$ UntilEndOfTurn\n\
         SVar:Lock:Mode$ CantBeCast | Caster$ Opponent | ValidCard$ Card\n\
         SVar:Flash:Mode$ CastWithFlash | Caster$ You | ValidCard$ Artifact,Creature\n";
    let a = &read(multi_static).abilities[0];
    assert!(
        a.contains("Modifier::OpponentsCantCast(&Filter::Any)"),
        "{a}"
    );
    assert!(
        a.contains("Modifier::SpellsHaveFlash(Some(TypeSet::ARTIFACT.union(TypeSet::CREATURE)))"),
        "{a}"
    );
    // CantGainLife is a CR 101.2 prohibition (`PlayersCantGainLife`), never conflated with LifeGainNoLifeInstead.
    let no_life = read(
        "Name:No life\nTypes:Instant\n\
         A:SP$ Effect | StaticAbilities$ NoGain | Duration$ UntilEndOfTurn\n\
         SVar:NoGain:Mode$ CantGainLife | ValidPlayer$ Player\n",
    );
    assert!(
        no_life.abilities[0].contains("Modifier::PlayersCantGainLife(PlayerRel::EachPlayer)")
            && !no_life.abilities[0].contains("LifeGainNoLifeInstead"),
        "{}",
        no_life.abilities[0]
    );

    let phase_cond = "Name:Descend scout\nTypes:Creature Goblin\nPT:2/2\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You.descended | TriggerZones$ Battlefield | Execute$ Draw | CheckSVar$ X | SVarCompare$ GE1\n\
         SVar:Draw:DB$ Draw | Defined$ You | NumCards$ 1\n\
         SVar:X:Count$ThisTurnEntered_Battlefield_Goblin.YouCtrl\n";
    let a = &read(phase_cond).abilities[0];
    assert!(a.contains("Condition::DescendedThisTurn"), "{a}");
    assert!(
        a.contains(
            "Condition::EnteredSubtypeUnderYourControlThisTurn(&[subtypes::creature::GOBLIN])"
        ),
        "{a}"
    );

    let self_die_exile = "Name:Self exile\nTypes:Creature Goblin\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ Ex\n\
         SVar:Ex:DB$ ChangeZone | Defined$ TriggeredNewCardLKICopy | Origin$ Graveyard | Destination$ Exile\n";
    assert!(read(self_die_exile).abilities[0].contains("Effect::ExileSource"));
    assert!(refused(
        &self_die_exile.replace("ValidCard$ Card.Self", "ValidCard$ Creature.Other")
    ));
}

#[test]
fn wave5_self_blink_exile_counters_token_followups_and_search() {
    let blink_flip = "Name:Flip walker\nTypes:Creature Goblin\nPT:1/1\n\
         A:AB$ ChangeZone | Cost$ T | Defined$ Self | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ Ret\n\
         SVar:Ret:DB$ ChangeZone | Defined$ Remembered | Origin$ Exile | Destination$ Battlefield | Transformed$ True | SubAbility$ Cln\n\
         SVar:Cln:DB$ Cleanup | ClearRemembered$ True\n";
    assert!(
        read(blink_flip).abilities[0]
            .contains("Effect::ExileSelfReturnAsFace { face: 1, owner_control: true }")
    );

    let suspend_like = "Name:Hourglass spell\nTypes:Sorcery\nManaCost:2 B B\n\
         A:SP$ ChangeZone | Defined$ Self | Origin$ Stack | Destination$ Exile | RememberChanged$ True | SubAbility$ Put\n\
         SVar:Put:DB$ PutCounter | Defined$ Remembered | CounterType$ TIME | CounterNum$ 2 | SubAbility$ Cln\n\
         SVar:Cln:DB$ Cleanup | ClearRemembered$ True\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Exile | Execute$ Fin\n\
         SVar:Fin:DB$ ChangeZone | Defined$ Self | Origin$ Exile | Destination$ Graveyard\n";
    let body = read(suspend_like);
    assert!(
        body.abilities[0]
            .contains("Effect::ExileSourceWithCounters { kind: CounterKind::Time, n: 2 }"),
        "{}",
        body.abilities[0]
    );
    assert!(
        body.abilities[1].contains("Effect::MoveSourceToOwnerGraveyard"),
        "{}",
        body.abilities[1]
    );

    let tok_map = tokens();
    let living = "Name:Living blade\nTypes:Artifact Equipment\n\
         T:Mode$ ChangesZone | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Make\n\
         SVar:Make:DB$ Token | TokenScript$ r_1_1_goblin | RememberTokens$ True | SubAbility$ Att\n\
         SVar:Att:DB$ Attach | Object$ Self | Defined$ Remembered | SubAbility$ Cln\n\
         SVar:Cln:DB$ Cleanup | ClearRemembered$ True\n";
    let living_body = transcode(&parse(living), &cats(), Some(&tok_map)).expect("living blade");
    assert!(
        living_body.abilities[0].contains("Effect::CreateTokenThenAttachSelf"),
        "{}",
        living_body.abilities[0]
    );

    let fractal = "Name:Growing token\nTypes:Sorcery\nManaCost:2 G\n\
         A:SP$ Token | TokenScript$ r_1_1_goblin | RememberTokens$ True | SubAbility$ Pump\n\
         SVar:Pump:DB$ PutCounter | Defined$ Remembered | CounterType$ P1P1 | CounterNum$ 4 | SubAbility$ Cln\n\
         SVar:Cln:DB$ Cleanup | ClearRemembered$ True\n";
    let fractal_body = transcode(&parse(fractal), &cats(), Some(&tok_map)).expect("fractal");
    assert!(
        fractal_body.abilities[0].contains("Effect::CreateTokenThenCounters"),
        "{}",
        fractal_body.abilities[0]
    );

    let entomb = "Name:Grave tutor\nTypes:Instant\nManaCost:B\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Graveyard | ChangeType$ Card | ChangeNum$ 1\n";
    assert!(read(entomb).abilities[0].contains("Find::GRAVEYARD"));

    let squad = "Name:Pack search\nTypes:Sorcery\nManaCost:1 W\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Goblin | ChangeNum$ X\n\
         SVar:X:Count$ValidLibrary Goblin.YouOwn\n";
    assert!(read(squad).abilities[0].contains("Find::HAND.any_number()"));
}

#[test]
fn wave7_non_source_and_implicit_animate() {
    let implicit = read(
        "Name:AscendKnight\nManaCost:W\nTypes:Creature Human\nPT:1/1\n\
         A:AB$ Animate | Cost$ 1 W | Power$ 4 | Toughness$ 4 | Types$ Creature,Goblin | \
         Keywords$ Flying | Duration$ Permanent",
    );
    let a = implicit.abilities.join("");
    assert!(
        a.contains(
            "Effect::continuous(&Filter::This, Modifier::SetPT(4, 4), Duration::Indefinitely)"
        ),
        "{a}"
    );

    let aura = read(
        "Name:WarHarness\nManaCost:1 R\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         T:Mode$ Attacks | ValidCard$ Card.AttachedBy | Execute$ Anim\n\
         SVar:Anim:DB$ Animate | Defined$ Enchanted | Power$ 5 | Toughness$ 5 | Triggers$ Tr\n\
         SVar:Tr:Mode$ Attacks | ValidCard$ Card.Self | Execute$ Ping\n\
         SVar:Ping:DB$ GainLife | LifeAmount$ 1",
    );
    let a = aura.abilities.join("");
    assert!(
        a.contains("Effect::continuous(&Filter::AttachedToBySource, Modifier::GrantTriggeredDefinition(&GRANTED_TRIGGER_1), Duration::UntilEndOfTurn)"),
        "{a}"
    );

    let parent = read(
        "Name:SculptRelic\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Draw | ValidTgts$ Artifact | NumCards$ 1 | SubAbility$ Anim\n\
         SVar:Anim:DB$ Animate | Defined$ ParentTarget | Power$ 4 | Toughness$ 4 | \
         Types$ Creature | Abilities$ TapPing\n\
         SVar:TapPing:AB$ DealDamage | Cost$ T | ValidTgts$ Any | NumDmg$ 1",
    );
    let a = parent.abilities.join("");
    assert!(
        a.contains("Effect::continuous(&Filter::This, Modifier::GrantActivatedDefinition(&GRANTED_ACTIVATED_1), Duration::UntilEndOfTurn)"),
        "{a}"
    );

    let trig = read(
        "Name:AwakenGuide\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Land.YouCtrl | Execute$ Anim\n\
         SVar:Anim:DB$ Animate | Defined$ TriggeredCardLKICopy | Power$ 3 | Toughness$ 3 | Types$ Creature",
    );
    assert!(
        trig.abilities[0].contains("Effect::continuous(&Filter::Reference, Modifier::SetPT(3, 3), Duration::UntilEndOfTurn)"),
        "{:?}",
        trig.abilities
    );

    assert!(refused(
        "Name:BadPlayerAnim\nManaCost:U\nTypes:Instant\nA:SP$ Animate | ValidTgts$ Player | Types$ Creature"
    ));
}

#[test]
fn wave8a_goad_static_command_and_continuous_check_svar() {
    let goad = read(
        "Name:FuryGoad\nManaCost:2 R\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ Continuous | AffectedDefined$ Enchanted | Affected$ Creature | AddPower$ 2 | Goad$ True",
    );
    assert!(
        goad.abilities.contains(&"static_ability!(Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]), Modifier::ModifyPT(2, 0))".to_string())
            && goad.abilities.contains(&"static_ability!(Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource]), Modifier::Goaded)".to_string()),
        "{:?}",
        goad.abilities
    );

    let cmd_static = read(
        "Name:EmblemLord\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | EffectZone$ Command | Affected$ Creature.YouCtrl | AddPower$ 1 | AddToughness$ 1\n\
         S:Mode$ Continuous | EffectZone$ Battlefield,Command | Affected$ Creature.YouCtrl | IsPresent$ Card.IsCommander+YouOwn+YouCtrl | AddKeyword$ Vigilance",
    );
    assert_eq!(
        cmd_static.abilities,
        [
            "static_ability!(Filter::YOUR_CREATURE, Modifier::ModifyPT(1, 1), zone = ZoneRef::Command)",
            "static_ability!(Filter::YOUR_CREATURE, Modifier::AddKeyword(KeywordSet::VIGILANCE), zone = ZoneRef::BattlefieldOrCommand, condition = Some(Condition::ControlYourCommander))",
        ]
    );

    let corrupted = read(
        "Name:PlagueBanner\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | AffectedDefined$ Self | AddPower$ 2 | AddAbility$ Drain | CheckSVar$ P | SVarCompare$ GE3\n\
         SVar:P:PlayerCountOpponents$HighestCounters.Poison\n\
         SVar:Drain:AB$ GainLife | Cost$ 1 | LifeAmount$ 1",
    );
    assert!(
        corrupted.abilities[0].contains("condition = Some(Condition::OpponentPoisonAtLeast(3))")
            && corrupted.abilities[1]
                .contains("condition = Some(Condition::OpponentPoisonAtLeast(3))"),
        "{:?}",
        corrupted.abilities
    );

    let combined = read(
        "Name:TideWatch\nManaCost:2 U\nTypes:Creature\nPT:2/3\n\
         S:Mode$ Continuous | AffectedDefined$ Self | Condition$ PlayerTurn | CheckSVar$ Count$Domain | SVarCompare$ GE4 | CheckSecondSVar$ P | AddKeyword$ Flying\n\
         SVar:P:PlayerCountOpponents$HighestCounters.Poison",
    );
    assert!(
        combined.abilities[0].contains("Condition::All(&[Condition::YourTurn, Condition::AmountAtLeast(Amount::BasicLandTypesAmong(&Filter::YOUR_LAND), 4), Condition::OpponentPoisonAtLeast(1)])"),
        "{:?}",
        combined.abilities
    );

    let cant_attack = read(
        "Name:ShyBehemoth\nManaCost:3 G\nTypes:Creature\nPT:5/5\n\
         S:Mode$ CantAttack | ValidCard$ Card.Self | IsPresent$ Creature.YouCtrl | PresentCompare$ EQ0\n\
         S:Mode$ CantAttack,CantBlock | ValidCard$ Card.Self | CheckSVar$ PlayerCountPropertyYou$HasPropertyMaxSpeed | SVarCompare$ NE1",
    );
    assert_eq!(
        cant_attack.abilities,
        [
            "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::CANT_ATTACK), condition = Some(Condition::ControlCountAtMost(&Filter::YOUR_CREATURE, 0)))",
            "static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::CANT_ATTACK.union(KeywordSet::CANT_BLOCK)), condition = Some(Condition::Not(&Condition::MaxSpeed)))",
        ]
    );

    assert!(refused(
        "Name:BareSelfCant\nManaCost:1 R\nTypes:Creature\nPT:3/3\nS:Mode$ CantAttack | ValidCard$ Card.Self"
    ));
}

#[test]
fn bargain_and_your_turn_conditions() {
    let bargained = read(
        "Name:BargainWisp\nManaCost:1 U\nTypes:Creature\nPT:2/1\nK:Bargain\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1 | ConditionCheckSVar$ BargainedCheck | ConditionSVarCompare$ GE1\n\
         SVar:BargainedCheck:Count$Bargained.1.0",
    );
    assert!(bargained.bargain);
    assert!(
        bargained.abilities[0].contains("Condition::Bargained"),
        "{:?}",
        bargained.abilities
    );

    let not_bargained = read(
        "Name:AntiBargainWisp\nManaCost:1 U\nTypes:Creature\nPT:2/1\nK:Bargain\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1 | ConditionCheckSVar$ BargainedCheck | ConditionSVarCompare$ EQ0\n\
         SVar:BargainedCheck:Count$Bargained.1.0",
    );
    assert!(
        not_bargained.abilities[0].contains("Condition::Not(&Condition::Bargained)"),
        "{:?}",
        not_bargained.abilities
    );

    let turn_watcher = read(
        "Name:TurnWatcher\nManaCost:1 W\nTypes:Creature\nPT:2/2\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddPower$ 2 | CheckSVar$ IsMyTurn | SVarCompare$ GE1\n\
         S:Mode$ Continuous | Affected$ Card.Self | AddToughness$ 2 | CheckSVar$ IsMyTurn | SVarCompare$ EQ0\n\
         SVar:IsMyTurn:PlayerCountPropertyYou$HasPropertyActive",
    );
    assert!(
        turn_watcher.abilities[0].contains("Condition::YourTurn")
            && turn_watcher.abilities[1].contains("Condition::Not(&Condition::YourTurn)"),
        "{:?}",
        turn_watcher.abilities
    );

    for bad in [
        "Name:BadNoBargainKeyword\nManaCost:1 U\nTypes:Creature\nPT:2/1\nT:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw\nSVar:TrigDraw:DB$ Draw | NumCards$ 1 | ConditionCheckSVar$ BargainedCheck | ConditionSVarCompare$ GE1\nSVar:BargainedCheck:Count$Bargained.1.0",
        "Name:BadDupBargain\nManaCost:1 U\nTypes:Creature\nPT:2/1\nK:Bargain\nK:Bargain",
        "Name:BadExtraBargain\nManaCost:1 U\nTypes:Creature\nPT:2/1\nK:Bargain:Extra",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn emerge_and_prototype_keywords() {
    let simple_emerge = read("Name:EmergeBeast\nManaCost:7\nTypes:Creature\nPT:5/5\nK:Emerge:5 U");
    assert_eq!(
        simple_emerge.emerge.as_deref(),
        Some("Some(Emerge::creature(mana!(\"{5}{U}\")))")
    );
    assert!(simple_emerge.has_own_rules());

    let filtered_emerge =
        read("Name:ArtifactEmerger\nManaCost:7\nTypes:Creature\nPT:5/5\nK:Emerge:5 U:Artifact");
    assert!(
        filtered_emerge
            .emerge
            .as_deref()
            .is_some_and(|e| e.starts_with("Some(Emerge::from(mana!(\"{5}{U}\")")
                && e.contains("Filter::ARTIFACT")),
        "{:?}",
        filtered_emerge.emerge
    );

    let composite_emerge = read(
        "Name:CompositeEmerger\nManaCost:7\nTypes:Creature\nPT:5/5\nK:Emerge:4 G:Artifact.Green",
    );
    assert!(
        composite_emerge
            .emerge
            .as_deref()
            .is_some_and(|e| e.starts_with("Some(Emerge::from(mana!(\"{4}{G}\")")),
        "{:?}",
        composite_emerge.emerge
    );

    let proto = read(
        "Name:ProtoConstruct\nManaCost:7\nTypes:Artifact Creature\nPT:6/6\nK:Prototype:1 B:3:3",
    );
    assert_eq!(
        proto.prototype.as_deref(),
        Some("Some(Prototype::new(mana!(\"{1}{B}\"), 3, 3))")
    );
    assert!(proto.has_own_rules());

    for bad in [
        "Name:BadDupEmerge\nManaCost:7\nTypes:Creature\nPT:5/5\nK:Emerge:5 U\nK:Emerge:6 U",
        "Name:BadEmptyEmerge\nManaCost:7\nTypes:Creature\nPT:5/5\nK:Emerge:",
        "Name:BadEmergeMana\nManaCost:7\nTypes:Creature\nPT:5/5\nK:Emerge:NotMana",
        "Name:BadEmergeEmptyFilter\nManaCost:7\nTypes:Creature\nPT:5/5\nK:Emerge:5 U:",
        "Name:BadEmergeExtra\nManaCost:7\nTypes:Creature\nPT:5/5\nK:Emerge:5 U:Artifact:Extra",
        "Name:BadDupProto\nManaCost:7\nTypes:Artifact Creature\nPT:6/6\nK:Prototype:1 B:2:2\nK:Prototype:2 B:3:3",
        "Name:BadArityProto\nManaCost:7\nTypes:Artifact Creature\nPT:6/6\nK:Prototype:1 B:3",
        "Name:BadPtProto\nManaCost:7\nTypes:Artifact Creature\nPT:6/6\nK:Prototype:1 B:X:3",
        "Name:BadManaProto\nManaCost:7\nTypes:Artifact Creature\nPT:6/6\nK:Prototype:NotMana:3:3",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn myriad_and_extort_keywords_and_duplicates() {
    let single_myriad = read("Name:LegionScout\nManaCost:2 R\nTypes:Creature\nPT:3/1\nK:Myriad");
    assert_eq!(single_myriad.keywords, ["KeywordSet::MYRIAD"]);
    assert!(single_myriad.abilities.is_empty());

    let double_myriad =
        read("Name:DoubleLegion\nManaCost:2 G\nTypes:Creature\nPT:2/2\nK:Myriad\nK:Myriad");
    assert_eq!(double_myriad.keywords, ["KeywordSet::MYRIAD"]);
    assert_eq!(
        double_myriad.abilities,
        ["static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::MYRIAD))"]
    );

    let single_extort = read("Name:TitheCleric\nManaCost:1 W\nTypes:Creature\nPT:1/3\nK:Extort");
    assert_eq!(single_extort.keywords, ["KeywordSet::EXTORT"]);
    assert!(single_extort.abilities.is_empty());

    let double_extort =
        read("Name:DoubleCleric\nManaCost:1 B\nTypes:Creature\nPT:1/4\nK:Extort\nK:Extort");
    assert_eq!(double_extort.keywords, ["KeywordSet::EXTORT"]);
    assert_eq!(
        double_extort.abilities,
        ["static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::EXTORT))"]
    );

    for bad in [
        "Name:BadMyriad\nManaCost:2 R\nTypes:Creature\nPT:2/2\nK:Myriad:1",
        "Name:BadExtort\nManaCost:1 W\nTypes:Creature\nPT:1/2\nK:Extort:1",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave18b_ring_tempts_clash_detain_and_venture() {
    let ring = read(
        "Name:RingSpell\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ RingTemptsYou | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert!(
        ring.abilities[0].contains("Effect::RingTemptsYou, Effect::draw(1)"),
        "{:?}",
        ring.abilities
    );

    let venture = read(
        "Name:DungeonGuide\nManaCost:2 W\nTypes:Creature Scout\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigV\n\
         SVar:TrigV:DB$ Venture | Defined$ You",
    );
    assert!(
        venture.abilities[0].contains("Effect::VentureIntoTheDungeon"),
        "{:?}",
        venture.abilities
    );

    // Clash with win & loss branches, plus spell-return `Defined$ Parent | Origin$ Stack | Destination$ Hand`.
    let clash_return = read(
        "Name:SparkClash\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 1 | SubAbility$ DBClash\n\
         SVar:DBClash:DB$ Clash | WinSubAbility$ DBBounce | OtherwiseSubAbility$ DBGain\n\
         SVar:DBBounce:DB$ ChangeZone | Defined$ Parent | Origin$ Stack | Destination$ Hand\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ 1",
    );
    assert!(
        clash_return.abilities[0].contains(
            "Effect::Clash { on_win: &[Effect::bounce(TargetSpec::ThisObject)], \
             on_loss: &[Effect::gain_life(1)] }"
        ),
        "{:?}",
        clash_return.abilities
    );

    // Clash after a targeted line where `WinSubAbility$` is an untargeted sweep (`prior_target_used`).
    let clash_after_target = read(
        "Name:ForestClash\nManaCost:3 G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ValidTgts$ Card.YouCtrl | SubAbility$ DBClash\n\
         SVar:DBClash:DB$ Clash | WinSubAbility$ DBUntap\n\
         SVar:DBUntap:DB$ UntapAll | ValidCards$ Forest",
    );
    assert!(
        clash_after_target.abilities[0].contains("Effect::Clash { on_win: &[Effect::UntapAll"),
        "{:?}",
        clash_after_target.abilities
    );

    // Detain: single target, multi-target `TargetMin$ 0`, filter sweep, self, event object.
    let detain_multi = read(
        "Name:ArrestOrder\nManaCost:1 W\nTypes:Sorcery\n\
         A:SP$ Detain | ValidTgts$ Creature.OppCtrl | TargetMin$ 0 | TargetMax$ 2 | IsCurse$ True",
    );
    assert!(
        detain_multi.abilities[0].contains("Effect::Detain { target: TargetSpec::Object(")
            && detain_multi.abilities[0].contains("TargetReq::up_to(")
            && detain_multi.abilities[0].contains(", 2)"),
        "{:?}",
        detain_multi.abilities
    );

    let detain_sweep = read(
        "Name:LawMarshal\nManaCost:3 W U\nTypes:Creature Soldier\nPT:4/4\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDet\n\
         SVar:TrigDet:DB$ Detain | Defined$ Valid Permanent.OppCtrl+nonLand+cmcLE4",
    );
    assert!(
        detain_sweep.abilities[0].contains("Effect::DetainFilter { filter: &DETAIN1 }"),
        "{:?}",
        detain_sweep.abilities
    );

    for bad in [
        "Name:BadRing\nManaCost:B\nTypes:Sorcery\nA:SP$ RingTemptsYou | Defined$ Opponent",
        "Name:BadVenture\nManaCost:W\nTypes:Sorcery\nA:SP$ Venture | Defined$ Opponent",
        "Name:BadClashDef\nManaCost:U\nTypes:Sorcery\nA:SP$ Clash | Defined$ Opponent",
        "Name:BadClashRem\nManaCost:U\nTypes:Sorcery\nA:SP$ Clash | RememberClasher$ True",
        "Name:BadClashTgt\nManaCost:U\nTypes:Sorcery\nA:SP$ Clash | ValidTgts$ Opponent",
        "Name:BadDetainRem\nManaCost:W\nTypes:Sorcery\nA:SP$ Detain | Defined$ Valid Permanent.IsRemembered",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave18b_mobilize_endure_firebend_cipher_champion_and_extra_combat() {
    let mob = read(
        "Name:BannerCaptain\nManaCost:2 R\nTypes:Creature Warrior\nPT:2/2\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigMob\n\
         SVar:TrigMob:DB$ Mobilize | Num$ 2",
    );
    assert!(
        mob.abilities[0].contains("Effect::Mobilize(Amount::Fixed(2))"),
        "{:?}",
        mob.abilities
    );

    let endure = read(
        "Name:SteppeWarden\nManaCost:2 W\nTypes:Creature Soldier\nPT:2/2\n\
         A:AB$ Endure | Cost$ 2 W T | Num$ 3",
    );
    assert!(
        endure.abilities[0].contains("Effect::Endure(Amount::Fixed(3))"),
        "{:?}",
        endure.abilities
    );

    let fire = read(
        "Name:FlameAdept\nManaCost:1 R\nTypes:Creature Monk\nPT:2/1\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigFire\n\
         SVar:TrigFire:DB$ Firebend | Amount$ 2",
    );
    assert!(
        fire.abilities[0].contains("Effect::Firebend(Amount::Fixed(2))"),
        "{:?}",
        fire.abilities
    );

    let cipher = read(
        "Name:ShadowCode\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Draw | NumCards$ 1 | SubAbility$ DBCipher\n\
         SVar:DBCipher:DB$ Cipher",
    );
    assert!(
        cipher.abilities[0].contains("Effect::draw(1), Effect::Cipher"),
        "{:?}",
        cipher.abilities
    );

    let champ = read(
        "Name:FaeKeeper\nManaCost:3 U\nTypes:Creature Faerie\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | ValidCard$ Card.Self | Execute$ TrigRet\n\
         SVar:TrigRet:DB$ ReturnChampioned",
    );
    assert!(
        champ.abilities[0].contains("Effect::ReturnChampioned"),
        "{:?}",
        champ.abilities
    );

    // Relentless Assault shape: main phase guard + extra combat + main2.
    let extra_main = read(
        "Name:SecondWave\nManaCost:2 R R\nTypes:Sorcery\n\
         A:SP$ AddPhase | ExtraPhase$ Combat | FollowedBy$ Main2 | ConditionPhases$ Main1,Main2",
    );
    assert!(
        extra_main.abilities[0]
            .contains("Effect::AdditionalCombatPhase { followed_by_main_phase: true }")
            && extra_main.abilities[0].contains("Condition::DuringCombat"),
        "{:?}",
        extra_main.abilities
    );

    // Combat trigger shape: `AfterPhase$ EndCombat` without extra guard when inside Attacks trigger.
    let extra_combat = read(
        "Name:WarAngel\nManaCost:4 R W\nTypes:Creature Angel\nPT:3/4\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigExtra\n\
         SVar:TrigExtra:DB$ AddPhase | ExtraPhase$ Combat | AfterPhase$ EndCombat",
    );
    assert!(
        extra_combat.abilities[0]
            .contains("Effect::AdditionalCombatPhase { followed_by_main_phase: false }"),
        "{:?}",
        extra_combat.abilities
    );

    for bad in [
        // Endure always acts on `res.source`, never on a spell or triggered copy.
        "Name:BadEndureSpell\nManaCost:1 W\nTypes:Sorcery\nA:SP$ Endure | Num$ 2",
        "Name:BadEndureDef\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Creature.YouCtrl | Execute$ E\n\
         SVar:E:DB$ Endure | Defined$ TriggeredCardLKICopy | Num$ 1",
        "Name:BadCipherPerm\nManaCost:1 U\nTypes:Creature\nPT:1/1\nA:AB$ Cipher | Cost$ T",
        "Name:BadChampSpell\nManaCost:1 W\nTypes:Sorcery\nA:SP$ ReturnChampioned",
        "Name:BadPhaseType\nManaCost:2 U\nTypes:Sorcery\nA:SP$ AddPhase | ExtraPhase$ Upkeep",
        "Name:BadPhaseFollow\nManaCost:2 R\nTypes:Sorcery\nA:SP$ AddPhase | ExtraPhase$ Combat | FollowedBy$ End",
        "Name:BadPhaseFirst\nManaCost:2 R\nTypes:Sorcery\nA:SP$ AddPhase | ExtraPhase$ Combat | ConditionFirstCombat$ True",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave18b_choose_number_choose_player_and_wave9_filters() {
    let choose_num = read(
        "Name:ShapeForm\nManaCost:6\nTypes:Artifact Creature\nPT:0/7\n\
         A:AB$ ChooseNumber | Cost$ T | Defined$ You | Min$ 0 | Max$ 7",
    );
    assert!(
        choose_num.abilities[0].contains("Effect::ChooseNumber { min: 0, max: 7 }"),
        "{:?}",
        choose_num.abilities
    );

    // ChoosePlayer -> Draw for ChosenPlayer + Cleanup ClearChosenPlayer.
    let choose_draw = read(
        "Name:TributeChoice\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ ChoosePlayer | Defined$ You | Choices$ Player.Opponent | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ ChosenPlayer | NumCards$ 1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearChosenPlayer$ True",
    );
    assert!(
        choose_draw.abilities[0].contains(
            "Effect::ChoosePlayerFrom { among: PlayerRel::EachOpponent }, \
             Effect::DrawCardsFor { amount: Amount::Fixed(1), who: PlayerRel::Chosen }"
        ),
        "{:?}",
        choose_draw.abilities
    );

    // The Black Gate shape: ChoosePlayer withMostLife -> CantBeBlockedByPlayerThisTurn.
    let gate = read(
        "Name:ObsidianGate\nTypes:Land\n\
         A:AB$ ChoosePlayer | Cost$ 1 B T | Defined$ You | Choices$ Player.withMostLife | ChoiceTitle$ Choose | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | ValidTgts$ Creature | RememberObjects$ ChosenPlayer & Targeted | ExileOnMoved$ Battlefield | StaticAbilities$ CantBlock\n\
         SVar:CantBlock:Mode$ CantBlockBy | ValidAttacker$ Creature.IsRemembered | ValidBlocker$ Creature.RememberedPlayerCtrl",
    );
    assert!(
        gate.abilities[0].contains("Effect::ChoosePlayerFrom { among: PlayerRel::MostLifeOrTied }")
            && gate.abilities[0]
                .contains("Effect::CantBeBlockedByPlayerThisTurn { by: PlayerRel::Chosen }"),
        "{:?}",
        gate.abilities
    );

    // ChoosePlayer after an object-targeted line keeps the original object target intact.
    let destroy_then_choose = read(
        "Name:BargainDoom\nManaCost:2 B\nTypes:Instant\n\
         A:SP$ Destroy | ValidTgts$ Creature | SubAbility$ DBChoose\n\
         SVar:DBChoose:DB$ ChoosePlayer | Choices$ Opponent | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ ChosenPlayer | NumCards$ 1",
    );
    assert!(
        destroy_then_choose.abilities[0]
            .contains("Effect::destroy(TargetSpec::Object(&Filter::CREATURE))")
            && destroy_then_choose.abilities[0]
                .contains("Effect::ChoosePlayerFrom { among: PlayerRel::EachOpponent }")
            && destroy_then_choose.abilities[0].contains(
                "Effect::DrawCardsFor { amount: Amount::Fixed(1), who: PlayerRel::Chosen }"
            ),
        "{:?}",
        destroy_then_choose.abilities
    );

    // Wave 9–10 filters: RingBearer, YOUR_RING_BEARER, SameManaValueAsSource, SharesColorWithSource.
    assert_eq!(filter("Card.IsRingbearer"), "Filter::RingBearer");
    assert_eq!(
        filter("Card.IsRingbearer+YouCtrl"),
        "Filter::YOUR_RING_BEARER"
    );
    assert_eq!(
        filter("Card.YouCtrl+IsRingbearer"),
        "Filter::YOUR_RING_BEARER"
    );
    assert_eq!(filter("RingBearer.YouCtrl"), "Filter::YOUR_RING_BEARER");
    assert_eq!(
        filter("Creature.!IsRingbearer"),
        "Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::RingBearer)])"
    );
    assert_eq!(
        filter("Card.SharesCMCWith"),
        "Filter::SameManaValueAsSource"
    );
    assert_eq!(
        filter("Card.SharesCMCWith Self"),
        "Filter::SameManaValueAsSource"
    );
    assert_eq!(
        filter("Card.SharesColorWith"),
        "Filter::SharesColorWithSource"
    );
    assert_eq!(
        filter("Card.!SharesColorWith Self"),
        "Filter::Not(&Filter::SharesColorWithSource)"
    );

    let same_cmc_svar = read(
        "Name:EchoSearch\nManaCost:1 U U\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Card.cmcEQX\n\
         SVar:X:Count$CardManaCost",
    );
    assert!(
        same_cmc_svar.abilities[0].contains("Filter::SameManaValueAsSource")
            || same_cmc_svar
                .statics
                .contains("Filter::SameManaValueAsSource"),
        "{:?} / {}",
        same_cmc_svar.abilities,
        same_cmc_svar.statics
    );

    for bad in [
        "Name:BadNumSpell\nManaCost:U\nTypes:Sorcery\nA:SP$ ChooseNumber | Min$ 0 | Max$ 3",
        "Name:BadNumSecret\nManaCost:2 U\nTypes:Artifact\nA:AB$ ChooseNumber | Cost$ T | Min$ 0 | Max$ 3 | Secretly$ True",
        "Name:BadNumOrder\nManaCost:2 U\nTypes:Artifact\nA:AB$ ChooseNumber | Cost$ T | Min$ 5 | Max$ 2",
        "Name:BadChooseRandom\nManaCost:1 R\nTypes:Sorcery\nA:SP$ ChoosePlayer | Choices$ Opponent | Random$ True | SubAbility$ D\nSVar:D:DB$ Draw | Defined$ ChosenPlayer | NumCards$ 1",
        "Name:BadChooseUnused\nManaCost:1 U\nTypes:Sorcery\nA:SP$ ChoosePlayer | Choices$ Opponent | SubAbility$ D\nSVar:D:DB$ Draw | Defined$ You | NumCards$ 1",
        "Name:BadChooseOppDef\nManaCost:1 U\nTypes:Sorcery\nA:SP$ ChoosePlayer | Defined$ Opponent | Choices$ Player | SubAbility$ D\nSVar:D:DB$ Draw | Defined$ ChosenPlayer | NumCards$ 1",
        "Name:BadSharesImprint\nManaCost:2 U\nTypes:Sorcery\nA:SP$ DestroyAll | ValidCards$ Creature.SharesCMCWith Imprinted",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave18b_target_min_zero_and_remember_this_way_chains() {
    // Omitted TargetMax$ with TargetMin$ 0 defaults to up to 1.
    let up_to_one_omitted = read(
        "Name:OptionalZap\nManaCost:R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | TargetMin$ 0 | NumDmg$ 2",
    );
    assert!(
        up_to_one_omitted.abilities[0].contains("TargetReq::up_to_one("),
        "{:?}",
        up_to_one_omitted.abilities
    );

    // Multi-target Animate converts `Effect::continuous(&Filter::This, ...)` -> `Effect::ModifyTargets`.
    let multi_animate = read(
        "Name:TwinAwaken\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ Animate | ValidTgts$ Land.YouCtrl | TargetMin$ 0 | TargetMax$ 2 | Power$ 3 | Toughness$ 3 | Types$ Creature",
    );
    assert!(
        multi_animate.abilities[0].contains("TargetReq::up_to(")
            && multi_animate.abilities[0].contains(", 2)")
            && multi_animate.abilities[0].contains("Effect::ModifyTargets {")
            && !multi_animate.abilities[0].contains("Effect::continuous(&Filter::This"),
        "{:?}",
        multi_animate.abilities
    );

    // TargetOnly with `TargetMin$ 0 | TargetMax$ 2` chaining into Pump.
    let target_only_multi = read(
        "Name:BoostPair\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 2 | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | Defined$ Targeted | NumAtt$ 1 | NumDef$ 1",
    );
    assert!(
        target_only_multi.abilities[0].contains("TargetReq::up_to(")
            && target_only_multi.abilities[0].contains(", 2)")
            && target_only_multi.abilities[0].contains("Effect::PumpTarget"),
        "{:?}",
        target_only_multi.abilities
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave18b_remember_this_way_chains() {
    // DestroyAll | RememberDestroyed$ True -> GainLife (`RememberedLKI$Valid Card.YouCtrl/Times.2`).
    let sweep_gain = read(
        "Name:MercyWrath\nManaCost:3 W W\nTypes:Sorcery\n\
         A:SP$ DestroyAll | ValidCards$ Creature | RememberDestroyed$ True | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:RememberedLKI$Valid Card.YouCtrl/Times.2",
    );
    assert!(
        sweep_gain.abilities[0].contains("Effect::destroy_all(&Filter::CREATURE)")
            && sweep_gain.abilities[0]
                .contains("Amount::Times { base: &Amount::ThisWay(ThisWay::Destroyed(")
            && sweep_gain.abilities[0].contains("factor: 2"),
        "{:?}",
        sweep_gain.abilities
    );

    // DestroyAll | RememberDestroyed$ True -> PutCounter on Self (`Remembered$Amount`).
    let sweep_counters = read(
        "Name:RelicEater\nManaCost:4 G G\nTypes:Creature Beast\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigD\n\
         SVar:TrigD:DB$ DestroyAll | ValidCards$ Artifact,Enchantment | RememberDestroyed$ True | SubAbility$ DBPC\n\
         SVar:DBPC:DB$ PutCounter | CounterType$ P1P1 | CounterNum$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        sweep_counters.abilities[0].contains(
            "Effect::AddCounterFilter { filter: &Filter::This, kind: CounterKind::P1P1, \
             amount: Amount::ThisWay(ThisWay::Destroyed(&Filter::Any)) }"
        ),
        "{:?}",
        sweep_counters.abilities
    );

    // Mill | RememberMilled$ True -> coalesced `ConditionDefined$ Remembered` (LoseLife + GainLife).
    let mill_cond = read(
        "Name:GraveWhisper\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ Mill | NumCards$ 3 | Defined$ You | RememberMilled$ True | SubAbility$ DBLose\n\
         SVar:DBLose:DB$ LoseLife | Defined$ Player.Opponent | LifeAmount$ 2 | ConditionDefined$ Remembered | ConditionPresent$ Creature | ConditionCompare$ GE1 | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ 2 | ConditionDefined$ Remembered | ConditionPresent$ Creature | ConditionCompare$ GE1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        mill_cond.abilities[0].contains(
            "Condition::AmountAtLeast(Amount::ThisWay(ThisWay::Milled(&Filter::CREATURE)), 1)"
        ) && mill_cond.abilities[0].contains("Effect::LoseLife")
            && mill_cond.abilities[0].contains("Effect::gain_life(2)"),
        "{:?}",
        mill_cond.abilities
    );

    // Mill X -> targeted LoseLife using `Count$ValidGraveyard Creature.IsRemembered/Times.2`.
    let mill_count = read(
        "Name:DarkMill\nManaCost:X B\nTypes:Sorcery\n\
         A:SP$ Mill | NumCards$ X | Defined$ You | RememberMilled$ True | SubAbility$ DBLose\n\
         SVar:DBLose:DB$ LoseLife | ValidTgts$ Player | LifeAmount$ Y | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$xPaid\n\
         SVar:Y:Count$ValidGraveyard Creature.IsRemembered/Times.2",
    );
    assert!(
        mill_count.abilities[0]
            .contains("Effect::Mill { amount: Amount::X, target: PlayerRel::You }")
            && mill_count.abilities[0].contains(
                "Effect::LoseLife { amount: Amount::Times { \
                 base: &Amount::ThisWay(ThisWay::Milled(&Filter::CREATURE)), factor: 2 }, \
                 target: PlayerRel::Chosen }"
            ),
        "{:?}",
        mill_count.abilities
    );

    // Discard up to 2 -> draw that many (`DiscardUpToThenDraw`).
    let discard_up_to = read(
        "Name:RummageTwo\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ Discard | Defined$ You | NumCards$ 2 | Optional$ True | Mode$ TgtChoose | RememberDiscarded$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        discard_up_to.abilities[0].contains("Effect::DiscardUpToThenDraw { count: 2 }"),
        "{:?}",
        discard_up_to.abilities
    );

    // Discard hand -> draw that many plus one (`ThisWay::Discarded`).
    let discard_hand = read(
        "Name:FreshHand\nManaCost:2 R W\nTypes:Instant\n\
         A:SP$ Discard | Defined$ You | Mode$ Hand | RememberDiscarded$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount/Plus.1",
    );
    assert!(
        discard_hand.abilities[0].contains("Effect::DiscardHand { who: PlayerRel::You }")
            && discard_hand.abilities[0]
                .contains("Amount::Plus { base: &Amount::ThisWay(ThisWay::Discarded), offset: 1 }"),
        "{:?}",
        discard_hand.abilities
    );

    // Sacrifice with `StrictAmount$ True` (count >= 2).
    let sac_strict = read(
        "Name:SwampTax\nManaCost:4 B B\nTypes:Creature Demon\nPT:5/5\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ TrigSac\n\
         SVar:TrigSac:DB$ Sacrifice | Defined$ You | Amount$ 2 | SacValid$ Land | StrictAmount$ True | RememberSacrificed$ True | SubAbility$ DBTap\n\
         SVar:DBTap:DB$ Tap | Defined$ Self | ConditionCheckSVar$ X | ConditionSVarCompare$ LT2 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        sac_strict.abilities[0]
            .contains("Effect::SacrificeNOrElse { filter: &Filter::LAND, count: 2"),
        "{:?}",
        sac_strict.abilities
    );

    // Multi-player mill -> total cards milled across all players (`ThisWay::Milled`).
    let multi_mill = read(
        "Name:SharedMill\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Mill | Defined$ Player | NumCards$ 3 | RememberMilled$ True | SubAbility$ G\n\
         SVar:G:DB$ GainLife | Defined$ You | LifeAmount$ X\nSVar:X:Remembered$Amount",
    );
    assert!(
        multi_mill.abilities[0]
            .contains("Effect::Mill { amount: Amount::Fixed(3), target: PlayerRel::EachPlayer }")
            && multi_mill.abilities[0].contains("Amount::ThisWay(ThisWay::Milled(&Filter::Any))"),
        "{:?}",
        multi_mill.abilities
    );

    // Refusals: multi-player mill EQ1, multi-player discard Remember*, EQ1 on multi-mill, TargetOnly -> GraveyardToTop multi.
    for bad in [
        "Name:BadMultiMillEq1\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Mill | Defined$ Player | NumCards$ 1 | RememberMilled$ True | SubAbility$ G\n\
         SVar:G:DB$ GainLife | Defined$ You | LifeAmount$ 2 | ConditionDefined$ Remembered | ConditionPresent$ Creature | ConditionCompare$ EQ1",
        "Name:BadMultiDiscard\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ Discard | Defined$ Player | Mode$ Hand | RememberDiscarded$ True | SubAbility$ D\n\
         SVar:D:DB$ Draw | Defined$ Player | NumCards$ X\nSVar:X:Remembered$Amount",
        "Name:BadMillEq1\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ Mill | Defined$ You | NumCards$ 3 | RememberMilled$ True | SubAbility$ G\n\
         SVar:G:DB$ GainLife | Defined$ You | LifeAmount$ 2 | ConditionDefined$ Remembered | ConditionPresent$ Creature | ConditionCompare$ EQ1",
        "Name:BadTargetOnlyGyTop\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Card | TgtZone$ Graveyard | TargetMin$ 0 | TargetMax$ 2 | SubAbility$ R\n\
         SVar:R:DB$ ChangeZone | Defined$ Targeted | Origin$ Graveyard | Destination$ Library | LibraryPosition$ 0",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[allow(clippy::too_many_lines)]
#[test]
fn wave19b_filters_addendum_choose_card_reveal_hand_and_token_refusals() {
    // `cmcLEX` / `cmcEQX` on an X-cost permanent's ETB self-trigger (`Invasion of Ikoria` shape).
    let etb_x_search = read(
        "Name:XSearchBeast\nManaCost:X G G\nTypes:Creature Beast\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigSearch\n\
         SVar:TrigSearch:DB$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeType$ Creature.nonGoblin+cmcLEX\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        etb_x_search.statics.contains("Filter::CmcAtMostX"),
        "{}\n{:?}",
        etb_x_search.statics,
        etb_x_search.abilities
    );
    let etb_eq_x = read(
        "Name:XExactTutor\nManaCost:X W\nTypes:Creature Cleric\nPT:1/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigSearch\n\
         SVar:TrigSearch:DB$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Artifact.cmcEQX\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        etb_eq_x.statics.contains("Filter::CmcExactlyX"),
        "{}\n{:?}",
        etb_eq_x.statics,
        etb_eq_x.abilities
    );

    // Addendum (`ConditionPlayerTurn$ True | ConditionPhases$ Main1,Main2 | ConditionDefined$ Self | ConditionPresent$ Card.wasCast`).
    let addendum = read(
        "Name:LawBounce\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Creature.OppCtrl | Origin$ Battlefield | Destination$ Hand | SubAbility$ DBAddendum\n\
         SVar:DBAddendum:DB$ Draw | ConditionPlayerTurn$ True | ConditionPhases$ Main1,Main2 | ConditionDefined$ Self | ConditionPresent$ Card.wasCast\n\
         Oracle:Return target creature an opponent controls to its owner's hand. Addendum — If you cast this spell during your main phase, draw a card.",
    );
    assert!(
        addendum.abilities[0].contains(
            "Condition::All(&[Condition::SourceWasCastByYou, Condition::YourTurn, \
             Condition::Not(&Condition::BeforeStep(StepKind::FirstMain)), \
             Condition::BeforeStep(StepKind::End), Condition::Not(&Condition::DuringCombat)])"
        ),
        "{:?}",
        addendum.abilities
    );

    // Standalone `ConditionPresent$ Card.wasCastByYou` and `ConditionPhases$ Main1,Main2`.
    let cast_by_you = read(
        "Name:CastBonus\nManaCost:2 G\nTypes:Creature Elf\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1 | ConditionDefined$ Self | ConditionPresent$ Card.wasCastByYou",
    );
    assert!(
        cast_by_you.abilities[0].contains("Condition::SourceWasCastByYou"),
        "{:?}",
        cast_by_you.abilities
    );
    let main_phase_draw = read(
        "Name:MainDraw\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ GainLife | LifeAmount$ 2 | SubAbility$ DBMainDraw\n\
         SVar:DBMainDraw:DB$ Draw | NumCards$ 1 | ConditionPhases$ Main1,Main2 | ConditionPlayerTurn$ True",
    );
    assert!(
        main_phase_draw.abilities[0].contains("Effect::draw(1)")
            && main_phase_draw.abilities[0].contains(
                "Condition::All(&[Condition::YourTurn, \
                 Condition::Not(&Condition::BeforeStep(StepKind::FirstMain)), \
                 Condition::BeforeStep(StepKind::End), Condition::Not(&Condition::DuringCombat)])"
            ),
        "{:?}",
        main_phase_draw.abilities
    );

    // `ChooseCard`: `Defined$ Opponent`, `Mandatory$ False`, `ControlledByPlayer$ You`, `IsCurse$`, cleanup tail.
    let opp_choose = read(
        "Name:OppPick\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ ChooseCard | Defined$ Opponent | Choices$ Creature | ControlledByPlayer$ You | Mandatory$ False | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ ChangeZone | Defined$ ChosenCard | Origin$ Battlefield | Destination$ Exile | IsCurse$ True | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearChosenCard$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert!(
        opp_choose.abilities[0].contains("RepeatDomain::EachOpponent")
            && opp_choose.abilities[0].contains("min: 0, max: Amount::Fixed(1)")
            && opp_choose.abilities[0].contains("movement: ChooseMove::Exile")
            && opp_choose.abilities[0].contains("Effect::draw(1)"),
        "{:?}",
        opp_choose.abilities
    );

    // `RevealHand`: `Look$ True` (`Peek` shape + inside `ChoosePlayer` chain) and public `RevealHand` with `IsCurse$`.
    let peek = read(
        "Name:HandPeek\nManaCost:U\nTypes:Instant\n\
         A:SP$ RevealHand | ValidTgts$ Player | Look$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1\n\
         Oracle:Look at target player's hand. Draw a card.",
    );
    assert!(
        peek.abilities[0].contains("Effect::LookAtChosenHand, Effect::draw(1)"),
        "{:?}",
        peek.abilities
    );
    let spyglass = read(
        "Name:HandSpyglass\nManaCost:2\nTypes:Artifact\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigLook\n\
         SVar:TrigLook:DB$ ChoosePlayer | Defined$ You | Choices$ Opponent | SubAbility$ DBLook\n\
         SVar:DBLook:DB$ RevealHand | Defined$ ChosenPlayer | Look$ True | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearChosenPlayer$ True",
    );
    assert!(
        spyglass.abilities[0].contains(
            "Effect::ChoosePlayerFrom { among: PlayerRel::EachOpponent }, Effect::LookAtChosenHand"
        ),
        "{:?}",
        spyglass.abilities
    );
    let public_rev = read(
        "Name:HandShow\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ RevealHand | ValidTgts$ Opponent | IsCurse$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1\n\
         Oracle:Target opponent reveals their hand. Draw a card.",
    );
    assert!(
        public_rev.abilities[0]
            .contains("Effect::RevealHand { who: PlayerRel::Chosen }, Effect::draw(1)"),
        "{:?}",
        public_rev.abilities
    );

    // Wave 13a `Effect::ChooseCardEx` unlocks `ChoiceZone$ Hand` (Exile) and
    // `ControlledByPlayer$ Chooser` (`Filter::ControlledByChooser`).
    let choose_hand = read(
        "Name:GoodChooseHand\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ ChooseCard | ChoiceZone$ Hand | Choices$ Creature.YouOwn | Mandatory$ True | SubAbility$ M\n\
         SVar:M:DB$ ChangeZone | Defined$ ChosenCard | Origin$ Hand | Destination$ Exile",
    );
    assert!(
        choose_hand.abilities[0].contains("Effect::ChooseCardEx")
            && choose_hand.abilities[0].contains("zone: ChooseZone::Hand(PlayerRel::You)")
            && choose_hand.abilities[0].contains("movement: ChooseMove::Exile"),
        "{:?}",
        choose_hand.abilities
    );
    let choose_opp_ctrl = read(
        "Name:GoodChooseOppCtrl\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ ChooseCard | Defined$ Opponent | Choices$ Creature | ControlledByPlayer$ Chooser | Mandatory$ True | SubAbility$ M\n\
         SVar:M:DB$ Destroy | Defined$ ChosenCard",
    );
    assert!(
        choose_opp_ctrl.abilities[0]
            .contains("Effect::RepeatEach { domain: RepeatDomain::EachOpponent")
            && choose_opp_ctrl.abilities[0].contains("Effect::ChooseCardEx")
            && choose_opp_ctrl
                .statics
                .contains("Filter::ControlledByChooser"),
        "{:?} / {:?}",
        choose_opp_ctrl.abilities,
        choose_opp_ctrl.statics
    );

    // Honest refusals: `modified`, bare `wasCast` filter, non-Addendum `Card.wasCast`,
    // `RevealHand` `Look$ True` on non-chosen seat, dynamic `Token.{TokenPower,TokenToughness}`.
    for bad in [
        "Name:BadWasCastFilter\nManaCost:1 U\nTypes:Instant\nA:SP$ Destroy | ValidTgts$ Creature.wasCast",
        "Name:BadNonYouWasCast\nManaCost:3 W\nTypes:Artifact\nT:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ D\nSVar:D:DB$ Draw | ConditionDefined$ Self | ConditionPresent$ Card.wasCast\nOracle:When this enters, if it was cast, draw a card.",
        "Name:BadLookNonChosen\nManaCost:2 U\nTypes:Creature Ninja\nPT:1/2\nT:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ L\nSVar:L:DB$ RevealHand | Defined$ TriggeredTarget | Look$ True",
        "Name:BadSizedToken\nManaCost:X G\nTypes:Sorcery\nA:SP$ Token | TokenScript$ g_x_x_wurm | TokenPower$ X | TokenToughness$ X\nSVar:X:Count$xPaid",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave20b_repeat_process_and_refusals() {
    let cats = cats();
    let toks = tokens();
    let read_tok = |text: &str| {
        let script = parse(text);
        transcode(&script, &cats, Some(&toks)).unwrap_or_else(|| {
            panic!(
                "expected translation, refused: {:?}\n{text}",
                refusal_reason(&script, &cats, Some(&toks))
            )
        })
    };

    // Bounded MaxRepeat$ X + FlipCoin token creation.
    let flock = read_tok(
        "Name:BoundedFlip\nManaCost:X R\nTypes:Sorcery\n\
         A:SP$ Repeat | MaxRepeat$ X | RepeatSubAbility$ DBFlip | Mandatory$ True\n\
         SVar:DBFlip:DB$ FlipCoin | WinSubAbility$ DBTok\n\
         SVar:DBTok:DB$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ You\n\
         SVar:X:Count$xPaid\n\
         Oracle:Flip X coins. For each flip you win, create a 1/1 red Goblin creature token.",
    );
    assert!(
        flock.abilities[0].contains(
            "Effect::RepeatProcess { effects: &[Effect::FlipCoin { \
             won: &[Effect::CreateToken { token: &generated_tokens::GOBLIN_1_1_RED }], lost: &[] }], \
             max_times: Some(Amount::X), pre_condition: None, post_condition: None, \
             optional: false, optional_before_first: false, per_pass_this_way: false }"
        ),
        "{:?}",
        flock.abilities
    );

    // Bounded MaxRepeat$ 2 + battlefield RepeatPresent$ / RepeatCompare$.
    let land_repeat = read_tok(
        "Name:LandRepeat\nManaCost:2 G\nTypes:Creature Wizard\nPT:2/2\n\
         A:AB$ Repeat | Cost$ T | RepeatSubAbility$ DBDraw | MaxRepeat$ 2 \
         | RepeatPresent$ Land.YouCtrl | RepeatCompare$ GE8\n\
         SVar:DBDraw:DB$ Draw | NumCards$ 1\n\
         Oracle:{T}: Draw a card. If you control eight or more lands, repeat this process once.",
    );
    assert!(
        land_repeat.abilities[0].contains("Effect::RepeatProcess { effects: &[Effect::draw(1)]")
            && land_repeat.abilities[0].contains("max_times: Some(Amount::Fixed(2))")
            && land_repeat.abilities[0].contains(
                "post_condition: Some(Condition::AmountAtLeast(\
                 Amount::CountOf { filter: &Filter::YOUR_LAND, zone: ZoneSel::Battlefield }, 8))"
            ),
        "{:?}",
        land_repeat.abilities
    );

    // Optional repeat with per-pass reveal + mana value life loss (Ad Nauseam shape).
    let ad_naus = read_tok(
        "Name:RevealRepeat\nManaCost:3 B B\nTypes:Instant\n\
         A:SP$ Repeat | RepeatSubAbility$ DBDig | RepeatOptional$ True\n\
         SVar:DBDig:DB$ Dig | DigNum$ 1 | Reveal$ True | ChangeNum$ All | ChangeValid$ Card \
         | DestinationZone$ Hand | RememberChanged$ True | SubAbility$ DBLose\n\
         SVar:DBLose:DB$ LoseLife | LifeAmount$ X | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost\n\
         Oracle:Reveal the top card of your library and put it into your hand. \
         You lose life equal to its mana value. You may repeat this process any number of times.",
    );
    assert!(
        ad_naus.abilities[0].contains(
            "Effect::RepeatProcess { effects: &[\
             Effect::RevealTopAllMatching { count: 1, filter: &Filter::Any, found: Find::HAND }, \
             Effect::LoseLife { amount: Amount::ThisWay(ThisWay::RevealedManaValue), target: PlayerRel::You }], \
             max_times: None, pre_condition: None, post_condition: None, \
             optional: true, optional_before_first: false, per_pass_this_way: true }"
        ),
        "{:?}",
        ad_naus.abilities
    );

    // Optional before first ("as many times as you choose").
    let before_first = read_tok(
        "Name:ChoiceLoop\nManaCost:4 R\nTypes:Sorcery\n\
         A:SP$ Repeat | RepeatSubAbility$ DBExile | RepeatOptional$ True\n\
         SVar:DBExile:DB$ Dig | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile\n\
         Oracle:As many times as you choose, you may exile the top card of your library.",
    );
    assert!(
        before_first.abilities[0]
            .contains("optional: true, optional_before_first: true, per_pass_this_way: false"),
        "{:?}",
        before_first.abilities
    );
}

#[test]
fn wave20b_repeat_process_remembered_post_and_refusals() {
    // Per-pass remembered post-condition (mill until a land is milled).
    let mill_loop = read(
        "Name:MillUntilLand\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ Repeat | RepeatSubAbility$ DBMill | RepeatDefined$ Remembered \
         | RepeatPresent$ Land | RepeatCompare$ EQ0 | SubAbility$ DBClean\n\
         SVar:DBMill:DB$ Mill | Defined$ You | NumCards$ 1 | RememberMilled$ True\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:Mill a card. Repeat this process until you mill a land card.",
    );
    assert!(
        mill_loop.abilities[0]
            .contains("Effect::Mill { amount: Amount::Fixed(1), target: PlayerRel::You }")
            && mill_loop.abilities[0].contains(
                "post_condition: Some(Condition::Not(&Condition::AmountAtLeast(\
                 Amount::ThisWay(ThisWay::Milled(&Filter::LAND)), 1)))"
            )
            && mill_loop.abilities[0].contains("per_pass_this_way: true"),
        "{:?}",
        mill_loop.abilities
    );

    // Refusals: unbounded repeat, opponent decider, zero cap, nested targets, unknown param.
    for bad in [
        "Name:BadUnbounded\nManaCost:1 U\nTypes:Sorcery\nA:SP$ Repeat | RepeatSubAbility$ D\nSVar:D:DB$ Draw | NumCards$ 1",
        "Name:BadDecider\nManaCost:1 R\nTypes:Sorcery\nA:SP$ Repeat | RepeatSubAbility$ D | RepeatOptional$ True | RepeatOptionalDecider$ Opponent\nSVar:D:DB$ Draw | NumCards$ 1",
        "Name:BadZeroCap\nManaCost:1 R\nTypes:Sorcery\nA:SP$ Repeat | RepeatSubAbility$ D | MaxRepeat$ 0\nSVar:D:DB$ Draw | NumCards$ 1",
        "Name:BadNestedTgt\nManaCost:1 R\nTypes:Sorcery\nA:SP$ Repeat | RepeatSubAbility$ D | MaxRepeat$ 2\nSVar:D:DB$ DealDamage | ValidTgts$ Any | NumDmg$ 1",
        "Name:BadParam\nManaCost:1 R\nTypes:Sorcery\nA:SP$ Repeat | RepeatSubAbility$ D | MaxRepeat$ 2 | FooBar$ True\nSVar:D:DB$ Draw | NumCards$ 1",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave20b_exile_targets_tokens_and_this_way_exiled() {
    let cats = cats();
    let toks = tokens();
    let read_tok = |text: &str| {
        let script = parse(text);
        transcode(&script, &cats, Some(&toks)).unwrap_or_else(|| {
            panic!(
                "expected translation, refused: {:?}\n{text}",
                refusal_reason(&script, &cats, Some(&toks))
            )
        })
    };

    // Curse of the Swine shape: RememberLKI$ True + UseImprinted$ True + ImprintedController.
    let swine = read_tok(
        "Name:ExileSwap\nManaCost:X U U\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Creature | TargetMin$ X | TargetMax$ X \
         | Origin$ Battlefield | Destination$ Exile | RememberLKI$ True | SubAbility$ DBRep\n\
         SVar:DBRep:DB$ RepeatEach | UseImprinted$ True | DefinedCards$ DirectRemembered \
         | RepeatSubAbility$ DBTok | SubAbility$ DBClean | ChangeZoneTable$ True\n\
         SVar:DBTok:DB$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ ImprintedController\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$xPaid\n\
         Oracle:Exile X target creatures. For each creature exiled this way, its controller creates a 1/1 red Goblin creature token.",
    );
    assert!(
        swine.abilities[0].contains(
            "Effect::ExileTargetsCreateTokens { token: &generated_tokens::GOBLIN_1_1_RED }"
        ) && swine.abilities[0].contains("TargetReq::x_targets("),
        "{:?}",
        swine.abilities
    );

    // Midnight Ritual shape: TargetMin$ X | TargetMax$ X + DB$ Token | TokenAmount$ X -> ThisWay::Exiled.
    let ritual = read_tok(
        "Name:GraveExileTokens\nManaCost:X 2 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Creature.YouOwn \
         | TargetMin$ X | TargetMax$ X | SubAbility$ DBTok\n\
         SVar:DBTok:DB$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ You | TokenAmount$ X\n\
         SVar:X:Count$xPaid\n\
         Oracle:Exile X target creature cards from your graveyard. For each creature card exiled this way, create a 1/1 red Goblin creature token.",
    );
    assert!(
        ritual.abilities[0].contains("Effect::exile(TargetSpec::CardInGraveyard(")
            && ritual.abilities[0].contains(
                "Effect::CreateTokenN { token: &generated_tokens::GOBLIN_1_1_RED, \
                 amount: Amount::ThisWay(ThisWay::Exiled(&Filter::Any)) }"
            ),
        "{:?}",
        ritual.abilities
    );

    // Exile from graveyard -> AddManaAmountFor using Remembered$Amount + Count$ValidExile.
    let exile_mana = read_tok(
        "Name:ExileForMana\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Card.YouOwn \
         | TargetMin$ 0 | TargetMax$ Yard | RememberChanged$ True | SubAbility$ DBMana\n\
         SVar:DBMana:DB$ Mana | Produced$ R | Amount$ N | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:Yard:Count$ValidGraveyard Card.YouOwn\n\
         SVar:N:Remembered$Amount\n\
         Oracle:Exile any number of target cards from your graveyard. Add {R} for each card exiled this way.",
    );
    assert!(
        exile_mana.abilities[0].contains(
            "Effect::AddManaAmountFor { who: PlayerRel::You, color: ManaColor::Red, \
             amount: Amount::ThisWay(ThisWay::ExiledFromGraveyard(&Filter::Any)) }"
        ),
        "{:?}",
        exile_mana.abilities
    );

    // Stack -> Exile with RememberChanged$ True + Count$ValidExile -> ThisWay::Exiled.
    let stack_exile = read_tok(
        "Name:ExileSpellGain\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Stack | Destination$ Exile | ValidTgts$ Card \
         | TargetType$ Spell | TgtZone$ Stack | RememberChanged$ True | SubAbility$ DBLose\n\
         SVar:DBLose:DB$ LoseLife | Defined$ Opponent | LifeAmount$ N | IsCurse$ True | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:N:Count$ValidExile Card.IsRemembered\n\
         Oracle:Exile target spell. Each opponent loses 1 life for each card exiled this way.",
    );
    assert!(
        stack_exile.abilities[0].contains(
            "Effect::LoseLife { amount: Amount::ThisWay(ThisWay::Exiled(&Filter::Any)), \
             target: PlayerRel::Opponent }"
        ),
        "{:?}",
        stack_exile.abilities
    );

    // Refusals: wrong token owner / amount on exile-for-tokens, Play.* (until 11b), TokenPower, modified.
    for bad in [
        "Name:BadOwnerSwap\nManaCost:X U\nTypes:Sorcery\nA:SP$ ChangeZone | ValidTgts$ Creature | TargetMin$ X | TargetMax$ X | Origin$ Battlefield | Destination$ Exile | RememberLKI$ True | SubAbility$ R\nSVar:R:DB$ RepeatEach | UseImprinted$ True | DefinedCards$ DirectRemembered | RepeatSubAbility$ T\nSVar:T:DB$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ You\nSVar:X:Count$xPaid",
        "Name:BadTwoSwap\nManaCost:X U\nTypes:Sorcery\nA:SP$ ChangeZone | ValidTgts$ Creature | TargetMin$ X | TargetMax$ X | Origin$ Battlefield | Destination$ Exile | RememberLKI$ True | SubAbility$ R\nSVar:R:DB$ RepeatEach | UseImprinted$ True | DefinedCards$ DirectRemembered | RepeatSubAbility$ T\nSVar:T:DB$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ ImprintedController | TokenAmount$ 2\nSVar:X:Count$xPaid",
        "Name:BadPlaySub\nManaCost:2 R\nTypes:Sorcery\nA:SP$ Play | Defined$ Remembered | SubAbility$ D\nSVar:D:DB$ Draw | NumCards$ 1",
        "Name:BadDynamicTok\nManaCost:X G\nTypes:Sorcery\nA:SP$ Token | TokenScript$ r_1_1_goblin | TokenPower$ X | TokenToughness$ X\nSVar:X:Count$xPaid",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave20b_hand_bottom_change_zone_and_mill_chains() {
    // Form A (Volcanic Spite): Hand -> Bottom of library, if you do, draw a card.
    let spite = read(
        "Name:SpiteBurn\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 3 | SubAbility$ DBBottom\n\
         SVar:DBBottom:DB$ ChangeZone | Origin$ Hand | Destination$ Library | LibraryPosition$ -1 \
         | ChangeNum$ 1 | RememberChanged$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionDefined$ Remembered | ConditionPresent$ Card \
         | ConditionCompare$ GE1 | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:Deals 3 damage to target creature. You may put a card from your hand on the bottom of your library. If you do, draw a card.",
    );
    assert!(
        spite.abilities[0].contains(
            "Effect::BottomCardFromHand { player: PlayerRel::You, filter: &Filter::Any, \
             then: &[Effect::draw(1)] }"
        ),
        "{:?}",
        spite.abilities
    );

    // Form B (Valakut Awakening): put any number from hand on bottom, draw that many plus one.
    let awake = read(
        "Name:BottomWheel\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Hand | Destination$ Library | LibraryPosition$ -1 \
         | ChangeType$ Card | ChangeNum$ X | RememberChanged$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ Y | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$ValidHand Card.YouOwn\n\
         SVar:Y:Count$RememberedSize/Plus.1\n\
         Oracle:Put any number of cards from your hand on the bottom of your library, then draw that many cards plus one.",
    );
    assert!(
        awake.abilities[0].contains("Effect::PutAnyFromHandOnBottom")
            && awake.abilities[0].contains(
                "Effect::DrawCards { amount: Amount::Plus { \
                 base: &Amount::ThisWay(ThisWay::PutIntoLibraryFromHand), offset: 1 } }"
            ),
        "{:?}",
        awake.abilities
    );

    // Self Graveyard -> Exile without Defined$ (`defaults_to_self_zone_change`).
    let self_gy_exile = read(
        "Name:SelfGyExile\nManaCost:1 B\nTypes:Creature Goblin\nPT:1/1\n\
         A:AB$ ChangeZone | Cost$ 1 | Origin$ Graveyard | Destination$ Exile | ActivationZone$ Graveyard\n\
         Oracle:{1}: Exile this card from your graveyard.",
    );
    assert!(
        self_gy_exile.abilities[0].contains("Effect::ExileSource"),
        "{:?}",
        self_gy_exile.abilities
    );

    // Hidden Graveyard -> Exile with RememberChanged$ True + LE0 else branch (Entrails Feaster shape).
    let feaster = read(
        "Name:GraveFeaster\nManaCost:B\nTypes:Creature Goblin\nPT:1/1\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ TrigEx | TriggerZones$ Battlefield\n\
         SVar:TrigEx:DB$ ChangeZone | ChangeType$ Creature | ChangeNum$ 1 | Origin$ Graveyard \
         | Destination$ Exile | Optional$ True | IsCurse$ True | RememberChanged$ True \
         | Hidden$ True | Chooser$ You | SubAbility$ DBPut\n\
         SVar:DBPut:DB$ PutCounter | Defined$ Self | CounterNum$ 1 | CounterType$ P1P1 \
         | ConditionCheckSVar$ X | SubAbility$ DBTap\n\
         SVar:DBTap:DB$ Tap | Defined$ Self | ConditionCheckSVar$ X | ConditionSVarCompare$ LE0 \
         | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount\n\
         Oracle:At the beginning of your upkeep, you may exile a creature card from a graveyard. If you do, put a +1/+1 counter on this. If you don't, tap it.",
    );
    assert!(
        feaster.abilities[0].contains("Effect::ChooseCard { chooser: PlayerRel::You, zone: ChooseZone::Graveyard(PlayerRel::EachPlayer)")
            && feaster.abilities[0].contains("ThisWay::ExiledFromGraveyard(&Filter::Any)")
            && feaster.abilities[0].contains("Condition::Not(&Condition::AmountAtLeast("),
        "{:?}",
        feaster.abilities
    );

    // Sweep (Battlefield -> Hand hidden any-number + counted Pump).
    let sweep = read(
        "Name:MountainSweep\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Hand | ChangeType$ Mountain.YouCtrl \
         | ChangeNum$ X | Hidden$ True | RememberChanged$ True | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ Pump | ValidTgts$ Creature | NumAtt$ +Y | NumDef$ +Y | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$Valid Mountain.YouCtrl\n\
         SVar:Y:Remembered$Amount\n\
         Oracle:Sweep — Return any number of Mountains you control to their owner's hand. Target creature gets +1/+1 until end of turn for each Mountain returned this way.",
    );
    assert!(
        sweep.abilities[0].contains("movement: ChooseMove::Hand")
            && sweep.abilities[0].contains(
                "Effect::PumpTarget { power: Amount::ThisWay(ThisWay::ReturnedToHand(&Filter::Any)), \
                 toughness: Amount::ThisWay(ThisWay::ReturnedToHand(&Filter::Any))"
            ),
        "{:?}",
        sweep.abilities
    );
}

#[test]
fn wave20b_mill_fallback_and_target_min_extensions() {
    // Mill -> RememberMilled with UseAllOriginZones$ + ForgetChanged$ + "Select up to",
    // plus Mill -> conditional ChangeZone fallback via remembered_this_way_chain.
    let mill_take = read(
        "Name:MillTakeClean\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ Mill | NumCards$ 4 | Defined$ You | RememberMilled$ True | SubAbility$ DBTake\n\
         SVar:DBTake:DB$ ChangeZone | Origin$ Graveyard,Exile | UseAllOriginZones$ True \
         | Destination$ Hand | ChangeType$ Creature.IsRemembered | ChangeNum$ 1 | Hidden$ True \
         | ForgetChanged$ True | SelectPrompt$ Select up to one creature card | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        mill_take.abilities[0].contains("Effect::MillMayTakeOne { amount: Amount::Fixed(4)"),
        "{:?}",
        mill_take.abilities
    );

    let mill_fallback = read(
        "Name:MillCondBounce\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ Mill | NumCards$ 3 | Defined$ You | RememberMilled$ True | SubAbility$ DBBounce\n\
         SVar:DBBounce:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand \
         | ValidTgts$ Creature | ConditionDefined$ Remembered | ConditionPresent$ Instant \
         | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        mill_fallback.abilities[0].contains("ThisWay::Milled("),
        "{:?}",
        mill_fallback.abilities
    );

    // TargetMin$ 0 | TargetMax$ 1 on GainControl and superset Count$ValidGraveyard Card.YouOwn.
    let up_to_ctrl = read(
        "Name:UpToGainCtrl\nManaCost:4 U\nTypes:Sorcery\n\
         A:SP$ GainControl | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 1",
    );
    assert!(
        up_to_ctrl.abilities[0].contains("TargetReq::up_to_one("),
        "{:?}",
        up_to_ctrl.abilities
    );
    let superset_gy = read(
        "Name:AnyGoblinGy\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Exile \
         | ValidTgts$ Goblin.YouOwn | TargetMin$ 0 | TargetMax$ X\n\
         SVar:X:Count$ValidGraveyard Card.YouOwn",
    );
    assert!(
        superset_gy.abilities[0].contains("TargetReq::up_to("),
        "{:?}",
        superset_gy.abilities
    );
}

#[test]
fn wave20b_sacrifice_all_remember_sac_and_mana_reflected() {
    // SacrificeAll Defined$ Self, Defined$ Equipped, and Defined$ DelayTriggerRemembered.
    let sac_self = read(
        "Name:SacSelfAll\nManaCost:2\nTypes:Artifact\n\
         A:AB$ SacrificeAll | Cost$ 1 | Defined$ Self | Controller$ You",
    );
    assert!(
        sac_self.abilities[0].contains("Effect::SacrificeSelf"),
        "{:?}",
        sac_self.abilities
    );
    let sac_eq = read(
        "Name:SacEquipAll\nManaCost:3\nTypes:Artifact Equipment\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ S\n\
         SVar:S:DB$ SacrificeAll | Defined$ Equipped",
    );
    assert!(
        sac_eq.abilities[0]
            .contains("Effect::SacrificeAll { filter: &Filter::AttachedToBySource }"),
        "{:?}",
        sac_eq.abilities
    );
    let sac_delayed = read(
        "Name:SacDelayedAll\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ 2 | SubAbility$ DT\n\
         SVar:DT:DB$ DelayedTrigger | Mode$ Phase | Phase$ End of Turn | Execute$ DS | RememberObjects$ Targeted\n\
         SVar:DS:DB$ SacrificeAll | Defined$ DelayTriggerRemembered",
    );
    assert!(
        sac_delayed.abilities[0].contains("Effect::SacrificeEvent"),
        "{:?}",
        sac_delayed.abilities
    );

    // Thalakos Deceiver shape: SacrificeAll | Defined$ Self | RememberSacrificed$ True + targeted GainControl.
    let deceiver = read(
        "Name:SelfSacSteal\nManaCost:3 U\nTypes:Creature Wizard\nPT:1/1\n\
         T:Mode$ AttackerUnblocked | ValidCard$ Card.Self | Execute$ TrigSac | OptionalDecider$ You\n\
         SVar:TrigSac:DB$ SacrificeAll | Defined$ Self | RememberSacrificed$ True | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainControl | ValidTgts$ Creature | ConditionDefined$ Remembered \
         | ConditionPresent$ Card | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        deceiver.abilities[0].contains(
            "Effect::SacrificeSelfThen { then: &[Effect::continuous(&Filter::This, Modifier::GainControl, Duration::Indefinitely)] }"
        ) && deceiver.abilities[0]
            .contains("targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE)))"),
        "{:?}",
        deceiver.abilities
    );

    // Victimize shape: Sacrifice + RememberSacrificed$ True + targeted multi-reanimate follow-up.
    let victimize = read(
        "Name:SacReanimateTwo\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ Sacrifice | Defined$ You | SacValid$ Creature | RememberSacrificed$ True | SubAbility$ Rise\n\
         SVar:Rise:DB$ ChangeZone | ValidTgts$ Creature.YouOwn | TargetMin$ 2 | TargetMax$ 2 \
         | Origin$ Graveyard | Destination$ Battlefield | ConditionCheckSVar$ X \
         | ConditionSVarCompare$ EQ1 | Tapped$ True | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        victimize.abilities[0].contains("Effect::SacrificeAndIf {")
            && victimize.abilities[0].contains("Effect::GraveyardToBattlefieldModified {")
            && victimize.abilities[0].contains("TargetReq::exactly("),
        "{:?}",
        victimize.abilities
    );

    // Single-sac LKI follow-up for controller -> SacrificeOneThen + SacrificedPower.
    let sac_lki = read(
        "Name:SacGainPower\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ Sacrifice | Defined$ You | SacValid$ Creature | RememberSacrificed$ True | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ X | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:RememberedLKI$CardPower",
    );
    assert!(
        sac_lki.abilities[0].contains(
            "Effect::SacrificeOneThen { filter: &Filter::CREATURE, if_it_was: &Filter::Any, \
             then: &[Effect::GainLife { amount: Amount::SacrificedPower }] }"
        ),
        "{:?}",
        sac_lki.abilities
    );
}

#[test]
fn wave20b_mana_reflected_db_and_sacrifice_refusals() {
    // DB$ ManaReflected under T:Mode$ TapsForMana (TriggeredActivator, TriggeredCardController, You).
    let flare = read(
        "Name:ManaDoubler\nManaCost:2 R\nTypes:Enchantment\n\
         T:Mode$ TapsForMana | ValidCard$ Land | Execute$ TrigMana | TriggerZones$ Battlefield | Static$ True\n\
         SVar:TrigMana:DB$ ManaReflected | ColorOrType$ Type | ReflectProperty$ Produced | Defined$ TriggeredActivator",
    );
    assert!(
        flare.abilities[0].contains("Effect::AddManaLikeEvent { amount: 1 }"),
        "{:?}",
        flare.abilities
    );
    let wake = read(
        "Name:YourManaDoubler\nManaCost:3 G W\nTypes:Enchantment\n\
         T:Mode$ TapsForMana | ValidCard$ Land | Activator$ You | Execute$ TrigMana | TriggerZones$ Battlefield | Static$ True\n\
         SVar:TrigMana:DB$ ManaReflected | ColorOrType$ Type | ReflectProperty$ Produced | Defined$ You\n\
         Oracle:Whenever you tap a land for mana, add one mana of any type that land produced.",
    );
    assert!(
        wake.abilities[0].contains("Effect::AddManaLikeEvent { amount: 1 }"),
        "{:?}",
        wake.abilities
    );

    // Honest refusals: SacrificeAll Enchanted/Opponent, multi-sac RememberSacrificed count (NEVER ThisWay::Died),
    // opponent-sac LKI, ManaReflected Defined$ You on opponent trigger or outside TapsForMana.
    for bad in [
        "Name:BadSacEnch\nManaCost:1 B\nTypes:Enchantment Aura\nK:Enchant:Creature\nT:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ S\nSVar:S:DB$ SacrificeAll | Defined$ Enchanted",
        "Name:BadSacCtrl\nManaCost:2 B\nTypes:Sorcery\nA:SP$ SacrificeAll | ValidCards$ Creature | Controller$ Opponent",
        "Name:BadMultiSacCount\nManaCost:4 R\nTypes:Sorcery\nA:SP$ SacrificeAll | ValidCards$ Creature.YouCtrl | RememberSacrificed$ True | SubAbility$ T\nSVar:T:DB$ Token | TokenScript$ r_1_1_goblin | TokenAmount$ X\nSVar:X:Remembered$Amount",
        "Name:BadOppSacLki\nManaCost:2 B\nTypes:Instant\nA:SP$ Sacrifice | ValidTgts$ Opponent | SacValid$ Creature | RememberSacrificed$ True | SubAbility$ G\nSVar:G:DB$ GainLife | Defined$ You | LifeAmount$ X\nSVar:X:RememberedLKI$CardToughness",
        "Name:BadManaReflectOpp\nManaCost:2 R\nTypes:Enchantment\nT:Mode$ TapsForMana | ValidCard$ Land | Activator$ Opponent | Execute$ M | TriggerZones$ Battlefield | Static$ True\nSVar:M:DB$ ManaReflected | ColorOrType$ Type | ReflectProperty$ Produced | Defined$ You\nOracle:Whenever an opponent taps a land for mana, add one mana of any type that land produced.",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave21b_effect_play_sources_and_modes() {
    // Targeted graveyard Play with up-to-1 target, AnyManaColor, Optional, exile replacement.
    let gy_tgt = read(
        "Name:GraveReplay\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ Play | TgtZone$ Graveyard | ValidTgts$ Instant,Sorcery | TargetMin$ 0 | TargetMax$ 1 \
         | ValidSA$ Spell | ManaConversion$ AnyType->AnyType | Optional$ True | ReplaceGraveyard$ Exile",
    );
    assert!(
        gy_tgt.abilities[0]
            .contains("Effect::Play { source: PlaySource::Subject(TargetSpec::CardInGraveyard(")
            && gy_tgt.abilities[0].contains("cost: PlayCostMode::AnyManaColor")
            && gy_tgt.abilities[0].contains(
                "cast_only: true, copy: false, exile_instead_of_graveyard: true, optional: true"
            )
            && gy_tgt.abilities[0].contains("TargetReq::up_to_one("),
        "{:?}",
        gy_tgt.abilities
    );

    // LinkedExile with alternative PlayCost$ and cmcEQX, plus legacy MayCopyCastLinkedExile preservation.
    let linked = read(
        "Name:VaultMirror\nManaCost:3\nTypes:Artifact\n\
         A:AB$ Play | Cost$ X T | ValidZone$ Exile | Valid$ Card.ExiledWithSource | ValidSA$ Spell.cmcEQX \
         | CopyCard$ True | PlayCost$ 1 | Optional$ True\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        linked.abilities[0].contains("source: PlaySource::LinkedExile")
            && linked.abilities[0].contains("cost: PlayCostMode::Alternative(mana!(\"{1}\"))")
            && linked.abilities[0].contains("copy: true")
            && linked.abilities[0].contains("filter: &Filter::CmcExactlyX"),
        "{:?}\n{}",
        linked.abilities,
        linked.statics
    );

    // Zone sources: implicit Hand zone (ValidZone omitted), Graveyard OppOwn, and Library top card.
    let from_hand = read(
        "Name:FreeHandSpell\nManaCost:4 R\nTypes:Sorcery\n\
         A:SP$ Play | Valid$ Instant.YouOwn,Sorcery.YouOwn | ValidSA$ Spell | WithoutManaCost$ True | Optional$ True",
    );
    assert!(
        from_hand.abilities[0]
            .contains("source: PlaySource::Zone { zone: ZoneRef::Hand, owner: PlayerRel::You }")
            && from_hand.abilities[0].contains("cost: PlayCostMode::WithoutPayingManaCost"),
        "{:?}",
        from_hand.abilities
    );
    let top_lib = read(
        "Name:TopDeckPlay\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ Play | Defined$ TopOfLibrary | Optional$ True",
    );
    assert!(
        top_lib.abilities[0]
            .contains("source: PlaySource::Zone { zone: ZoneRef::Library, owner: PlayerRel::You }")
            && top_lib.abilities[0].contains("cast_only: false"),
        "{:?}",
        top_lib.abilities
    );

    // ThisObject (Valid$ Card.Self in Exile) and EventObject via ChooseCard + Play.SubAbility!
    let self_copy = read(
        "Name:ExileEcho\nManaCost:3 U\nTypes:Artifact\n\
         A:AB$ Play | Cost$ 2 | ValidZone$ Exile | Valid$ Card.Self | CopyCard$ True | WithoutManaCost$ True",
    );
    assert!(
        self_copy.abilities[0].contains("source: PlaySource::Subject(TargetSpec::ThisObject)")
            && self_copy.abilities[0].contains("cast_only: true, copy: true"),
        "{:?}",
        self_copy.abilities
    );
    let chosen_sub = read(
        "Name:ChooseThenPlay\nManaCost:3 B\nTypes:Sorcery\n\
         A:SP$ ChooseCard | Defined$ You | Choices$ Instant.YouOwn | ChoiceZone$ Graveyard | SubAbility$ DBPlay\n\
         SVar:DBPlay:DB$ Play | Defined$ ChosenCard | WithoutManaCost$ True | Optional$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert!(
        chosen_sub.abilities[0].contains("source: PlaySource::Subject(TargetSpec::EventObject)")
            && chosen_sub.abilities[0].contains("Effect::draw(1)"),
        "{:?}",
        chosen_sub.abilities
    );
}

#[test]
fn play_preserves_printed_graveyard_scope() {
    for (printed_owner, expected_owner) in [("a", "EachPlayer"), ("your", "You")] {
        let body = read(&format!(
            "Name:Grave Reader\nManaCost:3 U\nTypes:Creature Wizard\nPT:3/2\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ Replay\n\
             SVar:Replay:DB$ Play | ValidZone$ Graveyard | Valid$ Card.YouOwn | ValidSA$ Instant.cmcLEX,Sorcery.cmcLEX | WithoutManaCost$ True | Optional$ True | ReplaceGraveyard$ Exile\n\
             SVar:X:Count$Valid Wizard.YouCtrl\n\
             Oracle:When this creature enters, you may cast an instant or sorcery spell with mana value less than or equal to the number of Wizards you control from {printed_owner} graveyard without paying its mana cost. If that spell would be put into a graveyard, exile it instead."
        ));
        assert!(
            body.abilities[0].contains(&format!(
                "zone: ZoneRef::Graveyard, owner: PlayerRel::{expected_owner} }}"
            )),
            "{:?}",
            body.abilities
        );
    }
}

#[test]
fn play_preserves_strict_printed_mana_value_bound() {
    for (comparison, strict) in [("less than", true), ("less than or equal to", false)] {
        let body = read(&format!(
            "Name:Unearth a Spell\nManaCost:2 R\nTypes:Instant\n\
             A:SP$ DigUntil | Valid$ Card.nonLand | FoundDestination$ Exile | RevealedDestination$ Exile | RememberFound$ True | SubAbility$ TryCast\n\
             SVar:TryCast:DB$ Play | Defined$ Remembered | ValidSA$ Spell.cmcLEX | WithoutManaCost$ True | Optional$ True | SubAbility$ TakeCard\n\
             SVar:TakeCard:DB$ ChangeZone | Defined$ Remembered | Origin$ Exile | Destination$ Hand | SubAbility$ Clear\n\
             SVar:Clear:DB$ Cleanup | ClearRemembered$ True\n\
             SVar:X:Count$Valid Mountain.YouCtrl\n\
             Oracle:Exile cards from the top of your library until you exile a nonland card. You may cast that card without paying its mana cost if the spell's mana value is {comparison} the number of Mountains you control. If you don't cast that card this way, put it into your hand."
        ));
        assert!(body.statics.contains("Filter::CmcAtMostAmount("));
        assert_eq!(
            body.statics
                .contains("Filter::Not(&Filter::CmcEqualsAmount("),
            strict,
            "{}",
            body.statics
        );
    }
}

#[test]
fn wave21b_effect_play_refusals() {
    for bad in [
        // AllowRepeats$ not supported by Effect::Play
        "Name:BadRepeatPlay\nManaCost:6 U\nTypes:Sorcery\nA:SP$ Play | TgtZone$ Graveyard | ValidTgts$ Instant | CopyCard$ True | WithoutManaCost$ True | Amount$ 3 | AllowRepeats$ True",
        // Sideboard zone refused
        "Name:BadSideboard\nManaCost:3 R\nTypes:Sorcery\nA:SP$ Play | ValidZone$ Sideboard | Valid$ Card.YouOwn",
        // Library-wide search refused (only top card allowed)
        "Name:BadLibraryAll\nManaCost:5 G\nTypes:Sorcery\nA:SP$ Play | ValidZone$ Library | Valid$ Creature.YouOwn\nOracle:You may cast a creature spell from your library.",
        // Unbound Defined$ Remembered refused
        "Name:BadRemembered\nManaCost:2 R\nTypes:Sorcery\nA:SP$ Play | Defined$ Remembered | WithoutManaCost$ True",
        // CmcAtMostX without announced X refused
        "Name:BadNoX\nManaCost:2 U\nTypes:Artifact\nA:AB$ Play | Cost$ T | ValidZone$ Exile | Valid$ Card.ExiledWithSource | ValidSA$ Spell.cmcLEx",
        // RememberPlayed$ unsupported
        "Name:BadRememberPlayed\nManaCost:2 U\nTypes:Sorcery\nA:SP$ Play | ValidZone$ Hand | Valid$ Card.YouOwn | RememberPlayed$ True",
        // Owner qualifier on ValidSA$ must never be dropped on non-Zone PlaySource
        "Name:BadSaOwner\nManaCost:2 U\nTypes:Artifact\nA:AB$ Play | Cost$ T | Defined$ ExiledWith | ValidSA$ Spell.YouOwn",
        // Non-single controller refused
        "Name:BadMultiCtrl\nManaCost:3 U\nTypes:Sorcery\nA:SP$ Play | ValidZone$ Hand | Valid$ Card.YouOwn | Controller$ Player",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave21b_vote_options_and_objects() {
    // MajorityOrTied (Will of the council) + targeted Split Decision shape (TgtZone$ Stack).
    let wotc = read(
        "Name:CouncilSplit\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Vote | ValidTgts$ Instant,Sorcery | TgtZone$ Stack | TargetType$ Spell | Defined$ Player \
         | Choices$ DBDeny,DBCopy | VoteTiedAbility$ DBCopy\n\
         SVar:DBDeny:DB$ Counter | Defined$ Targeted | SpellDescription$ Denial\n\
         SVar:DBCopy:DB$ CopySpellAbility | Defined$ Targeted | MayChooseTarget$ True | SpellDescription$ Duplication",
    );
    assert!(
        wotc.abilities[0].contains("Effect::Vote { options: &[VoteOption { name: \"Denial\"")
            && wotc.abilities[0]
                .contains("outcome: VoteOutcome::MajorityOrTied { tied_option: 1 }, secret: false")
            && wotc.abilities[0].contains("TargetSpec::Spell("),
        "{:?}",
        wotc.abilities
    );

    // AllWinners (no VoteTiedAbility$).
    let all_win = read(
        "Name:CouncilAll\nManaCost:4 W\nTypes:Sorcery\n\
         A:SP$ Vote | Defined$ Player | Choices$ VDraw,VLife\n\
         SVar:VDraw:DB$ Draw | Defined$ You | NumCards$ 1 | SpellDescription$ Lore\n\
         SVar:VLife:DB$ GainLife | Defined$ You | LifeAmount$ 3 | SpellDescription$ Balm",
    );
    assert!(
        all_win.abilities[0].contains("outcome: VoteOutcome::AllWinners, secret: false"),
        "{:?}",
        all_win.abilities
    );

    // PerOptionCount with VoteNum -> ThisWay::VoteCount(0/1) (Token + PutCounter, Sacrifice + Discard, Repeat).
    let dilemma = read_with_tokens(
        "Name:CouncilDilemma\nManaCost:4 W\nTypes:Creature Wizard\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TVote\n\
         SVar:TVote:DB$ Vote | Defined$ Player | Choices$ VBoost,VTokens | StoreVoteNum$ True\n\
         SVar:VBoost:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ VoteNum | SpellDescription$ Strength\n\
         SVar:VTokens:DB$ Token | TokenScript$ r_1_1_goblin | TokenAmount$ VoteNum | SpellDescription$ Numbers",
    );
    assert!(
        dilemma.abilities[0].contains("ThisWay::VoteCount(0)")
            && dilemma.abilities[0].contains("ThisWay::VoteCount(1)")
            && dilemma.abilities[0].contains("outcome: VoteOutcome::PerOptionCount"),
        "{:?}",
        dilemma.abilities
    );
    let capital = read(
        "Name:CouncilPenalty\nManaCost:4 B B\nTypes:Sorcery\n\
         A:SP$ Vote | Defined$ Player | Choices$ VSac,VDisc | StoreVoteNum$ True\n\
         SVar:VSac:DB$ Sacrifice | Defined$ Opponent | SacValid$ Creature | Amount$ VoteNum | SpellDescription$ Death\n\
         SVar:VDisc:DB$ Discard | Defined$ Opponent | NumCards$ VoteNum | Mode$ TgtChoose | SpellDescription$ Taxes",
    );
    assert!(
        capital.abilities[0].contains("Effect::SacrificeAmountForPlayers")
            && capital.abilities[0].contains("Effect::DiscardAmountForPlayers")
            && capital.abilities[0].contains("ThisWay::VoteCount(0)")
            && capital.abilities[0].contains("ThisWay::VoteCount(1)"),
        "{:?}",
        capital.abilities
    );
    let repeat_vote = read(
        "Name:CouncilRepeat\nManaCost:4 G\nTypes:Sorcery\n\
         A:SP$ Vote | Defined$ Player | Choices$ VRep,VGain | StoreVoteNum$ True\n\
         SVar:VRep:DB$ Repeat | MaxRepeat$ VoteNum | RepeatSubAbility$ DDraw | SpellDescription$ Wild\n\
         SVar:DDraw:DB$ Draw | Defined$ You | NumCards$ 1\n\
         SVar:VGain:DB$ GainLife | Defined$ You | LifeAmount$ VoteNum | SpellDescription$ Calm",
    );
    assert!(
        repeat_vote.abilities[0].contains("Effect::RepeatProcess")
            && repeat_vote.abilities[0]
                .contains("max_times: Some(Amount::ThisWay(ThisWay::VoteCount(0)))"),
        "{:?}",
        repeat_vote.abilities
    );

    // RepeatPerVote (EachVote$ True + RememberedPlayer -> EventPlayer).
    let per_vote = read(
        "Name:CouncilEachVote\nManaCost:5 U\nTypes:Sorcery\n\
         A:SP$ Vote | Defined$ Player | Secretly$ True | Choices$ VTime,VDraw | EachVote$ True\n\
         SVar:VTime:DB$ AddTurn | Defined$ You | NumTurns$ 1 | SpellDescription$ Time\n\
         SVar:VDraw:DB$ Draw | Defined$ RememberedPlayer | NumCards$ 1 | SpellDescription$ Tribute",
    );
    assert!(
        per_vote.abilities[0].contains(
            "Effect::DrawCardsFor { amount: Amount::Fixed(1), who: PlayerRel::EventPlayer }"
        ) && per_vote.abilities[0].contains("outcome: VoteOutcome::RepeatPerVote, secret: true"),
        "{:?}",
        per_vote.abilities
    );
}

#[test]
fn wave21b_vote_objects_and_refusals() {
    // VoteObjects: Battlefield Exile, Graveyard ReturnToHand (Zone$ Graveyard), Secret + UpTo + DestroyAll + if_no_votes.
    let judgment = read(
        "Name:CouncilExile\nManaCost:1 W W\nTypes:Sorcery\n\
         A:SP$ Vote | Defined$ Player | VoteSubAbility$ DBExile | VoteCard$ Permanent.nonLand+YouDontCtrl \
         | VoteMessage$ for a nonland permanent you don't control\n\
         SVar:DBExile:DB$ ChangeZone | Defined$ Remembered | Origin$ Battlefield | Destination$ Exile",
    );
    assert!(
        judgment.abilities[0].contains("Effect::VoteObjects { zone: ZoneRef::Battlefield")
            && judgment.abilities[0].contains("optional: false, secret: false, action: VoteObjectAction::Exile, if_no_votes: &[] }"),
        "{:?}",
        judgment.abilities
    );
    let squire = read(
        "Name:CouncilReturn\nManaCost:4 W\nTypes:Sorcery\n\
         A:SP$ Vote | Defined$ Player | VoteSubAbility$ DBRet | Zone$ Graveyard \
         | VoteCard$ Creature.YouOwn,Artifact.YouOwn | VoteMessage$ for a card\n\
         SVar:DBRet:DB$ ChangeZone | Defined$ Remembered | Origin$ Graveyard | Destination$ Hand",
    );
    assert!(
        squire.abilities[0].contains("Effect::VoteObjects { zone: ZoneRef::Graveyard")
            && squire.abilities[0].contains("action: VoteObjectAction::ReturnToHand"),
        "{:?}",
        squire.abilities
    );
    let vault = read(
        "Name:SecretDilemma\nManaCost:3 W B\nTypes:Sorcery\n\
         A:SP$ Vote | Defined$ Player | Secretly$ True | VoteCard$ Creature | VoteMessage$ for a creature \
         | UpTo$ True | VoteSubAbility$ DBDest\n\
         SVar:DBDest:DB$ DestroyAll | ValidCards$ Creature.IsRemembered | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ Player | NumCards$ 1 | ConditionCheckSVar$ X | ConditionSVarCompare$ EQ0\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        vault.abilities[0].contains("optional: true, secret: true, action: VoteObjectAction::Destroy { no_regen: false }")
            && vault.abilities[0].contains("if_no_votes: &[Effect::DrawCardsFor { amount: Amount::Fixed(1), who: PlayerRel::EachPlayer }]"),
        "{:?}",
        vault.abilities
    );

    for bad in [
        "Name:BadVoteDef\nManaCost:2 U\nTypes:Sorcery\nA:SP$ Vote | Defined$ Opponent | Choices$ A,B\nSVar:A:DB$ Draw | Defined$ You | NumCards$ 1 | SpellDescription$ A\nSVar:B:DB$ GainLife | Defined$ You | LifeAmount$ 1 | SpellDescription$ B",
        "Name:BadVotePlayer\nManaCost:2 U\nTypes:Sorcery\nA:SP$ Vote | Defined$ Player | VotePlayer$ Player | StoreVoteNum$ True",
        "Name:BadRememberVoted\nManaCost:2 U\nTypes:Sorcery\nA:SP$ Vote | Defined$ Player | VoteCard$ Creature | RememberVotedObjects$ True | VoteSubAbility$ E\nSVar:E:DB$ ChangeZone | Defined$ Remembered | Origin$ Battlefield | Destination$ Exile",
        "Name:BadConflict\nManaCost:2 U\nTypes:Sorcery\nA:SP$ Vote | Defined$ Player | Choices$ A,B | StoreVoteNum$ True | EachVote$ True\nSVar:A:DB$ Draw | Defined$ You | NumCards$ 1 | SpellDescription$ A\nSVar:B:DB$ GainLife | Defined$ You | LifeAmount$ 1 | SpellDescription$ B",
        "Name:BadDestFilter\nManaCost:2 B\nTypes:Sorcery\nA:SP$ Vote | Defined$ Player | VoteCard$ Creature | VoteSubAbility$ D\nSVar:D:DB$ DestroyAll | ValidCards$ Creature.IsRemembered+nonBlack",
        "Name:BadObjAct\nManaCost:2 U\nTypes:Sorcery\nA:SP$ Vote | Defined$ Player | VoteCard$ Creature | VoteSubAbility$ T\nSVar:T:DB$ Tap | Defined$ Remembered",
        "Name:BadOwnVoter\nManaCost:7 U U\nTypes:Sorcery\nA:SP$ Vote | Defined$ Player | Choices$ A,B | EachVote$ True\nSVar:A:DB$ Draw | Defined$ You | NumCards$ 1 | SpellDescription$ Time\nSVar:B:DB$ GainControl | ValidTgts$ Permanent.RememberedPlayerOwn | SpellDescription$ Money",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave21b_reduce_next_spell_cost_and_refusals() {
    // Dynamic SetChosenNumber$ + one-shot ForgetOnCast$ (Kaza shape).
    let kaza = read(
        "Name:WizardDiscount\nManaCost:U R\nTypes:Creature Wizard\nPT:1/2\n\
         A:AB$ Effect | Cost$ T | StaticAbilities$ Disc | ForgetOnCast$ Instant.YouCtrl,Sorcery.YouCtrl | SetChosenNumber$ X\n\
         SVar:Disc:Mode$ ReduceCost | ValidCard$ Instant,Sorcery | Type$ Spell | Activator$ You | Amount$ Num\n\
         SVar:Num:Count$ChosenNumber\n\
         SVar:X:Count$Valid Wizard.YouCtrl",
    );
    assert!(
        kaza.abilities[0].contains("Effect::ReduceNextSpellCost { who: PlayerRel::You")
            && kaza.abilities[0].contains("generic: Amount::CountOf {")
            && kaza.abilities[0].contains("mana: ManaCost::ZERO, colored_only: false, next_only: true, duration: Duration::UntilEndOfTurn }"),
        "{:?}",
        kaza.abilities
    );

    // Dynamic SetChosenNumber$ non-one-shot (Will, Scion of Peace shape).
    let will_peace = read(
        "Name:LifeDiscount\nManaCost:1 W U\nTypes:Creature Wizard\nPT:2/4\n\
         A:AB$ Effect | Cost$ T | StaticAbilities$ Disc | SetChosenNumber$ X\n\
         SVar:Disc:Mode$ ReduceCost | ValidCard$ Card.White,Card.Blue | Type$ Spell | Activator$ You | Amount$ Num\n\
         SVar:Num:Count$ChosenNumber\n\
         SVar:X:Count$LifeYouGainedThisTurn",
    );
    assert!(
        will_peace.abilities[0].contains("Effect::ReduceNextSpellCost { who: PlayerRel::You")
            && will_peace.abilities[0].contains("generic: Amount::LifeGainedThisTurn")
            && will_peace.abilities[0]
                .contains("next_only: false, duration: Duration::UntilEndOfTurn }"),
        "{:?}",
        will_peace.abilities
    );

    // Targeted/remembered player discount until your next turn (Will Kenrith shape).
    let kenrith = read(
        "Name:TargetedDiscount\nManaCost:4 U U\nTypes:Sorcery\n\
         A:SP$ Effect | ValidTgts$ Player | RememberObjects$ Targeted | StaticAbilities$ Disc | Duration$ UntilYourNextTurn\n\
         SVar:Disc:Mode$ ReduceCost | ValidCard$ Instant,Sorcery | Type$ Spell | Activator$ Player.IsRemembered | Amount$ 2",
    );
    assert!(
        kenrith.abilities[0].contains("Effect::ReduceNextSpellCost { who: PlayerRel::Chosen")
            && kenrith.abilities[0].contains("generic: Amount::Fixed(2)")
            && kenrith.abilities[0]
                .contains("next_only: false, duration: Duration::UntilYourNextTurn }"),
        "{:?}",
        kenrith.abilities
    );

    // Colored Cost$ with ColorOnly$ True + Permanent filter (card_zone = true permanent types).
    let colored = read(
        "Name:ColoredDiscount\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ Effect | StaticAbilities$ Disc | ForgetOnCast$ Permanent.YouCtrl\n\
         SVar:Disc:Mode$ ReduceCost | ValidCard$ Permanent | Type$ Spell | Activator$ You | Cost$ U | ColorOnly$ True",
    );
    assert!(
        colored.abilities[0].contains(
            "generic: Amount::Fixed(0), mana: mana!(\"{U}\"), colored_only: true, next_only: true",
        ) && colored.statics.contains("TypeSet::PLANESWALKER"),
        "{:?}\n{}",
        colored.abilities,
        colored.statics
    );

    // Fixed-amount `ForgetOnCast$ Card.YouCtrl` without `SetChosenNumber$` lowers in Wave 22b.
    let fixed_next = read(
        "Name:FixedNextDiscount\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ Effect | StaticAbilities$ Disc | ForgetOnCast$ Card.YouCtrl\n\
         SVar:Disc:Mode$ ReduceCost | ValidCard$ Card | Type$ Spell | Activator$ You | Amount$ 1",
    );
    assert!(
        fixed_next.abilities[0].contains(
            "Effect::ReduceNextSpellCost { who: PlayerRel::You, filter: &Filter::Any, \
             generic: Amount::Fixed(1), mana: ManaCost::ZERO, colored_only: false, \
             next_only: true, duration: Duration::UntilEndOfTurn }"
        ),
        "{:?}",
        fixed_next.abilities
    );

    for bad in [
        // Mismatched ForgetOnCast$ vs ValidCard$
        "Name:BadMismatch\nManaCost:1 R\nTypes:Sorcery\nA:SP$ Effect | StaticAbilities$ Disc | ForgetOnCast$ Instant.YouCtrl\nSVar:Disc:Mode$ ReduceCost | ValidCard$ Sorcery | Type$ Spell | Activator$ You | Amount$ 2",
        // SetChosenNumber$ without Count$ChosenNumber
        "Name:BadUnusedChosen\nManaCost:1 R\nTypes:Sorcery\nA:SP$ Effect | StaticAbilities$ Disc | SetChosenNumber$ X\nSVar:Disc:Mode$ ReduceCost | ValidCard$ Card | Type$ Spell | Activator$ You | Amount$ 2\nSVar:X:Count$Valid Goblin.YouCtrl",
        // Remembered card filter refused
        "Name:BadRemCard\nManaCost:1 R\nTypes:Sorcery\nA:SP$ Effect | StaticAbilities$ Disc | ForgetOnCast$ Card.YouCtrl\nSVar:Disc:Mode$ ReduceCost | ValidCard$ Card.IsRemembered | Type$ Spell | Activator$ You | Amount$ 2",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave21b_initiative_set_pt_and_conditions() {
    // TakeInitiative (You, targeted Player, TriggeredPlayer) + VentureIntoUndercity.
    let init_you = read(
        "Name:InitEnter\nManaCost:3 W\nTypes:Creature Wizard\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TInit\n\
         SVar:TInit:DB$ TakeInitiative | Defined$ You",
    );
    assert!(
        init_you.abilities[0].contains("Effect::TakeInitiative(PlayerRel::You)"),
        "{:?}",
        init_you.abilities
    );
    let init_tgt = read(
        "Name:InitTarget\nManaCost:2 W\nTypes:Sorcery\nA:SP$ TakeInitiative | ValidTgts$ Player",
    );
    assert!(
        init_tgt.abilities[0].contains("Effect::TakeInitiative(PlayerRel::Chosen)"),
        "{:?}",
        init_tgt.abilities
    );
    let undercity =
        read("Name:GoUndercity\nManaCost:2 B\nTypes:Sorcery\nA:SP$ Venture | Dungeon$ Undercity");
    assert!(
        undercity.abilities[0].contains("Effect::VentureIntoUndercity"),
        "{:?}",
        undercity.abilities
    );

    // Effect::SetPTByCountFilter: standalone AnimateAll + compound Animate with Types$.
    let hand_pt = read(
        "Name:HandPower\nManaCost:4 G G\nTypes:Artifact\n\
         A:AB$ AnimateAll | Cost$ 4 G G | ValidCards$ Creature.YouCtrl | Power$ X | Toughness$ Y\n\
         SVar:X:Count$ValidHand Card.YouOwn/Twice\n\
         SVar:Y:Count$PartySize",
    );
    assert!(
        hand_pt.abilities[0].contains("Effect::SetPTByCountFilter {")
            && hand_pt.abilities[0]
                .contains("power: PtCount::Twice(&PtCount::CardsInHand(PlayerRel::You))")
            && hand_pt.abilities[0].contains("toughness: PtCount::PartySize(PlayerRel::You)"),
        "{:?}",
        hand_pt.abilities
    );
    let compound = read(
        "Name:ForestBender\nManaCost:2 G\nTypes:Creature Wizard\nPT:1/1\n\
         A:AB$ Animate | Cost$ T | ValidTgts$ Forest | Power$ X | Toughness$ X | Types$ Creature,Goblin\n\
         SVar:X:Count$Valid Wizard.YouCtrl",
    );
    assert!(
        compound.abilities[0].contains("Modifier::AddType(TypeSet::CREATURE)")
            && compound.abilities[0].contains("Modifier::AddSubtype(subtypes::creature::GOBLIN)")
            && compound.abilities[0].contains(
                "Effect::SetPTByCountFilter { filter: &Filter::This, power: PtCount::YouControl("
            ),
        "{:?}",
        compound.abilities
    );

    // Conditions: IsMonarch, HasInitiative, CompletedADungeon (named, Count$ splits, SVar compares).
    let conds = read(
        "Name:CrownCheck\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ Draw | Defined$ You | NumCards$ X | SubAbility$ DBGain\n\
         SVar:X:Count$Initiative.2.1\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ 3 | ConditionCheckSVar$ D | ConditionSVarCompare$ GE1 | SubAbility$ DBMon\n\
         SVar:D:DungeonsCompleted$Amount\n\
         SVar:DBMon:DB$ LoseLife | Defined$ Opponent | LifeAmount$ 2 | ConditionCheckSVar$ M | ConditionSVarCompare$ EQ0\n\
         SVar:M:PlayerCountPlayers$HasPropertyisMonarch",
    );
    assert!(
        conds.abilities[0].contains("Condition::HasInitiative(PlayerRel::You)")
            && conds.abilities[0].contains("Condition::CompletedADungeon(PlayerRel::You)")
            && conds.abilities[0]
                .contains("Condition::Not(&Condition::IsMonarch(PlayerRel::EachPlayer))"),
        "{:?}",
        conds.abilities
    );

    for bad in [
        "Name:BadInitAll\nManaCost:3 W\nTypes:Sorcery\nA:SP$ TakeInitiative | Defined$ Player",
        "Name:BadDungeon\nManaCost:2 B\nTypes:Sorcery\nA:SP$ Venture | Dungeon$ Tomb",
        "Name:BadPtSvar\nManaCost:4 G\nTypes:Sorcery\nA:SP$ AnimateAll | ValidCards$ Creature.YouCtrl | Power$ X | Toughness$ X\nSVar:X:Count$TotalSpellsCastThisTurn",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave22b_effect_static_modes_and_one_shot_spells() {
    // Player-rule Effect statics: CantGainLife (including DamageDone TriggeredTarget),
    // CantChangeLife, CantLoseLife, CantSearchLibrary, CantPlayLand, EffectZone$ Command, Stackable$ False.
    let scar = read(
        "Name:ScarLasher\nManaCost:R R\nTypes:Creature Goblin\nPT:2/2\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | Execute$ TrigLock | TriggerZones$ Battlefield\n\
         SVar:TrigLock:DB$ Effect | RememberObjects$ TriggeredTarget | StaticAbilities$ NoGain | Duration$ Permanent | Stackable$ False\n\
         SVar:NoGain:Mode$ CantGainLife | ValidPlayer$ Player.IsRemembered | EffectZone$ Command",
    );
    assert!(
        scar.abilities[0].contains("Modifier::PlayersCantGainLife(PlayerRel::DamagedPlayer)"),
        "{:?}",
        scar.abilities
    );
    let locks = read(
        "Name:QuietWard\nManaCost:2 W\nTypes:Instant\n\
         A:SP$ Effect | StaticAbilities$ S1,S2,S3,S4 | Duration$ UntilEndOfTurn\n\
         SVar:S1:Mode$ CantChangeLife | ValidPlayer$ Player\n\
         SVar:S2:Mode$ CantLoseLife | ValidPlayer$ Opponent\n\
         SVar:S3:Mode$ CantSearchLibrary | ValidPlayer$ Opponent\n\
         SVar:S4:Mode$ CantPlayLand | Player$ Player",
    );
    let lock_out = locks.abilities.join("\n");
    for expected in [
        "Modifier::LifeTotalCantChange { who: PlayerRel::EachPlayer }",
        "Modifier::CantLoseLife { who: PlayerRel::EachOpponent }",
        "Modifier::OpponentsCantSearch",
        "Modifier::PlayersCantPlay(&Filter::LAND)",
    ] {
        assert!(
            lock_out.contains(expected),
            "missing {expected}: {lock_out}"
        );
    }

    // One-shot ForgetOnCast$ specials: Improvise and mana-any-type.
    let improvise = read(
        "Name:VaultArch\nTypes:Land\n\
         A:AB$ Effect | Cost$ U T | StaticAbilities$ GrantImp | ForgetOnCast$ Card.YouCtrl\n\
         SVar:GrantImp:Mode$ Continuous | Affected$ Card.YouCtrl | AffectedZone$ Stack | AddKeyword$ Improvise",
    );
    assert!(
        improvise.abilities[0].contains("Effect::NextSpellThisTurnHasImprovise"),
        "{:?}",
        improvise.abilities
    );
    let star = read(
        "Name:PolarStar\nManaCost:4\nTypes:Artifact\n\
         A:AB$ Effect | Cost$ 4 T | StaticAbilities$ AnyMana | ForgetOnCast$ Card.YouCtrl\n\
         SVar:AnyMana:Mode$ ManaConvert | ValidPlayer$ You | ValidSA$ Spell | Optional$ True | ManaConversion$ AnyType->AnyType",
    );
    assert!(
        star.abilities[0].contains("Modifier::OneSpellManaAnyType"),
        "{:?}",
        star.abilities
    );

    // Refusals: per-player CantPlayLand, resolution-pinned CantChangeLife/CantLoseLife,
    // all-player CantSearchLibrary, non-combat CantPreventDamage, Mode$ MustBlock.
    for bad in [
        "Name:BadLandLock\nManaCost:1 R\nTypes:Sorcery\nA:SP$ Effect | StaticAbilities$ S\nSVar:S:Mode$ CantPlayLand | Player$ You",
        "Name:BadRemChange\nManaCost:2 W\nTypes:Instant\nA:SP$ Effect | ValidTgts$ Player | RememberObjects$ Targeted | StaticAbilities$ S\nSVar:S:Mode$ CantChangeLife | ValidPlayer$ Player.IsRemembered",
        "Name:BadRemLose\nManaCost:2 W\nTypes:Instant\nA:SP$ Effect | ValidTgts$ Player | RememberObjects$ Targeted | StaticAbilities$ S\nSVar:S:Mode$ CantLoseLife | ValidPlayer$ Player.IsRemembered",
        "Name:BadAllSearch\nManaCost:2 U\nTypes:Instant\nA:SP$ Effect | StaticAbilities$ S\nSVar:S:Mode$ CantSearchLibrary | ValidPlayer$ Player",
        "Name:BadNoPrev\nManaCost:1 R\nTypes:Instant\nA:SP$ Effect | StaticAbilities$ S\nSVar:S:Mode$ CantPreventDamage",
        "Name:BadMustBlock\nManaCost:1 G\nTypes:Instant\nA:SP$ Effect | ValidTgts$ Creature | RememberObjects$ Targeted | StaticAbilities$ S\nSVar:S:Mode$ MustBlock | ValidCreature$ Creature.IsRemembered",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave22b_effect_continuous_modifiers_and_player_protection() {
    // Player.IsRemembered Hexproof & Protection from everything, plus permanent
    // RemoveKeyword$, CantHaveKeyword$, MayAssignDamageAsUnblocked$, hidden keywords.
    let hex_player = read(
        "Name:AegisTouch\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Effect | ValidTgts$ Player | RememberObjects$ Targeted | StaticAbilities$ S1,S2 | Duration$ UntilEndOfTurn\n\
         SVar:S1:Mode$ Continuous | Affected$ Player.IsRemembered | AddKeyword$ Hexproof\n\
         SVar:S2:Mode$ Continuous | Affected$ Player.IsRemembered | AddKeyword$ Protection from everything",
    );
    let hp_out = hex_player.abilities.join("\n");
    assert!(
        hp_out.contains("Modifier::PlayerHexproof { who: PlayerRel::Chosen }")
            && hp_out
                .contains("Modifier::PlayerProtectionFromEverything { who: PlayerRel::Chosen }"),
        "{hp_out}"
    );

    let perm_mods = read(
        "Name:ShearLight\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ Effect | ValidTgts$ Creature | RememberObjects$ Targeted | StaticAbilities$ S1,S2,S3\n\
         SVar:S1:Mode$ Continuous | Affected$ Card.IsRemembered | RemoveKeyword$ Flying & Haste | CantHaveKeyword$ Hexproof\n\
         SVar:S2:Mode$ Continuous | Affected$ Card.IsRemembered | MayAssignDamageAsUnblocked$ True\n\
         SVar:S3:Mode$ Continuous | Affected$ Card.IsRemembered | AddHiddenKeyword$ CARDNAME can't be blocked.",
    );
    let pm_out = perm_mods.abilities.join("\n");
    for expected in [
        "Modifier::RemoveKeyword(KeywordSet::FLYING.union(KeywordSet::HASTE))",
        "Modifier::CantHaveKeywords(KeywordSet::HEXPROOF)",
        "Modifier::MayAssignCombatDamageAsThoughUnblocked",
        "Modifier::AddKeyword(KeywordSet::UNBLOCKABLE)",
    ] {
        assert!(pm_out.contains(expected), "missing {expected}: {pm_out}");
    }

    let alone = read(
        "Name:LoneDuel\nManaCost:W\nTypes:Instant\n\
         A:SP$ Effect | ValidTgts$ Creature | RememberObjects$ Targeted | StaticAbilities$ S1\n\
         SVar:S1:Mode$ Continuous | Affected$ Card.IsRemembered | AddHiddenKeyword$ CARDNAME can only attack alone.",
    );
    assert!(
        alone.abilities[0].contains("Modifier::AttacksAloneOnly"),
        "{:?}",
        alone.abilities
    );
}

#[test]
fn wave22b_remember_changed_dig_mill_and_exile_play_unlocks() {
    // Fixed chains.rs early-exit: targeted BF->Exile RememberChanged$ True chained to GainLife/Token.
    let exile_gain = read(
        "Name:SoulReap\nManaCost:2 W\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ X | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        exile_gain.abilities[0].contains("Effect::exile(TargetSpec::Object(&Filter::CREATURE))")
            && exile_gain.abilities[0]
                .contains("Amount::ThisWay(ThisWay::ExiledFromBattlefield(&Filter::Any))"),
        "{:?}",
        exile_gain.abilities
    );

    // TopOfLibrary Library->Exile standalone and RememberChanged$ True.
    let top_exile = read(
        "Name:TopBanish\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Defined$ TopOfLibrary | Origin$ Library | Destination$ Exile | ChangeNum$ 2 | RememberChanged$ True | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ X\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        top_exile.abilities[0]
            .contains("Effect::ExileTop { who: PlayerRel::You, count: Amount::Fixed(2) }")
            && top_exile.abilities[0].contains("Amount::ThisWay(ThisWay::Exiled(&Filter::Any))"),
        "{:?}",
        top_exile.abilities
    );

    // Dig | DigNum$ 1 | DestinationZone$ Exile without ChangeNum$ All (both standalone & impulse play).
    let dig1_play = read(
        "Name:QuickSpark\nManaCost:1 R\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 1 | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | Name$ QuickSpark MayPlay | RememberObjects$ Remembered | ForgetOnMoved$ Exile | ForgetOnCast$ False | Stackable$ False | StaticAbilities$ STPlay | SubAbility$ DBClean\n\
         SVar:STPlay:Mode$ Continuous | Affected$ Card.IsRemembered | AffectedZone$ Exile | MayPlay$ True | EffectZone$ Command\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        dig1_play.abilities[0].contains("Effect::ExileTopToPlay"),
        "{:?}",
        dig1_play.abilities
    );
    let dig1_plain = read(
        "Name:PlainTopEx\nManaCost:R\nTypes:Instant\nA:SP$ Dig | DigNum$ 1 | DestinationZone$ Exile",
    );
    assert!(
        dig1_plain.abilities[0]
            .contains("Effect::ExileTop { who: PlayerRel::You, count: Amount::Fixed(1) }"),
        "{:?}",
        dig1_plain.abilities
    );

    // Multi-player Mill + redundant `.inZoneGraveyard` on Remembered$Valid, plus "you may return" MillMayTakeOne.
    let dread_shape = read_with_tokens(
        "Name:GraveCall\nManaCost:X B B\nTypes:Sorcery\n\
         A:SP$ Mill | Defined$ Player | NumCards$ X | RememberMilled$ True | SubAbility$ DBTok\n\
         SVar:DBTok:DB$ Token | TokenScript$ r_1_1_goblin | TokenAmount$ Y | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$xPaid\n\
         SVar:Y:Remembered$Valid Creature.inZoneGraveyard",
    );
    assert!(
        dread_shape.abilities[0]
            .contains("Effect::Mill { amount: Amount::X, target: PlayerRel::EachPlayer }")
            && dread_shape.abilities[0]
                .contains("Amount::ThisWay(ThisWay::Milled(&Filter::CREATURE))"),
        "{:?}",
        dread_shape.abilities
    );
    let till_shape = read(
        "Name:NightTill\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ Mill | NumCards$ 4 | Defined$ You | RememberMilled$ True | SubAbility$ DBTake\n\
         SVar:DBTake:DB$ ChangeZone | Origin$ Graveyard | Destination$ Hand | ChangeType$ Permanent.IsRemembered\n\
         Oracle:Mill four cards, then you may return a permanent card from among them to your hand.",
    );
    assert!(
        till_shape.abilities[0].contains("Effect::MillMayTakeOne"),
        "{:?}",
        till_shape.abilities
    );

    for bad in [
        "Name:BadZonePlay\nManaCost:1 R\nTypes:Sorcery\nA:SP$ Dig | DigNum$ 1 | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ E\nSVar:E:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ S\nSVar:S:Mode$ Continuous | Affected$ Card.IsRemembered | AffectedZone$ Exile | MayPlay$ True | EffectZone$ Graveyard",
        "Name:BadStackable\nManaCost:1 R\nTypes:Sorcery\nA:SP$ Dig | DigNum$ 1 | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ E\nSVar:E:DB$ Effect | RememberObjects$ Remembered | Stackable$ True | StaticAbilities$ S\nSVar:S:Mode$ Continuous | Affected$ Card.IsRemembered | AffectedZone$ Exile | MayPlay$ True",
        "Name:BadOptDigEx\nManaCost:R\nTypes:Instant\nA:SP$ Dig | DigNum$ 1 | DestinationZone$ Exile | Optional$ True",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave22b_hand_exile_targetmin_and_zone_all() {
    // Hand -> Exile (mandatory & optional), self Hand->Exile, self Exile->Hand, spell Stack->Hand.
    let hand_ex = read(
        "Name:MindDrain\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Hand | Destination$ Exile | ChangeType$ Card | ChangeNum$ 1 | Hidden$ True | Mandatory$ True",
    );
    assert!(
        hand_ex.abilities[0].contains("Effect::ExileChosenFromHandWithSource(&Filter::Any)"),
        "{:?}",
        hand_ex.abilities
    );
    let opt_hand_ex = read(
        "Name:OptDrain\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Hand | Destination$ Exile | ChangeType$ Card | ChangeNum$ 1 | Hidden$ True | Optional$ True",
    );
    assert!(
        opt_hand_ex.abilities[0].contains(
            "Effect::MayDo { effects: &[Effect::ExileChosenFromHandWithSource(&Filter::Any)] }"
        ),
        "{:?}",
        opt_hand_ex.abilities
    );
    let self_hand_ex = read(
        "Name:SelfBanish\nManaCost:1 W\nTypes:Creature Goblin\nPT:2/2\n\
         A:AB$ ChangeZone | Cost$ 1 | Defined$ Self | Origin$ Hand | Destination$ Exile | ActivationZone$ Hand",
    );
    assert!(
        self_hand_ex.abilities[0].contains("Effect::ExileSource"),
        "{:?}",
        self_hand_ex.abilities
    );
    let self_zones = read(
        "Name:LoopSpell\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ Draw | NumCards$ 1 | SubAbility$ DBRet\n\
         SVar:DBRet:DB$ ChangeZone | Origin$ Stack | Destination$ Hand",
    );
    assert!(
        self_zones.abilities[0].contains("Effect::bounce(TargetSpec::ThisObject)"),
        "{:?}",
        self_zones.abilities
    );

    // TargetMin: OneEach + YouDontCtrl, and reordered-atom Count$Valid.
    let each_opp = read(
        "Name:EachExile\nManaCost:2 W B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Creature.YouDontCtrl | TargetMin$ 0 | TargetMax$ OneEach | TargetsForEachPlayer$ True | Origin$ Battlefield | Destination$ Exile\n\
         SVar:OneEach:PlayerCountOpponents$Amount",
    );
    assert!(
        each_opp.abilities[0].contains("TargetSpec::ObjectOfEachOpponent("),
        "{:?}",
        each_opp.abilities
    );
    let reorder_max = read(
        "Name:GravePurge\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Creature.YouOwn+Goblin | TgtZone$ Graveyard | TargetMin$ 0 | TargetMax$ X | Origin$ Graveyard | Destination$ Exile\n\
         SVar:X:Count$ValidGraveyard Creature.Goblin+YouOwn",
    );
    assert!(
        reorder_max.abilities[0].contains("Effect::exile(TargetSpec::CardInGraveyard("),
        "{:?}",
        reorder_max.abilities
    );

    // ChangeZoneAll GY->Exile Card.YouCtrl & GY->BF GainControl$ True.
    let cza_ex = read(
        "Name:ClearYard\nManaCost:1 W\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ChangeType$ Card.YouCtrl | Origin$ Graveyard | Destination$ Exile",
    );
    assert!(
        cza_ex.abilities[0].contains("Effect::ExileGraveyard { player: PlayerRel::You }"),
        "{:?}",
        cza_ex.abilities
    );
    let cza = read(
        "Name:MassRaise\nManaCost:4 B B\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ChangeType$ Creature.YouOwn | Origin$ Graveyard | Destination$ Battlefield | GainControl$ True",
    );
    assert!(
        cza.abilities[0].contains(
            "Effect::YourGraveyardToBattlefield { filter: &Filter::CREATURE, tapped: false }"
        ),
        "{:?}",
        cza.abilities
    );

    for bad in [
        "Name:BadOppHandEx\nManaCost:B\nTypes:Sorcery\nA:SP$ ChangeZone | Origin$ Hand | Destination$ Exile | DefinedPlayer$ Opponent | ChangeType$ Card | ChangeNum$ 1 | Hidden$ True",
        "Name:BadTwoHandEx\nManaCost:B\nTypes:Sorcery\nA:SP$ ChangeZone | Origin$ Hand | Destination$ Exile | ChangeType$ Card | ChangeNum$ 2 | Hidden$ True",
        "Name:BadFaceDownHandEx\nManaCost:B\nTypes:Sorcery\nA:SP$ ChangeZone | Origin$ Hand | Destination$ Exile | ChangeType$ Card | ChangeNum$ 1 | Hidden$ True | ExileFaceDown$ True",
        "Name:BadMaxMismatch\nManaCost:B\nTypes:Sorcery\nA:SP$ ChangeZone | ValidTgts$ Creature.YouOwn | TgtZone$ Graveyard | TargetMin$ 0 | TargetMax$ X | Origin$ Graveyard | Destination$ Exile\nSVar:X:Count$ValidGraveyard Artifact.YouOwn",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave22b_filters_pump_counters_and_multiplayer_effects() {
    // filter atom cmcLE<ColorsOfManaSpentToCast>.
    let conv_filter = read(
        "Name:PrismSnare\nManaCost:2 U\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Creature.cmcLEX | Origin$ Battlefield | Destination$ Hand\n\
         SVar:X:Count$ColorsOfManaSpentToCast",
    );
    assert!(
        conv_filter.statics.contains("Filter::CmcAtMostColorsSpent"),
        "{}",
        conv_filter.statics
    );

    // Pump (Taps TriggeredCard, DamageDone TriggeredTarget, Self & Targeted) & PutCounter DamageDoneOnce.
    let self_and_tgt = read(
        "Name:PairBoost\nManaCost:2 G\nTypes:Creature Goblin\nPT:2/2\n\
         A:AB$ Pump | Cost$ 1 G | ValidTgts$ Creature | Defined$ Self & Targeted | NumAtt$ +1 | NumDef$ +1",
    );
    assert!(
        self_and_tgt.abilities[0].contains("Effect::PumpTarget {")
            && self_and_tgt.abilities[0].contains("Effect::PumpFilter { filter: &Filter::This"),
        "{:?}",
        self_and_tgt.abilities
    );
    let tap_pump = read(
        "Name:TapBoost\nManaCost:1 G\nTypes:Enchantment\n\
         T:Mode$ Taps | ValidCard$ Creature.YouCtrl | Execute$ TrigPump | TriggerZones$ Battlefield\n\
         SVar:TrigPump:DB$ Pump | Defined$ TriggeredCard | NumAtt$ +1 | NumDef$ +1",
    );
    assert!(
        tap_pump.abilities[0].contains("Effect::PumpEventObject {"),
        "{:?}",
        tap_pump.abilities
    );
    let dmg_cnt = read(
        "Name:VenomMark\nManaCost:2 B\nTypes:Creature Goblin\nPT:2/2\n\
         T:Mode$ DamageDoneOnce | ValidTarget$ Creature | Execute$ TrigCnt | TriggerZones$ Battlefield\n\
         SVar:TrigCnt:DB$ PutCounter | Defined$ TriggeredTarget | CounterType$ M1M1 | CounterNum$ 1",
    );
    assert!(
        dmg_cnt.abilities[0].contains("Effect::AddCounterFilter { filter: &Filter::Reference"),
        "{:?}",
        dmg_cnt.abilities
    );

    // Multi-player Draw, GainLife, LoseLife (`TargetedAndYou`, `You & <rel>`).
    let shared = read(
        "Name:SharedGift\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ GainLife | ValidTgts$ Opponent | Defined$ TargetedAndYou | LifeAmount$ 3",
    );
    assert!(
        shared.abilities[0].contains("Effect::gain_life(3)")
            && shared.abilities[0].contains(
                "Effect::GainLifeFor { amount: Amount::Fixed(3), who: PlayerRel::Chosen }"
            ),
        "{:?}",
        shared.abilities
    );
    let shared_loss = read(
        "Name:SharedPain\nManaCost:2 B\nTypes:Enchantment\n\
         T:Mode$ AttackersDeclared | AttackingPlayer$ Opponent | Execute$ TrigLoss | TriggerZones$ Battlefield\n\
         SVar:TrigLoss:DB$ LoseLife | Defined$ TriggeredAttackingPlayer & You | LifeAmount$ 1",
    );
    assert!(
        shared_loss.abilities[0]
            .contains("Effect::LoseLife { amount: Amount::Fixed(1), target: PlayerRel::You }")
            && shared_loss.abilities[0].contains(
                "Effect::LoseLife { amount: Amount::Fixed(1), target: PlayerRel::EventPlayer }"
            ),
        "{:?}",
        shared_loss.abilities
    );
    let dmg_draw = read(
        "Name:SharedSight\nManaCost:2 U\nTypes:Creature Wizard\nPT:2/2\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ TrigDraw | TriggerZones$ Battlefield\n\
         SVar:TrigDraw:DB$ Draw | Defined$ You & TriggeredTarget | NumCards$ 2",
    );
    assert!(
        dmg_draw.abilities[0].contains("Effect::draw(2)")
            && dmg_draw.abilities[0].contains(
                "Effect::DrawCardsFor { amount: Amount::Fixed(2), who: PlayerRel::DamagedPlayer }"
            ),
        "{:?}",
        dmg_draw.abilities
    );
}

#[test]
fn wave23b_repeating_modal_and_mode_counts() {
    // Choose three with repeating modes + UnlessPayer$ ThisTargetedController.
    let confluence = read(
        "Name:ArcaneConfluence\nManaCost:3 U U\nTypes:Instant\n\
         A:SP$ Charm | CharmNum$ 3 | CanRepeatModes$ True | Choices$ MCounter,MBounce,MDraw\n\
         SVar:MCounter:DB$ Counter | TargetType$ Spell | TgtPrompt$ Select target spell | ValidTgts$ Card | UnlessCost$ 3 | UnlessPayer$ ThisTargetedController\n\
         SVar:MBounce:DB$ ChangeZone | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Hand\n\
         SVar:MDraw:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert!(
        confluence.abilities[0].contains("AbilityDef::Modal {")
            && confluence.abilities[0].contains("choose: ModeCount::THREE, can_repeat_modes: true")
            && confluence.abilities[0].contains(
                "Effect::PlayerMayPayOr { player: PlayerRel::ControllerOfTarget, mana: Amount::Fixed(3), effect: &Effect::CounterTargetSpell }"
            ),
        "{:?}",
        confluence.abilities
    );

    // Choose four & choose five with repeating modes, plus MinCharmNum$ 0 (up to 4).
    let four = read(
        "Name:FourModes\nManaCost:5 G G\nTypes:Sorcery\n\
         A:SP$ Charm | CharmNum$ 4 | CanRepeatModes$ True | Choices$ M1,M2\n\
         SVar:M1:DB$ GainLife | Defined$ You | LifeAmount$ 4\n\
         SVar:M2:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert!(
        four.abilities[0].contains("choose: ModeCount::FOUR, can_repeat_modes: true"),
        "{:?}",
        four.abilities
    );
    let five = read(
        "Name:FiveModes\nManaCost:4 R\nTypes:Instant\n\
         A:SP$ Charm | CharmNum$ 5 | CanRepeatModes$ True | Choices$ M1,M2\n\
         SVar:M1:DB$ DealDamage | ValidTgts$ Any | NumDmg$ 1\n\
         SVar:M2:DB$ GainLife | Defined$ You | LifeAmount$ 2",
    );
    assert!(
        five.abilities[0].contains("choose: ModeCount::FIVE, can_repeat_modes: true"),
        "{:?}",
        five.abilities
    );
    let up_to_four = read(
        "Name:ReckoningModes\nManaCost:4 W\nTypes:Sorcery\n\
         A:SP$ Charm | MinCharmNum$ 0 | CharmNum$ 4 | CanRepeatModes$ True | Choices$ M1,M2\n\
         SVar:M1:DB$ GainLife | Defined$ You | LifeAmount$ 2\n\
         SVar:M2:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert!(
        up_to_four.abilities[0]
            .contains("choose: ModeCount { min: 0, max: 4 }, can_repeat_modes: true"),
        "{:?}",
        up_to_four.abilities
    );

    // Non-repeating choose-three spell with Defined$ You, and AB$ Charm with Defined$ You.
    let non_rep_three = read(
        "Name:TriChoice\nManaCost:3 U\nTypes:Instant\n\
         A:SP$ Charm | Defined$ You | CharmNum$ 3 | Choices$ M1,M2,M3,M4\n\
         SVar:M1:DB$ Draw | Defined$ You | NumCards$ 1\n\
         SVar:M2:DB$ GainLife | Defined$ You | LifeAmount$ 2\n\
         SVar:M3:DB$ Scry | ScryNum$ 1\n\
         SVar:M4:DB$ LoseLife | Defined$ Opponent | LifeAmount$ 1",
    );
    assert!(
        non_rep_three.abilities[0].contains("AbilityDef::ModalSpell")
            && non_rep_three.abilities[0].contains("choose: ModeCount::THREE"),
        "{:?}",
        non_rep_three.abilities
    );
    let act_charm = read(
        "Name:JitteBlade\nManaCost:2\nTypes:Artifact\n\
         A:AB$ Charm | Cost$ SubCounter<1/CHARGE> | Defined$ You | Choices$ M1,M2\n\
         SVar:M1:DB$ GainLife | Defined$ You | LifeAmount$ 2\n\
         SVar:M2:DB$ Draw | Defined$ You | NumCards$ 1",
    );
    assert!(
        act_charm.abilities[0].starts_with("modal_activated!("),
        "{:?}",
        act_charm.abilities
    );

    // Refusals: Pawprint$ (Seasons), CharmNum$ X, CharmNum$ 9 (>8), MinCharmNum$ 0 without repeat,
    // Defined$ Opponent on Charm, Spree with CanRepeatModes$.
    for bad in [
        "Name:SeasonBad\nManaCost:4 W W\nTypes:Sorcery\n\
         A:SP$ Charm | Pawprint$ 5 | CanRepeatModes$ True | Choices$ M1,M2\n\
         SVar:M1:DB$ GainLife | Defined$ You | LifeAmount$ 2\n\
         SVar:M2:DB$ Draw | Defined$ You | NumCards$ 1",
        "Name:TooManyRepeats\nManaCost:6 U\nTypes:Sorcery\n\
         A:SP$ Charm | CharmNum$ 9 | CanRepeatModes$ True | Choices$ M1,M2\n\
         SVar:M1:DB$ GainLife | Defined$ You | LifeAmount$ 1\n\
         SVar:M2:DB$ Draw | Defined$ You | NumCards$ 1",
        "Name:ZeroNonRepeat\nManaCost:3 W\nTypes:Sorcery\n\
         A:SP$ Charm | MinCharmNum$ 0 | CharmNum$ 2 | Choices$ M1,M2\n\
         SVar:M1:DB$ GainLife | Defined$ You | LifeAmount$ 1\n\
         SVar:M2:DB$ Draw | Defined$ You | NumCards$ 1",
        "Name:OppCharm\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ Charm | Defined$ Opponent | Choices$ M1,M2\n\
         SVar:M1:DB$ GainLife | Defined$ You | LifeAmount$ 1\n\
         SVar:M2:DB$ Draw | Defined$ You | NumCards$ 1",
        "Name:SpreeRepeat\nManaCost:R\nTypes:Sorcery\nK:Spree\n\
         A:SP$ Charm | CanRepeatModes$ True | CharmNum$ 2 | Choices$ M1,M2\n\
         SVar:M1:DB$ GainLife | Defined$ You | LifeAmount$ 1 | ModeCost$ 1\n\
         SVar:M2:DB$ Draw | Defined$ You | NumCards$ 1 | ModeCost$ 2",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave23b_targets_with_same_controller() {
    // Spell multi-graveyard exile with TargetsWithSameController$ True.
    let feast = read(
        "Name:GraveFeast\nManaCost:B\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Exile | TgtZone$ Graveyard \
         | ValidTgts$ Card | TargetMin$ 0 | TargetMax$ 3 | TargetsWithSameController$ True",
    );
    assert!(
        feast.abilities[0].contains(
            "TargetReq::up_to(TargetSpec::CardInGraveyard(&Filter::Any, PlayerRel::EachPlayer), 3)\
             .with_constraint(TargetConstraint::SameController)"
        ),
        "{:?}",
        feast.abilities
    );

    // Activated ability with TargetsWithSameController$ True.
    let beetles = read(
        "Name:GraveBeetles\nManaCost:B\nTypes:Creature Goblin\nPT:1/1\n\
         A:AB$ ChangeZone | Cost$ 2 B T | Origin$ Graveyard | Destination$ Exile | TgtZone$ Graveyard \
         | ValidTgts$ Card | TargetMin$ 0 | TargetMax$ 3 | TargetsWithSameController$ True",
    );
    assert!(
        beetles.abilities[0].starts_with("activated!(")
            && beetles.abilities[0].contains(".with_constraint(TargetConstraint::SameController)"),
        "{:?}",
        beetles.abilities
    );

    // Charm mode with TargetsWithSameController$ True.
    let charm = read(
        "Name:DarkCharm\nManaCost:B\nTypes:Instant\n\
         A:SP$ Charm | Choices$ MDrain,MExile\n\
         SVar:MDrain:DB$ LoseLife | ValidTgts$ Opponent | LifeAmount$ 1\n\
         SVar:MExile:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | TgtZone$ Graveyard \
         | ValidTgts$ Card | TargetMin$ 0 | TargetMax$ 3 | TargetsWithSameController$ True",
    );
    assert!(
        charm.abilities[0].contains(".with_constraint(TargetConstraint::SameController)"),
        "{:?}",
        charm.abilities
    );

    // Refusals: missing ValidTgts$, single-target (max=1), player target, invalid value.
    for bad in [
        "Name:NoTgt\nManaCost:B\nTypes:Sorcery\nA:SP$ Draw | Defined$ You | NumCards$ 1 | TargetsWithSameController$ True",
        "Name:SingleTgt\nManaCost:B\nTypes:Instant\nA:SP$ ChangeZone | Origin$ Graveyard | Destination$ Exile | TgtZone$ Graveyard | ValidTgts$ Card | TargetMin$ 1 | TargetMax$ 1 | TargetsWithSameController$ True",
        "Name:PlayerTgt\nManaCost:2 B\nTypes:Sorcery\nA:SP$ LoseLife | ValidTgts$ Player | TargetMin$ 1 | TargetMax$ 2 | LifeAmount$ 1 | TargetsWithSameController$ True",
        "Name:BadVal\nManaCost:B\nTypes:Instant\nA:SP$ ChangeZone | Origin$ Graveyard | Destination$ Exile | TgtZone$ Graveyard | ValidTgts$ Card | TargetMin$ 0 | TargetMax$ 3 | TargetsWithSameController$ Maybe",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave23b_excess_damage() {
    // Excess damage -> GainLife.
    let life = read(
        "Name:SearingBalm\nManaCost:2 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 4 | ExcessSVar$ Excess | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ Excess",
    );
    assert!(
        life.abilities[0].contains("Effect::DealDamage {")
            && life.abilities[0].contains("Amount::ThisWay(ThisWay::ExcessDamage)"),
        "{:?}",
        life.abilities
    );

    // Excess damage -> unconditional TokenAmount$ Excess + TokenTapped$ True.
    let tok = read_with_tokens(
        "Name:ExcessGoblins\nManaCost:X R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ X | ExcessSVar$ Excess | SubAbility$ DBTok\n\
         SVar:DBTok:DB$ Token | TokenAmount$ Excess | TokenScript$ r_1_1_goblin | TokenTapped$ True\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        tok.abilities[0].contains("Effect::CreateTokenModified {")
            && tok.abilities[0].contains("amount: Amount::ThisWay(ThisWay::ExcessDamage)")
            && tok.abilities[0].contains("tapped: true, haste: false"),
        "{:?}",
        tok.abilities
    );

    // Excess damage -> conditional Investigate + PumpAll (+Excess).
    let inv = read_with_tokens(
        "Name:TorchClue\nManaCost:X R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ X | ExcessSVar$ Excess | SubAbility$ DBInv\n\
         SVar:DBInv:DB$ Investigate | ConditionCheckSVar$ Excess | ConditionSVarCompare$ GE1\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        inv.abilities[0].contains("Effect::IfCondition {")
            && inv.abilities[0]
                .contains("Condition::AmountAtLeast(Amount::ThisWay(ThisWay::ExcessDamage), 1)")
            && inv.abilities[0].contains("Effect::CreateToken { token: &generated_tokens::CLUE }"),
        "{:?}",
        inv.abilities
    );
    let pump = read(
        "Name:ExcessRally\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 5 | ExcessSVar$ Excess | SubAbility$ DBPump\n\
         SVar:DBPump:DB$ PumpAll | ValidCards$ Creature.YouCtrl | NumAtt$ +Excess | NumDef$ +0",
    );
    assert!(
        pump.abilities[0].contains("Effect::PumpFilter {")
            && pump.abilities[0].contains("power: Amount::ThisWay(ThisWay::ExcessDamage)"),
        "{:?}",
        pump.abilities
    );

    // ExcessSVarCondition$ Creature on AnyTarget -> Condition::All(&[ObjectMatches, AmountAtLeast]).
    let cond_draw = read(
        "Name:ExcessInsight\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature,Planeswalker | NumDmg$ 4 | ExcessSVar$ Excess \
         | ExcessSVarCondition$ Creature | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1 | ConditionCheckSVar$ Excess | ConditionSVarCompare$ GE1",
    );
    assert!(
        cond_draw.abilities[0].contains("Condition::All(&[Condition::ObjectMatches")
            && cond_draw.abilities[0]
                .contains("Condition::AmountAtLeast(Amount::ThisWay(ThisWay::ExcessDamage), 1)"),
        "{:?}",
        cond_draw.abilities
    );

    // Excess damage -> Dig (impulse exile with permission), Scry, Surveil, and activated -> LoseLife.
    let dig = read(
        "Name:ImpulseBlast\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 4 | ExcessSVar$ Excess | SubAbility$ DBDig\n\
         SVar:DBDig:DB$ Dig | DigNum$ Excess | DestinationZone$ Exile | ChangeNum$ All | RememberChanged$ True | SubAbility$ DBMayPlay\n\
         SVar:DBMayPlay:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ MayPlay | Duration$ UntilEndOfTurn | SubAbility$ DBClean\n\
         SVar:MayPlay:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        dig.abilities[0].contains("Effect::ExileTopWithPermission {")
            && dig.abilities[0].contains("Amount::ThisWay(ThisWay::ExcessDamage)"),
        "{:?}",
        dig.abilities
    );
    let scry_surveil = read(
        "Name:ScryBlast\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 3 | ExcessSVar$ Excess | SubAbility$ DBScry\n\
         SVar:DBScry:DB$ Scry | ScryNum$ Excess",
    );
    assert!(
        scry_surveil.abilities[0]
            .contains("Effect::Scry { amount: Amount::ThisWay(ThisWay::ExcessDamage) }"),
        "{:?}",
        scry_surveil.abilities
    );
    let act = read(
        "Name:ExcessStriker\nManaCost:2 R\nTypes:Creature Goblin\nPT:2/2\n\
         A:AB$ DealDamage | Cost$ 2 R T | ValidTgts$ Creature | NumDmg$ 3 | ExcessSVar$ Excess | SubAbility$ DBLose\n\
         SVar:DBLose:DB$ LoseLife | Defined$ Opponent | LifeAmount$ Excess",
    );
    assert!(
        act.abilities[0].starts_with("activated!(")
            && act.abilities[0].contains("Amount::ThisWay(ThisWay::ExcessDamage)"),
        "{:?}",
        act.abilities
    );
}

#[test]
fn wave23b_excess_damage_refusals() {
    // Refusals: missing SubAbility$, conflicting SVar definition, ExcessSVarCondition$ Creature
    // with unconditional numeric use, player-only ValidTgts$, Alchemy follow-up.
    for bad in [
        "Name:NoSub\nManaCost:1 R\nTypes:Instant\nA:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 3 | ExcessSVar$ Excess",
        "Name:ConflictSvar\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 3 | ExcessSVar$ Excess | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ Excess\n\
         SVar:Excess:Count$xPaid",
        "Name:CondNumeric\nManaCost:2 R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature,Planeswalker | NumDmg$ 4 | ExcessSVar$ Excess \
         | ExcessSVarCondition$ Creature | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ Excess",
        "Name:PlayerOnlyExcess\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Player | NumDmg$ 3 | ExcessSVar$ Excess | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ Excess",
        "Name:AlchemyExcess\nManaCost:1 R\nTypes:Instant\n\
         A:SP$ DealDamage | ValidTgts$ Creature | NumDmg$ 3 | ExcessSVar$ Excess | SubAbility$ DBAlch\n\
         SVar:DBAlch:DB$ MakeCard | Name$ Lightning Bolt | ConditionCheckSVar$ Excess | ConditionSVarCompare$ GE1",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn pump_opponents_only_lure_uses_opponent_creature_filter() {
    let tarrasque = read(
        "Name:You Look Upon the Tarrasque\nManaCost:4 G\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ +5 | NumDef$ +5 \
         | KW$ Indestructible & HIDDEN All creatures able to block CARDNAME do so. \
         | SpellDescription$ Gather Your Courage — Target creature gets +5/+5 and gains indestructible until end of turn. All creatures your opponents control able to block that creature this turn do so.\n",
    );
    assert!(
        tarrasque.abilities[0]
            .contains("Modifier::MustBeBlockedByAllAbleMatching(&Filter::OPPONENT_CREATURE)"),
        "{:?}",
        tarrasque.abilities
    );
}
