//! Reading zone changes, tokens, counters on the way in, equip, cycling,
//! auras and investigating.

use super::*;

#[test]
fn a_zone_change_is_read_as_the_pair_it_is() {
    let body = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Hand | ValidTgts$ Creature\n",
    );
    assert!(body.abilities.join("").contains("Effect::bounce("));

    let body = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Creature\n",
    );
    assert!(body.abilities.join("").contains("Effect::exile("));

    let body = read(
        "Name:X\nTypes:Creature\n\
         A:AB$ ChangeZone | Cost$ T | Origin$ Battlefield | Destination$ Exile | Defined$ Self\n",
    );
    assert!(body.abilities.join("").contains("Effect::ExileSource"));
}

#[test]
fn putting_a_creature_in_a_graveyard_is_not_destroying_it() {
    // CR 701.8b: destruction checks indestructible and a zone change does
    // not, so the nearest effect is the wrong effect — a card written
    // this way would quietly kill creatures that survive.
    assert!(refused(
        "Name:X\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Graveyard | ValidTgts$ Creature\n"
    ));
}

/// A spell's `Cost$` carries the card's mana cost *and* whatever the
/// printing charges beside it, and only the first half has a home on the
/// face. The branch read neither and refused nothing, so Crop Rotation
/// shipped as a one-mana tutor that sacrifices no land and Kaervek's
/// Spite as three mana for five life off a target — both
/// `Coverage::Implemented`, both offered to a deckbuilder as playable.
#[test]
fn an_additional_cost_on_a_spell_is_refused_and_not_dropped() {
    // Crop Rotation.
    assert!(refused(
        "Name:X\nTypes:Instant\n\
         A:SP$ ChangeZone | Cost$ G Sac<1/Land> | Origin$ Library | \
         Destination$ Battlefield | ChangeType$ Land | ChangeNum$ 1"
    ));
    // Kaervek's Spite: two additional costs, and the first of them is
    // one `cost_pieces` can read, so the refusal has to come from the
    // spell branch rather than from a part nobody could map.
    assert!(refused(
        "Name:X\nTypes:Instant\n\
         A:SP$ LoseLife | Cost$ B B B Sac<All/Permanent> Discard<0/Hand> | \
         ValidTgts$ Player | LifeAmount$ 5"
    ));
    // The mana half alone is not an additional cost: it is the card's
    // own mana cost, which the face already carries.
    let body = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ LoseLife | Cost$ B B B | ValidTgts$ Player | LifeAmount$ 5",
    );
    assert_eq!(body.abilities.len(), 1, "{:?}", body.abilities);
    assert!(body.abilities[0].starts_with("spell!("));
    // `XMin1` is "X can't be 0", a printed restriction and not mana —
    // and it carries no brackets, which is the whole reason the reading
    // is an allow-list rather than a hunt for `<…>`.
    assert!(refused(
        "Name:X\nTypes:Sorcery\nA:SP$ Mill | Cost$ XMin1 X B | NumCards$ 1"
    ));
    // And neither is an announced `X`, which is why this reads the token
    // itself: `cost_pieces` prices an activation and denies a bare `X`,
    // so asking it here would have refused a card over its mana cost.
    let body = read(
        "Name:X\nTypes:Sorcery\n\
         A:SP$ Draw | Cost$ X U | NumCards$ 1",
    );
    assert_eq!(body.abilities.len(), 1, "{:?}", body.abilities);
}

#[test]
fn anything_unread_refuses_the_whole_card() {
    // An unknown effect API.
    assert!(refused(
        "Name:X\nTypes:Sorcery\nA:SP$ Animate | Defined$ Self"
    ));
    // A known API with a parameter no rule claims.
    assert!(refused(
        "Name:X\nTypes:Sorcery\nA:SP$ Draw | NumCards$ 1 | UnlessCost$ 2"
    ));
    // A keyword that is data rather than a bit, and that no rule reads.
    // This was `K:Cycling:2` until one rule read it, which is the whole
    // point of the line: the example has to be a keyword nothing here
    // understands *today*, and typecycling is the nearest one — a
    // different sentence (CR 702.29e, a library search) that #51 will
    // take and this assertion will then have to move again.
    assert!(refused(
        "Name:X\nTypes:Creature Goblin\nPT:1/1\nK:TypeCycling:Basic:2"
    ));
    // A line kind with rules in it that this module does not model.
    assert!(refused(
        "Name:X\nTypes:Creature Goblin\nPT:1/1\n\
         R:Event$ Moved | Destination$ Graveyard | ValidCard$ Card.Self | ReplaceWith$ Exile"
    ));
    // A `SubAbility$` whose SVar is missing.
    assert!(refused(
        "Name:X\nTypes:Sorcery\nA:SP$ Draw | NumCards$ 1 | SubAbility$ Missing"
    ));
}

