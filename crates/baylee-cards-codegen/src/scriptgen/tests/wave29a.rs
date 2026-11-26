use super::*;

fn cats_w29a() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Advisor".into(),
            "Azra".into(),
            "Citizen".into(),
            "Cleric".into(),
            "Demon".into(),
            "Dragon".into(),
            "Drake".into(),
            "Frog".into(),
            "Goblin".into(),
            "Hero".into(),
            "Hound".into(),
            "Human".into(),
            "Hydra".into(),
            "Knight".into(),
            "Mercenary".into(),
            "Nightmare".into(),
            "Rogue".into(),
            "Spirit".into(),
            "Time Lord".into(),
            "Vampire".into(),
            "Warlock".into(),
            "Warrior".into(),
            "Wizard".into(),
            "Wurm".into(),
            "Zombie".into(),
        ],
        artifact: vec![
            "Clue".into(),
            "Equipment".into(),
            "Food".into(),
            "Vehicle".into(),
        ],
        enchantment: vec!["Aura".into(), "Saga".into()],
        land: vec![
            "Forest".into(),
            "Island".into(),
            "Mountain".into(),
            "Plains".into(),
            "Swamp".into(),
        ],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

fn read_w29a(text: &str) -> CardBody {
    let parsed = parse(text);
    transcode(&parsed, &cats_w29a(), None).unwrap_or_else(|| {
        panic!(
            "should be read in full, refused: {:?}\n{text}",
            refusal_reason(&parsed, &cats_w29a(), None)
        )
    })
}

fn refused_w29a(text: &str) -> bool {
    transcode(&parse(text), &cats_w29a(), None).is_none()
}

#[test]
fn necropotence_stub_exile_discard_and_top_face_down_end_step() {
    let body = read_w29a(
        "Name:Necropotence\n\
         ManaCost:B B B\n\
         Types:Enchantment\n\
         R:Event$ BeginPhase | ActiveZones$ Battlefield | ValidPlayer$ You | Phase$ Draw | Skip$ True | Description$ Skip your draw step.\n\
         T:Mode$ Discarded | ValidCard$ Card.YouCtrl | TriggerZones$ Battlefield | Execute$ TrigChange | TriggerDescription$ Whenever you discard a card, exile that card from your graveyard.\n\
         SVar:TrigChange:DB$ ChangeZone | Defined$ TriggeredCard | Origin$ Graveyard | Destination$ Exile\n\
         A:AB$ ChangeZone | Cost$ PayLife<1> | Defined$ TopOfLibrary | Origin$ Library | Destination$ Exile | ExileFaceDown$ True | RememberChanged$ True | SubAbility$ DelayedReturn | AILogic$ Necropotence | AILifeThreshold$ 1 | SpellDescription$ Exile the top card of your library face down. Put that card into your hand at the beginning of your next end step.\n\
         SVar:DelayedReturn:DB$ DelayedTrigger | Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | Execute$ TrigReturn | RememberObjects$ RememberedLKI | SubAbility$ DBCleanup\n\
         SVar:TrigReturn:DB$ ChangeZone | Origin$ Exile | Destination$ Hand | Defined$ DelayTriggerRememberedLKI\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:Skip your draw step.\\nWhenever you discard a card, exile that card from your graveyard.\\nPay 1 life: Exile the top card of your library face down. Put that card into your hand at the beginning of your next end step.",
    );
    let joined = body.abilities.join("\n");
    assert!(
        joined.contains("Effect::ExileDiscardedCardFromYourGraveyard"),
        "{joined}"
    );
    assert!(
        joined.contains("Effect::ExileTopFaceDownUntilYourNextEndStep"),
        "{joined}"
    );
}

#[test]
fn purgatory_stub_dies_exile_linked_and_upkeep_return() {
    let body = read_w29a(
        "Name:Purgatory\n\
         ManaCost:2 W B\n\
         Types:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Creature.!token+YouOwn | Execute$ TrigChangeZone | TriggerZones$ Battlefield | TriggerDescription$ Whenever a nontoken creature is put into your graveyard from the battlefield, exile that card.\n\
         SVar:TrigChangeZone:DB$ ChangeZone | Defined$ TriggeredNewCardLKICopy | Origin$ Graveyard | Destination$ Exile\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigReturn | TriggerDescription$ At the beginning of your upkeep, you may pay {4} and 2 life. If you do, return a card exiled with CARDNAME to the battlefield.\n\
         SVar:TrigReturn:AB$ ChangeZone | Cost$ 4 PayLife<2> | ChangeType$ Card.ExiledWithSource | Mandatory$ True | Origin$ Exile | Destination$ Battlefield | Hidden$ True\n\
         Oracle:Whenever a nontoken creature is put into your graveyard from the battlefield, exile that card.\\nAt the beginning of your upkeep, you may pay {4} and 2 life. If you do, return a card exiled with Purgatory to the battlefield.",
    );
    let joined = body.abilities.join("\n");
    assert!(
        joined.contains("Effect::exile_linked(TargetSpec::EventObject)"),
        "{joined}"
    );
    assert!(joined.contains("ChooseZone::LinkedExile"), "{joined}");
}

