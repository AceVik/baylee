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
    assert!(refused("Name:X\nTypes:Instant\nK:MayFlashCost:2"));
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

    // Unsupported alternate modes stay refused.
    let flip = parse(
        "Name:Top\nManaCost:1 W\nTypes:Creature Human\nK:Flying\n\
         AlternateMode:Flip\n\
         ALTERNATE\n\
         Name:Bottom\nTypes:Legendary Creature Human\nK:Vigilance",
    );
    assert_eq!(
        refusal_reason(&flip, &cats(), Some(&tokens())).as_deref(),
        Some("unsupported alternate mode `Flip`")
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
         K:etbCounter:P1P1:1:Adamant$ White:CARDNAME enters with a counter if adamant.",
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