/// A vanilla creature has no rules for this module to read, and must not
/// be reported as a card it understood.
#[test]
fn a_card_with_no_rules_lines_is_not_a_transcoded_card() {
    assert!(refused(
        "Name:Grizzly Bears\nManaCost:1 G\nTypes:Creature Bear\nPT:2/2"
    ));
}

#[test]
fn a_valid_string_becomes_the_filter_it_describes() {
    let body = read("Name:X\nTypes:Sorcery\nA:SP$ Destroy | ValidTgts$ Creature.YouCtrl+nonToken");
    assert!(body.statics.contains(
        "Filter::And(&[Filter::CREATURE, Filter::ControlledByYou, Filter::Not(&Filter::IsToken)])"
    ));
}

/// A token effect names the constant the **ledger** filed the token
/// under, through the one module both halves of the ledger come out of.
#[test]
fn a_token_effect_names_the_constant_the_ledger_filed_it_under() {
    let body = read_with_tokens(
        "Name:Raise the Alarm\nTypes:Instant\n\
         A:SP$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ You",
    );
    assert!(
        body.abilities[0]
            .contains("Effect::CreateToken { token: &generated_tokens::GOBLIN_1_1_RED }"),
        "{}",
        body.abilities[0]
    );
}

/// "Create two 1/1 white Soldier creature tokens" is one effect with a
/// number, and one token is not a count of one — `CreateToken` already
/// means that, and two spellings of the commonest token effect there is
/// would be two things to keep in step.
#[test]
fn a_count_is_written_only_where_there_is_something_to_count() {
    let two = read_with_tokens(
        "Name:Raise the Alarm\nTypes:Instant\n\
         A:SP$ Token | TokenAmount$ 2 | TokenScript$ r_1_1_goblin | TokenOwner$ You",
    );
    assert!(
        two.abilities[0].contains(
            "Effect::CreateTokenN { token: &generated_tokens::GOBLIN_1_1_RED, \
             amount: Amount::Fixed(2) }"
        ),
        "{}",
        two.abilities[0]
    );
    let one = read_with_tokens(
        "Name:X\nTypes:Instant\n\
         A:SP$ Token | TokenAmount$ 1 | TokenScript$ u_1_1_wizard_flying | TokenOwner$ You",
    );
    assert!(
        one.abilities[0].contains(
            "Effect::CreateToken { token: \
             &generated_tokens::WIZARD_1_1_BLUE_FLYING }"
        ),
        "{}",
        one.abilities[0]
    );
}

/// A token the reader refuses refuses the card that makes it. The
/// alternative is a card that puts an inert permanent on the battlefield
/// and claims `Implemented` — a Treasure that cannot be sacrificed for
/// mana is not a Treasure.
///
/// This used to be asserted *of* the Treasure, because a token with an
/// ability was refused outright. It is now asserted of the shape that is
/// still refused — a token whose ability makes a token — and the
/// Treasure is the case below.
#[test]
fn a_token_that_cannot_be_read_refuses_the_card() {
    let line =
        "Name:X\nTypes:Instant\nA:SP$ Token | TokenScript$ r_1_1_goblin_maker | TokenOwner$ You";
    assert!(refused_with_tokens(line));
    assert_eq!(
        refusal_reason(&parse(line), &cats(), Some(&tokens())).as_deref(),
        Some("token script `r_1_1_goblin_maker`")
    );
}

/// And the card that makes an ability-bearing token names it like any
/// other. The 98 cards that create a Treasure are what this is about:
/// they were refused, whole, because the Treasure was.
#[test]
fn a_card_may_make_a_token_that_carries_an_ability() {
    let body = read_with_tokens(
        "Name:X\nTypes:Instant\nA:SP$ Token | TokenScript$ r_1_1_goblin_sac | TokenOwner$ You",
    );
    assert!(
        body.abilities[0]
            .contains("Effect::CreateToken { token: &generated_tokens::GOBLIN_1_1_RED }"),
        "{}",
        body.abilities[0]
    );
}