#[test]
fn dwell_on_the_past_and_krosan_reclamation_stubs_shuffle_targets() {
    let body = read_w29a(
        "Name:Dwell on the Past\n\
         ManaCost:G\n\
         Types:Sorcery\n\
         A:SP$ TargetOnly | ValidTgts$ Player | SubAbility$ DBChangeZone | IsCurse$ True | SpellDescription$ Target player shuffles up to four target cards from their graveyard into their library.\n\
         SVar:DBChangeZone:DB$ ChangeZone | TargetMin$ 0 | TargetMax$ 4 | TargetsWithDefinedController$ ParentTarget | Origin$ Graveyard | Destination$ Library | Shuffle$ True | ValidTgts$ Card\n\
         Oracle:Target player shuffles up to four target cards from their graveyard into their library.",
    );
    let joined = body.abilities.join("\n");
    assert!(
        joined.contains("Effect::PutOnBottomOfLibraryFromGraveyard"),
        "{joined}"
    );
    assert!(
        joined.contains("Effect::ShuffleLibrary { who: PlayerRel::FirstTargetPlayerOrController }"),
        "{joined}"
    );
    assert!(!joined.contains("Effect::WithSecondTarget"), "{joined}");
}

#[test]
fn skyship_weatherlight_and_negative_zone_portal_linked_exile() {
    let body = read_w29a(
        "Name:Skyship Weatherlight\n\
         ManaCost:4\n\
         Types:Legendary Artifact\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigSearch | TriggerDescription$ When CARDNAME enters, search your library for any number of artifact and/or creature cards, exile them, then shuffle.\n\
         SVar:TrigSearch:DB$ ChangeZone | ChangeType$ Creature.YouOwn,Artifact.YouOwn | ChangeNum$ X | Origin$ Library | Destination$ Exile\n\
         A:AB$ ChangeZone | Cost$ 4 T | Hidden$ True | Origin$ Exile | Destination$ Hand | AtRandom$ True | ChangeType$ Card.ExiledWithSource | SpellDescription$ Choose a card at random that was exiled with CARDNAME. Put that card into its owner's hand.\n\
         SVar:X:Count$ValidLibrary Creature.YouOwn,Artifact.YouOwn\n\
         Oracle:When Skyship Weatherlight enters, search your library for any number of artifact and/or creature cards, exile them, then shuffle.\\n{4}, {T}: Choose a card at random that was exiled with Skyship Weatherlight. Put that card into its owner's hand.",
    );
    let joined = body.abilities.join("\n");
    assert!(joined.contains("ChooseZone::LinkedExile"), "{joined}");
    assert!(joined.contains("at_random: true"), "{joined}");

    let portal = read_w29a(
        "Name:Negative Zone Portal\n\
         ManaCost:2\n\
         Types:Artifact\n\
         A:AB$ ChangeZone | Cost$ 2 T | Origin$ Graveyard | Destination$ Exile | TgtPrompt$ Select target card in an opponent's graveyard | ValidTgts$ Card.OppCtrl | SubAbility$ DBDraw | SpellDescription$ Exile target card from an opponent's graveyard. If it's a creature card, draw a card.\n\
         SVar:DBDraw:DB$ Draw | Defined$ You | ConditionDefined$ Targeted | ConditionPresent$ Creature | ConditionCompare$ EQ1\n\
         SVar:X:Count$ValidExile Creature.ExiledWithSource\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | CheckSVar$ X | SVarCompare$ GE4 | Execute$ TrigFlip | TriggerZones$ Battlefield | TriggerDescription$ At the beginning of your upkeep, if there are four or more creature cards exiled with this artifact, flip a coin. If you lose the flip, sacrifice this artifact and return a card exiled with it at random to its owner's hand.\n\
         SVar:TrigFlip:DB$ FlipCoin | LoseSubAbility$ DBSacrifice\n\
         SVar:DBSacrifice:DB$ Sacrifice | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZone | Origin$ Exile | Destination$ Hand | AtRandom$ True | Hidden$ True | ChangeType$ Card.ExiledWithSource | ChangeNum$ 1\n\
         Oracle:{2}, {T}: Exile target card from an opponent's graveyard. If it's a creature card, draw a card.\\nAt the beginning of your upkeep, if there are four or more creature cards exiled with this artifact, flip a coin. If you lose the flip, sacrifice this artifact and return a card exiled with it at random to its owner's hand.",
    );
    let p_joined = portal.abilities.join("\n");
    assert!(p_joined.contains("Effect::exile_linked("), "{p_joined}");
    assert!(p_joined.contains("ChooseZone::LinkedExile"), "{p_joined}");
}

