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

    // Cyclopean Snare: a sub-ability `DB$ ChangeZone` with neither its own
    // `ValidTgts$` nor `Defined$` bounces the source (`TargetSpec::ThisObject`),
    // not the parent ability's creature target.
    let snare = read(
        "Name:Cyclopean Snare\nTypes:Artifact\n\
         A:AB$ Tap | Cost$ 3 T | ValidTgts$ Creature | SubAbility$ DBBounce\n\
         SVar:DBBounce:DB$ ChangeZone | Origin$ Battlefield | Destination$ Hand\n",
    );
    assert!(
        snare
            .abilities
            .join("")
            .contains("Effect::bounce(TargetSpec::ThisObject)"),
        "{:?}",
        snare.abilities
    );

    // Tractor Beam: `Description$ Enchanted permanent doesn't untap…` overrides
    // the corpus's `ValidCard$ Creature.EnchantedBy` so an uncrewed Spacecraft
    // is also kept tapped.
    let beam = read(
        "Name:Tractor Beam\nTypes:Enchantment Aura\n\
         K:Enchant:Creature,Artifact:creature or artifact\n\
         R:Event$ Untap | ActiveZones$ Battlefield | ValidCard$ Creature.EnchantedBy \
         | ValidStepTurnToController$ You | Layer$ CantHappen \
         | Description$ Enchanted permanent doesn't untap during its controller's untap step.\n",
    );
    assert!(
        beam.abilities.contains(
            &"static_ability!(Filter::AttachedToBySource, Modifier::DoesNotUntap)".to_string()
        ),
        "{:?}",
        beam.abilities
    );
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
fn an_additional_cost_on_a_spell_is_read_or_refused_and_never_dropped() {
    // Crop Rotation: a single paid sacrifice lands on `mandatory_additional_costs`.
    let crop = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ ChangeZone | Cost$ G Sac<1/Land> | Origin$ Library | \
         Destination$ Battlefield | ChangeType$ Land | ChangeNum$ 1",
    );
    assert_eq!(
        crop.mandatory_additional_costs,
        ["CostPart::Sacrifice(&Filter::YOUR_LAND)"]
    );
    // Kaervek's Spite: `Sac<All/Permanent>` is not a paid mandatory cost part,
    // so it stays refused rather than being dropped.
    assert!(refused(
        "Name:X\nTypes:Instant\n\
         A:SP$ LoseLife | Cost$ B B B Sac<All/Permanent> Discard<0/Hand> | \
         ValidTgts$ Player | LifeAmount$ 5"
    ));
    // Random discard is a paid mandatory additional cost on spells.
    let rand = read(
        "Name:X\nTypes:Instant\n\
         A:SP$ Draw | Cost$ R Discard<1/Random> | NumCards$ 2",
    );
    assert_eq!(rand.mandatory_additional_costs, ["CostPart::DiscardRandom"]);
    // An unsupported cost token on a spell still stays refused rather than dropped.
    assert!(refused(
        "Name:X\nTypes:Instant\n\
         A:SP$ Draw | Cost$ R SubCounter<1/P1P1> | NumCards$ 2"
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
    // This was `K:Cycling:2`, then `K:TypeCycling:Basic:2`, then
    // `K:Ninjutsu:1 U`, then `K:Retrace` until each rule read them; the
    // example stays a parameterized keyword that is still unmodeled here.
    assert!(refused("Name:X\nTypes:Instant\nK:MayFlashSac:2"));
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
            .contains("Effect::CreateToken { token: &generated_tokens::GOBLIN_1_1_RED_A852C8CC6 }"),
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

    // `Count$Domain` lowers to `Amount::BasicLandTypesAmong(&Filter::YOUR_LAND)`.
    let domain = parse(
        "Name:X\nTypes:Sorcery\nA:SP$ Token | Cost$ G | TokenScript$ r_1_1_goblin \
         | TokenOwner$ You | TokenAmount$ X\nSVar:X:Count$Domain",
    );
    let domain_body = transcode(&domain, &cats(), Some(&tokens())).expect("domain token count");
    assert!(
        domain_body.abilities[0].contains("Amount::BasicLandTypesAmong(&Filter::YOUR_LAND)"),
        "{}",
        domain_body.abilities[0]
    );

    // The same `X`, counted a way nothing here can say.
    let unknown_count = parse(
        "Name:X\nTypes:Sorcery\nA:SP$ Token | Cost$ G | TokenScript$ r_1_1_goblin \
         | TokenOwner$ You | TokenAmount$ X\nSVar:X:Count$ValidExile Card.YouOwn",
    );
    assert_eq!(
        refusal_reason(&unknown_count, &cats(), Some(&tokens())).as_deref(),
        Some("token amount `X` = `Count$ValidExile Card.YouOwn`")
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

    let rootcast = read_with_tokens(
        "Name:X\nTypes:Instant\nA:SP$ Token | ValidTgts$ Player | TokenScript$ r_1_1_goblin",
    );
    assert!(rootcast.abilities[0].contains("Effect::CreateTokensFor { player: PlayerRel::Chosen"));
}

#[test]
fn token_modifiers_and_owners_lower_cleanly() {
    let tapped = read_with_tokens(
        "Name:X\nTypes:Instant\nA:SP$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ You | TokenTapped$ True | TokenAmount$ 2",
    );
    assert!(
        tapped.abilities[0].contains(
            "Effect::CreateTokenModified { token: &generated_tokens::GOBLIN_1_1_RED, amount: Amount::Fixed(2), tapped: true, haste: false }"
        ),
        "{}",
        tapped.abilities[0]
    );

    let haste_eot = read_with_tokens(
        "Name:X\nTypes:Instant\nA:SP$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ You | PumpKeywords$ Haste | PumpDuration$ EOT",
    );
    assert!(
        haste_eot.abilities[0].contains(
            "Effect::CreateTokenModified { token: &generated_tokens::GOBLIN_1_1_RED, amount: Amount::Fixed(1), tapped: false, haste: true }"
        ),
        "{}",
        haste_eot.abilities[0]
    );

    // Permanent haste (no `PumpDuration$`) stays refused.
    let perm_haste = parse(
        "Name:X\nTypes:Instant\nA:SP$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ You | PumpKeywords$ Haste",
    );
    assert_eq!(
        refusal_reason(&perm_haste, &cats(), Some(&tokens())).as_deref(),
        Some("unclaimed parameter `Token.PumpKeywords`")
    );

    let attacking = read_with_tokens(
        "Name:X\nTypes:Instant\nA:SP$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ You | TokenTapped$ True | TokenAttacking$ True | AtEOT$ Sacrifice",
    );
    assert!(
        attacking.abilities[0].contains(
            "Effect::CreateTokenAttacking { token: &generated_tokens::GOBLIN_1_1_RED, amount: Amount::Fixed(1), sacrifice_at_next_end_step: true }"
        ),
        "{}",
        attacking.abilities[0]
    );

    let each_player = read_with_tokens(
        "Name:X\nTypes:Sorcery\nA:SP$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ Player | TokenAmount$ 2",
    );
    assert!(
        each_player.abilities[0].contains(
            "Effect::CreateTokensFor { player: PlayerRel::EachPlayer, token: &generated_tokens::GOBLIN_1_1_RED, amount: Amount::Fixed(2) }"
        ),
        "{}",
        each_player.abilities[0]
    );
}

#[test]
fn alternate_mode_and_flavor_variant_rules() {
    // Split card with token on back face and hoisted filter statics on both faces.
    let split = parse(
        "Name:Front\nManaCost:R\nTypes:Sorcery\n\
         A:SP$ DealDamage | ValidTgts$ Creature.nonBlack | NumDmg$ 2\n\
         AlternateMode:Split\n\
         ALTERNATE\n\
         Name:Back\nManaCost:3 G\nTypes:Sorcery\n\
         A:SP$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ You\n\
         A:SP$ Destroy | ValidTgts$ Creature.nonBlack",
    );
    assert_eq!(token_stems(&split), vec!["r_1_1_goblin"]);
    let body = transcode(&split, &cats(), Some(&tokens())).expect("split transcodes");
    assert!(!body.back_is_adventure);
    let back = body.back.as_ref().expect("back face");
    assert_eq!(body.abilities.len(), 1);
    assert_eq!(back.abilities.len(), 2);
    // Shared filter static deduplicated across faces.
    assert_eq!(body.statics.matches("static ").count(), 1);
    assert!(back.statics.is_empty());

    // Adventure with vanilla creature front face and spell back face.
    let adv = parse(
        "Name:Hero\nManaCost:2 G\nTypes:Creature Elf\nPT:3/3\n\
         AlternateMode:Adventure\n\
         ALTERNATE\n\
         Name:Quest\nManaCost:G\nTypes:Instant Adventure\n\
         A:SP$ Draw | NumCards$ 1",
    );
    let adv_body = transcode(&adv, &cats(), Some(&tokens())).expect("adventure transcodes");
    assert!(adv_body.back_is_adventure);
    assert!(!adv_body.has_own_rules());
    assert_eq!(adv_body.back.as_ref().unwrap().abilities.len(), 1);

    // Room split cards transcode both doors.
    let room = parse(
        "Name:Left\nManaCost:1 W\nTypes:Enchantment Room\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigGain\n\
         SVar:TrigGain:DB$ GainLife | LifeAmount$ 2\n\
         AlternateMode:Split\n\
         ALTERNATE\n\
         Name:Right\nManaCost:2 W\nTypes:Enchantment Room\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw | NumCards$ 1",
    );
    let room_body = transcode(&room, &cats(), Some(&tokens())).expect("room split transcodes");
    assert_eq!(room_body.abilities.len(), 1);
    assert_eq!(room_body.back.as_ref().unwrap().abilities.len(), 1);

    // Flip alternate mode is now supported; unsupported modes like Alchemy Specialize stay refused.
    let flip = parse(
        "Name:Top\nManaCost:1 W\nTypes:Creature Human\nK:Flying\n\
         AlternateMode:Flip\n\
         ALTERNATE\n\
         Name:Bottom\nTypes:Legendary Creature Human\nK:Vigilance",
    );
    let flip_body = transcode(&flip, &cats(), Some(&tokens())).expect("Flip should transcode");
    assert!(flip_body.back_is_flip);
    assert!(flip_body.back.is_some());

    let specialize = parse(
        "Name:AlchemyCard\nManaCost:1 W\nTypes:Creature Human\nK:Flying\n\
         AlternateMode:Specialize\n\
         ALTERNATE\n\
         Name:Specialized\nTypes:Creature Human\nK:Vigilance",
    );
    assert_eq!(
        refusal_reason(&specialize, &cats(), Some(&tokens())).as_deref(),
        Some("unsupported alternate mode `Specialize`")
    );

    // UniversesWithin flavor-name variant is pure alias metadata; Attraction Variant stays refused.
    let uw = read(
        "Name:Original\nManaCost:R\nTypes:Instant\n\
         Variant:UniversesWithin:FlavorName:Alias Name\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 3",
    );
    assert_eq!(uw.abilities.len(), 1);
    let attr = parse("Name:Ride\nTypes:Artifact Attraction\nVariant:A:Lights:2 6\nK:Flying");
    assert!(transcode(&attr, &cats(), Some(&tokens())).is_none());
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
            "A:SP$ Token | TokenScript$ r_1_1_goblin | TokenOwner$ You | TokenAttacking$ True",
            "an attacking token with unsupported entry modifiers",
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
    let non_any_sub = read(
        "Name:X\nTypes:Artifact\n\
         A:AB$ Draw | Cost$ SubCounter<1/P1P1/Creature.YouCtrl/a creature you control> | NumCards$ 1",
    );
    assert_eq!(
        non_any_sub.abilities,
        [
            "activated!(cost!(RemoveAnyCounter { count: 1, kind: Some(CounterKind::P1P1), filter: &Filter::CREATURE }), &[Effect::draw(1)])"
        ]
    );
    for cost in [
        "SubCounter<X/CHARGE>",
        "SubCounter<1/LOYALTY>",
        "SubCounter<1/P1P1/Creature.OppCtrl/an opponent's creature>",
        "SubCounter<1/TIME/Permanent.inZoneBattlefield;Card.suspended/a permanent or suspended card/Battlefield,Exile>",
        "SubCounter<0/P1P1/Creature.YouCtrl>",
        "SubCounter<256/P1P1/Creature.YouCtrl>",
        "SubCounter<All/AIM/OriginalHost/Host>",
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
        // An unsupported phase window stays refused.
        "A:AB$ Animate | Cost$ 2 | Defined$ Self | Power$ 3 | Toughness$ 6 | \
         Types$ Creature | ActivationPhases$ Main1",
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
    // Narrowed `Equip` (`CR 702.6a`) lowers to a sorcery-speed `AttachSelf`
    // ability with the narrowed creature-you-control filter while non-creature
    // or opponent targets stay refused.
    let narrowed = read(
        "Name:X\nTypes:Artifact Equipment\n\
         K:Equip:1:Creature.Legendary+YouCtrl:legendary creature",
    );
    assert!(
        narrowed
            .statics
            .contains("Filter::HasSupertype(SupertypeSet::LEGENDARY)")
            && narrowed.statics.contains("Filter::ControlledByYou")
            && narrowed.abilities[0].contains("Effect::AttachSelf")
            && narrowed.abilities[0].contains("ActivationTiming::SorcerySpeed"),
        "{} | {:?}",
        narrowed.statics,
        narrowed.abilities
    );
    for bad_equip in [
        "Name:X\nTypes:Artifact Equipment\nK:Equip:1:Planeswalker.YouCtrl:planeswalker",
        "Name:X\nTypes:Artifact Equipment\nK:Equip:1:Creature.OppCtrl:opponent creature",
    ] {
        let parsed = parse(bad_equip);
        assert!(transcode(&parsed, &cats(), None).is_none(), "{bad_equip}");
        assert_eq!(
            refusal_reason(&parsed, &cats(), None).as_deref(),
            Some("an `Equip` that narrows what it may attach to")
        );
    }
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
        watermark: None,
        border_color: None,
        security_stamp: None,
        set: None,
        set_name: None,
        collector_number: None,
        rarity: None,
        layout: None,
        power: None,
        toughness: None,
        loyalty: None,
        defense: None,
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
    // Auras enchanting players lower to player TargetSpecs (`Player`,
    // `Opponent`, `Player.You`), while qualified player filters refuse cleanly.
    for (spec, expected) in [
        ("Player:player", "TargetSpec::AnyPlayer"),
        ("Opponent", "TargetSpec::AnyOpponent"),
        ("Player.Opponent:an opponent", "TargetSpec::AnyOpponent"),
        ("Player.OppCtrl", "TargetSpec::AnyOpponent"),
        ("Player.You:you", "TargetSpec::Player(PlayerRel::You)"),
        ("You", "TargetSpec::Player(PlayerRel::You)"),
    ] {
        let aura = read(&format!("Name:X\nTypes:Enchantment Aura\nK:Enchant:{spec}"));
        assert!(aura.statics.is_empty(), "{spec}: {}", aura.statics);
        assert_eq!(
            aura.abilities,
            [format!(
                "spell!(&[Effect::AttachSelf {{ target: {expected} }}], targets = Some(TargetReq::one({expected})))"
            )],
            "{spec}"
        );
    }
    for bad in ["Player.Active", "Player,Creature"] {
        let parsed = parse(&format!("Name:X\nTypes:Enchantment Aura\nK:Enchant:{bad}"));
        assert!(transcode(&parsed, &cats(), None).is_none(), "{bad}");
        assert_eq!(
            refusal_reason(&parsed, &cats(), None).as_deref(),
            Some(format!("an `Enchant {bad}` player qualifier").as_str()),
            "{bad}"
        );
    }
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
         K:etbCounter:P1P1:1:Adamant$ Purple:CARDNAME enters with a counter if unknown adamant.",
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

#[test]
fn escaped_etb_counter_lowers_to_with_counters_if() {
    let body = read(
        "Name:EscapingTroll\nTypes:Creature\nPT:3/3\n\
         K:etbCounter:P1P1:2:ValidCard$ Card.Self+escaped:CARDNAME escapes with two counters.",
    );
    assert_eq!(
        body.enter_modifiers,
        [
            "EnterModifier::WithCountersIf { kind: CounterKind::P1P1, count: Amount::Fixed(2), condition: Condition::Escaped }"
        ]
    );
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

#[test]
fn a_card_that_only_amasses_still_names_its_army_token() {
    assert_eq!(
        token_stems(&parse(
            "Name:X\nManaCost:1 B\nTypes:Sorcery\n\
             A:SP$ Amass | Type$ Zombie | Num$ 2"
        )),
        ["b_0_0_zombie_army"]
    );
    assert_eq!(
        token_stems(&parse(
            "Name:X\nManaCost:2 B\nTypes:Creature\nPT:2/2\n\
             T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | \
             Execute$ Trig | TriggerDescription$ x\n\
             SVar:Trig:DB$ Amass | Type$ Orc | Num$ 1"
        )),
        ["b_0_0_orc_army"]
    );
    assert_eq!(
        token_stems(&parse(
            "Name:X\nManaCost:2 B\nTypes:Sorcery\n\
             A:SP$ Amass | Type$ Zombie | Num$ 1 | SubAbility$ More\n\
             SVar:More:DB$ Amass | Type$ Zombie | Num$ 2"
        )),
        ["b_0_0_zombie_army"]
    );
    assert!(
        token_stems(&parse(
            "Name:X\nTypes:Land\n\
             A:AB$ Mana | Cost$ T | Produced$ B | \
             SpellDescription$ Add {B}. Amass Zombies 2."
        ))
        .is_empty()
    );
}

#[test]
fn wave16a_bloodthirst_devour_and_snow() {
    let bt = read(
        "Name:BloodthirstBrute\nManaCost:2 R\nTypes:Creature Beast\nPT:2/2\n\
         K:Bloodthirst:2\nK:Bloodthirst:X",
    );
    assert_eq!(bt.keywords, ["KeywordSet::BLOODTHIRST"]);
    assert_eq!(
        bt.enter_modifiers,
        [
            "EnterModifier::Bloodthirst(Amount::Fixed(2))",
            "EnterModifier::Bloodthirst(Amount::DamageDealtToOpponentsThisTurn)",
        ]
    );

    let mut c = cats();
    c.artifact.push("Food".into());
    c.normalize();
    let devour_food = transcode(
        &parse(
            "Name:DevourOgre\nManaCost:3 R G\nTypes:Creature Ogre\nPT:3/3\n\
             K:Devour:3:Food:desc\nK:etbCounter:P1P1:DevAmt\nSVar:DevAmt:Count$Devoured Card/Twice",
        ),
        &c,
        None,
    )
    .expect("devour food");
    assert_eq!(devour_food.keywords, ["KeywordSet::DEVOUR"]);
    assert!(
        devour_food.enter_modifiers[0].contains("EnterModifier::Devour")
            && devour_food.enter_modifiers[0].contains("factor: 3")
            && devour_food.statics.contains("subtypes::artifact::FOOD"),
        "{} / {:?}",
        devour_food.statics,
        devour_food.enter_modifiers
    );
    assert!(
        devour_food.enter_modifiers[1].contains("Amount::EntrySacrifices")
            && devour_food.enter_modifiers[1].contains("factor: 2"),
        "{:?}",
        devour_food.enter_modifiers
    );

    let snow = read(
        "Name:SnowVigil\nManaCost:1 S\nTypes:Creature Bear\nPT:2/2\n\
         K:Cumulative upkeep:S\nK:Echo:1 S\n\
         K:etbCounter:P1P1:Count$CastTotalManaSpent Snow\n\
         A:AB$ Draw | Cost$ T | NumCards$ X | IsPresent$ Permanent.YouCtrl | CheckSVar$ X\n\
         SVar:X:Count$CastTotalManaSpent Snow",
    );
    assert!(snow.abilities[0].contains("mana!(\"{S}\")"));
    assert!(snow.abilities[1].contains("mana!(\"{1}{S}\")"));
    assert!(snow.enter_modifiers[0].contains("Amount::SnowManaSpentToCast"));
    assert!(snow.abilities[2].contains("Condition::SnowManaWasSpent"));

    for bad in [
        "Name:BadBt\nManaCost:1 R\nTypes:Enchantment\nK:Bloodthirst:2",
        "Name:ZeroBt\nManaCost:1 R\nTypes:Creature Goblin\nPT:2/2\nK:Bloodthirst:0",
        "Name:RelDevour\nManaCost:2 G\nTypes:Creature Beast\nPT:2/2\nK:Devour:2:Creature.YouCtrl",
        "Name:SubDevour\nManaCost:2 G\nTypes:Creature Beast\nPT:2/2\n\
         K:Devour:2\nK:etbCounter:P1P1:Amt\nSVar:Amt:Count$Devoured Card.Goblin",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave16a_fabricate_hideaway_exploit_and_evolve() {
    let c = cats();
    let servo_tokens = TokenLookup::held(&[(
        "c_1_1_a_servo",
        "Name:Servo\nTypes:Artifact Creature\nPT:1/1",
    )]);
    let fab_script =
        "Name:ServoMaker\nManaCost:2 W\nTypes:Creature Artificer\nPT:1/3\nK:Fabricate:2";
    assert_eq!(token_stems(&parse(fab_script)), ["c_1_1_a_servo"]);
    let fab = transcode(&parse(fab_script), &c, Some(&servo_tokens)).expect("fabricate");
    assert!(fab.abilities[0].contains("Effect::Fabricate { count: 2,"));

    let hide = read("Name:HideawayNest\nTypes:Land\nK:Hideaway:4\nK:Hideaway:3");
    assert_eq!(
        hide.abilities,
        [
            "triggered!(Trigger::ETB, &[Effect::Hideaway(4)])",
            "triggered!(Trigger::ETB, &[Effect::Hideaway(3)])",
        ]
    );

    let exploit = read(
        "Name:ExploitAdept\nManaCost:1 U\nTypes:Creature Wizard\nPT:1/1\nK:Exploit\nK:Exploit",
    );
    assert_eq!(exploit.keywords, ["KeywordSet::EXPLOIT"]);
    assert_eq!(
        exploit.abilities,
        ["static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::EXPLOIT))"]
    );

    let evolve =
        read("Name:EvolveOoze\nManaCost:G\nTypes:Creature Ooze\nPT:0/1\nK:Evolve\nK:Evolve");
    assert_eq!(evolve.keywords, ["KeywordSet::EVOLVE"]);
    assert_eq!(
        evolve.abilities,
        ["static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::EVOLVE))"]
    );

    for bad in [
        "Name:SpellFab\nManaCost:2 W\nTypes:Sorcery\nK:Fabricate:2\nA:SP$ Draw | NumCards$ 1",
        "Name:ZeroHide\nTypes:Artifact\nK:Hideaway:0",
        "Name:SpellExploit\nManaCost:1 U\nTypes:Instant\nK:Exploit\nA:SP$ Draw | NumCards$ 1",
        "Name:ArtEvolve\nManaCost:2\nTypes:Artifact\nK:Evolve",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
    assert!(refused(fab_script), "Fabricate requires token lookup");
}

#[test]
fn wave16a_etb_counter_escaped_and_compare_x_unlocks() {
    let escaped = read(
        "Name:EscapedBeast\nManaCost:2 B G\nTypes:Creature Zombie Hydra\nPT:0/0\n\
         K:etbCounter:P1P1:Count$Escaped.12.6",
    );
    assert_eq!(escaped.enter_modifiers.len(), 2);
    assert!(escaped.enter_modifiers[0].contains("Amount::Fixed(12)"));
    assert!(escaped.enter_modifiers[0].contains("Condition::Escaped"));
    assert!(escaped.enter_modifiers[1].contains("Amount::Fixed(6)"));
    assert!(escaped.enter_modifiers[1].contains("Condition::Not("));

    let hydra = read(
        "Name:CompareHydra\nManaCost:X G R\nTypes:Creature Hydra\nPT:0/0\n\
         K:etbCounter:P1P1:Count$Compare X LT5.X.Z\n\
         SVar:X:Count$xPaid\nSVar:Z:Count$xPaid/Twice",
    );
    assert_eq!(
        hydra.enter_modifiers,
        [
            "EnterModifier::WithCountersIf { kind: CounterKind::P1P1, count: Amount::X, condition: Condition::Not(&Condition::XAtLeast(5)) }",
            "EnterModifier::WithCountersIf { kind: CounterKind::P1P1, count: Amount::DoubleX, condition: Condition::XAtLeast(5) }",
        ]
    );

    let direct_x_cmp = read(
        "Name:ThresholdHydra\nManaCost:X G\nTypes:Creature Hydra\nPT:0/0\n\
         K:etbCounter:P1P1:2:CheckSVar$ X | SVarCompare$ GE5:desc\nSVar:X:Count$xPaid",
    );
    assert_eq!(
        direct_x_cmp.enter_modifiers,
        [
            "EnterModifier::WithCountersIf { kind: CounterKind::P1P1, count: Amount::Fixed(2), condition: Condition::XAtLeast(5) }"
        ]
    );

    for bad in [
        "Name:BadEsc\nManaCost:2 G\nTypes:Creature Beast\nPT:0/0\nK:etbCounter:P1P1:Count$Escaped.12",
        "Name:ZeroEsc\nManaCost:2 G\nTypes:Creature Beast\nPT:0/0\nK:etbCounter:P1P1:Count$Escaped.0.0",
        "Name:NoXHydra\nManaCost:4 G\nTypes:Creature Hydra\nPT:0/0\n\
         K:etbCounter:P1P1:Count$Compare X LT5.X.Z\nSVar:X:Count$xPaid\nSVar:Z:Count$xPaid/Twice",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn wave17a_riot_unleash_and_keyword_atoms() {
    let single_riot = read("Name:RiotBrute\nManaCost:1 R G\nTypes:Creature Ogre\nPT:3/3\nK:Riot");
    assert_eq!(single_riot.keywords, ["KeywordSet::RIOT"]);
    assert!(single_riot.abilities.is_empty());

    let dup_riot =
        read("Name:TwinRiot\nManaCost:2 R G\nTypes:Creature Beast\nPT:4/4\nK:Riot\nK:Riot");
    assert_eq!(dup_riot.keywords, ["KeywordSet::RIOT"]);
    assert_eq!(
        dup_riot.abilities,
        ["static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::RIOT))"]
    );

    let single_unleash =
        read("Name:ClownFiend\nManaCost:B R\nTypes:Creature Devil\nPT:2/1\nK:Unleash");
    assert_eq!(single_unleash.keywords, ["KeywordSet::UNLEASH"]);
    assert!(single_unleash.abilities.is_empty());

    let dup_unleash =
        read("Name:TwinUnleash\nManaCost:3 B\nTypes:Creature Horror\nPT:3/3\nK:Unleash\nK:Unleash");
    assert_eq!(dup_unleash.keywords, ["KeywordSet::UNLEASH"]);
    assert_eq!(
        dup_unleash.abilities,
        ["static_ability!(Filter::This, Modifier::AddKeyword(KeywordSet::UNLEASH))"]
    );

    let perm_artifact = read("Name:RiotCart\nManaCost:2\nTypes:Artifact\nK:Riot\nK:Unleash");
    assert_eq!(
        perm_artifact.keywords,
        ["KeywordSet::RIOT", "KeywordSet::UNLEASH"]
    );

    for bad in [
        "Name:LandRiot\nTypes:Land\nK:Riot",
        "Name:SpellRiot\nManaCost:R\nTypes:Sorcery\nK:Riot\nA:SP$ Draw | NumCards$ 1",
        "Name:LandUnleash\nTypes:Land\nK:Unleash",
        "Name:SpellUnleash\nManaCost:B\nTypes:Instant\nK:Unleash\nA:SP$ Draw | NumCards$ 1",
        "Name:ParamRiot\nManaCost:1 R\nTypes:Creature Warrior\nPT:2/2\nK:Riot:2",
        "Name:ParamUnleash\nManaCost:1 B\nTypes:Creature Devil\nPT:2/1\nK:Unleash:1",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }

    for (atom, expected) in [
        (
            "Creature.withRiot",
            "Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::RIOT)])",
        ),
        (
            "Creature.withoutUnleash",
            "Filter::And(&[Filter::CREATURE, Filter::Not(&Filter::HasKeyword(KeywordSet::UNLEASH))])",
        ),
        (
            "Creature.withPartner",
            "Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::PARTNER)])",
        ),
        (
            "Card.withRetrace",
            "Filter::HasKeyword(KeywordSet::RETRACE)",
        ),
        (
            "Creature.withWarp",
            "Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::WARP)])",
        ),
        (
            "Creature.withBloodthirst",
            "Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::BLOODTHIRST)])",
        ),
        (
            "Creature.withDevour",
            "Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::DEVOUR)])",
        ),
        ("Card.withSpree", "Filter::HasKeyword(KeywordSet::SPREE)"),
        (
            "Creature.withExploit",
            "Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::EXPLOIT)])",
        ),
        (
            "Creature.withEvolve",
            "Filter::And(&[Filter::CREATURE, Filter::HasKeyword(KeywordSet::EVOLVE)])",
        ),
    ] {
        assert_eq!(filter(atom), expected, "{atom}");
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave18b_change_zone_origin_alternative_defined_player_and_remember_changed() {
    // `ChangeZone.OriginAlternative`: Library <-> Graveyard with YouOwn stripped.
    let lib_or_gy = read(
        "Name:GraveTutor\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | OriginAlternative$ Graveyard | Destination$ Hand | \
         ChangeType$ Creature.YouOwn | ShuffleNonMandatory$ True",
    );
    assert!(
        lib_or_gy.abilities[0].contains(
            "Effect::SearchLibraryOrGraveyard { filter: &Filter::CREATURE, find: &Find::HAND }"
        ),
        "{:?}",
        lib_or_gy.abilities
    );

    let gy_or_lib = read(
        "Name:BattleTutor\nManaCost:3 G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | OriginAlternative$ Library | Destination$ Battlefield | \
         Tapped$ True | ChangeType$ Land.Basic",
    );
    assert!(
        gy_or_lib.abilities[0].contains("Effect::SearchLibraryOrGraveyard")
            && gy_or_lib.abilities[0].contains("find: &Find::BATTLEFIELD_TAPPED")
            && !gy_or_lib.abilities[0].contains("MayDo"),
        "{:?}",
        gy_or_lib.abilities
    );
    let optional_tutor = read(
        "Name:OptionalGraveTutor\nManaCost:2 W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | OriginAlternative$ Graveyard | Destination$ Hand | \
         ChangeType$ Card.YouOwn+namedAjani | Optional$ True | ShuffleNonMandatory$ True",
    );
    assert!(
        optional_tutor.abilities[0]
            .contains("Effect::MayDo { effects: &[Effect::SearchLibraryOrGraveyard {")
            && optional_tutor.abilities[0].contains("find: &Find::HAND"),
        "{:?}",
        optional_tutor.abilities
    );

    // A player target chooses the library independently of the searcher.
    let targeted_library = "Name:Library Cut\nManaCost:U\nTypes:Sorcery\n\
                            A:SP$ ChangeZone | Origin$ Library | Destination$ Exile | \
                            ValidTgts$ Player | ChangeType$ Card";
    for (extra, owner_searches) in [
        ("", false),
        (" | DefinedPlayer$ Targeted", true),
        (" | Chooser$ TargetedPlayer", true),
    ] {
        let body = read(&format!("{targeted_library}{extra}"));
        assert!(body.abilities[0].contains(&format!(
            "Effect::SearchLibraryOf {{ library: PlayerRel::Chosen, owner_searches: {owner_searches},"
        )));
    }

    // `ChangeZone.DefinedPlayer`: target's controller searches their own library.
    let path_like = read(
        "Name:ExileAndRamp\nManaCost:W\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Exile | SubAbility$ DBRamp\n\
         SVar:DBRamp:DB$ ChangeZone | DefinedPlayer$ TargetedController | Origin$ Library | Destination$ Battlefield | \
         ChangeType$ Land.Basic | Tapped$ True | Optional$ True | Reveal$ True",
    );
    assert!(
        path_like.abilities[0].contains(
            "Effect::SearchLibraryOf { library: PlayerRel::ControllerOfTarget, owner_searches: true,"
        ),
        "{:?}",
        path_like.abilities
    );

    // `ChangeZone.DefinedPlayer`: you search target opponent's library with GainControl$ True.
    let bribery_like = read(
        "Name:StealFromDeck\nManaCost:3 U U\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Opponent | DefinedPlayer$ Targeted | Chooser$ You | \
         Origin$ Library | Destination$ Battlefield | ChangeType$ Creature | GainControl$ True | IsCurse$ True",
    );
    assert!(
        bribery_like.abilities[0].contains(
            "Effect::SearchLibraryOf { library: PlayerRel::Chosen, owner_searches: false, \
             filter: &Filter::CREATURE, mana_value: None, finds: &[Find::BATTLEFIELD], optional: false }"
        ),
        "{:?}",
        bribery_like.abilities
    );

    // `ChangeZone.DefinedPlayer`: group search (`Player` -> `RepeatEach`).
    let group_search = read(
        "Name:SharedHarvest\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | DefinedPlayer$ Player | Origin$ Library | Destination$ Battlefield | \
         ChangeType$ Land.Basic | Tapped$ True | Optional$ True",
    );
    assert!(
        group_search.abilities[0].contains(
            "Effect::RepeatEach { domain: RepeatDomain::EachPlayer, effects: &[Effect::SearchLibraryOf { \
             library: PlayerRel::EventPlayer, owner_searches: true,"
        ),
        "{:?}",
        group_search.abilities
    );

    // `ChangeZone` targeting a player with `Origin$ Graveyard | Hidden$ True | DefinedPlayer$ Targeted`.
    let gy_player_exile = read(
        "Name:GraveNibbler\nManaCost:B\nTypes:Creature Merfolk\nPT:1/1\n\
         A:AB$ ChangeZone | Cost$ T | ValidTgts$ Player | DefinedPlayer$ Targeted | Origin$ Graveyard | \
         Destination$ Exile | ChangeType$ Card | Hidden$ True | Mandatory$ True | IsCurse$ True",
    );
    assert!(
        gy_player_exile.abilities[0].contains("target = Some(TargetSpec::AnyPlayer)")
            && gy_player_exile.abilities[0]
                .contains("zone: ChooseZone::Graveyard(PlayerRel::Chosen)"),
        "{:?}",
        gy_player_exile.abilities
    );

    // `ChangeZone` graveyard any-number `TargetMin$ 0 | TargetMax$ X` with YouCtrl vs YouOwn equivalence.
    let gy_any = read(
        "Name:GraveShuffle\nManaCost:G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Creature.YouCtrl | TargetMin$ 0 | TargetMax$ X | \
         Origin$ Graveyard | Destination$ Library | Shuffle$ True\n\
         SVar:X:Count$ValidGraveyard Creature.YouOwn",
    );
    assert!(
        gy_any.abilities[0].contains("TargetReq::up_to(") && gy_any.abilities[0].contains(", 255)"),
        "{:?}",
        gy_any.abilities
    );

    // `ChangeZoneAll | RememberChanged$ True` -> `ThisWay::ExiledFromBattlefield` & `PutIntoLibraryFromHand`.
    let exile_all_gain = read(
        "Name:BanishTide\nManaCost:4 W W\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | Origin$ Battlefield | Destination$ Exile | ChangeType$ Creature | \
         RememberChanged$ True | SubAbility$ DBGain\n\
         SVar:DBGain:DB$ GainLife | Defined$ You | LifeAmount$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        exile_all_gain.abilities[0]
            .contains("Effect::GainLife { amount: Amount::ThisWay(ThisWay::ExiledFromBattlefield(&Filter::Any)) }"),
        "{:?}",
        exile_all_gain.abilities
    );

    let whirl_hand = read(
        "Name:WhirlDeck\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | Defined$ You | Origin$ Hand | Destination$ Library | Shuffle$ True | \
         RememberChanged$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        whirl_hand.abilities[0].contains(
            "Effect::DrawCards { amount: Amount::ThisWay(ThisWay::PutIntoLibraryFromHand) }"
        ),
        "{:?}",
        whirl_hand.abilities
    );

    // Refusals: unsupported OriginAlternative, DefinedPlayer, Chooser, CR 400.3 Hand steal, unrestricted Reveal.
    for bad in [
        "Name:BadAltHand\nManaCost:2 G\nTypes:Sorcery\nA:SP$ ChangeZone | Origin$ Library | OriginAlternative$ Graveyard,Hand | Destination$ Hand | ChangeType$ Creature",
        "Name:BadAltSideboard\nManaCost:2 G\nTypes:Sorcery\nA:SP$ ChangeZone | Origin$ Library | OriginAlternative$ Sideboard | Destination$ Hand | ChangeType$ Creature",
        "Name:BadDefOther\nManaCost:2 G\nTypes:Sorcery\nA:SP$ ChangeZone | DefinedPlayer$ Player.Other | Origin$ Library | Destination$ Battlefield | ChangeType$ Land.Basic",
        "Name:BadStealHand\nManaCost:3 U\nTypes:Sorcery\nA:SP$ ChangeZone | ValidTgts$ Opponent | DefinedPlayer$ Targeted | Chooser$ You | Origin$ Library | Destination$ Hand | ChangeType$ Card",
        "Name:BadStealNoGainCtrl\nManaCost:3 U\nTypes:Sorcery\nA:SP$ ChangeZone | ValidTgts$ Opponent | DefinedPlayer$ Targeted | Chooser$ You | Origin$ Library | Destination$ Battlefield | ChangeType$ Creature",
        "Name:BadRevealAny\nManaCost:1 B\nTypes:Sorcery\nA:SP$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Card | Reveal$ True",
        "Name:BadHandDefOpp\nManaCost:2 G\nTypes:Sorcery\nA:SP$ ChangeZone | DefinedPlayer$ Opponent | Origin$ Hand | Destination$ Battlefield | ChangeType$ Creature | Optional$ True",
        "Name:BadMultiWhirl\nManaCost:2 U\nTypes:Sorcery\nA:SP$ ChangeZoneAll | Defined$ Player | Origin$ Hand | Destination$ Library | Shuffle$ True | RememberChanged$ True | SubAbility$ D\nSVar:D:DB$ Draw | Defined$ Player | NumCards$ X\nSVar:X:Remembered$Amount",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[allow(clippy::too_many_lines)]
#[test]
fn wave19b_change_zone_library_destination_alt_shuffle_false_hand_exile_and_all() {
    // `ChangeZone.DestinationAlternative`: Battlefield, Stack with `TargetMin$ 0`, StackOrBattlefield.
    let tuck_bf = read(
        "Name:SpaceTuck\nManaCost:3 U\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Artifact,Creature | AlternativeDecider$ TargetedOwner | \
         Origin$ Battlefield | Destination$ Library | DestinationAlternative$ Library | \
         LibraryPositionAlternative$ -1 | SubAbility$ DBSurveil\n\
         SVar:DBSurveil:DB$ Surveil | Amount$ 1",
    );
    assert!(
        tuck_bf.abilities[0]
            .contains("Effect::OwnerPutsOnTopOrBottom { target: TargetSpec::Object(")
            && tuck_bf.abilities[0].contains("Effect::surveil(1)"),
        "{:?}",
        tuck_bf.abilities
    );

    let tuck_stack = read(
        "Name:SpellTuck\nManaCost:2 U U\nTypes:Creature Elemental\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigTuck\n\
         SVar:TrigTuck:DB$ ChangeZone | ValidTgts$ Card.inZoneStack+Creature,Card.inZoneStack+Planeswalker | \
         TgtZone$ Stack | TargetMin$ 0 | TargetMax$ 1 | AlternativeDecider$ TargetedOwner | \
         Origin$ Stack | Destination$ Library | DestinationAlternative$ Library | LibraryPositionAlternative$ -1",
    );
    assert!(
        tuck_stack.abilities[0]
            .contains("Effect::OwnerPutsOnTopOrBottom { target: TargetSpec::Spell(")
            && tuck_stack.abilities[0].contains("TargetReq::up_to_one("),
        "{:?}",
        tuck_stack.abilities
    );

    let tuck_either = read(
        "Name:GustTuck\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Creature,Card.inZoneStack | TgtZone$ Stack,Battlefield | \
         AlternativeDecider$ TargetedOwner | Origin$ Battlefield,Stack | Destination$ Library | \
         DestinationAlternative$ Library | LibraryPositionAlternative$ -1",
    );
    assert!(
        tuck_either.abilities[0]
            .contains("Effect::OwnerPutsOnTopOrBottom { target: TargetSpec::StackOrBattlefield("),
        "{:?}",
        tuck_either.abilities
    );

    // `ChangeZone` Battlefield -> Library: fixed position, `IsCurse$`, dynamic `LibraryPosition$ X`,
    // `(Shuffle$ True, LibraryPosition$ 0)`, and multi-target beneath top.
    let second_top = read(
        "Name:OustLike\nManaCost:W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Library | ValidTgts$ Creature | LibraryPosition$ 1",
    );
    assert!(
        second_top.abilities[0]
            .contains("Effect::PutTargetIntoLibraryBeneathTop { count: Amount::Fixed(1) }"),
        "{:?}",
        second_top.abilities
    );
    let third_top = read(
        "Name:ThirdTuck\nManaCost:3 W U\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Library | ValidTgts$ Permanent.nonLand | \
         LibraryPosition$ 2 | IsCurse$ True",
    );
    assert!(
        third_top.abilities[0]
            .contains("Effect::PutTargetIntoLibraryBeneathTop { count: Amount::Fixed(2) }"),
        "{:?}",
        third_top.abilities
    );
    let colossus = read(
        "Name:IslandGiant\nManaCost:5 U U\nTypes:Creature Giant\nPT:5/6\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigTuck\n\
         SVar:TrigTuck:DB$ ChangeZone | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Library | LibraryPosition$ X\n\
         SVar:X:Count$Valid Island.YouCtrl",
    );
    assert!(
        colossus.abilities[0]
            .contains("Effect::PutTargetIntoLibraryBeneathTop { count: Amount::CountOf {"),
        "{:?}",
        colossus.abilities
    );
    let shuf_top = read(
        "Name:ShuffleTop\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Library | ValidTgts$ Creature | LibraryPosition$ 0 | Shuffle$ True",
    );
    assert!(
        shuf_top.abilities[0].contains("Effect::ShuffleTargetsIntoOwnersLibraries"),
        "{:?}",
        shuf_top.abilities
    );
    let multi_beneath = read(
        "Name:TwinTuck\nManaCost:4 U\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Library | ValidTgts$ Creature | \
         TargetMin$ 0 | TargetMax$ 2 | LibraryPosition$ 1",
    );
    assert!(
        multi_beneath.abilities[0].contains("TargetReq::up_to(")
            && multi_beneath.abilities[0]
                .contains("Effect::PutTargetIntoLibraryBeneathTop { count: Amount::Fixed(1) }"),
        "{:?}",
        multi_beneath.abilities
    );

    // `ChangeZone` `Shuffle$ False`: `SearchLibraryBeforeShuffle` and split-destination `SearchLibrary`.
    let before_shuf = read(
        "Name:PairSearch\nManaCost:3 G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeType$ Forest | Shuffle$ False | SubAbility$ DBTwo\n\
         SVar:DBTwo:DB$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeType$ Island | Shuffle$ False | SubAbility$ DBShuf\n\
         SVar:DBShuf:DB$ Shuffle",
    );
    assert!(
        before_shuf.abilities[0].contains("Effect::SearchLibraryBeforeShuffle {")
            && before_shuf.abilities[0].contains("Effect::ShuffleLibrary { who: PlayerRel::You }"),
        "{:?}",
        before_shuf.abilities
    );

    let split_scry = read(
        "Name:WanderSearch\nManaCost:3 G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Library | ChangeType$ Land.Basic | ChangeNum$ 2 | \
         RememberChanged$ True | Reveal$ True | Shuffle$ False | SubAbility$ DB1\n\
         SVar:DB1:DB$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeType$ Land.IsRemembered | \
         ChangeNum$ 1 | Mandatory$ True | NoLooking$ True | Tapped$ True | Shuffle$ False | SubAbility$ DB2\n\
         SVar:DB2:DB$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Land.IsRemembered | \
         Mandatory$ True | NoLooking$ True | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True | SubAbility$ DBScry\n\
         SVar:DBScry:DB$ Scry | ScryNum$ 1",
    );
    assert!(
        split_scry.abilities[0].contains("Effect::SearchLibrary {")
            && split_scry.abilities[0]
                .contains("finds: &[Find::BATTLEFIELD_TAPPED, Find::HAND], optional: true }")
            && split_scry.abilities[0].contains("Effect::scry(1)")
            && split_scry.statics.contains("SupertypeSet::BASIC"),
        "{:?}\n{}",
        split_scry.abilities,
        split_scry.statics
    );

    let split_orb = read(
        "Name:WayOrb\nManaCost:3\nTypes:Artifact\n\
         A:AB$ ChangeZone | Cost$ 2 T Sac<1/CARDNAME> | Origin$ Library | Destination$ Library | \
         ChangeType$ Land.Basic | ChangeNum$ 2 | RememberChanged$ True | Reveal$ True | Shuffle$ False | SubAbility$ DB1\n\
         SVar:DB1:DB$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeType$ Land.IsRemembered | \
         ChangeNum$ 1 | Mandatory$ True | ForgetChanged$ True | NoLooking$ True | Tapped$ True | Shuffle$ True | SubAbility$ DB2\n\
         SVar:DB2:DB$ ChangeZone | Origin$ Library | Destination$ Hand | Defined$ Remembered | \
         ConditionDefined$ Remembered | ConditionPresent$ Card | ForgetChanged$ True | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        split_orb.abilities[0].contains("Effect::SearchLibrary {")
            && split_orb.abilities[0]
                .contains("finds: &[Find::BATTLEFIELD_TAPPED, Find::HAND], optional: true }")
            && split_orb.statics.contains("SupertypeSet::BASIC"),
        "{:?}\n{}",
        split_orb.abilities,
        split_orb.statics
    );

    // `ChangeZone` Hand -> Exile: optional `Hidden$ True` / `Optional$ True`, and cast-linked exile.
    let opt_imprint = read(
        "Name:ArtPortal\nManaCost:4\nTypes:Artifact\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigEx\n\
         SVar:TrigEx:DB$ ChangeZone | Imprint$ True | Origin$ Hand | Destination$ Exile | \
         ChangeType$ Artifact | ChangeNum$ 1 | Hidden$ True | Optional$ True",
    );
    assert!(
        opt_imprint.abilities[0].contains(
            "Effect::MayDo { effects: &[Effect::ExileChosenFromHandWithSource(&Filter::ARTIFACT)] }"
        ),
        "{:?}",
        opt_imprint.abilities
    );
    let hand_cast = read(
        "Name:VaultMirror\nManaCost:3\nTypes:Artifact\n\
         A:AB$ ChangeZone | Cost$ T | Origin$ Hand | Destination$ Exile | ChangeType$ Card.nonLand | \
         ChangeNum$ 1 | Optional$ True | RememberChanged$ True | SubAbility$ DBEff\n\
         SVar:DBEff:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ ST | Duration$ Permanent | SubAbility$ DBClean\n\
         SVar:ST:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        hand_cast.abilities[0].contains("Effect::ExileChosenFromHandWithSourceAndCast("),
        "{:?}",
        hand_cast.abilities
    );

    // `ChangeZoneAll`: `LibraryPosition$` / `RandomOrder$` alongside `Shuffle$ True`, and `GraveyardToLibraryBottom` with `IsCurse$`.
    let shuf_with_pos = read(
        "Name:HandCycle\nManaCost:1 U\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | Defined$ Player | Origin$ Hand | Destination$ Library | \
         LibraryPosition$ -1 | RandomOrder$ True | Shuffle$ True",
    );
    assert!(
        shuf_with_pos.abilities[0]
            .contains("Effect::ShuffleIntoLibrary { who: PlayerRel::EachPlayer, hand: true, graveyard: false }"),
        "{:?}",
        shuf_with_pos.abilities
    );
    let gy_bottom = read(
        "Name:GraveBottom\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | ValidTgts$ Player | Origin$ Graveyard | Destination$ Library | \
         LibraryPosition$ -1 | RandomOrder$ True | IsCurse$ True",
    );
    assert!(
        gy_bottom.abilities[0]
            .contains("Effect::GraveyardToLibraryBottom { player: PlayerRel::Chosen }"),
        "{:?}",
        gy_bottom.abilities
    );

    // Refusals: `LibraryPosition$ 1` + `DestinationAlternative$`, multi-target `DestinationAlternative$`,
    // non-owner `DestinationAlternative$`, `Shuffle$ False` without shuffle, `Intuition` split,
    // `LibraryPosition$ -2`, non-imprint opponent Hand -> Exile, Battlefield sweep to bottom.
    for bad in [
        "Name:BadSecondOrBottom\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Creature | AlternativeDecider$ TargetedOwner | Origin$ Battlefield | \
         Destination$ Library | LibraryPosition$ 1 | DestinationAlternative$ Library | LibraryPositionAlternative$ -1",
        "Name:BadMultiAlt\nManaCost:3 U\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ 2 | AlternativeDecider$ TargetedOwner | \
         Origin$ Battlefield | Destination$ Library | DestinationAlternative$ Library | LibraryPositionAlternative$ -1",
        "Name:BadYouChooseAlt\nManaCost:1 W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Card | TgtZone$ Graveyard | Origin$ Graveyard | \
         Destination$ Library | DestinationAlternative$ Library | LibraryPositionAlternative$ -1",
        "Name:BadNoShuffle\nManaCost:1 G\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Hand | ChangeType$ Land.Basic | Shuffle$ False | SubAbility$ D\n\
         SVar:D:DB$ Draw | Defined$ You | NumCards$ 1",
        "Name:BadIntuitionSplit\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Library | Destination$ Library | ChangeType$ Creature | ChangeNum$ 3 | \
         Mandatory$ True | RememberChanged$ True | Reveal$ True | Shuffle$ False | SubAbility$ DB1\n\
         SVar:DB1:DB$ ChangeZone | ValidTgts$ Opponent | DefinedPlayer$ You | Chooser$ Targeted | \
         ChangeType$ Creature.IsRemembered | Origin$ Library | Destination$ Hand | ChangeNum$ 1 | \
         Mandatory$ True | Shuffle$ False | NoLooking$ True | SubAbility$ DB2\n\
         SVar:DB2:DB$ ChangeZone | Origin$ Library | Destination$ Graveyard | ChangeType$ Creature.IsRemembered | \
         ChangeNum$ 2 | Mandatory$ True | NoLooking$ True",
        "Name:BadNegTwoPos\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Library | ValidTgts$ Creature | LibraryPosition$ -2",
        "Name:BadHandLandCast\nManaCost:2\nTypes:Artifact\n\
         A:AB$ ChangeZone | Cost$ T | Origin$ Hand | Destination$ Exile | ChangeType$ Land | \
         ChangeNum$ 1 | RememberChanged$ True | SubAbility$ DBEff\n\
         SVar:DBEff:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ ST | Duration$ Permanent\n\
         SVar:ST:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile",
        "Name:BadHallowedBurial\nManaCost:3 W W\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | Origin$ Battlefield | Destination$ Library | LibraryPosition$ -1 | ChangeType$ Creature",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }

    // Wave 13a `Effect::ChooseCardEx` unlocks targeted opponent self-exiling cards from hand.
    let opp_hand_exile = read(
        "Name:OppHandExileTwo\nManaCost:3 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Hand | Destination$ Exile | ValidTgts$ Opponent | DefinedPlayer$ Targeted | \
         Chooser$ Targeted | ChangeType$ Card | ChangeNum$ 2 | Mandatory$ True",
    );
    assert!(
        opp_hand_exile.abilities[0].contains("Effect::ChooseCardEx")
            && opp_hand_exile.abilities[0].contains("chooser: PlayerRel::Chosen")
            && opp_hand_exile.abilities[0].contains("zone: ChooseZone::Hand(PlayerRel::Chosen)")
            && opp_hand_exile.abilities[0].contains("min: 2, max: Amount::Fixed(2)")
            && opp_hand_exile.abilities[0].contains("movement: ChooseMove::Exile"),
        "{:?}",
        opp_hand_exile.abilities
    );
}

fn wave23b_sized_tokens() -> TokenLookup {
    TokenLookup::held(&[
        (
            "g_x_x_goblin",
            "Name:Goblin Token\nTypes:Creature Goblin\nColors:green\nPT:*/*",
        ),
        (
            "r_x_1_goblin_trample",
            "Name:Goblin Token\nTypes:Creature Goblin\nColors:red\nPT:*/1\nK:Trample",
        ),
        (
            "c_x_x_cda_goblin",
            "Name:Goblin Token\nTypes:Creature Goblin\nPT:*/*\n\
             S:Mode$ Continuous | CharacteristicDefining$ True | SetPower$ X | SetToughness$ X | Description$ Power and toughness.\n\
             SVar:X:Count$Valid Creature.YouCtrl",
        ),
        (
            "r_1_1_goblin",
            "Name:Goblin\nTypes:Creature Goblin\nColors:red\nPT:1/1",
        ),
    ])
}

#[test]
fn wave23b_create_token_sized_and_tapped_blink() {
    let sized_tokens = wave23b_sized_tokens();
    let read_sized = |text: &str| {
        let parsed = parse(text);
        transcode(&parsed, &cats(), Some(&sized_tokens)).unwrap_or_else(|| {
            panic!(
                "expected success, refused: {:?}",
                refusal_reason(&parsed, &cats(), Some(&sized_tokens))
            )
        })
    };

    // Symmetric X/X token sized by paid X.
    let xx = read_sized(
        "Name:GrowGoblin\nManaCost:X G\nTypes:Sorcery\n\
         A:SP$ Token | TokenScript$ g_x_x_goblin | TokenOwner$ You | TokenPower$ X | TokenToughness$ X\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        xx.abilities[0].contains("Effect::CreateTokenSized {")
            && xx.abilities[0].contains("amount: Amount::Fixed(1)")
            && xx.abilities[0].contains("power: Amount::X, toughness: Amount::X")
            && xx.abilities[0].contains(
                "who: PlayerRel::You, tapped: false, attacking: false, haste_until_eot: false, cleanup: CopyCleanup::None"
            ),
        "{:?}",
        xx.abilities
    );

    // Asymmetric */1 token sized only by TokenPower$, with haste + AtEOT$ Sacrifice.
    let forge = read_sized(
        "Name:FlameMill\nManaCost:2 R\nTypes:Artifact\n\
         A:AB$ Token | Cost$ T | TokenAmount$ 2 | TokenScript$ r_x_1_goblin_trample | TokenOwner$ You \
         | TokenPower$ X | PumpKeywords$ Haste | PumpDuration$ EOT | AtEOT$ Sacrifice\n\
         SVar:X:Count$Valid Goblin.YouCtrl",
    );
    assert!(
        forge.abilities[0].contains("Effect::CreateTokenSized {")
            && forge.abilities[0].contains("amount: Amount::Fixed(2)")
            && forge.abilities[0].contains("toughness: Amount::Fixed(1)")
            && forge.abilities[0].contains("haste_until_eot: true")
            && forge.abilities[0].contains(
                "Effect::AtNextEndStepForCreatedTokens { effects: &[Effect::SacrificeEvent] }"
            ),
        "{:?}",
        forge.abilities
    );

    // Tapped + attacking sized token for You with AtEOT$ Sacrifice, and non-attacking for TargetedPlayer.
    let tap_atk = read_sized(
        "Name:ChargeToken\nManaCost:3 R\nTypes:Instant\n\
         A:SP$ Token | TokenScript$ g_x_x_goblin | TokenPower$ 3 | TokenToughness$ 3 | TokenTapped$ True | TokenAttacking$ True | AtEOT$ Sacrifice",
    );
    assert!(
        tap_atk.abilities[0].contains(
            "tapped: true, attacking: true, haste_until_eot: false, cleanup: CopyCleanup::SacrificeAtNextEndStep"
        ),
        "{:?}",
        tap_atk.abilities
    );
    let gift = read_sized(
        "Name:GiftSized\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ Token | ValidTgts$ Player | TokenScript$ g_x_x_goblin | TokenOwner$ TargetedPlayer | TokenPower$ 4 | TokenToughness$ 4",
    );
    assert!(
        gift.abilities[0].contains("who: PlayerRel::Chosen, tapped: false, attacking: false"),
        "{:?}",
        gift.abilities
    );

    // Activated sacrifice cost feeding TokenPower$/TokenToughness$ via Sacrificed$CardPower.
    let sac_sized = read_sized(
        "Name:OozeVat\nManaCost:1 G\nTypes:Enchantment\n\
         A:AB$ Token | Cost$ 1 G Sac<1/Creature> | TokenScript$ g_x_x_goblin | TokenPower$ X | TokenToughness$ X\n\
         SVar:X:Sacrificed$CardPower",
    );
    assert!(
        sac_sized.abilities[0]
            .contains("power: Amount::SacrificedPower, toughness: Amount::SacrificedPower"),
        "{:?}",
        sac_sized.abilities
    );
}

#[test]
fn wave23b_create_token_sized_followups_and_tapped_blink() {
    let sized_tokens = wave23b_sized_tokens();
    let read_sized = |text: &str| {
        let parsed = parse(text);
        transcode(&parsed, &cats(), Some(&sized_tokens)).unwrap_or_else(|| {
            panic!(
                "expected success, refused: {:?}",
                refusal_reason(&parsed, &cats(), Some(&sized_tokens))
            )
        })
    };

    // RememberedThisWay sized token after DestroyAll, plus RememberTokens -> PutCounter follow-up.
    let rebirth = read_sized(
        "Name:SweepSized\nManaCost:4 W W\nTypes:Sorcery\n\
         A:SP$ DestroyAll | ValidCards$ Creature | RememberDestroyed$ True | SubAbility$ DBTok\n\
         SVar:DBTok:DB$ Token | TokenScript$ g_x_x_goblin | TokenOwner$ You | TokenPower$ X | TokenToughness$ X | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount",
    );
    assert!(
        rebirth.abilities[0].contains("Effect::CreateTokenSized {")
            && rebirth.abilities[0].contains("Amount::ThisWay(ThisWay::Destroyed(&Filter::Any))"),
        "{:?}",
        rebirth.abilities
    );
    let rem_cnt = read_sized(
        "Name:SizedWithCounters\nManaCost:X G G\nTypes:Sorcery\n\
         A:SP$ Token | TokenScript$ g_x_x_goblin | TokenPower$ X | TokenToughness$ X | RememberTokens$ True | SubAbility$ DBPut\n\
         SVar:DBPut:DB$ PutCounter | Defined$ Remembered | CounterType$ P1P1 | CounterNum$ 2 | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        rem_cnt.abilities[0].contains("Effect::CreateTokenSized {")
            && rem_cnt.abilities[0].contains("Effect::AddCounterCreatedTokens"),
        "{:?}",
        rem_cnt.abilities
    );

    // Immediate blink returning tapped lowers to Effect::MoveAndModify { blink: true, tapped: true }.
    let tapped_blink = read(
        "Name:DisplaceTap\nManaCost:2 W\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DBRet\n\
         SVar:DBRet:DB$ ChangeZone | Defined$ Remembered | Origin$ Exile | Destination$ Battlefield | Tapped$ True | SubAbility$ DBClean\n\
         SVar:DBClean:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        tapped_blink.abilities[0].contains("Effect::MoveAndModify {")
            && tapped_blink.abilities[0].contains("blink: true, owner_control: true, tapped: true"),
        "{:?}",
        tapped_blink.abilities
    );
}

#[test]
fn wave23b_create_token_sized_refusals() {
    let sized_tokens = wave23b_sized_tokens();
    let refused_sized =
        |text: &str| transcode(&parse(text), &cats(), Some(&sized_tokens)).is_none();
    // Refusals: plain token on */* or */1 without CDA, partial stats on */*,
    // overriding a CDA token's stats, opponent attacking sized token, missing PumpDuration$,
    // AtEOT$ Sacrifice for targeted opponent, and invalid BlinkReturn.Tapped.
    for bad in [
        "Name:PlainStar\nManaCost:2 G\nTypes:Sorcery\nA:SP$ Token | TokenScript$ g_x_x_goblin",
        "Name:PlainHalfStar\nManaCost:2 R\nTypes:Sorcery\nA:SP$ Token | TokenScript$ r_x_1_goblin_trample",
        "Name:MissingTough\nManaCost:2 G\nTypes:Sorcery\nA:SP$ Token | TokenScript$ g_x_x_goblin | TokenPower$ 3",
        "Name:CdaOverride\nManaCost:2 G\nTypes:Sorcery\nA:SP$ Token | TokenScript$ c_x_x_cda_goblin | TokenPower$ 3 | TokenToughness$ 3",
        "Name:OppAtk\nManaCost:2 R\nTypes:Instant\nA:SP$ Token | TokenScript$ g_x_x_goblin | TokenOwner$ Opponent | TokenPower$ 2 | TokenToughness$ 2 | TokenAttacking$ True",
        "Name:HasteNoDur\nManaCost:2 R\nTypes:Sorcery\nA:SP$ Token | TokenScript$ g_x_x_goblin | TokenPower$ 2 | TokenToughness$ 2 | PumpKeywords$ Haste",
        "Name:SacOpp\nManaCost:2 R\nTypes:Sorcery\nA:SP$ Token | ValidTgts$ Opponent | TokenScript$ r_x_1_goblin_trample | TokenOwner$ TargetedPlayer | TokenPower$ 2 | AtEOT$ Sacrifice",
        "Name:BadBlinkTap\nManaCost:2 W\nTypes:Instant\nA:SP$ ChangeZone | ValidTgts$ Creature | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | SubAbility$ DBRet\nSVar:DBRet:DB$ ChangeZone | Defined$ Remembered | Origin$ Exile | Destination$ Battlefield | Tapped$ Maybe",
    ] {
        assert!(refused_sized(bad), "must refuse: {bad}");
    }
}

fn cats_zones_extra() -> SubtypeCatalogs {
    let mut c = cats();
    c.creature
        .extend(["God", "Samurai", "Spirit", "Rat"].map(Into::into));
    c.land.push("Plains".into());
    c.normalize();
    c
}

#[test]
fn target_same_name_zone_all_sweeps_and_hour_of_glory() {
    let extra_cats = cats_zones_extra();
    let read_extra = |text: &str| {
        let parsed = parse(text);
        transcode(&parsed, &extra_cats, None)
            .unwrap_or_else(|| panic!("refused: {:?}", refusal_reason(&parsed, &extra_cats, None)))
    };
    let sever = read_extra(
        "Name:Sever the Bloodline\nManaCost:3 B\nTypes:Sorcery\nK:Flashback:5 B B\n\
         A:SP$ TargetOnly | ValidTgts$ Creature | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ ChangeZoneAll | Origin$ Battlefield | Destination$ Exile | \
         ChangeType$ TargetedCard.Self,Creature.NotDefinedTargeted+sharesNameWith Targeted",
    );
    assert!(
        sever.abilities[0].contains("Effect::RememberTargetsThen { then: &[Effect::ExileAll {")
            && sever.abilities[0].contains("linked: false")
            && sever.statics.contains("Filter::IsRemembered")
            && sever.statics.contains("Filter::SharesNameWithRemembered"),
        "{:?} / {}",
        sever.abilities,
        sever.statics
    );

    let echoing_truth = read_extra(
        "Name:Echoing Truth\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Permanent.nonLand | SubAbility$ DBBounce\n\
         SVar:DBBounce:DB$ ChangeZoneAll | Origin$ Battlefield | Destination$ Hand | \
         ChangeType$ TargetedCard.Self,Permanent.NotDefinedTargeted+sharesNameWith Targeted",
    );
    assert!(
        echoing_truth.abilities[0]
            .contains("Effect::RememberTargetsThen { then: &[Effect::ReturnAllToHand {"),
        "{:?}",
        echoing_truth.abilities
    );

    let legions = read_extra(
        "Name:Legions to Ashes\nManaCost:1 W B\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Permanent.nonLand+OppCtrl | SubAbility$ ExileAll\n\
         SVar:ExileAll:DB$ ChangeZoneAll | Origin$ Battlefield | Destination$ Exile | \
         ChangeType$ TargetedCard.Self,Card.NotDefinedTargeted+token+sharesNameWith Targeted+ControlledBy TargetedController",
    );
    assert!(
        legions.abilities[0].contains("Effect::RememberTargetsThen { then: &[Effect::ExileAll {")
            && legions.statics.contains("Filter::IsToken")
            && legions
                .statics
                .contains("Filter::ControlledBy(PlayerRel::ControllerOfTarget)"),
        "{:?} / {}",
        legions.abilities,
        legions.statics
    );

    let detention_sphere = read_extra(
        "Name:Detention Sphere\nManaCost:1 W U\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | \
         OptionalDecider$ You | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ TargetOnly | ValidTgts$ Permanent.nonLand+!namedDetention Sphere | \
         SubAbility$ DBChangeZoneAll\n\
         SVar:DBChangeZoneAll:DB$ ChangeZoneAll | ChangeType$ Targeted.Self,Targeted.sameName | \
         Origin$ Battlefield | Destination$ Exile\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | ValidCard$ Card.Self | \
         Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZoneAll | ChangeType$ Card.ExiledWithSource | Origin$ Exile | \
         Destination$ Battlefield",
    );
    assert!(
        detention_sphere.abilities[0].contains("Effect::RememberTargetsThen")
            && detention_sphere.abilities[0].contains("Effect::ExileAll")
            && detention_sphere.abilities[0].contains("linked: true")
            && detention_sphere.abilities[1].contains("Effect::ReturnLinkedToBattlefield"),
        "{:?}",
        detention_sphere.abilities
    );
}

#[test]
fn target_same_name_graveyard_sweeps_hour_of_glory_and_refusals() {
    let extra_cats = cats_zones_extra();
    let read_extra = |text: &str| {
        let parsed = parse(text);
        transcode(&parsed, &extra_cats, None)
            .unwrap_or_else(|| panic!("refused: {:?}", refusal_reason(&parsed, &extra_cats, None)))
    };
    let refused_extra = |text: &str| transcode(&parse(text), &extra_cats, None).is_none();

    let echoing_return = read_extra(
        "Name:Echoing Return\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Creature.YouOwn | TgtZone$ Graveyard | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZoneAll | Origin$ Graveyard | Destination$ Hand | Defined$ You | \
         ChangeType$ TargetedCard.Self,Card.NotDefinedTargeted+sharesNameWith Targeted",
    );
    assert!(
        echoing_return.abilities[0]
            .contains("Effect::RememberTargetsThen { then: &[Effect::GraveyardAllToHand {"),
        "{:?}",
        echoing_return.abilities
    );

    let rat_king = read_extra(
        "Name:Rat King, Verminister\nManaCost:1 B\nTypes:Legendary Creature Rat\nPT:1/1\n\
         A:AB$ TargetOnly | Cost$ T Sac<3/Rat> | ValidTgts$ Creature.YouOwn | TgtZone$ Graveyard | \
         SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZoneAll | Origin$ Graveyard | Destination$ Battlefield | \
         Tapped$ True | Defined$ You | \
         ChangeType$ TargetedCard.Self,Card.NotDefinedTargeted+sharesNameWith Targeted",
    );
    assert!(
        rat_king.abilities[0]
            .contains("Effect::RememberTargetsThen { then: &[Effect::ChooseCardEx {")
            && rat_king.abilities[0]
                .contains("not_chosen_movement: ChooseMove::Battlefield { tapped: true }"),
        "{:?}",
        rat_king.abilities
    );

    let hour_of_glory = read_extra(
        "Name:Hour of Glory\nManaCost:3 B\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Creature | \
         SubAbility$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZoneAll | Origin$ Hand | Destination$ Exile | \
         ConditionDefined$ Targeted | ConditionPresent$ God | Defined$ TargetedController | \
         ChangeType$ TargetedCard.sameName",
    );
    assert!(
        hour_of_glory.abilities[0].contains("Effect::RememberTargetsThen")
            && hour_of_glory.abilities[0].contains("Effect::IfTargetMatchesElse { filter: &")
            && hour_of_glory.abilities[0]
                .contains("Effect::RevealHand { who: PlayerRel::ControllerOfTarget }")
            && hour_of_glory.abilities[0].contains("filter: &Filter::SharesNameWithRemembered")
            && hour_of_glory.statics.contains("subtypes::creature::GOD"),
        "{:?} / {}",
        hour_of_glory.abilities,
        hour_of_glory.statics
    );

    for bad in [
        "Name:DeputyLike\nManaCost:1 W U\nTypes:Creature\nPT:1/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ TargetOnly | ValidTgts$ Permanent.nonLand | SubAbility$ DBChangeZoneAll\n\
         SVar:DBChangeZoneAll:DB$ ChangeZoneAll | Origin$ Battlefield | Destination$ Exile | \
         ChangeType$ TargetedCard.Self,Permanent.NotDefinedTargeted+sharesNameWith Targeted | \
         Duration$ UntilHostLeavesPlay",
        "Name:NoSameName\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Creature | SubAbility$ DBEx\n\
         SVar:DBEx:DB$ ChangeZoneAll | Origin$ Battlefield | Destination$ Exile | \
         ChangeType$ TargetedCard.Self,Creature.NotDefinedTargeted",
        "Name:BadUseAllOrigins\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Creature | SubAbility$ DBEx\n\
         SVar:DBEx:DB$ ChangeZoneAll | Origin$ Battlefield | Destination$ Exile | UseAllOriginZones$ True | \
         ChangeType$ TargetedCard.Self,Creature.NotDefinedTargeted+sharesNameWith Targeted",
        "Name:BadUnconstrainedMandatorySearch\nManaCost:2 W\nTypes:Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigS\n\
         SVar:TrigS:DB$ ChangeZone | Origin$ Library | Destination$ Battlefield | ChangeType$ Card | ChangeNum$ 1 | RememberChanged$ True | NoShuffle$ True | SubAbility$ DB2\n\
         SVar:DB2:DB$ ChangeZone | Optional$ True | Origin$ Library | Destination$ Battlefield | ConditionCheckSVar$ X | ConditionSVarCompare$ GTY | ChangeType$ Land.Plains | ChangeNum$ 1 | RememberChanged$ True | NoShuffle$ True | SubAbility$ DBS\n\
         SVar:DBS:DB$ Shuffle | Defined$ You | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ GE1\n\
         SVar:X:PlayerCountOpponents$HighestValid Land.YouCtrl\n\
         SVar:Y:Count$Valid Land.YouCtrl",
    ] {
        assert!(refused_extra(bad), "must refuse: {bad}");
    }
}

#[test]
fn zone_change_remembered_capture_and_conditional_search() {
    let extra_cats = cats_zones_extra();
    let tok_lookup = TokenLookup::held(&[(
        "w_1_1_spirit_flying",
        "Name:Spirit\nTypes:Creature Spirit\nColors:white\nPT:1/1\nK:Flying",
    )]);
    let read_extra = |text: &str| {
        let parsed = parse(text);
        transcode(&parsed, &extra_cats, Some(&tok_lookup)).unwrap_or_else(|| {
            panic!(
                "refused: {:?}",
                refusal_reason(&parsed, &extra_cats, Some(&tok_lookup))
            )
        })
    };

    let conscription = read_extra(
        "Name:Heartless Conscription\nManaCost:6 B B\nTypes:Sorcery\n\
         A:SP$ ChangeZoneAll | Origin$ Battlefield | Destination$ Exile | ChangeType$ Creature | \
         RememberChanged$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ RememberedCard | StaticAbilities$ STPlay | \
         SubAbility$ DBChange | ForgetOnMoved$ Exile | Duration$ Permanent\n\
         SVar:STPlay:Mode$ Continuous | MayPlay$ True | MayPlayIgnoreType$ True | \
         Affected$ Card.IsRemembered | AffectedZone$ Exile\n\
         SVar:DBChange:DB$ ChangeZone | Origin$ Stack | Destination$ Exile | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        conscription.abilities[0].contains("Effect::CaptureAffected")
            && conscription.abilities[0].contains("Effect::GrantRememberedExilePermission")
            && conscription.abilities[0].contains("spend_mana_as_any_type()")
            && conscription.abilities[0].contains("Effect::ExileSource"),
        "{:?}",
        conscription.abilities
    );

    let author = read_extra(
        "Name:Author of Shadows\nManaCost:4 B\nTypes:Creature Shade Warlock\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigChangeZoneAll\n\
         SVar:TrigChangeZoneAll:DB$ ChangeZoneAll | ChangeType$ Card.YouDontOwn | Origin$ Graveyard | \
         Destination$ Exile | RememberChanged$ True | SubAbility$ DBChooseCard\n\
         SVar:DBChooseCard:DB$ ChooseCard | Defined$ You | Choices$ Card.nonLand+IsRemembered | \
         ChoiceTitle$ Choose a nonland card exiled this way | ChoiceZone$ Exile | Mandatory$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ ChosenCard | StaticAbilities$ STPlay | \
         Duration$ Permanent | ExileOnMoved$ Exile | SubAbility$ DBCleanup\n\
         SVar:STPlay:Mode$ Continuous | MayPlay$ True | MayPlayIgnoreColor$ True | \
         Affected$ Card.IsRemembered | AffectedZone$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearChosenCard$ True",
    );
    assert!(
        author.abilities[0].contains("Effect::CaptureAffected")
            && author.abilities[0]
                .contains("Effect::ExileGraveyard { player: PlayerRel::EachOpponent }")
            && author.abilities[0].contains("zone: ChooseZone::Remembered")
            && author.abilities[0].contains("Effect::ExileWithPermission")
            && author.abilities[0].contains("spend_mana_as_any_color()"),
        "{:?}",
        author.abilities
    );

    let summoners_sending = read_extra(
        "Name:Summoner's Sending\nManaCost:1 W\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | TriggerZones$ Battlefield | \
         Execute$ TrigExile | OptionalDecider$ You\n\
         SVar:TrigExile:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Creature | \
         SubAbility$ DBToken | RememberChanged$ True\n\
         SVar:DBToken:DB$ Token | TokenScript$ w_1_1_spirit_flying | TokenOwner$ You | \
         ConditionDefined$ Remembered | ConditionPresent$ Card.Creature | ImprintTokens$ True | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | Defined$ Imprinted | CounterType$ P1P1 | CounterNum$ 1 | \
         ConditionDefined$ Remembered | ConditionPresent$ Card.cmcGE4 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True",
    );
    assert!(
        summoners_sending.abilities[0].contains("ThisWay::ExiledFromGraveyard")
            && summoners_sending.abilities[0].contains("Effect::CreateToken {")
            && summoners_sending.abilities[0].contains("Effect::RepeatEachObject {")
            && summoners_sending.abilities[0].contains("CounterKind::P1P1")
            && summoners_sending.statics.contains("Filter::CmcAtLeast(4)"),
        "{:?} / {}",
        summoners_sending.abilities,
        summoners_sending.statics
    );

    let claim_jumper = read_extra(
        "Name:Claim Jumper\nManaCost:2 W\nTypes:Creature Rabbit Mercenary\nPT:3/3\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | CheckSVar$ X | \
         SVarCompare$ GTY | OptionalDecider$ You | ValidCard$ Card.Self | Execute$ TrigChange\n\
         SVar:TrigChange:DB$ ChangeZone | Origin$ Library | Destination$ Battlefield | \
         ChangeType$ Land.Plains | ChangeNum$ 1 | RememberChanged$ True | Tapped$ True | NoShuffle$ True | \
         SubAbility$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | Optional$ True | Origin$ Library | Destination$ Battlefield | \
         ConditionCheckSVar$ X | ConditionSVarCompare$ GTY | ChangeType$ Land.Plains | ChangeNum$ 1 | \
         RememberChanged$ True | Tapped$ True | NoShuffle$ True | SubAbility$ DBShuffle\n\
         SVar:DBShuffle:DB$ Shuffle | Defined$ You | ConditionDefined$ Remembered | \
         ConditionPresent$ Card | ConditionCompare$ GE1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:PlayerCountOpponents$HighestValid Land.YouCtrl\n\
         SVar:Y:Count$Valid Land.YouCtrl",
    );
    assert!(
        claim_jumper.abilities[0].contains("Effect::SearchLibraryBeforeShuffle")
            && claim_jumper.abilities[0].contains("Condition::OpponentControlsMore(&Filter::LAND)")
            && claim_jumper.abilities[0].contains("Effect::ShuffleLibrary { who: PlayerRel::You }"),
        "{:?}",
        claim_jumper.abilities
    );
}

#[test]
fn zone_change_single_origin_and_event_objects() {
    let extra_cats = cats_zones_extra();
    let read_extra = |text: &str| {
        let parsed = parse(text);
        transcode(&parsed, &extra_cats, None)
            .unwrap_or_else(|| panic!("refused: {:?}", refusal_reason(&parsed, &extra_cats, None)))
    };
    let balrog = read_extra(
        "Name:The Balrog, Flame of Udun\nManaCost:3 B R\nTypes:Legendary Creature\nPT:7/7\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | TriggerZones$ Battlefield | \
         Destination$ Graveyard | ValidCard$ Creature.Legendary+OppCtrl | Execute$ TrigChange\n\
         SVar:TrigChange:DB$ ChangeZone | Defined$ Self | Origin$ Battlefield | \
         Destination$ Library | LibraryPosition$ -1",
    );
    assert!(
        balrog.abilities[0].contains("Effect::PutSourceOnBottomOfLibrary"),
        "{:?}",
        balrog.abilities
    );

    let jailbreak = read_extra(
        "Name:Jailbreak Scheme\nManaCost:U\nTypes:Sorcery\nK:Spree\n\
         A:SP$ Charm | Choices$ DBPutCounter,DBChangeZone | MinCharmNum$ 1 | CharmNum$ 2\n\
         SVar:DBPutCounter:DB$ PutCounter | ModeCost$ 3 | ValidTgts$ Creature | CounterType$ P1P1 | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | ExileOnMoved$ Battlefield | RememberObjects$ ParentTarget | StaticAbilities$ Unblockable\n\
         SVar:Unblockable:Mode$ CantBlockBy | ValidAttacker$ Card.IsRemembered\n\
         SVar:DBChangeZone:DB$ ChangeZone | ModeCost$ 2 | ValidTgts$ Artifact,Creature | \
         AlternativeDecider$ ThisTargetedOwner | Origin$ Battlefield | Destination$ Library | \
         DestinationAlternative$ Library | LibraryPositionAlternative$ -1",
    );
    assert!(
        jailbreak.abilities[0].contains("Effect::OwnerPutsOnTopOrBottom"),
        "{:?}",
        jailbreak.abilities
    );

    let krark = read_extra(
        "Name:Krark, the Thumbless\nManaCost:1 R\nTypes:Legendary Creature Goblin Wizard\nPT:2/2\n\
         T:Mode$ SpellCast | ValidCard$ Instant,Sorcery | ValidActivatingPlayer$ You | \
         TriggerZones$ Battlefield | Execute$ TrigFlip\n\
         SVar:TrigFlip:DB$ FlipCoin | WinSubAbility$ DBCopy | LoseSubAbility$ DBBounce\n\
         SVar:DBBounce:DB$ ChangeZone | Defined$ TriggeredSpellAbility | Origin$ Stack | Destination$ Hand\n\
         SVar:DBCopy:DB$ CopySpellAbility | Defined$ TriggeredSpellAbility | MayChooseTarget$ True",
    );
    assert!(
        krark.abilities[0].contains("Effect::bounce(TargetSpec::EventObject)"),
        "{:?}",
        krark.abilities
    );

    let oathkeeper = read_extra(
        "Name:Oathkeeper, Takeno's Daisho\nManaCost:3\nTypes:Legendary Artifact Equipment\nK:Equip:2\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | \
         ValidCard$ Card.EquippedBy | Execute$ TrigReturn\n\
         SVar:TrigReturn:DB$ ChangeZone | Destination$ Battlefield | \
         Defined$ TriggeredNewCardLKICopy | GainControl$ True | \
         ConditionDefined$ TriggeredNewCardLKICopy | ConditionPresent$ Card.Samurai | ConditionCompare$ GE1",
    );
    assert!(
        oathkeeper.abilities[1].contains("Effect::reanimate(TargetSpec::EventObject)")
            && oathkeeper.abilities[1].contains("Condition::ObjectMatches"),
        "{:?}",
        oathkeeper.abilities
    );
}

#[test]
fn zone_change_up_to_x_single_origin_distortion_and_refusals() {
    let extra_cats = cats_zones_extra();
    let read_extra = |text: &str| {
        let parsed = parse(text);
        transcode(&parsed, &extra_cats, None)
            .unwrap_or_else(|| panic!("refused: {:?}", refusal_reason(&parsed, &extra_cats, None)))
    };
    let refused_extra = |text: &str| transcode(&parse(text), &extra_cats, None).is_none();

    let divergent = read_extra(
        "Name:Divergent Equation\nManaCost:X X U\nTypes:Instant\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Hand | \
         ValidTgts$ Instant.YouOwn,Sorcery.YouOwn | TargetMin$ 0 | TargetMax$ X | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ ChangeZone | Origin$ Stack | Destination$ Exile\n\
         SVar:X:Count$xPaid",
    );
    assert!(
        divergent.abilities[0].contains("TargetReq::up_to_x(")
            && divergent.abilities[0].contains("Effect::ExileSource"),
        "{:?}",
        divergent.abilities
    );

    let perilous = read_extra(
        "Name:Perilous Vault\nManaCost:4\nTypes:Artifact\n\
         A:AB$ ChangeZoneAll | Cost$ 5 T Exile<1/CARDNAME> | UseAllOriginZones$ True | \
         Origin$ Battlefield | Destination$ Exile | ChangeType$ Permanent.nonLand",
    );
    assert!(
        perilous.abilities[0].contains("Effect::ExileAll"),
        "{:?}",
        perilous.abilities
    );

    let distortion = read_extra(
        "Name:Thought Distortion\nManaCost:4 B B\nTypes:Sorcery\n\
         A:SP$ RevealHand | ValidTgts$ Opponent | IsCurse$ True | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ ChangeZoneAll | Origin$ Hand,Graveyard | Destination$ Exile | \
         ChangeType$ Card.nonCreature+nonLand+TargetedPlayerCtrl\n\
         Oracle:Target opponent reveals their hand. Exile all noncreature, nonland cards from that player's hand and graveyard.",
    );
    assert!(
        distortion.abilities[0].contains("Effect::RevealHand { who: PlayerRel::Chosen }")
            && distortion.abilities[0].contains("zone: ChooseZone::Hand(PlayerRel::Chosen)")
            && distortion.abilities[0].contains("zone: ChooseZone::Graveyard(PlayerRel::Chosen)"),
        "{:?}",
        distortion.abilities
    );

    assert!(refused_extra(
        "Name:BadNoOriginETB\nManaCost:2\nTypes:Artifact\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Creature | Execute$ TrigRet\n\
         SVar:TrigRet:DB$ ChangeZone | Destination$ Battlefield | Defined$ TriggeredNewCardLKICopy",
    ));
    assert!(refused_extra(
        "Name:BadStackBounceNonSpell\nManaCost:2 U\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Creature | Execute$ TrigBounce\n\
         SVar:TrigBounce:DB$ ChangeZone | Defined$ TriggeredSpellAbility | Origin$ Stack | Destination$ Hand",
    ));
}

#[test]
fn wave20_attach_player_and_distinct_mana_targets() {
    // Archnemesis: `DB$ Attach | Object$ Self | Defined$ TriggeredAttackingPlayer` -> `TargetSpec::Player(PlayerRel::EventPlayer)`.
    let archnemesis = read(
        "Name:ArchnemesisLike\nManaCost:3 B\nTypes:Enchantment Aura\n\
         K:Enchant:Opponent\n\
         T:Mode$ AttackersDeclared | AttackedTarget$ You | TriggerZones$ Battlefield | Execute$ TrigAttach | OptionalDecider$ You\n\
         SVar:TrigAttach:DB$ Attach | Object$ Self | Defined$ TriggeredAttackingPlayer",
    );
    assert!(
        archnemesis.abilities.iter().any(|a| {
            a.contains("Effect::AttachSelf { target: TargetSpec::Player(PlayerRel::EventPlayer) }")
        }),
        "{:?}",
        archnemesis.abilities
    );

    // Ardenn: `ValidTgts$ Permanent,Player` + `Defined$ Targeted` + `Object$ Valid Aura.YouCtrl,Equipment.YouCtrl`.
    let mut ardenn_cats = cats();
    ardenn_cats.creature.push("Kor".into());
    ardenn_cats.creature.push("Scout".into());
    ardenn_cats.artifact.push("Equipment".into());
    ardenn_cats.enchantment.push("Aura".into());
    ardenn_cats.normalize();
    let ardenn = transcode(
        &parse(
            "Name:ArdennLike\nManaCost:2 W\nTypes:Creature Kor Scout\nPT:2/2\n\
             T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigAttach | OptionalDecider$ You\n\
             SVar:TrigAttach:DB$ Attach | Defined$ Targeted | ValidTgts$ Permanent,Player | Object$ Valid Aura.YouCtrl,Equipment.YouCtrl | Optional$ True",
        ),
        &ardenn_cats,
        None,
    )
    .expect("ArdennLike should transcode");
    let ardenn_trig = &ardenn.abilities[0];
    assert!(
        ardenn_trig.contains("TargetSpec::PlayerOrObject(&")
            && ardenn_trig.contains("Effect::AttachChosen {")
            && ardenn_trig.contains("optional: true"),
        "{ardenn_trig}"
    );

    // Agadeem's Awakening: `TargetMin$ 0 | TargetMax$ Y` with superset `Count$ValidGraveyard Creature.YouOwn`
    // and `TargetsWithDifferentCMC$ True` -> `TargetReq { distinct_mana_values: true, ..TargetReq::up_to(..., 255) }`.
    let agadeem = read(
        "Name:AgadeemLike\nManaCost:X B B B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | TargetMin$ 0 | TargetMax$ Y | ValidTgts$ Creature.YouOwn+cmcLEX | TargetsWithDifferentCMC$ True\n\
         SVar:X:Count$xPaid\n\
         SVar:Y:Count$ValidGraveyard Creature.YouOwn",
    );
    assert!(
        agadeem.abilities[0].contains("TargetReq { distinct_mana_values: true, ..TargetReq::up_to(TargetSpec::CardInGraveyard(")
            && agadeem.abilities[0].contains("Effect::GraveyardToBattlefield {"),
        "{}",
        agadeem.abilities[0]
    );

    // Grind // Dust: `Creature.counters_GE1_M1M1` paired with `Count$Valid Creature.counters_GE_M1M1`.
    let dust = read(
        "Name:DustLike\nManaCost:3 W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Creature.counters_GE1_M1M1 | TargetMin$ 0 | TargetMax$ MaxTargets | Origin$ Battlefield | Destination$ Exile\n\
         SVar:MaxTargets:Count$Valid Creature.counters_GE_M1M1",
    );
    assert!(
        dust.abilities[0].contains("Effect::exile(TargetSpec::Object(")
            && dust.abilities[0].contains("TargetReq::up_to(TargetSpec::Object(")
            && dust.abilities[0].contains(", 255)"),
        "{}",
        dust.abilities[0]
    );

    // Honest refusals: optional self-attach to event player, Permanent,Player on Destroy,
    // invalid/single-target TargetsWithDifferentCMC, mismatched counter upper bound, and Scheming Symmetry.
    for bad in [
        "Name:BadOptionalPlayerAttach\nManaCost:1 W\nTypes:Enchantment Aura\nK:Enchant:Player\n\
         T:Mode$ AttackersDeclared | AttackedTarget$ You | TriggerZones$ Battlefield | Execute$ TrigAttach\n\
         SVar:TrigAttach:DB$ Attach | Object$ Self | Defined$ TriggeredAttackingPlayer | Optional$ True",
        "Name:BadDestroyPermPlayer\nManaCost:3 B\nTypes:Sorcery\n\
         A:SP$ Destroy | ValidTgts$ Permanent,Player",
        "Name:BadDistinctCmcSingle\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature.YouOwn | TargetsWithDifferentCMC$ True",
        "Name:BadDistinctCmcMaybe\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | TargetMin$ 0 | TargetMax$ 2 | ValidTgts$ Creature.YouOwn | TargetsWithDifferentCMC$ Maybe",
        "Name:BadSupersetMax\nManaCost:X B B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | TargetMin$ 0 | TargetMax$ Y | ValidTgts$ Creature.YouOwn+cmcLEX | TargetsWithDifferentCMC$ True\n\
         SVar:X:Count$xPaid\nSVar:Y:Count$ValidGraveyard Artifact.YouOwn",
        "Name:BadCounterMax\nManaCost:3 W\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Creature.counters_GE1_M1M1 | TargetMin$ 0 | TargetMax$ MaxTargets | Origin$ Battlefield | Destination$ Exile\n\
         SVar:MaxTargets:Count$Valid Creature.counters_GE2_M1M1",
        "Name:SchemingSymmetryLike\nManaCost:B\nTypes:Sorcery\n\
         A:SP$ ChangeZone | ValidTgts$ Player | TargetMin$ 2 | TargetMax$ 2 | Origin$ Library | Destination$ Library | ChangeType$ Card | ChangeNum$ 1",
    ] {
        assert!(
            refused(bad),
            "must refuse:
{bad}"
        );
    }
}

#[test]
fn w20_target_only_group_followups() {
    let silence = read(
        "Name:Silence the Believers\nManaCost:2 B B\nTypes:Instant\nK:Strive:2 B\n\
         A:SP$ TargetOnly | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ MaxTgts | IsCurse$ True | RememberTargets$ True | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ ChangeZoneAll | ChangeType$ Card.IsRemembered,Aura.AttachedTo Targeted | Origin$ Battlefield | Destination$ Exile | UseAllOriginZones$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:MaxTgts:Count$Valid Creature",
    );
    assert!(
        silence.abilities.len() == 2
            && silence.abilities[0].contains("AbilityDef::strive(")
            && silence.abilities[1]
                .contains("Effect::RememberTargetsThen { then: &[Effect::ExileAll")
            && silence.abilities[1]
                .contains("TargetReq::up_to(TargetSpec::Object(&Filter::CREATURE), 255)"),
        "{:?}",
        silence.abilities
    );

    let wild_swing = read(
        "Name:Wild Swing\nManaCost:3 R\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Permanent.nonEnchantment | TargetMin$ 3 | TargetMax$ 3 | IsCurse$ True | SubAbility$ DBChooseRandom\n\
         SVar:DBChooseRandom:DB$ ChooseCard | Defined$ You | Amount$ 1 | AtRandom$ True | DefinedCards$ Targeted | SubAbility$ DBDestroy\n\
         SVar:DBDestroy:DB$ Destroy | Defined$ ChosenCard",
    );
    assert!(
        wild_swing.abilities[0]
            .contains("Effect::RememberTargetsThen { then: &[Effect::DestroyRandom")
            && wild_swing.abilities[0].contains("TargetReq::exactly(TargetSpec::Object(&"),
        "{}",
        wild_swing.abilities[0]
    );

    let incriminate = read(
        "Name:Incriminate\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Creature | TargetMin$ 2 | TargetMax$ 2 | TargetUnique$ True | TargetsWithSameController$ True | IsCurse$ True | SubAbility$ DBSac\n\
         SVar:DBSac:DB$ Sacrifice | Defined$ TargetedController | SacValid$ Card.targetedBy",
    );
    assert!(
        incriminate.abilities[0].contains("TargetReq::exactly(TargetSpec::Object(&Filter::CREATURE), 2).with_constraint(TargetConstraint::SameController)")
            && incriminate.abilities[0].contains("Effect::ChooseCardEx")
            && incriminate.abilities[0].contains("then: &[Effect::SacrificeEvent]")
            && incriminate.abilities[0].contains("for_unchosen: &[]"),
        "{}",
        incriminate.abilities[0]
    );

    let barrins = read(
        "Name:Barrin's Spite\nManaCost:2 U B\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Creature | TargetMin$ 2 | TargetMax$ 2 | TargetUnique$ True | TargetsWithSameController$ True | IsCurse$ True | SubAbility$ DBChooseSac\n\
         SVar:DBChooseSac:DB$ ChooseCard | Mandatory$ True | DefinedCards$ Targeted | Defined$ TargetedController | ChoiceTitle$ Choose one to sacrifice | SubAbility$ DBSac | AILogic$ WorstCard\n\
         SVar:DBSac:DB$ SacrificeAll | Defined$ ChosenCard | SubAbility$ DBBounce\n\
         SVar:DBBounce:DB$ ChangeZone | Defined$ Targeted.!ChosenCard | Origin$ Battlefield | Destination$ Hand",
    );
    assert!(
        barrins.abilities[0].contains("TargetReq::exactly(TargetSpec::Object(&Filter::CREATURE), 2).with_constraint(TargetConstraint::SameController)")
            && barrins.abilities[0].contains("then: &[Effect::SacrificeEvent]")
            && barrins.abilities[0].contains(
                "for_unchosen: &[Effect::ReturnToHand { target: TargetSpec::EventObject }]"
            ),
        "{}",
        barrins.abilities[0]
    );

    let mut deliver_cats = cats();
    deliver_cats.planeswalker.push("Bolas".into());
    deliver_cats.normalize();
    let deliver_script = parse(
        "Name:Deliver Unto Evil\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Card.YouCtrl | TargetMin$ 0 | TargetMax$ 4 | TgtZone$ Graveyard | SubAbility$ DBBranch\n\
         SVar:DBBranch:DB$ Branch | BranchConditionSVar$ X | TrueSubAbility$ DBChangeZone | FalseSubAbility$ DBChooseOpp | SubAbility$ DBExile\n\
         SVar:X:Count$Valid Planeswalker.Bolas+YouCtrl\n\
         SVar:DBChangeZone:DB$ ChangeZone | Defined$ Targeted | Origin$ Graveyard | Destination$ Hand\n\
         SVar:DBChooseOpp:DB$ ChoosePlayer | Choices$ Player.Opponent | SubAbility$ DBChoose\n\
         SVar:DBChoose:DB$ ChooseCard | DefinedCards$ Targeted | ChoiceZone$ Graveyard | Amount$ 2 | Mandatory$ True | Defined$ ChosenPlayer | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZone | Defined$ Targeted.!ChosenCard | Origin$ Graveyard | Destination$ Hand\n\
         SVar:DBExile:DB$ ChangeZone | Origin$ Stack | Destination$ Exile",
    );
    let deliver = transcode(&deliver_script, &deliver_cats, None).expect("Deliver Unto Evil reads");
    assert!(
        deliver.abilities[0].contains("Effect::IfCondition")
            && deliver.abilities[0]
                .contains("Effect::ChoosePlayerFrom { among: PlayerRel::Opponent")
            && deliver.abilities[0].contains("Effect::RememberTargetsThen")
            && deliver.abilities[0].contains("Effect::ChooseCardEx")
            && deliver.abilities[0].contains("Effect::ExileSource"),
        "{}",
        deliver.abilities[0]
    );
}

#[test]
fn w20_switch_block_and_target_only_refusals() {
    let sorrows = read(
        "Name:Sorrow's Path\nManaCost:no cost\nTypes:Land\n\
         A:AB$ SwitchBlock | Cost$ T | TargetMin$ 2 | TargetMax$ 2 | ValidTgts$ Creature.blocking+OppCtrl | TargetsWithSameController$ True | DefinedAttacker$ Valid Creature.blockedByValidThisTurn Targeted | DefinedBlocker$ Targeted | RemoveFromCombat$ True\n\
         T:Mode$ Taps | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigDamage\n\
         SVar:TrigDamage:DB$ DamageAll | ValidCards$ Creature.YouCtrl | ValidPlayers$ You | NumDmg$ 2",
    );
    assert!(
        sorrows.abilities.len() == 2
            && sorrows.abilities[0].contains("Effect::SwapBlockers")
            && sorrows.abilities[0].contains("TargetConstraint::SameController"),
        "{:?}",
        sorrows.abilities
    );

    // Honest refusal boundaries for Wave 20 multi-target / TargetOnly / SwitchBlock:
    for bad in [
        // Cannibalize / Retribution (`AddCounter` only handles single `res.targets`)
        "Name:CannibalizeLike\nManaCost:1 B\nTypes:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Creature | TargetMin$ 2 | TargetMax$ 2 | TargetsWithSameController$ True | IsCurse$ True | SubAbility$ DBChoose\n\
         SVar:DBChoose:DB$ ChooseCard | DefinedCards$ Targeted | Amount$ 1 | Mandatory$ True | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ ChangeZone | Defined$ ChosenCard | Origin$ Battlefield | Destination$ Exile | SubAbility$ DBCounters\n\
         SVar:DBCounters:DB$ PutCounter | Defined$ Targeted.!ChosenCard | CounterType$ P1P1 | CounterNum$ 2",
        // Bioshift (`MoveCounter` with `TargetMin$ 2 | TargetMax$ 2 | TargetsWithSameController$ True | CounterNum$ Any`)
        "Name:Bioshift\nManaCost:GU\nTypes:Instant\n\
         A:SP$ MoveCounter | ValidTgts$ Creature | TargetMin$ 2 | TargetMax$ 2 | TargetsWithSameController$ True | CounterType$ P1P1 | CounterNum$ Any",
        // Single-target TargetOnly with TargetUnique$ True cannot skip second-slot restriction
        "Name:SingleTargetUnique\nManaCost:1 U\nTypes:Instant\n\
         A:SP$ TargetOnly | ValidTgts$ Creature | TargetUnique$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ 1",
        // Spell-based SwitchBlock or SwitchBlock missing TargetsWithSameController$ True
        "Name:SpellSwitchBlock\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ SwitchBlock | TargetMin$ 2 | TargetMax$ 2 | ValidTgts$ Creature.blocking+OppCtrl | TargetsWithSameController$ True | DefinedAttacker$ Valid Creature.blockedByValidThisTurn Targeted | DefinedBlocker$ Targeted | RemoveFromCombat$ True",
        "Name:UnsameCtrlSwitchBlock\nManaCost:no cost\nTypes:Land\n\
         A:AB$ SwitchBlock | Cost$ T | TargetMin$ 2 | TargetMax$ 2 | ValidTgts$ Creature.blocking | DefinedAttacker$ Valid Creature.blockedByValidThisTurn Targeted | DefinedBlocker$ Targeted | RemoveFromCombat$ True",
    ] {
        assert!(refused(bad), "must refuse:\n{bad}");
    }
}

#[test]
fn untap_up_to_unlocks_and_refusals() {
    let cloud = read(
        "Name:Cloud of Faeries\nManaCost:1 U\nTypes:Creature Faerie\nPT:1/1\nK:Flying\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Any | Destination$ Battlefield | Execute$ TrigUntap\n\
         SVar:TrigUntap:DB$ Untap | UntapUpTo$ True | UntapType$ Land | Amount$ 2",
    );
    assert!(
        cloud.abilities[0].contains("Effect::UntapChosen { filter: &Filter::LAND, count: 2 }"),
        "{:?}",
        cloud.abilities
    );

    let unwind = read(
        "Name:Unwind\nManaCost:2 U\nTypes:Instant\n\
         A:SP$ Counter | TargetType$ Spell | ValidTgts$ Card.nonCreature | SubAbility$ DBUntap\n\
         SVar:DBUntap:DB$ Untap | UntapUpTo$ True | UntapType$ Land | Amount$ 3",
    );
    assert!(
        unwind.abilities[0].contains("Effect::UntapChosen { filter: &Filter::LAND, count: 3 }"),
        "{:?}",
        unwind.abilities
    );

    let teferi = read(
        "Name:Teferi\nManaCost:3 W U\nTypes:Legendary Planeswalker Teferi\nLoyalty:4\n\
         A:AB$ Draw | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | Defined$ You | NumCards$ 1 | SubAbility$ DelTrigUntap\n\
         SVar:DelTrigUntap:DB$ DelayedTrigger | Mode$ Phase | Phase$ End of Turn | Execute$ TrigUntap\n\
         SVar:TrigUntap:DB$ Untap | UntapUpTo$ True | UntapType$ Land | Amount$ 2 | Defined$ You",
    );
    assert!(
        teferi.abilities[0].contains("Effect::UntapChosen { filter: &Filter::LAND, count: 2 }"),
        "{:?}",
        teferi.abilities
    );

    for bad in [
        "Name:DynAmt\nManaCost:2 G\nTypes:Creature\nPT:2/2\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | TriggerZones$ Battlefield | Execute$ TrigUntap\n\
         SVar:TrigUntap:DB$ Untap | UntapUpTo$ True | UntapType$ Land | Amount$ X\nSVar:X:Count$Valid Creature",
        "Name:OtherPlayer\nManaCost:3 U\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | TriggerZones$ Battlefield | Execute$ TrigUntap\n\
         SVar:TrigUntap:DB$ Untap | UntapUpTo$ True | UntapType$ Land | Amount$ 1 | Defined$ TriggeredPlayer",
        "Name:ZeroUntap\nManaCost:U\nTypes:Instant\n\
         A:SP$ Untap | UntapUpTo$ True | UntapType$ Land | Amount$ 0",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn regenerate_defined_targeted_and_valid_sweep() {
    let necrobite = read(
        "Name:Necrobite\nManaCost:2 B\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | KW$ Deathtouch | SubAbility$ DBRegen\n\
         SVar:DBRegen:DB$ Regenerate | Defined$ Targeted",
    );
    assert!(
        necrobite.abilities[0]
            .contains("Effect::regenerate(TargetSpec::Object(&Filter::CREATURE))"),
        "{:?}",
        necrobite.abilities
    );

    let wrap = read(
        "Name:Wrap in Vigor\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ Regenerate | Defined$ Valid Creature.YouCtrl",
    );
    assert!(
        wrap.abilities[0].contains("Effect::RegenerateAll { filter: &"),
        "{:?}",
        wrap.abilities
    );

    let dabbling = read(
        "Name:Dark Dabbling\nManaCost:2 B\nTypes:Instant\n\
         A:SP$ Regenerate | ValidTgts$ Creature | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | SubAbility$ DBRegenerate\n\
         SVar:DBRegenerate:DB$ Regenerate | Defined$ Valid Creature.YouCtrl+!targetedBy | \
         ConditionPresent$ Instant.YouOwn,Sorcery.YouOwn | ConditionZone$ Graveyard | ConditionCompare$ GE2",
    );
    assert!(
        dabbling.abilities[0].contains("Effect::RegenerateAll { filter: &REGENERATE1 }")
            && dabbling.statics.contains("REGENERATE1")
            && dabbling.statics.contains("Filter::Not(&Filter::Reference)"),
        "{dabbling:?}"
    );

    for bad in [
        "Name:Soldevi\nManaCost:1\nTypes:Artifact Creature Soldier\nPT:1/1\n\
         A:AB$ Regenerate | Cost$ 1 | ValidTgts$ Opponent | Defined$ Self | RegenerationAbility$ DBTrig\n\
         SVar:DBTrig:DB$ Draw | NumCards$ 1",
        "Name:NoPriorTarget\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ Regenerate | Defined$ Valid Creature.YouCtrl+!targetedBy",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}

#[test]
fn tap_card_choices_choose_card_ex_and_capture() {
    let augusta = read(
        "Name:Augusta\nManaCost:2 W\nTypes:Legendary Creature Human Cleric\nPT:1/3\n\
         T:Mode$ AttackersDeclared | Execute$ DBUnTapAll | TriggerZones$ Battlefield | AttackingPlayer$ You\n\
         SVar:DBUnTapAll:DB$ UntapAll | ValidCards$ Creature.YouCtrl | SubAbility$ DBTap\n\
         SVar:DBTap:DB$ Tap | CardChoices$ Creature.YouCtrl+untapped | AnyNumber$ True | ChoiceAmount$ Count$Valid Creature.YouCtrl+untapped",
    );
    assert!(
        augusta.abilities[0].contains("Effect::ChooseCardEx { chooser: PlayerRel::You, zone: ChooseZone::Battlefield, filter: &TAP_CHOICE1, min: 0, max: Amount::Fixed(255), reveal_chosen: false, movement: ChooseMove::None, not_chosen_movement: ChooseMove::None, then: &[Effect::TapEventObject], for_unchosen: &[] }"),
        "{:?}",
        augusta.abilities
    );

    let harmony = read(
        "Name:Harmony of Nature\nManaCost:2 G\nTypes:Sorcery\n\
         A:SP$ Tap | CardChoices$ Creature.YouCtrl+untapped | AnyNumber$ True | ChoiceAmount$ Count$Valid Creature.YouCtrl+untapped | RememberTapped$ True | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | LifeAmount$ Z | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:Y:Remembered$Amount\n\
         SVar:Z:SVar$Y/Times.4",
    );
    assert!(
        harmony.abilities[0].contains("Effect::CaptureAffected { first: &[Effect::ChooseCardEx {")
            && harmony.abilities[0]
                .contains("Amount::Times { base: &Amount::RememberedCount, factor: 4 }"),
        "{:?}",
        harmony.abilities
    );

    let guild_summit = read(
        "Name:Guild Summit\nManaCost:2 U\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Any | Destination$ Battlefield | Execute$ GuestList | TriggerDescription$ When CARDNAME enters, you may tap any number of untapped creatures you control. Draw a card for each creature tapped this way.\n\
         SVar:GuestList:DB$ Tap | CardChoices$ Creature.YouCtrl+untapped | AnyNumber$ True | ChoiceAmount$ Count$Valid Creature.YouCtrl+untapped | RememberTapped$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | NumCards$ Y | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:Y:Remembered$Amount",
    );
    assert!(
        guild_summit.abilities[0]
            .contains("Effect::CaptureAffected { first: &[Effect::ChooseCardEx {")
            && guild_summit.abilities[0]
                .contains("Effect::DrawCards { amount: Amount::RememberedCount }"),
        "{:?}",
        guild_summit.abilities
    );

    let urge = read(
        "Name:Urge to Feed\nManaCost:B B\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | NumAtt$ -3 | NumDef$ -3 | IsCurse$ True | SubAbility$ VampiricUrge\n\
         SVar:VampiricUrge:DB$ Tap | CardChoices$ Creature.YouCtrl+untapped | AnyNumber$ True | ChoiceAmount$ Count$Valid Creature.YouCtrl+untapped | RememberTapped$ True | SubAbility$ VampiricFeed\n\
         SVar:VampiricFeed:DB$ PutCounter | CounterType$ P1P1 | CounterNum$ 1 | Defined$ Remembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        urge.abilities[0].contains("Effect::PumpTarget {")
            && urge.abilities[0].contains("Effect::CaptureAffected { first: &[Effect::ChooseCardEx {")
            && urge.abilities[0].contains("Effect::AddCounterFilter { filter: &Filter::IsRemembered, kind: CounterKind::P1P1, amount: Amount::Fixed(1) }"),
        "{:?}",
        urge.abilities
    );

    for bad in [
        "Name:MismatchedChoiceAmount\nManaCost:1 W\nTypes:Sorcery\n\
         A:SP$ Tap | CardChoices$ Creature.YouCtrl+untapped | AnyNumber$ True | ChoiceAmount$ Count$Valid Artifact.YouCtrl+untapped",
        "Name:FixedChoiceAmount\nManaCost:1 W\nTypes:Sorcery\n\
         A:SP$ Tap | CardChoices$ Creature.YouCtrl+untapped | ChoiceAmount$ 2",
        "Name:TargetedCardChoices\nManaCost:1 W\nTypes:Sorcery\n\
         A:SP$ Tap | CardChoices$ Creature.YouCtrl+untapped+!targetedBy | AnyNumber$ True",
    ] {
        assert!(refused(bad), "must refuse: {bad}");
    }
}