/// The number a player announced is a token amount, and only on a line
/// that announced one.
///
/// `X` is what 359 of the corpus's `TokenAmount$` values are — 356
/// scripts, a few of which write it twice — and it is
/// the same letter for thirty different counts: 58 of those scripts
/// define `SVar:X:Count$xPaid` — the X paid for — and the rest count
/// opponents, damage dealt, creatures in a graveyard. `Amount::X` reads
/// back only the first of them, so the definition is demanded.
///
/// The second half is the one that is invisible at the use site. A
/// **triggered** ability announces no number at all, so `Amount::X`
/// there is `x.unwrap_or(0)`: a card that compiles, claims
/// `Implemented` and makes nothing. Both counter-cases below are
/// refused by the same rule, and each names what it resolved through so
/// the report ranks the count rather than the letter.
#[test]
fn an_x_is_a_token_amount_only_where_a_player_announced_one() {
    let body = read_with_tokens(
        "Name:X\nTypes:Sorcery\nA:SP$ Token | Cost$ X G | TokenScript$ r_1_1_goblin \
         | TokenOwner$ You | TokenAmount$ X\nSVar:X:Count$xPaid",
    );
    assert!(
        body.abilities[0].contains(
            "Effect::CreateTokenN { token: &generated_tokens::GOBLIN_1_1_RED, \
             amount: Amount::X }"
        ),
        "{}",
        body.abilities[0]
    );

    // The same `X`, counted a way nothing here can say.
    let domain = parse(
        "Name:X\nTypes:Sorcery\nA:SP$ Token | Cost$ G | TokenScript$ r_1_1_goblin \
         | TokenOwner$ You | TokenAmount$ X\nSVar:X:Count$Domain",
    );
    assert_eq!(
        refusal_reason(&domain, &cats(), Some(&tokens())).as_deref(),
        Some("token amount `X` = `Count$Domain`")
    );

    // The right definition on a line that announces nothing.
    let triggered = parse(
        "Name:X\nTypes:Creature Goblin\nPT:1/1\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self \
         | Execute$ TrigToken\nSVar:TrigToken:DB$ Token | TokenScript$ r_1_1_goblin \
         | TokenOwner$ You | TokenAmount$ X\nSVar:X:Count$xPaid",
    );
    assert_eq!(
        refusal_reason(&triggered, &cats(), Some(&tokens())).as_deref(),
        Some("token amount `X` = `Count$xPaid`")
    );
}

/// An absent `TokenOwner$` is you only where the chain has no player to
/// mean instead. Rootcast Apprenticeship writes exactly that — "target
/// player creates a 1/1 green Squirrel creature token", with no
/// `TokenOwner$` on the line — and reading the absence as "you" there
/// would hand the token to the wrong side of the table.
#[test]
fn an_absent_owner_is_you_only_where_no_player_is_targeted() {
    let body = read_with_tokens("Name:X\nTypes:Instant\nA:SP$ Token | TokenScript$ r_1_1_goblin");
    assert!(body.abilities[0].contains("Effect::CreateToken"));

    let rootcast =
        parse("Name:X\nTypes:Instant\nA:SP$ Token | ValidTgts$ Player | TokenScript$ r_1_1_goblin");
    assert_eq!(
        refusal_reason(&rootcast, &cats(), Some(&tokens())).as_deref(),
        Some("a token created under another player's control")
    );
}

/// A named owner this cannot say is refused by name, and so is every
/// parameter no rule claimed — a token that enters tapped and one that
/// does not are different cards.
#[test]
fn a_token_parameter_with_no_rule_refuses_the_card() {
    for (line, why) in [
        (
            "A:SP$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ Targeted",
            "token owner `Targeted`",
        ),
        (
            "A:SP$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ You | TokenTapped$ True",
            "unclaimed parameter `Token.TokenTapped`",
        ),
        (
            "A:SP$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ You | TokenAmount$ X",
            "token amount `X`",
        ),
        (
            "A:SP$ Token | TokenOwner$ You",
            "a `Token` effect naming no `TokenScript$`",
        ),
    ] {
        let script = parse(&format!("Name:X\nTypes:Instant\n{line}"));
        assert_eq!(
            refusal_reason(&script, &cats(), Some(&tokens())).as_deref(),
            Some(why),
            "{line}"
        );
    }
}

/// A run with no token corpus is not a gap in the DSL, and says so in
/// those words: a report that ranked a missing directory as the top
/// blocker would send somebody to write a rule that already exists.
#[test]
fn a_run_with_no_token_corpus_is_not_reported_as_a_missing_rule() {
    let script = parse("Name:X\nTypes:Instant\nA:SP$ Token | TokenScript$ r_1_1_goblin");
    assert_eq!(
        refusal_reason(&script, &cats(), None).as_deref(),
        Some("`Token` with no token scripts to read it against")
    );
}