#[test]
fn nurgles_conscription_exiles_owner_of_graveyard_target_graveyard() {
    let body = read_w29a(
        "Name:Nurgle's Conscription\n\
         ManaCost:3 B B\n\
         Types:Instant\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | GainControl$ True | ValidTgts$ Creature.OppOwn | SubAbility$ DBExileGrave\n\
         SVar:DBExileGrave:DB$ ChangeZoneAll | Origin$ Graveyard | Destination$ Exile | Defined$ TargetedOwner\n\
         Oracle:Put target creature card from an opponent's graveyard onto the battlefield under your control, then exile that player's graveyard.",
    );
    let joined = body.abilities.join("\n");
    assert!(
        joined.contains("Effect::ExileGraveyard { player: PlayerRel::OwnerOfTarget }"),
        "{joined}"
    );
}

#[test]
fn harvest_wurm_stub_optional_return_basic_land_or_sacrifice() {
    let body = read_w29a(
        "Name:Harvest Wurm\n\
         ManaCost:1 G\n\
         Types:Creature Wurm\n\
         PT:3/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigChangeZone | TriggerDescription$ When CARDNAME enters, sacrifice it unless you return a basic land card from your graveyard to your hand.\n\
         SVar:TrigChangeZone:DB$ ChangeZone | Origin$ Graveyard | Destination$ Hand | Hidden$ True | ChangeType$ Land.Basic+YouOwn | RememberChanged$ True | SubAbility$ DBSac\n\
         SVar:DBSac:DB$ Sacrifice | SubAbility$ DBCleanup | ConditionCheckSVar$ X | ConditionSVarCompare$ LT1\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount\n\
         Oracle:When Harvest Wurm enters, sacrifice it unless you return a basic land card from your graveyard to your hand.",
    );
    let joined = body.abilities.join("\n");
    assert!(joined.contains("Effect::CaptureAffected"), "{joined}");
    assert!(joined.contains("optional: true"), "{joined}");
    assert!(joined.contains("Effect::SacrificeSelf"), "{joined}");
}

#[test]
fn yuna_and_fenrir_next_creature_spell_enters_with_p1p1() {
    let body = read_w29a(
        "Name:Yuna, Grand Summoner\n\
         ManaCost:1 G W U\n\
         Types:Legendary Creature Human Cleric\n\
         PT:1/5\n\
         A:AB$ Mana | Cost$ T | Produced$ Any | SubAbility$ DBEffectAddCounter | SpellDescription$ Add one mana of any color. When you next cast a creature spell this turn, that creature enters with two additional +1/+1 counters on it.\n\
         SVar:DBEffectAddCounter:DB$ Effect | Triggers$ TrigSpellCast\n\
         SVar:TrigSpellCast:Mode$ SpellCast | ValidCard$ Creature | ValidActivatingPlayer$ You | OneOff$ True | Execute$ EffAddCounter\n\
         SVar:EffAddCounter:DB$ Effect | Duration$ UntilEndOfTurn | RememberObjects$ TriggeredSpellAbility | ReplacementEffects$ RepETB\n\
         SVar:RepETB:Event$ Moved | Destination$ Battlefield | ValidCard$ Card.IsRemembered | ReplaceWith$ PutP1P1\n\
         SVar:PutP1P1:DB$ PutCounter | Defined$ ReplacedCard | CounterType$ P1P1 | CounterNum$ 2 | ETB$ True | SubAbility$ ExileEff\n\
         SVar:ExileEff:DB$ ChangeZone | Defined$ Self | Origin$ Command | Destination$ Exile\n\
         Oracle:{T}: Add one mana of any color. When you next cast a creature spell this turn, that creature enters with two additional +1/+1 counters on it.",
    );
    let joined = body.abilities.join("\n");
    assert!(
        joined.contains("Effect::WhenNextSpellCastThisTurn"),
        "{joined}"
    );
    assert!(
        joined.contains("Effect::EventSpellEntersWithP1P1 { n: 2 }"),
        "{joined}"
    );
}

