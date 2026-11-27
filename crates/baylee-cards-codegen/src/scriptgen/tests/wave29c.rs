use crate::catalog::SubtypeCatalogs;
use crate::scriptgen::{parse, refusal_reason, transcode};
use crate::tokengen::TokenLookup;

fn cats_w29c() -> SubtypeCatalogs {
    let mut c = SubtypeCatalogs {
        creature: vec![
            "Angel".into(),
            "Assassin".into(),
            "Beast".into(),
            "Crab".into(),
            "Demon".into(),
            "Dinosaur".into(),
            "Dragon".into(),
            "Druid".into(),
            "Eldrazi".into(),
            "Elemental".into(),
            "Elf".into(),
            "Fish".into(),
            "Giant".into(),
            "Goblin".into(),
            "Golem".into(),
            "Horror".into(),
            "Horse".into(),
            "Human".into(),
            "Illusion".into(),
            "Insect".into(),
            "Knight".into(),
            "Monkey".into(),
            "Mutant".into(),
            "Ninja".into(),
            "Ooze".into(),
            "Phyrexian".into(),
            "Rhino".into(),
            "Rogue".into(),
            "Shaman".into(),
            "Spy".into(),
            "Squirrel".into(),
            "Turtle".into(),
            "Vedalken".into(),
            "Wall".into(),
            "Warrior".into(),
            "Wizard".into(),
            "Zombie".into(),
        ],
        artifact: vec![
            "Clue".into(),
            "Equipment".into(),
            "Food".into(),
            "Treasure".into(),
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
        planeswalker: vec![
            "Daretti".into(),
            "Ellywick".into(),
            "Lukka".into(),
            "Vivien".into(),
        ],
        ..SubtypeCatalogs::default()
    };
    c.normalize();
    c
}

fn tokens_w29c() -> TokenLookup {
    TokenLookup::held(&[
        (
            "r_1_1_goblin",
            "Name:Goblin\nTypes:Creature Goblin\nColors:red\nPT:1/1",
        ),
        (
            "w_4_4_angel_flying",
            "Name:Angel\nTypes:Creature Angel\nColors:white\nPT:4/4\nK:Flying",
        ),
        (
            "r_6_6_dragon_flying",
            "Name:Dragon\nTypes:Creature Dragon\nColors:red\nPT:6/6\nK:Flying",
        ),
        (
            "g_2_2_monkey",
            "Name:Monkey\nTypes:Creature Monkey\nColors:green\nPT:2/2",
        ),
        (
            "bg_1_1_insect",
            "Name:Insect\nTypes:Creature Insect\nColors:black,green\nPT:1/1",
        ),
        (
            "g_4_4_rhino_warrior",
            "Name:Rhino Warrior\nTypes:Creature Rhino Warrior\nColors:green\nPT:4/4",
        ),
        (
            "c_a_treasure_sac",
            "Name:Treasure\nTypes:Artifact Treasure\n\
             A:AB$ Mana | Cost$ T Sac<1/CARDNAME> | Produced$ Any",
        ),
        (
            "c_a_food_sac",
            "Name:Food Token\nTypes:Artifact Food\n\
             A:AB$ GainLife | Cost$ 2 T Sac<1/CARDNAME> | LifeAmount$ 3",
        ),
        (
            "b_1_1_goblin_rogue",
            "Name:Goblin Rogue\nTypes:Creature Goblin Rogue\nColors:black\nPT:1/1",
        ),
        (
            "c_0_2_a_wall_defender",
            "Name:Wall\nTypes:Artifact Creature Wall\nColors:colorless\nPT:0/2\nK:Defender",
        ),
    ])
}

fn read_w29c(text: &str) -> crate::body::CardBody {
    let parsed = parse(text);
    let c = cats_w29c();
    let t = tokens_w29c();
    transcode(&parsed, &c, Some(&t)).unwrap_or_else(|| {
        panic!(
            "expected script to read in full, refused: {:?}\n{text}",
            refusal_reason(&parsed, &c, Some(&t))
        )
    })
}

fn refused_w29c(text: &str) -> bool {
    let parsed = parse(text);
    transcode(&parsed, &cats_w29c(), Some(&tokens_w29c())).is_none()
}

#[test]
fn wave29c_dig_until_and_cmc_filters() {
    // Genesis Storm: Commander Storm + DigUntil OptionalFoundMove + BottomRandom.
    let genesis = read_w29c(
        "Name:Genesis Storm\nManaCost:4 G G\nTypes:Sorcery\n\
         T:Mode$ SpellCast | ValidCard$ Card.Self | Execute$ TrigCopy\n\
         SVar:TrigCopy:DB$ CopySpellAbility | Defined$ TriggeredSpellAbility | Amount$ X | MayChooseTarget$ True\n\
         SVar:X:Count$TotalCommanderCastFromCommandZone\n\
         A:SP$ DigUntil | Valid$ Permanent.nonLand | FoundDestination$ Battlefield | OptionalFoundMove$ True | RevealedDestination$ Library | RevealedLibraryPosition$ -1 | RevealRandomOrder$ True\n\
         Oracle:When you cast this spell, copy it for each time you've cast your commander from the command zone this game.\\nReveal cards from the top of your library until you reveal a nonland permanent card. You may put that card onto the battlefield. Then put all cards revealed this way that weren't put onto the battlefield on the bottom of your library in a random order.",
    );
    assert!(
        genesis
            .abilities
            .iter()
            .any(|a| a.contains("Amount::XPlusCommanderCasts"))
            && genesis
                .abilities
                .iter()
                .any(|a| a.contains("not_chosen_movement: ChooseMove::BottomOfLibraryRandom")),
        "{:?}",
        genesis.abilities
    );

    // Old Stickfingers: SpellCast self trigger reading Count$xPaid via EventManaValue - 2.
    let stickfingers = read_w29c(
        "Name:Old Stickfingers\nManaCost:X B G\nTypes:Legendary Creature Horror\nPT:*/*\n\
         T:Mode$ SpellCast | ValidCard$ Card.Self | Execute$ TrigDigUntil\n\
         SVar:TrigDigUntil:DB$ DigUntil | Amount$ X | Valid$ Creature | FoundDestination$ Graveyard | RevealedDestination$ Library | RevealedLibraryPosition$ -1 | RevealRandomOrder$ True\n\
         SVar:X:Count$xPaid\n\
         S:Mode$ Continuous | CharacteristicDefining$ True | SetPower$ Y | SetToughness$ Y\n\
         SVar:Y:Count$ValidGraveyard Creature.YouOwn",
    );
    assert!(
        stickfingers.abilities[0]
            .contains("Amount::SaturatingSub { base: &Amount::EventManaValue, subtract: 2 }"),
        "{}",
        stickfingers.abilities[0]
    );

    // Raph & Mikey, Troublemakers: Attacks trigger DigUntil with Attacking$ True.
    let raph = read_w29c(
        "Name:Raph & Mikey, Troublemakers\nManaCost:5 RG RG\nTypes:Legendary Creature Mutant Ninja Turtle\nPT:7/7\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ TrigDigUntil | TriggerZones$ Battlefield\n\
         SVar:TrigDigUntil:DB$ DigUntil | Valid$ Creature | FoundDestination$ Battlefield | Tapped$ True | Attacking$ True | RevealedDestination$ Library | RevealedLibraryPosition$ -1 | RevealRandomOrder$ True",
    );
    assert!(
        raph.abilities[0].contains(
            "Effect::PutOntoBattlefieldAttacking { subject: DefinedObjects::Found, owner_control: false, tapped: true }"
        ),
        "{}",
        raph.abilities[0]
    );

    // Selvala's Stampede: Council's dilemma VoteNum inside DigUntil & ChangeZone.
    let selvala = read_w29c(
        "Name:Selvala's Stampede\nManaCost:4 G G\nTypes:Sorcery\n\
         A:SP$ Vote | Defined$ Player | Choices$ DBVoteWild,DBVoteFree | StoreVoteNum$ True\n\
         SVar:DBVoteWild:DB$ DigUntil | Amount$ VoteNum | Valid$ Creature.YouOwn | RevealedDestination$ Library | RevealedLibraryPosition$ 0 | FoundDestination$ Battlefield | Shuffle$ True | SpellDescription$ Wild\n\
         SVar:DBVoteFree:DB$ ChangeZone | Origin$ Hand | Destination$ Battlefield | ChangeType$ Permanent | ChangeNum$ VoteNum | SpellDescription$ Free",
    );
    assert!(
        selvala.abilities[0].contains("ThisWay::VoteCount(0)")
            && selvala.abilities[0].contains("ThisWay::VoteCount(1)"),
        "{}",
        selvala.abilities[0]
    );

    // Codie, Vociferous Codex: cmcLTX in DelayedTrigger SpellCast DigUntil.
    let codie = read_w29c(
        "Name:Codie, Vociferous Codex\nManaCost:3\nTypes:Legendary Artifact Creature\nPT:1/4\n\
         S:Mode$ CantBeCast | ValidCard$ Permanent | Caster$ You\n\
         A:AB$ Mana | Cost$ 4 T | Produced$ W U B R G | SubAbility$ DBTrigger\n\
         SVar:DBTrigger:DB$ DelayedTrigger | Mode$ SpellCast | ValidActivatingPlayer$ You | ThisTurn$ True | Execute$ DBDig\n\
         SVar:DBDig:DB$ DigUntil | Defined$ You | Valid$ Instant.cmcLTX,Sorcery.cmcLTX | FoundDestination$ Exile | RevealedDestination$ Exile | RememberRevealed$ True | ImprintFound$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | StaticAbilities$ MayPlay | RememberObjects$ Imprinted | ForgetOnMoved$ Exile | SubAbility$ DBRestRandomOrder\n\
         SVar:MayPlay:Mode$ Continuous | Affected$ Card.IsRemembered+nonLand | MayPlay$ True | MayPlayWithoutManaCost$ True | AffectedZone$ Exile\n\
         SVar:DBRestRandomOrder:DB$ ChangeZoneAll | ChangeType$ Card.IsRemembered | Origin$ Exile | Destination$ Library | LibraryPosition$ -1 | RandomOrder$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True\n\
         SVar:X:TriggeredCard$CardManaCost",
    );
    assert!(
        codie
            .statics
            .contains("Filter::CmcAtMostAmount(&Amount::EventManaValue)")
            && codie
                .statics
                .contains("Filter::Not(&Filter::CmcEqualsAmount(&Amount::EventManaValue))"),
        "{}",
        codie.statics
    );
}

#[test]
fn wave29c_copy_spell_and_copy_permanent() {
    // Increasing Vengeance: Count$wasCastFromGraveyard.2.1.
    let vengeance = read_w29c(
        "Name:Increasing Vengeance\nManaCost:R R\nTypes:Instant\nK:Flashback:3 R R\n\
         A:SP$ CopySpellAbility | ValidTgts$ Instant.YouCtrl,Sorcery.YouCtrl | TargetType$ Spell | Amount$ X | MayChooseTarget$ True\n\
         SVar:X:Count$wasCastFromGraveyard.2.1",
    );
    assert!(
        vengeance.abilities[0].contains("Effect::IfCastFromGraveyard"),
        "{}",
        vengeance.abilities[0]
    );

    // Show of Confidence: self-cast trigger copying for each other instant/sorcery cast this turn.
    let show = read_w29c(
        "Name:Show of Confidence\nManaCost:1 W\nTypes:Instant\n\
         T:Mode$ SpellCast | ValidCard$ Card.Self | Execute$ TrigCopy\n\
         SVar:TrigCopy:DB$ CopySpellAbility | Defined$ TriggeredSpellAbility | Amount$ X | MayChooseTarget$ True\n\
         SVar:X:Count$ThisTurnCast_Instant.YouCtrl+!CastSaSource,Sorcery.YouCtrl\n\
         A:SP$ PutCounter | ValidTgts$ Creature | CounterType$ P1P1 | CounterNum$ 1\n\
         Oracle:When you cast this spell, copy it for each other instant and sorcery spell you've cast this turn. You may choose new targets for the copies.\\nPut a +1/+1 counter on target creature.",
    );
    assert!(
        show.abilities
            .iter()
            .any(|a| a.contains("Amount::SpellsCastMatchingThisTurn") && a.contains("subtract: 1")),
        "{:?}",
        show.abilities
    );

    // Sea Gate Stormcaller: Kicked delayed SpellCast trigger split.
    let stormcaller = read_w29c(
        "Name:Sea Gate Stormcaller\nManaCost:1 U\nTypes:Creature Human Wizard\nPT:2/1\nK:Kicker:4 U\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigEffect\n\
         SVar:TrigEffect:DB$ DelayedTrigger | Mode$ SpellCast | ValidCard$ Instant.cmcLE2,Sorcery.cmcLE2 | ValidActivatingPlayer$ You | ThisTurn$ True | Execute$ TrigCopy\n\
         SVar:TrigCopy:DB$ CopySpellAbility | Defined$ TriggeredSpellAbility | Amount$ X | MayChooseTarget$ True\n\
         SVar:X:Count$Kicked.2.1\n\
         Oracle:Kicker {4}{U}\\nWhen Sea Gate Stormcaller enters, copy the next instant or sorcery spell with mana value 2 or less you cast this turn when you cast it. If Sea Gate Stormcaller was kicked, copy that spell twice instead. You may choose new targets for the copies.",
    );
    assert!(
        stormcaller
            .abilities
            .iter()
            .any(|a| a.contains("Effect::IfKicked")),
        "{:?}",
        stormcaller.abilities
    );

    // Schema Thief + The Cloning of Shredder + Phantom Steed.
    let schema = read_w29c(
        "Name:Schema Thief\nManaCost:3 U\nTypes:Creature Vedalken Rogue\nPT:3/3\nK:Flying\n\
         T:Mode$ DamageDone | ValidSource$ Card.Self | ValidTarget$ Player | CombatDamage$ True | Execute$ TrigToken\n\
         SVar:TrigToken:DB$ CopyPermanent | ValidTgts$ Artifact.ControlledBy TriggeredTarget | Defined$ Targeted",
    );
    assert!(
        schema.abilities[0].contains("TargetSpec::ObjectOfEventPlayer("),
        "{}",
        schema.abilities[0]
    );

    let shredder = read_w29c(
        "Name:The Cloning of Shredder\nManaCost:4 B B\nTypes:Enchantment Saga\n\
         K:Chapter:3:DBExile,DBCopy2,DBCopy2\n\
         SVar:DBExile:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | ValidTgts$ Creature.YouOwn | SubAbility$ DBCopy1 | RememberChanged$ True\n\
         SVar:DBCopy1:DB$ CopyPermanent | Defined$ Remembered | Controller$ You | NonLegendary$ True | AddTypes$ Mutant | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:DBCopy2:DB$ CopyPermanent | Defined$ ExiledWith | Controller$ You | NonLegendary$ True | AddTypes$ Mutant",
    );
    assert!(
        shredder
            .abilities
            .iter()
            .any(|a| a.contains("zone: ChooseZone::LinkedExile")),
        "{:?}",
        shredder.abilities
    );
}

#[test]
fn wave29c_dig_remember_changed_sac_and_mill() {
    // Aid from the Cowl & Inscribed Tablet & Burning Curiosity.
    let cowl = read_w29c(
        "Name:Aid from the Cowl\nManaCost:3 G G\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | Revolt$ True | TriggerZones$ Battlefield | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ Dig | DigNum$ 1 | Reveal$ True | Optional$ True | ChangeNum$ 1 | ChangeValid$ Permanent | DestinationZone$ Battlefield | DestinationZone2$ Library | LibraryPosition2$ 0 | RememberChanged$ True | SubAbility$ DBMoveToBottom\n\
         SVar:DBMoveToBottom:DB$ Dig | DigNum$ 1 | DestinationZone$ Library | Optional$ True | LibraryPosition$ -1 | LibraryPosition2$ 0 | ConditionPresent$ Card | ConditionDefined$ Remembered | ConditionCompare$ EQ0 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        cowl.abilities[0].contains("ChooseMove::BottomOfLibraryChosenOrder"),
        "{}",
        cowl.abilities[0]
    );

    let rulik = read_w29c(
        "Name:Rulik Mons, Warren Chief\nManaCost:1 R G G\nTypes:Legendary Creature Goblin\nPT:3/3\nK:Menace\n\
         T:Mode$ Attacks | ValidCard$ Card.Self | Execute$ DBDig\n\
         SVar:DBDig:DB$ Dig | DigNum$ 1 | ChangeNum$ 1 | ChangeValid$ Card.Land | Optional$ True | RememberChanged$ True | DestinationZone$ Battlefield | DestinationZone2$ Library | LibraryPosition2$ 0 | SubAbility$ DBToken\n\
         SVar:DBToken:DB$ Token | ConditionDefined$ Remembered | ConditionPresent$ Card.inZoneBattlefield | ConditionCompare$ EQ0 | TokenScript$ r_1_1_goblin | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        rulik.abilities[0].contains("zone: ZoneSel::Battlefield")
            && rulik.abilities[0].contains("GOBLIN"),
        "{}",
        rulik.abilities[0]
    );

    // Monkey Cage (stub!) + Pitiless Carnage + Fandaniel + Vivien on the Hunt + Jace's Mindseeker + Sludge Titan.
    let monkey = read_w29c(
        "Name:Monkey Cage\nManaCost:5\nTypes:Artifact\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Creature | TriggerZones$ Battlefield | Execute$ TrigSac\n\
         SVar:TrigSac:DB$ Sacrifice | SubAbility$ DBToken | RememberSacrificed$ True\n\
         SVar:DBToken:DB$ Token | ConditionDefined$ Remembered | ConditionPresent$ Card.Self | TokenAmount$ X | TokenScript$ g_2_2_monkey | TokenOwner$ You\n\
         SVar:X:TriggeredCard$CardManaCost",
    );
    assert!(
        monkey.abilities[0].contains("Effect::SacrificeSelfThen"),
        "{}",
        monkey.abilities[0]
    );

    let pitiless = read_w29c(
        "Name:Pitiless Carnage\nManaCost:3 B\nTypes:Sorcery\n\
         A:SP$ Sacrifice | Amount$ Count$Valid Permanent.YouCtrl | SacValid$ Permanent.YouCtrl | RememberSacrificed$ True | Optional$ True | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | NumCards$ Count$RememberedSize | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         K:Plot:1 B B",
    );
    assert!(
        pitiless.abilities[0].contains("Effect::SacrificeAnyNumber"),
        "{}",
        pitiless.abilities[0]
    );

    let vivien = read_w29c(
        "Name:Vivien on the Hunt\nManaCost:4 G G\nTypes:Legendary Planeswalker Vivien\nLoyalty:4\n\
         A:AB$ Sacrifice | Cost$ AddCounter<2/LOYALTY> | Planeswalker$ True | Optional$ True | SacValid$ Creature | RememberSacrificed$ True | SubAbility$ DBChangeZone | SpellDescription$ +2\n\
         SVar:DBChangeZone:DB$ ChangeZone | ConditionDefined$ Remembered | ConditionPresent$ Card | Origin$ Library | Destination$ Battlefield | ChangeType$ Creature.cmcEQX | ChangeNum$ 1 | SubAbility$ DBCleanup\n\
         SVar:X:Remembered$CardManaCost/Plus.1\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         A:AB$ Mill | Cost$ AddCounter<1/LOYALTY> | Planeswalker$ True | Defined$ You | NumCards$ 5 | RememberMilled$ True | SubAbility$ DBChangeZone2 | SpellDescription$ +1\n\
         SVar:DBChangeZone2:DB$ ChangeZone | Origin$ Graveyard,Exile | Destination$ Hand | Hidden$ True | ChangeType$ Creature.IsRemembered | ChangeNum$ Z | ChangeNumDesc$ any number of | SubAbility$ DBCleanup\n\
         SVar:Z:Remembered$Valid Creature\n\
         A:AB$ Token | Cost$ SubCounter<1/LOYALTY> | Planeswalker$ True | TokenScript$ g_4_4_rhino_warrior | SpellDescription$ -1",
    );
    assert_eq!(vivien.abilities.len(), 3);

    let mindseeker = read_w29c(
        "Name:Jace's Mindseeker\nManaCost:4 U U\nTypes:Creature Fish Illusion\nPT:4/4\nK:Flying\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ MindFishing\n\
         SVar:MindFishing:DB$ Mill | NumCards$ 5 | ValidTgts$ Opponent | RememberMilled$ True | SubAbility$ FishyCast\n\
         SVar:FishyCast:DB$ Play | ValidZone$ Graveyard,Exile | Valid$ Card.IsRemembered | Controller$ You | WithoutManaCost$ True | ValidSA$ Instant,Sorcery | Optional$ True | Amount$ 1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
    );
    assert!(
        mindseeker.abilities[0].contains("PlaySource::Subject(TargetSpec::EventObject)"),
        "{}",
        mindseeker.abilities[0]
    );
}

#[test]
fn wave29c_refusal_boundaries() {
    // Note: `TotalCommanderCastFromCommandZone` on `K:etbCounter` is now supported
    // via Wave 17b (`Amount::CommanderCasts(PlayerRel::You)`), see `tests/wave17b.rs`.
    for bad in [
        // Thousand-Year Storm: "cast before it" != total cast on resolution -> MUST stay refused!
        "Name:Thousand-Year Storm\nManaCost:4 U R\nTypes:Enchantment\n\
         T:Mode$ SpellCast | ValidCard$ Instant,Sorcery | ValidActivatingPlayer$ You | Execute$ TrigCopy | TriggerZones$ Battlefield\n\
         SVar:TrigCopy:DB$ CopySpellAbility | Defined$ TriggeredSpellAbility | Amount$ X | MayChooseTarget$ True\n\
         SVar:X:TriggerObjectsCurrentCastSpells$Valid Sorcery.YouCtrl,Instant.YouCtrl/Minus.1",
        // Mandatory multi-opponent sacrifice + OppNonRememberedController MUST NOT use PlayerMayPayCostOr!
        "Name:MandatoryOppSac\nManaCost:3 B\nTypes:Sorcery\n\
         A:SP$ Sacrifice | Defined$ Opponent | SacValid$ Creature | RememberSacrificed$ True | SubAbility$ DBLose\n\
         SVar:DBLose:DB$ LoseLife | Defined$ OppNonRememberedController | LifeAmount$ 3 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True",
        // Twobrid X spell self-cast trigger MUST refuse EventManaValue subtraction!
        "Name:TwobridXCast\nManaCost:X 2R 2R\nTypes:Sorcery\n\
         T:Mode$ SpellCast | ValidCard$ Card.Self | Execute$ TrigCopy\n\
         SVar:TrigCopy:DB$ CopySpellAbility | Defined$ TriggeredSpellAbility | Amount$ X\n\
         SVar:X:Count$xPaid\n\
         A:SP$ DealDamage | ValidTgts$ Any | NumDmg$ 1",
        // Sorcery Attacking$ DigUntil MUST stay refused!
        "Name:BadSorceryAttackingDig\nManaCost:4 R\nTypes:Sorcery\n\
         A:SP$ DigUntil | Valid$ Creature | FoundDestination$ Battlefield | Attacking$ True | RevealedDestination$ Library | RevealedLibraryPosition$ -1 | RevealRandomOrder$ True",
        // WithMayLook$ face-down Dig without LookAtTopExileFaceDown shape MUST stay refused!
        "Name:CloneShellLike\nManaCost:5\nTypes:Artifact Creature\nPT:2/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigDig\n\
         SVar:TrigDig:DB$ Dig | Defined$ You | DigNum$ 4 | DestinationZone$ Exile | ExileFaceDown$ True | WithMayLook$ True\n\
         Oracle:When Clone Shell enters, look at the top four cards of your library, exile one face down, then put the rest on the bottom of your library in any order.",
    ] {
        assert!(refused_w29c(bad), "must refuse:\n{bad}");
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn wave29c_pass2_unlocks() {
    // 1. Burning Curiosity: OptionalCost Blight<1> + Dig.RememberChanged split.
    let burning = read_w29c(
        "Name:Burning Curiosity\nManaCost:2 R\nTypes:Sorcery\n\
         S:Mode$ OptionalCost | EffectZone$ All | ValidCard$ Card.Self | ValidSA$ Spell | Cost$ Blight<1> | Description$ Blight 1\n\
         A:SP$ Dig | Defined$ You | DigNum$ X | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ DBMayPlay\n\
         SVar:DBMayPlay:DB$ Effect | StaticAbilities$ STPlay | Duration$ UntilTheEndOfYourNextTurn | RememberObjects$ Remembered | ForgetOnMoved$ Exile | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:STPlay:Mode$ Continuous | Affected$ Card.IsRemembered | AffectedZone$ Exile | MayPlay$ True\n\
         SVar:X:Count$OptionalGenericCostPaid.3.2\n\
         Oracle:As an additional cost to cast this spell, you may blight 1.\\nExile the top two cards of your library. If this spell's additional cost was paid, exile the top three cards instead. Until the end of your next turn, you may play those cards.",
    );
    assert!(
        burning.abilities[0].contains("Condition::OptionalCostPaid")
            && burning.abilities[0].contains("PlayWindow::ThroughYourNextTurn"),
        "{}",
        burning.abilities[0]
    );

    // 2. Slick Imitator: granted AB$ CopySpellAbility with Secondary$ True.
    let slick = read_w29c(
        "Name:Slick Imitator\nManaCost:1 U\nTypes:Creature Ooze\nPT:1/3\n\
         K:Start your engines\n\
         S:Mode$ Continuous | AffectedDefined$ Self | Condition$ MaxSpeed | AddAbility$ ABCopySpell | Description$ Max speed\n\
         SVar:ABCopySpell:AB$ CopySpellAbility | PrecostDesc$ Max speed — | Cost$ 1 Sac<1/CARDNAME/this creature> | ValidTgts$ Card.YouCtrl | TargetType$ Spell | Secondary$ True | MayChooseTarget$ True\n\
         Oracle:Start your engines!\\nMax speed — {1}, Sacrifice this creature: Copy target spell you control. You may choose new targets for the copy.",
    );
    assert!(!slick.abilities.is_empty());

    // 3. Kethek, Crucible Goliath: optional trigger sacrifice + omitted RevealedLibraryPosition$ -1.
    let kethek = read_w29c(
        "Name:Kethek, Crucible Goliath\nManaCost:2 R B\nTypes:Legendary Creature Phyrexian Beast\nPT:4/4\n\
         T:Mode$ Phase | Phase$ End of Turn | ValidPlayer$ You | Execute$ TrigSac | TriggerZones$ Battlefield\n\
         SVar:TrigSac:AB$ DigUntil | Cost$ Sac<1/Creature.StrictlyOther/another creature> | Defined$ You | Valid$ Card.Creature+nonLegendary+cmcLTX | FoundDestination$ Battlefield | RevealedDestination$ Library | RevealRandomOrder$ True\n\
         SVar:X:Sacrificed$CardManaCost\n\
         Oracle:At the beginning of your end step, you may sacrifice another creature. If you do, reveal cards from the top of your library until you reveal a nonlegendary creature card with lesser mana value, put it onto the battlefield, then put the rest on the bottom of your library in a random order.",
    );
    assert!(
        kethek.abilities[0].contains("Effect::CaptureAffected {")
            && kethek.abilities[0].contains("RevealUntilRest::BottomRandom")
            && kethek.statics.contains("Amount::RememberedManaValue"),
        "{}\n{}",
        kethek.abilities[0],
        kethek.statics
    );

    // 4. Illuna, Apex of Wishes & Part in Friendship: DigUntil battlefield-or-hand follow-ups.
    let illuna = read_w29c(
        "Name:Illuna, Apex of Wishes\nManaCost:2 G U R\nTypes:Legendary Creature Beast Elemental Dinosaur\nPT:6/6\n\
         K:Mutate:3 RG U U\nK:Flying\nK:Trample\n\
         T:Mode$ Mutates | ValidCard$ Card.Self | Execute$ TrigDigUntil\n\
         SVar:TrigDigUntil:DB$ DigUntil | Valid$ Permanent.nonLand | FoundDestination$ Exile | RevealedDestination$ Exile | RememberFound$ True | SubAbility$ DBChange\n\
         SVar:DBChange:DB$ ChangeZone | Defined$ Remembered | Origin$ Exile | Destination$ Battlefield | DestinationAlternative$ Hand | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:Mutate {3}{R/G}{U}{U}\\nFlying, trample\\nWhenever this creature mutates, exile cards from the top of your library until you exile a nonland permanent card. Put that card onto the battlefield or into your hand.",
    );
    assert!(
        illuna
            .abilities
            .iter()
            .any(|a| a.contains("not_chosen_movement: ChooseMove::Hand")),
        "{:?}",
        illuna.abilities
    );

    let friendship = read_w29c(
        "Name:Part in Friendship\nManaCost:4 G\nTypes:Enchantment\n\
         T:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Creature.YouCtrl+!token | TriggerZones$ Battlefield | Execute$ TrigDigUntil | ActivationLimit$ 1\n\
         SVar:TrigDigUntil:DB$ DigUntil | Valid$ Creature | NoMoveRevealed$ True | ImprintRevealed$ True | FoundDestination$ Library | FoundLibraryPosition$ 0 | RememberFound$ True | SubAbility$ DBBranch\n\
         SVar:DBBranch:DB$ Branch | BranchConditionSVar$ X | TrueSubAbility$ ToBattlefield | FalseSubAbility$ ToHand | SubAbility$ DBRestRandomOrder\n\
         SVar:ToBattlefield:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Battlefield\n\
         SVar:ToHand:DB$ ChangeZone | Defined$ Remembered | Origin$ Library | Destination$ Hand\n\
         SVar:DBRestRandomOrder:DB$ ChangeZoneAll | ChangeType$ Card.IsImprinted | Origin$ Library | Destination$ Library | LibraryPosition$ -1 | RandomOrder$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True | ClearImprinted$ True\n\
         SVar:X:Remembered$Valid Card.cmcLEY\n\
         SVar:Y:Count$Valid Land.YouCtrl\n\
         Oracle:Whenever a nontoken creature you control dies, reveal cards from the top of your library until you reveal a creature card. If its mana value is less than or equal to the number of lands you control, put it onto the battlefield. Otherwise, put it into your hand. Put the rest on the bottom of your library in a random order. This ability triggers only once each turn.",
    );
    assert!(
        friendship.abilities[0].contains("Effect::IfEventObjectMatches {")
            && friendship.abilities[0].contains("RevealUntilRest::BottomRandom"),
        "{}",
        friendship.abilities[0]
    );

    // 5. Jaxis, the Troublemaker, Aethermage's Touch, Vivien's Invocation, Crabomination.
    let jaxis = read_w29c(
        "Name:Jaxis, the Troublemaker\nManaCost:3 R\nTypes:Legendary Creature Human Warrior\nPT:2/3\n\
         A:AB$ CopyPermanent | Cost$ R T Discard<1/Card> | ValidTgts$ Creature.Other+YouCtrl | PumpKeywords$ Haste | AtEOT$ Sacrifice | SorcerySpeed$ True | RememberTokens$ True | SubAbility$ DBAnimate\n\
         SVar:DBAnimate:DB$ Animate | Defined$ Remembered | Triggers$ Dies | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:Dies:Mode$ ChangesZone | Origin$ Battlefield | Destination$ Graveyard | ValidCard$ Card.Self | Execute$ TrigDraw\n\
         SVar:TrigDraw:DB$ Draw\n\
         K:Blitz:1 R\n\
         Oracle:{R}, {T}, Discard a card: Create a token that's a copy of another target creature you control. It gains haste and \"When this creature dies, draw a card.\" Sacrifice it at the beginning of the next end step. Activate only as a sorcery.\\nBlitz {1}{R}",
    );
    assert!(!jaxis.abilities.is_empty());

    let viv_inv = read_w29c(
        "Name:Vivien's Invocation\nManaCost:5 G G\nTypes:Sorcery\n\
         A:SP$ Dig | DigNum$ 7 | ChangeNum$ 1 | ChangeValid$ Creature | Optional$ True | RestRandomOrder$ True | DestinationZone$ Battlefield | RememberChanged$ True | SubAbility$ DBTrig\n\
         SVar:DBTrig:DB$ ImmediateTrigger | RememberObjects$ Remembered | Execute$ DBDealDamage | ConditionDefined$ Remembered | ConditionPresent$ Card | SubAbility$ DBCleanup\n\
         SVar:DBDealDamage:DB$ DealDamage | ValidTgts$ Creature.OppCtrl | NumDmg$ X | DamageSource$ DelayTriggerRememberedLKI\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:TriggerRemembered$CardPower\n\
         Oracle:Look at the top seven cards of your library. You may put a creature card from among them onto the battlefield. Put the rest on the bottom of your library in a random order. When a creature is put onto the battlefield this way, it deals damage equal to its power to target creature an opponent controls.",
    );
    assert!(
        viv_inv.abilities[0].contains("Effect::EventObjectDealsDamageEqualToPower {"),
        "{}",
        viv_inv.abilities[0]
    );

    let crab = read_w29c(
        "Name:Crabomination\nManaCost:4 B B\nTypes:Creature Crab Demon\nPT:5/5\n\
         K:Emerge:5 B B:Artifact\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ ExileTop\n\
         SVar:ExileTop:DB$ Dig | ValidTgts$ Opponent | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | RememberChanged$ True | SubAbility$ RandomGraveyard\n\
         SVar:RandomGraveyard:DB$ ChangeZone | Origin$ Graveyard | Destination$ Exile | DefinedPlayer$ Targeted | Mandatory$ True | ChangeNum$ 1 | Hidden$ True | IsCurse$ True | AtRandom$ True | RememberChanged$ True | SubAbility$ RandomHand\n\
         SVar:RandomHand:DB$ ChangeZone | Origin$ Hand | Destination$ Exile | DefinedPlayer$ Targeted | Mandatory$ True | ChangeNum$ 1 | Hidden$ True | IsCurse$ True | AtRandom$ True | RememberChanged$ True | SubAbility$ CastOneExiled\n\
         SVar:CastOneExiled:DB$ Play | Valid$ Card.IsRemembered | ValidSA$ Spell | ValidZone$ Exile | Amount$ 1 | Optional$ True | WithoutManaCost$ True | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:Emerge from artifact {5}{B}{B}\\nWhen Crabomination enters, target opponent exiles the top card of their library, a card at random from their graveyard, and a card at random from their hand. You may cast a spell from among cards exiled this way without paying its mana cost.",
    );
    assert!(
        crab.abilities
            .iter()
            .any(|a| a.contains("Effect::ExileTop { who: PlayerRel::Chosen")),
        "{:?}",
        crab.abilities
    );

    // 6. Heretic's Punishment, Cache Grab, Desecration Demon, Plaguecrafter, Warren Weirding.
    let heretic = read_w29c(
        "Name:Heretic's Punishment\nManaCost:4 R\nTypes:Enchantment\n\
         A:AB$ Mill | Cost$ 3 R | NumCards$ 3 | RememberMilled$ True | SubAbility$ DBHereticDamage\n\
         SVar:DBHereticDamage:DB$ DealDamage | ValidTgts$ Any | NumDmg$ HereticX | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:HereticX:Remembered$GreatestCardManaCost\n\
         Oracle:{3}{R}: Mill three cards, then Heretic's Punishment deals damage to any target equal to the greatest mana value among the milled cards.",
    );
    assert!(
        heretic.abilities[0].contains("Amount::GreatestManaValueAmong("),
        "{}",
        heretic.abilities[0]
    );

    let cache = read_w29c(
        "Name:Cache Grab\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ Mill | NumCards$ 4 | RememberMilled$ True | SubAbility$ DBChangeZone\n\
         SVar:DBChangeZone:DB$ ChangeZone | Origin$ Graveyard,Exile | Destination$ Hand | ChangeType$ Permanent.IsRemembered | ChangeNum$ 1 | Optional$ True | Hidden$ True | RememberChanged$ True | ForgetOtherRemembered$ True | SubAbility$ DBToken\n\
         SVar:DBToken:DB$ Token | TokenScript$ c_a_food_sac | ConditionCheckSVar$ X | ConditionSVarCompare$ GT0 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Count$Valid Squirrel.YouCtrl/Plus.Y\n\
         SVar:Y:Remembered$Valid Squirrel\n\
         Oracle:Mill four cards. You may put a permanent card from among the cards milled this way into your hand. If you control a Squirrel or returned a Squirrel card to your hand this way, create a Food token.",
    );
    assert!(
        cache.abilities[0].contains("ZoneSel::HandOf(PlayerRel::You)"),
        "{}",
        cache.abilities[0]
    );

    let demon = read_w29c(
        "Name:Desecration Demon\nManaCost:2 B B\nTypes:Creature Demon\nPT:6/6\nK:Flying\n\
         T:Mode$ Phase | Phase$ BeginCombat | TriggerZones$ Battlefield | Execute$ TrigSac\n\
         SVar:TrigSac:DB$ Sacrifice | Defined$ Opponent | Amount$ 1 | SacValid$ Creature | RememberSacrificed$ True | Optional$ True | SubAbility$ DBSacSelf\n\
         SVar:DBSacSelf:DB$ Tap | Defined$ Self | ConditionCheckSVar$ X | SubAbility$ DBPutCounter\n\
         SVar:DBPutCounter:DB$ PutCounter | Defined$ Self | CounterType$ P1P1 | CounterNum$ 1 | ConditionCheckSVar$ X | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$Amount\n\
         Oracle:Flying\\nAt the beginning of each combat, any opponent may sacrifice a creature. If a player does, tap Desecration Demon and put a +1/+1 counter on it.",
    );
    assert!(
        demon.abilities[0].contains("RepeatDomain::EachOpponent")
            && demon.abilities[0].contains("Effect::MayDoFor {"),
        "{}",
        demon.abilities[0]
    );

    let plague = read_w29c(
        "Name:Plaguecrafter\nManaCost:2 B\nTypes:Creature Human Shaman\nPT:3/2\n\
         T:Mode$ ChangesZone | Origin$ Any | Destination$ Battlefield | ValidCard$ Card.Self | Execute$ TrigSac\n\
         SVar:TrigSac:DB$ Sacrifice | Defined$ Player | SacValid$ Creature,Planeswalker | RememberSacrificed$ True | SubAbility$ Discard\n\
         SVar:Discard:DB$ Discard | Mode$ TgtChoose | NumCards$ 1 | Defined$ NonRememberedController | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:When Plaguecrafter enters, each player sacrifices a creature or planeswalker. Each player who can't discards a card.",
    );
    assert!(
        plague.abilities[0].contains("RepeatDomain::EachPlayer")
            && plague.abilities[0].contains("Effect::CaptureAffected {"),
        "{}",
        plague.abilities[0]
    );

    let warren = read_w29c(
        "Name:Warren Weirding\nManaCost:1 B\nTypes:Kindred Sorcery Goblin\n\
         A:SP$ Sacrifice | ValidTgts$ Player | SacValid$ Creature | RememberSacrificed$ True | SubAbility$ DBToken\n\
         SVar:DBToken:DB$ Token | ConditionDefined$ Remembered | ConditionPresent$ Card.Goblin | ConditionCompare$ GE1 | TokenAmount$ 2 | TokenScript$ b_1_1_goblin_rogue | TokenOwner$ Targeted | RememberTokens$ True | SubAbility$ AdditionalPump\n\
         SVar:AdditionalPump:DB$ PumpAll | ValidCards$ Creature.IsRemembered | KW$ Haste | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:Target player sacrifices a creature. If a Goblin is sacrificed this way, that player creates two 1/1 black Goblin Rogue creature tokens, and those tokens gain haste until end of turn.",
    );
    assert!(
        warren.abilities[0].contains("KeywordSet::HASTE"),
        "{}",
        warren.abilities[0]
    );
}

#[test]
fn wave29c_pass3_unlocks_and_refusals() {
    // 1. Chain of Silence & Chain of Vapor (--stubs): UnlessAI$ + non-You optional self-copy.
    let chain_silence = read_w29c(
        "Name:Chain of Silence\nManaCost:1 W\nTypes:Instant\n\
         A:SP$ Pump | ValidTgts$ Creature | KW$ Prevent all damage that would be dealt by CARDNAME. | IsCurse$ True | SubAbility$ DBCopy\n\
         SVar:DBCopy:DB$ CopySpellAbility | Defined$ Parent | Controller$ TargetedController | Optional$ True | UnlessPayer$ TargetedController | UnlessCost$ Sac<1/Land> | UnlessSwitched$ True | UnlessAI$ Never | StackDescription$ None | MayChooseTarget$ True\n\
         Oracle:Prevent all damage target creature would deal this turn. That creature's controller may sacrifice a land. If the player does, they may copy this spell and may choose a new target for that copy.",
    );
    assert!(
        chain_silence.abilities[0]
            .contains("Effect::MayDoFor { player: PlayerRel::ControllerOfTarget")
            && chain_silence.abilities[0].contains("Effect::CopySpellOrAbility { target: TargetSpec::ThisObject, controller: PlayerRel::ControllerOfTarget"),
        "{}",
        chain_silence.abilities[0]
    );

    let chain_vapor = read_w29c(
        "Name:Chain of Vapor\nManaCost:U\nTypes:Instant\n\
         A:SP$ ChangeZone | ValidTgts$ Permanent.nonLand | Origin$ Battlefield | Destination$ Hand | SubAbility$ DBCopy\n\
         SVar:DBCopy:DB$ CopySpellAbility | Defined$ Parent | Controller$ TargetedController | UnlessPayer$ TargetedController | UnlessCost$ Sac<1/Land> | UnlessSwitched$ True | UnlessAI$ ChainOfVapor | StackDescription$ None | MayChooseTarget$ True\n\
         Oracle:Return target nonland permanent to its owner's hand. Then that permanent's controller may sacrifice a land. If the player does, they may copy this spell and may choose a new target for that copy.",
    );
    assert!(
        chain_vapor.abilities[0]
            .contains("Effect::MayDoFor { player: PlayerRel::ControllerOfTarget")
            && chain_vapor.abilities[0].contains("may_choose_new_targets: true"),
        "{}",
        chain_vapor.abilities[0]
    );

    // 2. Basalt Golem (--stubs): delayed end-of-combat SacrificeAll RememberSacrificed$ -> Token for DelayTriggerRememberedController.
    let basalt = read_w29c(
        "Name:Basalt Golem\nManaCost:5\nTypes:Artifact Creature Golem\nPT:2/4\n\
         S:Mode$ CantBlockBy | ValidAttacker$ Creature.Self | ValidBlocker$ Artifact.Creature\n\
         T:Mode$ AttackerBlockedByCreature | ValidCard$ Card.Self | ValidBlocker$ Creature | Execute$ TrigEndCombat\n\
         SVar:TrigEndCombat:DB$ DelayedTrigger | Mode$ Phase | Phase$ EndCombat | Execute$ TrigSacBlocker | RememberObjects$ TriggeredBlockerLKICopy\n\
         SVar:TrigSacBlocker:DB$ SacrificeAll | Defined$ DelayTriggerRememberedLKI | SubAbility$ DBToken | RememberSacrificed$ True\n\
         SVar:DBToken:DB$ Token | TokenScript$ c_0_2_a_wall_defender | TokenOwner$ DelayTriggerRememberedController | ConditionDefined$ Remembered | ConditionPresent$ Card | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:Basalt Golem can't be blocked by artifact creatures.\\nWhenever Basalt Golem becomes blocked by a creature, that creature's controller sacrifices it at end of combat. If the player does, they create a 0/2 colorless Wall artifact creature token with defender.",
    );
    assert!(
        basalt
            .abilities
            .iter()
            .any(|a| a.contains("Trigger::BecomesBlockedBy(")
                && a.contains("Effect::SacrificeEvent")
                && a.contains("PlayerRel::ControllerOfEvent")),
        "{:?}",
        basalt.abilities
    );

    // 3. Lammastide Weave: NameCard -> targeted player Mill 1 -> Remembered$Valid Card.NamedCard.
    let weave = read_w29c(
        "Name:Lammastide Weave\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ NameCard | Defined$ You | SubAbility$ DBMill\n\
         SVar:DBMill:DB$ Mill | NumCards$ 1 | ValidTgts$ Player | RememberMilled$ True | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | LifeAmount$ X | ConditionCheckSVar$ Y | ConditionSVarCompare$ EQ1 | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost\n\
         SVar:Y:Remembered$Valid Card.NamedCard\n\
         Oracle:Choose a card name, then target player mills a card. If a card with the chosen name was milled this way, you gain life equal to its mana value.\\nDraw a card.",
    );
    assert!(
        weave.abilities[0].contains("Effect::ChooseCardName {")
            && weave.abilities[0].contains("Amount::RememberedManaValue"),
        "{}",
        weave.abilities[0]
    );

    // 4. Ripples of Undeath: Mill 3 -> switched UnlessCost$ 1 PayLife<3> ChangeZone from Remembered.
    let ripples = read_w29c(
        "Name:Ripples of Undeath\nManaCost:1 B\nTypes:Enchantment\n\
         T:Mode$ Phase | Phase$ Main1 | ValidPlayer$ You | TriggerZones$ Battlefield | Execute$ TrigMill\n\
         SVar:TrigMill:DB$ Mill | NumCards$ 3 | RememberMilled$ True | SubAbility$ DBReturn\n\
         SVar:DBReturn:DB$ ChangeZone | UnlessCost$ 1 PayLife<3> | UnlessPayer$ You | UnlessSwitched$ True | Hidden$ True | Origin$ Graveyard,Exile | Destination$ Hand | ChangeType$ Card.IsRemembered | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         Oracle:At the beginning of your first main phase, mill three cards. Then you may pay {1} and 3 life. If you do, put a card from among those cards into your hand.",
    );
    assert!(
        ripples.abilities[0].contains("Effect::PlayerMayPayManaAndLifeEachThenElse {")
            && ripples.abilities[0].contains("min: 1"),
        "{}",
        ripples.abilities[0]
    );

    // 5. Honest refusals:
    // - DelayTriggerRememberedController outside a delayed trigger must stay refused.
    assert!(refused_w29c(
        "Name:BadDelayedOwner\nManaCost:2\nTypes:Instant\n\
         A:SP$ Token | TokenScript$ c_0_2_a_wall_defender | TokenOwner$ DelayTriggerRememberedController"
    ));
    // - Remembered$Valid Card.NamedCard without an earlier NameCard / chosen name must stay refused.
    assert!(refused_w29c(
        "Name:BadNoChosenName\nManaCost:1 G\nTypes:Instant\n\
         A:SP$ Mill | NumCards$ 1 | ValidTgts$ Player | RememberMilled$ True | SubAbility$ DBGainLife\n\
         SVar:DBGainLife:DB$ GainLife | LifeAmount$ X | ConditionCheckSVar$ Y | ConditionSVarCompare$ EQ1 | SubAbility$ DBCleanup\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:Remembered$CardManaCost\n\
         SVar:Y:Remembered$Valid Card.NamedCard"
    ));
    // - Face-down opponent-library exile with MayPlay (Gonti-style) has no face-down ExilePermission in DSL -> refused.
    assert!(refused_w29c(
        "Name:BadGontiFaceDown\nManaCost:2 B\nTypes:Instant\n\
         A:SP$ Dig | ValidTgts$ Opponent | DigNum$ 1 | ChangeNum$ All | DestinationZone$ Exile | ExileFaceDown$ True | WithMayLook$ True | RememberChanged$ True | SubAbility$ DBEffect\n\
         SVar:DBEffect:DB$ Effect | RememberObjects$ Remembered | StaticAbilities$ STPlay | Duration$ Permanent | ForgetOnMoved$ Exile | SubAbility$ DBCleanup\n\
         SVar:STPlay:Mode$ Continuous | MayPlay$ True | Affected$ Card.IsRemembered | AffectedZone$ Exile\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True"
    ));
}

#[test]
fn sacrifice_immediate_trigger_remembered_lki_and_toughness() {
    let tip = read_w29c(
        "Name:Tip the Scales\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ Sacrifice | SacValid$ Creature | RememberSacrificed$ True | SubAbility$ DBImmediateTrigger | SpellDescription$ Sacrifice a creature.\n\
         SVar:DBImmediateTrigger:DB$ ImmediateTrigger | Execute$ TrigPumpAll | ConditionDefined$ RememberedLKI | ConditionPresent$ Creature | RememberObjects$ RememberedLKI | SubAbility$ DBCleanup | SpellDescription$ When you do, all creatures get -X/-X until end of turn, where X is the sacrificed creature's toughness.\n\
         SVar:TrigPumpAll:DB$ PumpAll | ValidCards$ Creature | NumAtt$ -X | NumDef$ -X\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:TriggerRemembered$CardToughness",
    );
    assert!(
        tip.abilities[0].contains("Effect::SacrificeFilter")
            && tip.abilities[0].contains("ReflexiveEvent::Sacrificed(&Filter::CREATURE)")
            && tip.abilities[0].contains("Amount::Negated(&Amount::EventLastToughness)"),
        "{}",
        tip.abilities[0]
    );

    let minsc = read_w29c(
        "Name:Minsc & Boo, Timeless Heroes\nManaCost:2 R G\nTypes:Legendary Planeswalker Minsc\nLoyalty:3\n\
         A:AB$ Sacrifice | Cost$ SubCounter<2/LOYALTY> | Planeswalker$ True | Defined$ You | SacValid$ Creature | RememberSacrificed$ True | SubAbility$ DBImmediateTrigger\n\
         SVar:DBImmediateTrigger:DB$ ImmediateTrigger | Execute$ TrigDamage | ConditionDefined$ RememberedLKI | ConditionPresent$ Creature | RememberObjects$ RememberedLKI | SubAbility$ DBCleanup\n\
         SVar:TrigDamage:DB$ DealDamage | ValidTgts$ Any | NumDmg$ X | SubAbility$ DBDraw\n\
         SVar:DBDraw:DB$ Draw | ConditionDefined$ DelayTriggerRememberedLKI | ConditionPresent$ Goblin | NumCards$ X\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True\n\
         SVar:X:TriggerRemembered$CardPower",
    );
    assert!(
        minsc.abilities[0].contains("Effect::ReflexiveTrigger")
            && minsc.abilities[0].contains("Amount::EventPower")
            && minsc.abilities[0].contains("Effect::IfCondition"),
        "{}",
        minsc.abilities[0]
    );

    assert!(refused_w29c(
        "Name:BadSacLkiCompare\nManaCost:2 B\nTypes:Sorcery\n\
         A:SP$ Sacrifice | SacValid$ Creature | RememberSacrificed$ True | SubAbility$ DBTrig\n\
         SVar:DBTrig:DB$ ImmediateTrigger | Execute$ TrigPump | ConditionDefined$ RememberedLKI | ConditionPresent$ Creature | ConditionCompare$ EQ2 | RememberObjects$ RememberedLKI | SubAbility$ DBCleanup\n\
         SVar:TrigPump:DB$ PumpAll | ValidCards$ Creature | NumAtt$ -1 | NumDef$ -1\n\
         SVar:DBCleanup:DB$ Cleanup | ClearRemembered$ True"
    ));
}