#[test]
fn the_supported_api_list_matches_what_is_actually_read() {
    for api in SUPPORTED_APIS {
        assert!(is_supported_api(api));
    }
    assert!(!is_supported_api("Animate"));
}

/// A counter a permanent arrives under is a replacement effect
/// (CR 614.1c), so it lands on the face and not in `abilities` — and
/// a body that says nothing else is still a card.
#[test]
fn a_permanent_that_enters_with_counters_says_so_on_its_face() {
    let body = read("Name:X\nTypes:Creature\nK:etbCounter:P1P1:2");
    assert_eq!(
        body.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::Fixed(2) }"]
    );
    assert!(body.abilities.is_empty() && body.keywords.is_empty());
    // The reference's own "there is no condition", with the reminder
    // text behind it, is the same card.
    let spelled_out = read(
        "Name:X\nTypes:Creature\n\
         K:etbCounter:CHARGE:3:no Condition:CARDNAME enters with three charge counters on it.",
    );
    assert_eq!(
        spelled_out.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::Charge, amount: Amount::Fixed(3) }"]
    );
}

/// `X` is the one the spell was cast for (CR 107.3m) and nothing else,
/// which the card's own `SVar:X` is what says.
///
/// Both counter-tests are cards the corpus really prints: Hooded Hydra
/// means the announced X, and Sautekh Immortal means "for each creature
/// that died this turn" — and writes that meaning in a field a reader
/// counting colons would have taken for a condition.
#[test]
fn an_x_on_the_way_in_is_read_only_where_the_spell_paid_it() {
    let body = read("Name:X\nTypes:Creature\nK:etbCounter:P1P1:X\nSVar:X:Count$xPaid");
    assert_eq!(
        body.enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, amount: Amount::X }"]
    );
    // "For each creature that died this turn" is a count the DSL says.
    assert_eq!(
        read(
            "Name:X\nTypes:Creature\nK:etbCounter:P1P1:X\n\
             SVar:X:Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature"
        )
        .enter_modifiers,
        ["EnterModifier::WithCounters { kind: CounterKind::P1P1, \
          amount: Amount::CreaturesDiedThisTurn }"]
    );
    assert!(
        refused(
            "Name:X\nTypes:Creature\n\
             K:etbCounter:P1P1:X:Elite Troops \u{2014} CARDNAME enters with a +1/+1 counter \
             on it for each creature that died this turn.\n\
             SVar:X:Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature"
        ),
        "the description is not a condition, and reading past it would \
         have taken the X beside it for the spell's"
    );
}

/// Simulacrum: "you gain life equal to the damage dealt to you this
/// turn", the reference's `X` defined as that count; the same `X` for
/// the announced number is still the spell's.
#[test]
fn the_damage_dealt_to_you_this_turn_is_a_count() {
    let simulacrum = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ GainLife | Defined$ You | LifeAmount$ X\n\
         SVar:X:PlayerCountPropertyYou$DamageThisTurn",
    );
    let a = simulacrum.abilities.join("\n");
    assert!(
        a.contains("Effect::GainLife { amount: Amount::DamageDealtToYouThisTurn }"),
        "{a}"
    );
    assert!(
        refused(
            "Name:X\nTypes:Instant\n\
             A:SP$ GainLife | Defined$ You | LifeAmount$ X\n\
             SVar:X:PlayerCountPropertyYou$DamageToOppsThisTurn"
        ),
        "the damage dealt to opponents is another count"
    );
}