#[test]
fn restore_the_peace_rivers_rebuke_cerberus_hubris_and_essence_reliquary() {
    let restore = read_w29a(
        "Name:Restore the Peace\n\
         ManaCost:1 W U\n\
         Types:Instant\n\
         A:SP$ ChangeZoneAll | ChangeType$ Creature.dealtDamageThisTurn | Origin$ Battlefield | Destination$ Hand | SpellDescription$ Return each creature that dealt damage this turn to its owner's hand.\n\
         Oracle:Return each creature that dealt damage this turn to its owner's hand.",
    );
    assert!(restore.statics.contains("Filter::DealtDamageThisTurn"));

    let rebuke = read_w29a(
        "Name:River's Rebuke\n\
         ManaCost:4 U U\n\
         Types:Sorcery\n\
         A:SP$ ChangeZoneAll | ValidTgts$ Player | ChangeType$ Permanent.nonLand | Origin$ Battlefield | Destination$ Hand | SpellDescription$ Return all nonland permanents target player controls to their owners' hands.\n\
         Oracle:Return all nonland permanents target player controls to their owners' hands.",
    );
    assert!(
        rebuke
            .statics
            .contains("Filter::ControlledBy(PlayerRel::Chosen)")
    );

    let cerberus = read_w29a(
        "Name:Underworld Cerberus\n\
         ManaCost:3 B R\n\
         Types:Creature Hound\n\
         PT:6/6\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigExile | TriggerDescription$ When CARDNAME dies, exile it and each player returns all creature cards from their graveyard to their hand.\n\
         SVar:TrigExile:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | Defined$ TriggeredNewCardLKICopy | SubAbility$ DBReturnAll\n\
         SVar:DBReturnAll:DB$ ChangeZoneAll | UseAllOriginZones$ True | Origin$ Graveyard | Destination$ Hand | ChangeType$ Creature\n\
         Oracle:When Underworld Cerberus dies, exile it and each player returns all creature cards from their graveyard to their hand.",
    );
    assert!(
        cerberus
            .abilities
            .join("\n")
            .contains("ChooseZone::Graveyard(PlayerRel::EachPlayer)")
    );

    let hubris = read_w29a(
        "Name:Hubris\n\
         ManaCost:1 U\n\
         Types:Instant\n\
         A:SP$ ChangeZoneAll | ValidTgts$ Creature | ChangeType$ TargetedCard.Self,Aura.AttachedTo Targeted | Origin$ Battlefield | Destination$ Hand | UseAllOriginZones$ True | SpellDescription$ Return target creature and all Auras attached to it to their owners' hands.\n\
         Oracle:Return target creature and all Auras attached to it to their owners' hands.",
    );
    assert!(
        hubris
            .statics
            .contains("Filter::AttachedTo(&Filter::Reference)"),
        "{}",
        hubris.statics
    );

    let reliquary = read_w29a(
        "Name:Essence Reliquary\n\
         ManaCost:2 W\n\
         Types:Artifact\n\
         A:AB$ ChangeZoneAll | Cost$ T | Origin$ Battlefield | ValidTgts$ Permanent.YouCtrl+Other | ChangeType$ TargetedCard.Self,Aura.YouCtrl+AttachedTo Targeted | UseAllOriginZones$ True | Destination$ Hand | PlayerTurn$ True\n\
         Oracle:{T}: Return another target permanent you control and all Auras you control attached to it to their owner's hand. Activate only during your turn.",
    );
    assert!(
        reliquary
            .statics
            .contains("Filter::AttachedTo(&Filter::Reference)"),
        "{}",
        reliquary.statics
    );
}

#[test]
fn pass2_stubs_lat_nams_legacy_surprise_deployment_and_wormfang_drake() {
    let legacy = read_w29a(
        "Name:Lat-Nam's Legacy\n\
         ManaCost:1 U\n\
         Types:Instant\n\
         A:SP$ ChangeZone | Origin$ Hand | Destination$ Library | Shuffle$ True | ChangeType$ Card | RememberChanged$ True | SubAbility$ DelTrigSlowtrip | SpellDescription$ Shuffle a card from your hand into your library. If you do, draw two cards at the beginning of the next turn's upkeep.\n\
         SVar:DelTrigSlowtrip:DB$ DelayedTrigger | NextTurn$ True | Mode$ Phase | Phase$ Upkeep | ValidPlayer$ Player | Execute$ DrawSlowtrip\n\
         SVar:DrawSlowtrip:DB$ Draw | NumCards$ 2 | Defined$ You\n\
         Oracle:Shuffle a card from your hand into your library. If you do, draw two cards at the beginning of the next turn's upkeep.",
    );
    let l_joined = legacy.abilities.join("\n");
    assert!(
        l_joined.contains("Effect::ShuffleLibrary { who: PlayerRel::You }"),
        "{l_joined}"
    );
    assert!(l_joined.contains("Effect::AtNextTurnsUpkeep"), "{l_joined}");

    let surprise = read_w29a(
        "Name:Surprise Deployment\n\
         ManaCost:3 W\n\
         Types:Instant\n\
         A:SP$ ChangeZone | Origin$ Hand | Destination$ Battlefield | Optional$ You | ChangeType$ Creature.nonWhite | ChangeNum$ 1 | RememberChanged$ True | ActivationPhases$ BeginCombat->EndCombat | SubAbility$ DelTrig\n\
         SVar:DelTrig:DB$ DelayedTrigger | Mode$ Phase | Phase$ End Of Turn | Execute$ TrigBounce | RememberObjects$ RememberedLKI | SubAbility$ DBCleanup\n\
         SVar:TrigBounce:DB$ ChangeZone | Defined$ DelayTriggerRememberedLKI | Origin$ Battlefield | Destination$ Hand\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:Cast this spell only during combat.\\nYou may put a nonwhite creature card from your hand onto the battlefield. At the beginning of the next end step, return that creature to your hand.",
    );
    let s_joined = surprise.abilities.join("\n");
    assert!(
        s_joined.contains("Effect::PutFromHandOntoBattlefield"),
        "{s_joined}"
    );
    assert!(s_joined.contains("DelayedStep::EndStep"), "{s_joined}");
    assert!(
        s_joined.contains("Effect::ReturnToHand { target: TargetSpec::EventObject }"),
        "{s_joined}"
    );

    let drake = read_w29a(
        "Name:Wormfang Drake\n\
         ManaCost:2 U\n\
         Types:Creature Nightmare Drake\n\
         PT:3/4\n\
         K:Flying\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ ETBExileAbility | TriggerDescription$ When CARDNAME enters, sacrifice it unless you exile a creature you control other than CARDNAME.\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Any | ValidCard$ Card.Self | Execute$ ETBExileReturn | TriggerDescription$ When CARDNAME leaves the battlefield, return the exiled card to the battlefield under its owner's control.\n\
         SVar:ETBExileAbility:DB$ ChangeZone | Origin$ Battlefield | Destination$ Exile | RememberChanged$ True | Hidden$ True | Optional$ True | SubAbility$ ETBExileSacrifice | ChangeType$ Creature.Other+YouCtrl\n\
         SVar:ETBExileSacrifice:DB$ Sacrifice | Defined$ Card.Self | ConditionDefined$ Remembered | ConditionPresent$ Card | ConditionCompare$ EQ0\n\
         SVar:ETBExileReturn:DB$ ChangeZone | Defined$ Remembered | Origin$ Exile | Destination$ Battlefield | SubAbility$ ETBExileCleanup\n\
         SVar:ETBExileCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:Flying\\nWhen Wormfang Drake enters, sacrifice it unless you exile a creature you control other than Wormfang Drake.\\nWhen Wormfang Drake leaves the battlefield, return the exiled card to the battlefield under its owner's control.",
    );
    let d_joined = drake.abilities.join("\n");
    assert!(
        d_joined.contains("Effect::Champion { filter: &Filter::CREATURE }"),
        "{d_joined}"
    );
    assert!(d_joined.contains("Effect::ReturnChampioned"), "{d_joined}");
}