/// Scavenging Ghoul: "put a corpse counter on this creature for each
/// creature that died this turn" and "remove a corpse counter from this
/// creature: regenerate this creature". A counter cost is a fixed number
/// from the source; `X`, loyalty and the longer forms are refused.
#[test]
fn a_counter_removed_as_a_cost_and_a_count_of_the_turns_deaths() {
    let ghoul = read(
        "Name:X\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Phase | Phase$ End of Turn | TriggerZones$ Battlefield | \
         Execute$ TrigPutCounter\n\
         A:AB$ Regenerate | Cost$ SubCounter<1/CORPSE>\n\
         SVar:TrigPutCounter:DB$ PutCounter | Defined$ Self | CounterType$ CORPSE | \
         CounterNum$ X\n\
         SVar:X:Count$ThisTurnEntered_Graveyard_from_Battlefield_Creature",
    );
    let a = ghoul.abilities.join("\n");
    assert!(
        a.contains("RemoveCounterSelf { kind: counters::CORPSE, n: 1 }"),
        "{a}"
    );
    assert!(
        a.contains(
            "Effect::AddCounter { kind: counters::CORPSE, \
             amount: Amount::CreaturesDiedThisTurn }"
        ),
        "{a}"
    );
    for cost in [
        "SubCounter<X/CHARGE>",
        "SubCounter<1/LOYALTY>",
        "SubCounter<1/P1P1/Creature.YouCtrl/a creature you control>",
    ] {
        assert!(
            refused(&format!(
                "Name:X\nTypes:Artifact\nA:AB$ Draw | Cost$ {cost} | NumCards$ 1"
            )),
            "{cost}"
        );
    }
}

/// Jade Statue: "{2}: this becomes a 3/6 Golem artifact creature until
/// end of combat. Activate only during combat."
#[test]
fn a_creature_type_that_replaces_until_end_of_combat_only_during_combat() {
    let statue = read(
        "Name:X\nTypes:Artifact\n\
         A:AB$ Animate | Cost$ 2 | Defined$ Self | Power$ 3 | Toughness$ 6 | \
         Types$ Creature,Artifact,Goblin | RemoveCreatureTypes$ True | \
         Duration$ UntilEndOfCombat | ActivationPhases$ BeginCombat->EndCombat",
    );
    let a = statue.abilities.join("\n");
    for part in [
        "Modifier::ReplaceCreatureTypes(subtypes::creature::GOBLIN), Duration::UntilEndOfCombat",
        "Modifier::AddType(TypeSet::CREATURE), Duration::UntilEndOfCombat",
        "Modifier::SetPT(3, 6), Duration::UntilEndOfCombat",
        "condition = Some(Condition::DuringCombat)",
    ] {
        assert!(a.contains(part), "{part} in {a}");
    }
    for refused_line in [
        // "Activate only during your upkeep" is another sentence.
        "A:AB$ Animate | Cost$ 2 | Defined$ Self | Power$ 3 | Toughness$ 6 | \
         Types$ Creature | ActivationPhases$ Upkeep",
        // Replacing creature types with none named is losing them all.
        "A:AB$ Animate | Cost$ 2 | Defined$ Self | Power$ 3 | Toughness$ 6 | \
         Types$ Creature | RemoveCreatureTypes$ True",
        // A restriction fills the condition, and the `IsPresent$` family
        // beside it is still unread.
        "A:AB$ Draw | Cost$ T | NumCards$ 1 | PlayerTurn$ True | PresentZone$ Graveyard",
    ] {
        assert!(
            refused(&format!("Name:X\nTypes:Artifact\n{refused_line}")),
            "{refused_line}"
        );
    }
}

/// Equip prints a cost and the rules supply the rest (CR 702.6a), so
/// that is all the line says and all `equip!` takes.
#[test]
fn equip_is_read_as_the_ability_the_rules_define() {
    assert_eq!(
        read("Name:X\nTypes:Artifact Equipment\nK:Equip:2").abilities,
        ["equip!(cost!(\"{2}\"))"]
    );
    // The cost goes through the same parser an `A:` line's does, so a
    // card that equips for something other than mana is the same rule.
    assert_eq!(
        read("Name:X\nTypes:Artifact Equipment\nK:Equip:PayLife<3>").abilities,
        ["equip!(cost!(PayLife(3)))"]
    );
    // And a narrowed one is refused rather than written without its
    // restriction, which would let the Equipment move onto anything.
    let parsed = parse(
        "Name:X\nTypes:Artifact Equipment\n\
         K:Equip:1:Creature.Legendary+YouCtrl:legendary creature",
    );
    assert!(transcode(&parsed, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&parsed, &cats(), None).as_deref(),
        Some("an `Equip` that narrows what it may attach to")
    );
}