#[test]
#[allow(clippy::too_many_lines)]
fn pass2_capture_exile_play_and_remembered_followups() {
    let hex = read_w29a(
        "Name:Hex Magic\n\
         ManaCost:2 R\n\
         Types:Sorcery\n\
         A:SP$ ChangeZoneAll | Origin$ Hand | Destination$ Exile | ChangeType$ Card.YouOwn | RememberChanged$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ X | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ RememberedCard | StaticAbilities$ STPlay | SubAbility$ DBCleanup | ForgetOnMoved$ Exile | Duration$ UntilTheEndOfYourNextTurn\n\
         SVar:STPlay:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount\n\
         Oracle:Exile all the cards from your hand, then draw that many cards. Until the end of your next turn, you may play cards exiled this way.",
    );
    assert!(
        hex.abilities[0].contains("Effect::ExileWithPermission"),
        "{}",
        hex.abilities[0]
    );

    let ignite = read_w29a(
        "Name:Ignite the Future\n\
         ManaCost:3 R\n\
         Types:Sorcery\n\
         A:SP$ Dig | Defined$ You | DigNum$ 3 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ RememberedCard | StaticAbilities$ Play | ForgetOnMoved$ Exile | Duration$ UntilTheEndOfYourNextTurn | SubAbility$ DBEffect2\n\
         SVar:Play:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile\n\
         SVar:DBEffect2:DB$ Effect | RememberObjects$ RememberedCard | StaticAbilities$ Play2 | ForgetOnMoved$ Exile | Duration$ UntilTheEndOfYourNextTurn | ConditionDefined$ Self | ConditionPresent$ Card.wasCastFromGraveyard | ConditionCompare$ EQ1 | SubAbility$ DBCleanup\n\
         SVar:Play2:Mode$ Continuous | MayPlay$ True | MayPlayWithoutManaCost$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         K:Flashback:7 R\n\
         Oracle:Exile the top three cards of your library. Until the end of your next turn, you may play those cards. If this spell was cast from a graveyard, you may play cards this way without paying their mana costs.\\nFlashback {7}{R}",
    );
    assert!(
        ignite.abilities[0].contains("Effect::IfCastFromGraveyard"),
        "{}",
        ignite.abilities[0]
    );

    let anchor = read_w29a(
        "Name:Anchor to Reality\n\
         ManaCost:2 U U\n\
         Types:Sorcery\n\
         A:SP$ ChangeZone | Cost$ 2 U U Sac<1/Artifact;Creature/artifact or creature> | Origin$ Library | Destination$ Battlefield | ChangeType$ Equipment,Vehicle | RememberChanged$ True | SubAbility$ DBScry\n\
         SVar:DBScry:DB$ Scry | ConditionDefined$ Remembered | ConditionPresent$ Card | ScryNum$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$Compare Y LTZ.2.0\n\
         SVar:Y:Remembered$CardManaCost\n\
         SVar:Z:Sacrificed$CardManaCost\n\
         Oracle:As an additional cost to cast this spell, sacrifice an artifact or creature.\\nSearch your library for an Equipment or Vehicle card, put that card onto the battlefield, then shuffle. If it has mana value less than the sacrificed permanent's mana value, scry 2.",
    );
    assert!(
        anchor.abilities[0].contains("Amount::SacrificedManaValue"),
        "{}",
        anchor.abilities[0]
    );
    assert!(
        anchor.abilities[0].contains("AmountCmp::LessThan"),
        "{}",
        anchor.abilities[0]
    );

    let offspring = read_w29a(
        "Name:Offspring's Revenge\n\
         ManaCost:2 R W B\n\
         Types:Enchantment\n\
         T:Mode$ Phase | Phase$ BeginCombat | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigExile | TriggerDescription$ At the beginning of combat on your turn, exile target red, white, or black creature card from your graveyard. Create a token that's a copy of that card, except it's 1/1. It gains haste until your next turn.\n\
         SVar:TrigExile:DB$ ChangeZone | ValidTgts$ Creature.Red+YouCtrl,Creature.White+YouCtrl,Creature.Black+YouCtrl | Origin$ Graveyard | Destination$ Exile | RememberChanged$ True | SubAbility$ DBCopy\n\
         SVar:DBCopy:DB$ CopyPermanent | Defined$ Remembered | SetPower$ 1 | SetToughness$ 1 | PumpKeywords$ Haste | PumpDuration$ UntilYourNextTurn | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:At the beginning of combat on your turn, exile target red, white, or black creature card from your graveyard. Create a token that's a copy of that card, except it's 1/1. It gains haste until your next turn.",
    );
    assert!(
        offspring.abilities[0].contains("Duration::UntilYourNextTurn"),
        "{}",
        offspring.abilities[0]
    );

    let gyrus = read_w29a(
        "Name:Gyrus, Waker of Corpses\n\
         ManaCost:X B R G\n\
         Types:Legendary Creature Hydra\n\
         PT:0/0\n\
         K:etbCounter:P1P1:Y:no Condition:CARDNAME enters with a number of +1/+1 counters on it equal to the amount of mana spent to cast it.\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigExile | TriggerZones$ Battlefield | OptionalDecider$ You | TriggerDescription$ Whenever NICKNAME attacks, you may exile target creature card with lesser power from your graveyard. If you do, create a token that's a copy of that card and that's tapped and attacking. Exile the token at the end of combat.\n\
         SVar:TrigExile:DB$ ChangeZone | ValidTgts$ Creature.YouCtrl+powerLTZ | Origin$ Graveyard | Destination$ Exile | RememberChanged$ True | Optional$ True | SubAbility$ DBCopy\n\
         SVar:DBCopy:DB$ CopyPermanent | Defined$ Remembered | TokenTapped$ True | TokenAttacking$ True | AtEOT$ ExileCombat | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$xPaid\n\
         SVar:Y:Count$CastTotalManaSpent\n\
         SVar:Z:Count$CardPower\n\
         Oracle:Gyrus, Waker of Corpses enters with a number of +1/+1 counters on it equal to the amount of mana spent to cast it.\\nWhenever Gyrus attacks, you may exile target creature card with lesser power from your graveyard. If you do, create a token that's a copy of that card and that's tapped and attacking. Exile the token at end of combat.",
    );
    let g_joined = gyrus.abilities.join("\n");
    assert_eq!(g_joined.matches("Effect::MayDo").count(), 1, "{g_joined}");
    assert!(
        g_joined.contains("CopyCleanup::ExileAtEndOfCombat"),
        "{g_joined}"
    );

    let harvester = read_w29a(
        "Name:Abyssal Harvester\n\
         ManaCost:1 B B\n\
         Types:Creature Demon Warlock\n\
         PT:3/2\n\
         A:AB$ ChangeZone | Cost$ T | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Creature.ThisTurnEntered | RememberChanged$ True | SubAbility$ DBCopy\n\
         SVar:DBCopy:DB$ CopyPermanent | Defined$ Remembered | Controller$ You | AddTypes$ Nightmare | RememberTokens$ True | SubAbility$ DBChangeZoneAll\n\
         SVar:DBChangeZoneAll:DB$ ChangeZoneAll | ChangeType$ Nightmare.token+!IsRemembered+YouCtrl | Origin$ Battlefield | Destination$ Exile | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:{T}: Exile target creature card from a graveyard that was put there this turn. Create a token that's a copy of it, except it's a Nightmare in addition to its other types. Then exile all other Nightmare tokens you control.",
    );
    assert!(
        harvester.abilities[0].contains("Effect::ExileAll"),
        "{}",
        harvester.abilities[0]
    );

    let afterlife = read_w29a(
        "Name:Afterlife from the Loam\n\
         ManaCost:5 B B B\n\
         Types:Sorcery\n\
         K:Delve\n\
         A:SP$ ChangeZone | Origin$ Graveyard | Destination$ Battlefield | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ OneEach | TargetsForEachPlayer$ True | GainControl$ True | TgtPrompt$ For each player, choose up to one target creature card in that player's graveyard | StaticEffect$ Animate\n\
         SVar:Animate:Mode$ Continuous | Affected$ Card.IsRemembered | AddType$ Zombie\n\
         SVar:OneEach:PlayerCountPlayers$Amount\n\
         Oracle:Delve (Each card you exile from your graveyard while casting this spell pays for {1}.)\\nFor each player, choose up to one target creature card in that player's graveyard. Put those cards onto the battlefield under your control. They're Zombies in addition to their other types.",
    );
    assert!(
        afterlife.abilities[0].contains("TargetConstraint::OnePerPlayer"),
        "{}",
        afterlife.abilities[0]
    );
}

#[test]
fn honest_refusals_preserved_for_sideboard_wish_opponent_emblem_and_dynamic_up_to() {
    // Pure `Origin$ Sideboard` (without face-up exile clause) must NOT emit `WishToHand`.
    assert!(refused_w29a(
        "Name:Burning Wish\n\
         ManaCost:1 R\n\
         Types:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Sideboard | Destination$ Hand | ChangeType$ Sorcery.YouOwn | ChangeNum$ 1 | SubAbility$ DBExileSelf\n\
         SVar:DBExileSelf:DB$ ChangeZone | Origin$ Stack | Destination$ Exile | Defined$ Self\n\
         Oracle:You may reveal a sorcery card you own from outside the game and put it into your hand. Exile Burning Wish.",
    ));

    // Opponent-owned emblem must NOT emit `Effect::CreateEmblem` under `You`.
    assert!(refused_w29a(
        "Name:Chandra, Awakened Inferno\n\
         ManaCost:4 R R\n\
         Types:Legendary Planeswalker Chandra\n\
         Loyalty:6\n\
         A:AB$ Effect | Cost$ AddCounter<2/LOYALTY> | Planeswalker$ True | EffectOwner$ Player.Opponent | Name$ Emblem — Chandra | Triggers$ BOTTrig | Duration$ Permanent\n\
         SVar:BOTTrig:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | TriggerZones$ Command | Execute$ ChandraDmg\n\
         SVar:ChandraDmg:DB$ DealDamage | Defined$ TriggeredPlayer | NumDmg$ 1\n\
         Oracle:[+2]: Each opponent gets an emblem with \"At the beginning of your upkeep, this emblem deals 1 damage to you.\"",
    ));

    // Dynamic `TargetMin$ 0 | TargetMax$ X` (`Count$xPaid`) emits `TargetReq::up_to_x(...)`.
    let dynamic_up_to = read_w29a(
        "Name:Dynamic Up To\n\
         ManaCost:X U\n\
         Types:Sorcery\n\
         A:SP$ ChangeZone | Origin$ Battlefield | Destination$ Hand | ValidTgts$ Creature | TargetMin$ 0 | TargetMax$ X\n\
         SVar:X:Count$xPaid\n\
         Oracle:Return up to X target creatures to their owners' hands.",
    );
    assert!(
        dynamic_up_to.abilities[0]
            .contains("TargetReq::up_to_x(TargetSpec::Object(&Filter::CREATURE))"),
        "{}",
        dynamic_up_to.abilities[0]
    );

    // `MayPlayLimit$ 1` turn-long exile cast permission must NOT emit unlimited `GrantLinkedExilePermission`.
    assert!(refused_w29a(
        "Name:Serpent's Soul-Jar\n\
         ManaCost:2 B\n\
         Types:Artifact\n\
         A:AB$ Effect | Cost$ T PayLife<2> | StaticAbilities$ MayCast | Duration$ UntilEndOfTurn\n\
         SVar:MayCast:Mode$ Continuous | Affected$ Creature.ExiledWithEffectSource+nonLand | AffectedZone$ Exile | MayPlay$ True | MayPlayLimit$ 1\n\
         Oracle:{T}, Pay 2 life: Until end of turn, you may cast a creature spell from among cards exiled with Serpent's Soul-Jar.",
    ));

    // Multi-card `MayPlayLimit$ 1` inside `CaptureAffected::then` (`Flameskull`) must NOT grant per-card play on all cards.
    assert!(refused_w29a(
        "Name:Flameskull\n\
         ManaCost:1 R R\n\
         Types:Creature Skeleton\n\
         PT:3/1\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Battlefield | Destination$ Graveyard | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ ChangeZone | Defined$ TriggeredNewCardLKICopy | Origin$ Graveyard | Destination$ Exile | RememberChanged$ True | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ Dig | DigNum$ 1 | ConditionDefined$ Remembered | ConditionPresent$ Card.Self | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ RememberedCard | StaticAbilities$ MayPlay | Duration$ UntilTheEndOfYourNextTurn | ForgetOnMoved$ Exile | SubAbility$ DBCleanup\n\
         SVar:MayPlay:Mode$ Continuous | Affected$ Card.IsRemembered | MayPlay$ True | MayPlayLimit$ 1 | AffectedZone$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:When Flameskull dies, exile it. If you do, exile the top card of your library. Until the end of your next turn, you may play one of those cards.",
    ));

    // Battlefield -> Exile -> `Clone | Defined$ Remembered` must refuse (would copy printed card instead of battlefield LKI).
    assert!(refused_w29a(
        "Name:BadBattlefieldClone\n\
         ManaCost:2 U\n\
         Types:Creature Rogue\n\
         PT:2/2\n\
         A:AB$ ChangeZone | Cost$ T | Origin$ Battlefield | Destination$ Exile | ValidTgts$ Creature | RememberChanged$ True | SubAbility$ DBCopy\n\
         SVar:DBCopy:DB$ Clone | Defined$ Remembered | CloneTarget$ Self | Duration$ UntilEndOfTurn | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:{T}: Exile target creature. This creature becomes a copy of it until end of turn.",
    ));
}