/// Cycling prints a cost and the rules supply the rest (CR 702.29a), and
/// the string this writes is the one [`crate::landgen`] writes from the
/// printed text — so a card that both readers can reach comes out the
/// same either way, which is the only way the two can be allowed to
/// write the same sentence.
#[test]
fn cycling_is_read_as_the_ability_the_rules_define() {
    let want = "activated!(cost!(\"{2}\", DiscardSelf), &[Effect::draw(1)], \
                zone = ActivationZone::Hand)";
    assert_eq!(
        read("Name:X\nTypes:Land\nK:Cycling:2").abilities,
        [want],
        "the transcoder and the land reader have to agree byte for byte"
    );
    // Spelled out and not defaulted: `ScryfallCard` is what a payload
    // deserializes into, and a `..Default::default()` tail on it is the
    // thing that would stop the compiler naming a new field here.
    let land = crate::scryfall::ScryfallCard {
        id: "id".into(),
        oracle_id: Some("oracle".into()),
        name: "Test Land".into(),
        mana_cost: None,
        type_line: Some("Land".into()),
        oracle_text: Some("Cycling {2} ({2}, Discard this card: Draw a card.)".into()),
        colors: None,
        color_identity: None,
        set: None,
        set_name: None,
        collector_number: None,
        rarity: None,
        layout: None,
        power: None,
        toughness: None,
        loyalty: None,
        card_faces: None,
        image_uris: None,
        image_status: None,
    };
    assert_eq!(
        crate::landgen::recognize(&land, &cats())
            .expect("a land whose whole text is one cycling line")
            .abilities,
        [want]
    );

    // Space-separated and non-mana costs are the same sentence: the
    // reference writes 57 of them over 306 lines.
    assert_eq!(
        read("Name:X\nTypes:Creature\nK:Cycling:1 U").abilities,
        [
            "activated!(cost!(\"{1}{U}\", DiscardSelf), &[Effect::draw(1)], \
          zone = ActivationZone::Hand)"
        ]
    );
    assert_eq!(
        read("Name:X\nTypes:Creature\nK:Cycling:PayLife<2>").abilities,
        [
            "activated!(cost!(PayLife(2), DiscardSelf), &[Effect::draw(1)], \
          zone = ActivationZone::Hand)"
        ]
    );

    // A fourth field is refused by name. All 306 lines carry three
    // today, so a fourth is a sentence nobody has read.
    let parsed = parse("Name:X\nTypes:Creature\nK:Cycling:2:Island");
    assert!(transcode(&parsed, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&parsed, &cats(), None).as_deref(),
        Some("a `Cycling` with a fourth field `Island`")
    );
}

/// An Aura is a spell that targets what it will enchant (CR 303.4a) and
/// arrives attached to it, and the target is named once.
#[test]
fn an_aura_is_the_spell_that_attaches_it() {
    let body = read(
        "Name:X\nTypes:Enchantment Aura\nK:Enchant:Creature\n\
         S:Mode$ Continuous | Affected$ Creature.EnchantedBy | AddPower$ 2 | AddToughness$ 1",
    );
    assert_eq!(
        body.abilities[0],
        "spell!(&[Effect::AttachSelf { target: TargetSpec::Object(&Filter::CREATURE) }], \
         targets = Some(TargetReq::one(TargetSpec::Object(&Filter::CREATURE))))"
    );
    assert_eq!(
        body.abilities.len(),
        2,
        "the spell, and the one layer-7c modifier that +2/+1 is"
    );
    // "Enchant creature you control" is a filter the DSL already has a
    // constant for, so it is used twice under that name and declares
    // nothing — and the third field, the printed wording, is prose.
    let constant =
        read("Name:X\nTypes:Enchantment Aura\nK:Enchant:Creature.YouCtrl:creature you control");
    assert!(constant.statics.is_empty());
    assert_eq!(
        constant.abilities[0]
            .matches("&Filter::YOUR_CREATURE")
            .count(),
        2
    );
    // One that has no constant is named once and used twice, rather
    // than written out on both halves of the same sentence.
    let named = read("Name:X\nTypes:Enchantment Aura\nK:Enchant:Creature.tapped:tapped creature");
    assert_eq!(named.statics.matches("static ").count(), 1);
    assert_eq!(named.abilities[0].matches("&ENCHANT1").count(), 2);
    // An Aura on a player has nothing for `AttachSelf` to attach to.
    let parsed = parse("Name:X\nTypes:Enchantment Aura\nK:Enchant:Player");
    assert!(transcode(&parsed, &cats(), None).is_none());
    assert_eq!(
        refusal_reason(&parsed, &cats(), None).as_deref(),
        Some("an `Enchant Player`, which attaches to no object")
    );
}

/// "Enchanted creature" and "equipped creature" are one question, and
/// the answer is the permanent the source is attached to.
#[test]
fn what_a_permanent_is_attached_to_is_one_filter_under_two_words() {
    assert_eq!(
        filter("Creature.EnchantedBy"),
        "Filter::And(&[Filter::CREATURE, Filter::AttachedToBySource])"
    );
    assert_eq!(
        filter("Creature.EquippedBy"),
        filter("Creature.EnchantedBy")
    );
}

/// A condition the DSL cannot say refuses the card rather than dropping
/// the "if" and placing the counter every time.
#[test]
fn a_counter_that_arrives_only_sometimes_is_refused() {
    for script in [
        "Name:X\nTypes:Creature\n\
         K:etbCounter:P1P1:2:CheckSVar$ WasKicked:If CARDNAME was kicked, \
         it enters with two +1/+1 counters on it.",
        "Name:X\nTypes:Creature\n\
         K:etbCounter:P1P1:1:ValidCard$ Card.Self+escaped:CARDNAME escapes with a counter.",
        // And a counter the DSL has no id for is refused exactly as it
        // is on an ability: this is `counter_kind`'s rule, reached from
        // a second place.
        "Name:X\nTypes:Creature\nK:etbCounter:NOTACOUNTER:1",
    ] {
        let parsed = parse(script);
        assert!(transcode(&parsed, &cats(), None).is_none(), "{script}");
        assert_eq!(
            refusal_reason(&parsed, &cats(), None).as_deref(),
            Some("keyword `etbCounter`"),
            "and the refusal names the line it stopped on: {script}"
        );
    }
}

/// CR 701.16a: "'Investigate' means 'Create a Clue token.'"
///
/// The entry on the report said `effect Investigate`, ten pool stubs
/// deep, and the DSL turned out to be able to say all of it already —
/// the fifth time that has been true. So what is asserted here is the
/// *spelling*: that the word reaches `Effect::CreateToken` with the
/// Clue in it, and that it goes through the same token lookup every
/// `TokenScript$` goes through rather than naming a constant of its own.
#[test]
fn investigating_is_creating_a_clue_token() {
    let land = read_with_tokens(
        "Name:X\nTypes:Land\n\
         A:AB$ Investigate | Cost$ 4 T | SpellDescription$ Investigate.",
    );
    assert_eq!(
        land.abilities,
        [
            "activated!(cost!(\"{4}\", TapSelf), &[Effect::CreateToken { token: &generated_tokens::CLUE }])"
        ]
    );

    // `Num$` is the same question `TokenAmount$` asks, and reaches the
    // same two spellings through the same reader — one is `CreateToken`
    // and more is `CreateTokenN`, so the commonest token effect there is
    // does not acquire a second way of being written.
    let twice = read_with_tokens(
        "Name:X\nTypes:Land\n\
         A:AB$ Investigate | Cost$ T | Num$ 2",
    );
    assert_eq!(
        twice.abilities,
        [
            "activated!(Cost::TAP, &[Effect::CreateTokenN { token: &generated_tokens::CLUE, amount: Amount::Fixed(2) }])"
        ]
    );
}

/// Each way of refusing to investigate, named by the reason it gives.
///
/// `refused()` alone would pass on any of them for any reason at all,
/// including one from a different rule entirely — which is the failure
/// this file's own `refusal_reason` exists to make impossible. The
/// player check is asserted to fire **before** the token lookup, so a
/// machine with no token corpus still reports what is wrong with the
/// *card* rather than what is missing from the machine.
#[test]
fn who_investigates_is_refused_before_anything_about_this_machine_is() {
    let why = |text: &str, tokens: Option<&TokenLookup>| {
        refusal_reason(&parse(text), &cats(), tokens).expect("a reason is recorded")
    };
    let held = tokens();

    // Somebody else investigating: `Effect::CreateToken` has no room
    // for an owner, exactly as a `Token` line under another player's
    // control has none.
    for who in [
        "Defined$ Opponent",
        "ValidPlayer$ Player",
        "Defined$ TargetedController",
    ] {
        let text = format!("Name:X\nTypes:Land\nA:AB$ Investigate | Cost$ T | {who}");
        assert_eq!(
            why(&text, Some(&held)),
            "somebody other than you investigating",
            "for {who}"
        );
        // And with no token corpus it still says that, because the
        // question about the card comes first.
        assert_eq!(why(&text, None), "somebody other than you investigating");
    }

    // Two keys for one question. No line in the corpus writes both,
    // which is what makes this a refusal rather than a precedence rule
    // nobody could check against anything.
    assert_eq!(
        why(
            "Name:X\nTypes:Land\nA:AB$ Investigate | Cost$ T | Defined$ You | ValidPlayer$ You",
            Some(&held)
        ),
        "`Investigate` naming its player twice"
    );

    // "You may investigate" is a question this DSL cannot ask, and the
    // key is claimed by nothing, so the card refuses itself rather than
    // being read as the mandatory sentence beside it.
    assert!(
        why(
            "Name:X\nTypes:Land\nA:AB$ Investigate | Cost$ T | Optional$ True",
            Some(&held)
        )
        .contains("Optional"),
        "the optional clause is named, not swallowed: got {:?}",
        why(
            "Name:X\nTypes:Land\nA:AB$ Investigate | Cost$ T | Optional$ True",
            Some(&held)
        )
    );

    // And without the corpus the token half refuses in the same words
    // every other token refuses in.
    assert_eq!(
        why("Name:X\nTypes:Land\nA:AB$ Investigate | Cost$ T", None),
        "`Token` with no token scripts to read it against"
    );
}