#[test]
fn pass3_lazav_endless_horizons_jacob_frye_and_multi_exile_condition() {
    let lazav = read_w29a(
        "Name:Lazav, Familiar Stranger\n\
         ManaCost:1 U B\n\
         Types:Legendary Creature Rogue\n\
         PT:1/4\n\
         T:Mode$ CommitCrime | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigPutCounter | ActivationLimit$ 1\n\
         SVar:TrigPutCounter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1 | SubAbility$ DBExile\n\
         SVar:DBExile:DB$ ChangeZone | ChangeType$ Card | Origin$ Graveyard | Optional$ True | Hidden$ True | Destination$ Exile | RememberChanged$ True | SubAbility$ DBCopy\n\
         SVar:DBCopy:DB$ Clone | Defined$ Remembered.Creature | CloneTarget$ Self | Optional$ True | Duration$ UntilEndOfTurn | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:Whenever you commit a crime, put a +1/+1 counter on Lazav, Familiar Stranger. Then you may exile a card from a graveyard. If a creature card was exiled this way, you may have Lazav become a copy of that card until end of turn. This ability triggers only once each turn.",
    );
    let l_joined = lazav.abilities.join("\n");
    assert!(l_joined.contains("Effect::BecomeCopy"), "{l_joined}");
    assert!(l_joined.contains("until_eot: true"), "{l_joined}");

    let endless = read_w29a(
        "Name:Endless Horizons\n\
         ManaCost:3 W\n\
         Types:Enchantment\n\
         T:Mode$ ChangesZone | ValidCard$ Card.Self | Origin$ Any | Destination$ Battlefield | Execute$ TrigChangeZone\n\
         SVar:TrigChangeZone:DB$ ChangeZone | Origin$ Library | Destination$ Exile | ChangeType$ Plains | ChangeNum$ X | RememberChanged$ True\n\
         T:Mode$ Phase | Phase$ Upkeep | ValidPlayer$ You | Execute$ TrigRetrieve | TriggerZones$ Battlefield\n\
         SVar:TrigRetrieve:DB$ ChangeZone | ChangeType$ Card.IsRemembered+YouOwn | ChangeNum$ 1 | Origin$ Exile | Destination$ Hand | Hidden$ True\n\
         SVar:X:Count$ValidLibrary Plains.YouCtrl\n\
         Oracle:When Endless Horizons enters, search your library for any number of Plains cards, exile them, then shuffle.\\nAt the beginning of your upkeep, you may put a card you own exiled with Endless Horizons into your hand.",
    );
    let e_joined = endless.abilities.join("\n");
    assert!(e_joined.contains("Find::EXILE_LINKED"), "{e_joined}");
    assert!(e_joined.contains("ChooseZone::LinkedExile"), "{e_joined}");

    let dracosaur_like = read_w29a(
        "Name:Dracosaur Probe\n\
         ManaCost:3 R R\n\
         Types:Creature Dragon\n\
         PT:5/5\n\
         T:Mode$ Phase | Phase$ Upkeep | TriggerZones$ Battlefield | ValidPlayer$ You | Execute$ TrigExile\n\
         SVar:TrigExile:DB$ Dig | Defined$ You | DigNum$ 2 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBEffectYou\n\
         SVar:DBEffectYou:DB$ Effect | RememberObjects$ RememberedCard | StaticAbilities$ Play | SubAbility$ DBGain | ForgetOnMoved$ Exile\n\
         SVar:Play:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile\n\
         SVar:DBGain:DB$ GainLife | ConditionDefined$ Remembered | ConditionPresent$ Land | LifeAmount$ 2 | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionDefined$ Remembered | ConditionPresent$ Card.nonLand | NumCards$ 1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:At the beginning of your upkeep, exile the top two cards of your library. You may play them this turn. If you exiled a land card this way, you gain 2 life. If you exiled a nonland card this way, draw a card.",
    );
    assert!(
        dracosaur_like.abilities[0].contains("ZoneSel::ExileAll"),
        "{}",
        dracosaur_like.abilities[0]
    );
}