/// A body that is not a rules line at all is refused rather than
/// parsed into one.
///
/// `SVar:SacMe:1` is a bare number, and this read it as an API named
/// "1" and then removed a parameter that had never been pushed. It
/// panicked — in a generator, over a corpus, which is a run that stops
/// on card 1 of 2716 — and it held for as long as it did only because
/// every caller asked about a body some rules line had named.
/// [`token_stems`] asks about all of them, and found it on the first
/// run.
#[test]
fn a_body_with_no_api_at_all_is_refused_and_does_not_panic() {
    for body in ["1", "True", ""] {
        assert!(
            Params::parse(body).is_none(),
            "{body:?} was read as a rules line"
        );
    }
    // A counted `SVar` does have a `$` and so is read — as an API named
    // `Valid Creature.YouCtrl`, which matches nothing and is refused one
    // step later. That is the existing contract and not a second bug:
    // what the head means is the caller's question, and what this
    // function owes is an answer rather than a panic.
    assert_eq!(
        Params::parse("Count$Valid Creature.YouCtrl").map(|(api, _)| api),
        Some("Valid Creature.YouCtrl".to_string())
    );
    // And through the walk that reaches them: a card whose `SVar`s are
    // bare values is read, not a panic.
    assert!(
        token_stems(&parse(
            "Name:X\nTypes:Artifact\n\
             A:AB$ Draw | Cost$ 2 Sac<1/CARDNAME/this token> | NumCards$ 1\n\
             SVar:SacMe:1\n"
        ))
        .is_empty()
    );
}

/// The ledger is told about the token a line does not name.
///
/// The emitter and the ledger have to agree about which tokens a card
/// needs: [`Tx::investigate_effect`] writes `generated_tokens::CLUE`,
/// and if [`token_stems`] does not report the Clue then nothing ever
/// asked the ledger to assign it. Today that survives only because
/// another card in the pool names the Clue outright — which is luck,
/// not a rule, and luck of exactly the kind that holds until the card
/// naming it is cut.
#[test]
fn a_card_that_only_investigates_still_names_the_clue() {
    assert_eq!(
        token_stems(&parse("Name:X\nTypes:Land\nA:AB$ Investigate | Cost$ 4 T")),
        [CLUE_TOKEN_SCRIPT]
    );
    // Through an `SVar` chain as well, which is where the corpus puts
    // most of its token effects.
    assert_eq!(
        token_stems(&parse(
            "Name:X\nTypes:Creature\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | \
             Execute$ Trig | TriggerDescription$ x\n\
             SVar:Trig:DB$ Investigate"
        )),
        [CLUE_TOKEN_SCRIPT]
    );
    // Named once when the card investigates twice, since the list feeds
    // an append-only ledger where a repeat is a second id for one
    // permanent.
    assert_eq!(
        token_stems(&parse(
            "Name:X\nTypes:Land\n\
             A:AB$ Investigate | Cost$ 4 T\n\
             A:AB$ Investigate | Cost$ 6 T"
        )),
        [CLUE_TOKEN_SCRIPT]
    );
    // And the word in prose is not a line that makes one: this is the
    // reason the reading goes through `Params::parse` and not a search
    // for the word, which 21 of the corpus's 112 lines would fool.
    assert!(
        token_stems(&parse(
            "Name:X\nTypes:Land\n\
             A:AB$ Mana | Cost$ T | Produced$ W | \
             SpellDescription$ Add {W}. Investigate. (Create a Clue token.)"
        ))
        .is_empty()
    );
}
