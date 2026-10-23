# Argon PR #394 review — 23 October 2026

The imported review covered all 1,350 assigned card/test pairs, followed by an
independent Astra audit of the retained definitions and tests in six packages.
All review findings are closed. The validation evidence below distinguishes
the broad rules run, the configured database rerun and the final landing gates.

The measured candidate has **793 Implemented, 260 Partial and 297
Unimplemented** cards in the review scope (1,350 total). The 297 canonical
holds include 16 stubs already present in the imported review plus 281 recorded
by the subsequent integration/review inventory. Whole-pool counts include cards
outside this PR and must not be confused with these batch counts.

Unsafe approximations lose their complete optional ability or return to
unimplemented stubs. Missing costs, restrictions, drawbacks, opponent choices
and mandatory symmetric effects cannot qualify as playable Partial. Blank
noncreatures and creatures that immediately die without their size rule are
held too. Kjeldoran Guard and Orcish Farmer retain only their useful creature
bodies; their entire optional abilities are removed with explicit reasons.

## Cards held back

Selective `codegen --hold-back-file` writes canonical stubs. Each row records
the concrete finding linked from the generated source. A hold is not an
implemented rule and is excluded from playable coverage.

| Card | Package | Finding |
| --- | --- | --- |
| Illusionary Terrain | 1 | Only upkeep drawback remains; the actual land-type rule is missing. |
| Infernal Denizen | 1 | "If you can't [sacrifice two Swamps], tap this creature, and an opponent may gain control of a creature you control of their choice" → card only tapped (and sacrificed one Swamp), so the upkeep drawback was lighter than printed on a 5/7 with a steal ability |
| Infinite Hourglass | 1 | Counters are manipulated without the sole power payoff, leaving no useful printed ability. |
| Jester's Cap | 1 | Mandatory three-card exile is omitted; shuffle-only artifact has no meaningful printed payoff. |
| Jester's Mask | 1 | "Target opponent puts the cards from their hand on top of their library. Search that player's library for that many cards … into their hand" → hand went into the library and nothing came back, a pure hand wipe |
| Jeweled Amulet | 1 | Counters are manipulated without the sole mana payoff, leaving no useful printed ability. |
| Justice | 1 | Only upkeep drawback remains; the actual damage rule is missing. |
| Kjeldoran Royal Guard | 1 | The redirection consumer also redirects noncombat damage; the printed ability is restricted to combat damage. |
| Lim-Dûl's Hex | 1 | "deals 1 damage to that player unless they pay {B} or {3}" → 1 damage to each player unconditionally |
| Melee | 1 | No implemented spell effect remains; casting it does nothing. |
| Melting | 1 | supertype removal not expressible; accurate |
| Mercenaries | 1 | "Any player may activate this ability" ({3}: prevent the next damage it would deal to you) → absent, so no opponent can answer a plain 3/3 for four |
| Merieke Ri Berit | 1 | Permanent control remains while mandatory destruction when the source untaps is omitted, enabling repeated steals without losing previous creatures. |
| Musician | 1 | Counter absence is used instead of granted-ability absence; removing music counters permits duplicate upkeep taxation beyond the printed rule. |
| Naked Singularity | 1 | Mana replacement order takes an affected player choice away for lands with multiple basic types. |
| Oath of Lim-Dûl | 1 | "Whenever you lose life, for each 1 life you lost, sacrifice a permanent … unless you discard a card" → absent, leaving {B}{B}: draw a card with no cost |
| Orcish Conscripts | 1 | "can't attack unless at least two other creatures attack" and the same for blocking → dropped, an unrestricted 2/2 for {R} |
| Polar Kraken | 1 | "Cumulative upkeep—Sacrifice a land" → sacrificed one land per upkeep, not one per age counter, so from age 2 an 11/11 trampler was kept for less than printed |
| Pox | 1 | No implemented spell effect remains; casting it does nothing. |
| Reality Twist | 1 | Mana replacement order takes an affected player choice away for lands with multiple basic types. |
| Reclamation | 1 | The required defending player sacrifice choice is replaced by first matching land selection. |
| Apocalypse Chime | 2 | It pays its activation costs without the printed effect. |
| Baki's Curse | 2 | Fixed two damage omits mandatory extra damage to own multiply enchanted creatures; printed amount must count each recipient's Auras. |
| Coral Reef | 2 | "enters with four polyp counters", "Sacrifice an Island: Put two polyp counters", "Remove a polyp counter: +0/+1 counter" → Island sacrifice put nothing, the +0/+1 ability skipped the counter cost and was unlimited |
| Snowblind | 2 | Only Aura attachment remains, without a useful printed rule. |
| Snowfall | 2 | Only upkeep drawback remains; the mana payoff is missing. |
| Spoils of Evil | 2 | Mana and life use all opponents graveyards despite requiring a single targeted opponent. |
| Spoils of War | 2 | "X is the number of artifact and/or creature cards in an opponent's graveyard as you cast" + "Distribute X counters" → card let the caster pay any X and put all X counters on one creature (more than printed) |
| Stench of Evil | 2 | Partial honest (Plains destroyed; the per-land damage is absent). Missing-drawback check, borderline, kept: the 1 damage per land goes to every Plains controller, so it costs only a caster who wipes their own Plains; against an opponent's Plains the card does less than printed. PM may prefer HELD-BACK |
| Stunted Growth | 2 | "Target player chooses three cards from their hand and puts them on top of their library" → target discarded three cards (cards lost to the graveyard; a different action) |
| Vexing Arcanix | 2 | "Otherwise, they put it into their graveyard and this artifact deals 2 damage to them" → the card put the missed card into the graveyard and dealt nothing. The 2 damage is the price of the gamble: a player who targets themselves (they know their own library) mills and digs at no cost, stronger than printed. `Effect::RevealTopByName` has `matched` and `otherwise` destinations and no damage branch |
| Winter's Chill | 2 | "X can't be greater than snow lands", "its controller may pay {1} or {2}; if not, destroy at end of combat; if only {1}, prevent combat damage" → card prevented all damage to and from X attackers for free, never destroyed, no X cap, "this turn" instead of "this combat"; the test doc quoted text that is not printed |
| Zur's Weirding | 2 | Symmetric draw replacement and opponents' right to pay life to stop your draws are omitted; reveal-hands-only Partial is unsafe. |
| Arcane Denial | 3 | "Its controller may draw up to two cards at the beginning of the next turn's upkeep" (the printed drawback) was missing: a Counterspell with a free cantrip |
| Ashnod's Cylix | 3 | "Target player looks at the top three cards of their library, puts one back on top, then exiles the rest": card (after my first pass) looked at the controller's own top three for free, repeatably, with no put-back and no exile (a free peek, more than printed on a self-target) |
| Awesome Presence | 3 | "Can't be blocked unless defending player pays {3} for each creature they control that's blocking it": card only attaches (does less, harmless), reason is accurate; test covered only own creature |
| Bounty of the Hunt | 3 | Temporary power/toughness replaces actual counters, bypassing counter prevention and required interactions with opposite counters. |
| Browse | 3 | "put one of them into your hand, and exile the rest": the rest went to the bottom of the library, so the controller could dig for free again and again (more than printed) |
| Casting of Bones | 3 | "draw three cards, then discard one of them": card discarded any card of the hand, so a player holding other cards kept all three new ones (more than printed); test had an empty hand |
| Diminishing Returns | 3 | "You exile the top ten cards of your library" (the printed drawback) was missing: Timetwister with no cost; "up to seven" also became a flat seven |
| Diseased Vermin | 3 | Upkeep damage can affect an opponent never damaged by this creature after other opponents leave or control changes. |
| Fatal Lore | 3 | "An opponent chooses one" → the caster chose the mode, so they always take the better one; plays stronger than printed (a wrong-player choice) |
| Foresight | 3 | The delayed draw remains without the mandatory search and exile of three cards. |
| Giant Oyster | 3 | Mandatory removal of every minus-one counter when the source leaves or untaps is missing while the freeze/counters remain. |
| Gustha's Scepter | 3 | "Exile a card from your hand face down": card exiled the TOP CARD OF THE LIBRARY and a second tap returned all of them to hand: a draw engine (more than printed); returns all rather than one chosen |
| Hazduhr the Abbot | 3 | Prevention is retained while the mandatory redirected damage to the source is omitted, creating free prevention. |
| Helm of Obedience | 3 | "mills a card, then repeats ... until a creature card or X cards ...; sacrifice this artifact and put one of them onto the battlefield under your control": card milled a flat X, never stopped, took no creature and never sacrificed the artifact (a reusable mill engine, more than printed); X could be 0 |
| Ivory Gargoyle | 3 | "return it ... at the beginning of the next end step and you skip your next draw step" → card returned it at once and skipped nothing, so an immortal 2/2 flyer with no price |
| Koskun Falls | 3 | Only upkeep drawback remains; the attack restriction is missing. |
| Lat-Nam's Legacy | 3 | Mandatory hand-card shuffle can be declined, preserving hand cards and library order contrary to the printed instruction. |
| Leeches | 3 | "Target player loses all poison counters. Leeches deals that much damage to that player": the card is a sorcery that targets a player and does nothing (no piece removes poison counters or reads them as an amount); less than printed, but a dead card |
| Library of Lat-Nam | 3 | "An opponent chooses one" → the caster chose the mode (draw three later or tutor), the better one every time |
| Memory Lapse | 3 | "put it on top of its owner's library instead of into … graveyard": card countered to the graveyard (a harder counter than printed) |
| Orcish Mine | 3 | LastCounterRemoved only matches removals by ability controller; printed mandatory last-ore trigger must match any remover. Mining is also not the printed ore counter kind. |
| Retribution | 3 | Targets may come from different opponents, and counters replace the required opponent-chosen sacrifice. |
| Truce | 3 | Opponents life benefit and each-recipient draw are omitted or mis-scoped, removing the mandatory symmetric instruction. |
| Ancestral Memories | 4 | "Put two of them into your hand and the rest into your graveyard" → the rest went to the **bottom of the library** (no mill: five cards stay in the library, a different and stronger result than printed; the reason said so). No look-at-the-top effect sends the rest to the graveyard, and there is no other ability to strip |
| Basalt Golem | 4 | Wall creation depends on a death rather than a sacrifice; a sacrifice replaced by exile incorrectly denies the opponent token. |
| Bazaar of Wonders | 4 | Graveyard exile remains while the global same-name counter rule is omitted, bypassing the controller own casting restriction. |
| Benevolent Unicorn | 4 | The global spell-damage reduction is omitted, so the controller spells deal more damage while the creature remains. |
| Blind Fury | 4 | Trample removal remains without doubled combat damage from all creatures, removing the controller mandatory exposure to extra damage. |
| Bone Mask | 4 | "Exile cards from the top of your library equal to the damage prevented this way" → left out, so the prevention cost {2} alone (the exile is the whole price of the card; a missing printed cost plays stronger than printed). Reason said "does less" |
| Lim-Dûl's Vault | 4 | Mandatory shuffle of the rest of the library is omitted, preserving unprinted knowledge of its order. |
| Martyrdom | 4 | "next 1 damage … is dealt to this creature instead" → card granted PREVENTION of the damage to any target (spares the creature, any caster) |
| Misfortune | 4 | "An opponent chooses one" → the caster chose the mode (would always pick the helpful one: counters or life) |
| Omen of Fire | 4 | Island bounce remains while mandatory sacrifices by every player are omitted, sparing the caster printed sacrifices. |
| Phantasmal Sphere | 4 | "When this creature leaves the battlefield, target opponent creates an X/X blue Orb token" → token (a drawback of the card) missing: flying body that grows without any price, stronger than printed. Earlier I had only added tests for the upkeep price |
| Phyrexian Portal | 4 | "{3}: If your library has ten or more cards … piles … exile one … search the other" → card picked 1 of all ten, exiled nothing, no ten-card check (better than printed) |
| Soldevi Digger | 4 | "Put the top card of your graveyard on the bottom of your library" → card moved the WHOLE graveyard (its own reason admitted it) |
| Splintering Wind | 4 | "Create a 1/1 Splinter token … cumulative upkeep {G} … leaves: 1 damage to you and each creature you control" → no token and no damage to you: a repeatable {2}{G} ping that plays stronger than printed (token needs a generated-ledger entry) |
| Storm Cauldron | 4 | Land bounce remains while every player extra land play is omitted, strengthening the lock on opponents. |
| Suffocation | 4 | The condition can remain true after a later spell controlled by the caster; damage still hits the opponent rather than the controller of the last spell. |
| Sworn Defender | 4 | Toughness setting is retained without power setting, so a blocker of toughness one fails to reduce power to zero as printed. |
| Thought Lash | 4 | "Exile the top card of your library: Prevent the next 1 damage …" → a FREE, unlimited shield; the cumulative upkeep and the exile-your-library trigger were dropped too (card strictly better than printed) |
| Tidal Control | 4 | "Any player may activate this ability" → only the controller could counter red/green spells (missing drawback: stronger than printed) |
| Whirling Catapult | 4 | "{2}, Exile the top two cards of your library: …" → ping for {2} alone (skipped cost; its own comment admitted it) |
| Cadaverous Bloom | 5 | "Exile a card from your hand: Add {B}{B} or {G}{G}" - Argon shipped it blank (a Partial with no abilities, which passed every gate); a dead card to a player |
| Celestial Dawn | 5 | The restriction on spending nonwhite mana only as colorless is missing while retaining its other color/land changes. |
| Cycle of Life | 5 | Target filter accepts any creature entered this turn, including tokens, reanimated and blinked creatures never cast. |
| Dazzling Beauty | 5 | Suppressing assignment is not becoming blocked: gaining trample later still cannot deal player damage, and blocked-event triggers are missing. |
| Disempower | 5 | "Put target artifact or enchantment on top of its owner's library" - the owner chose top or bottom |
| Ekundu Cyclops | 5 | "must attack each combat if another creature attacks" missing; the 4-mana body is the benefit |
| Emberwilde Djinn | 5 | Loss of two life substitutes for an optional payment, allowing the choice below two life and bypassing payment restrictions. |
| Ether Well | 5 | "Put target creature on top of its owner's library; red: you may put it on the bottom" - owner chose, not the controller |
| Fallow Earth | 5 | "Put target land on top of its owner's library" - the owner chose top or bottom |
| Final Fortune | 5 | The loss is bound to an upcoming upkeep/end step rather than the specific extra turn granted; another extra turn changes its timing. |
| Flash | 5 | "sacrifice it unless you pay its mana cost reduced by {2}" missing - a free creature drop |
| Forbidden Crypt | 5 | "If you would draw a card, lose the game instead" and the graveyard clause missing; what was left was the benefit |
| Forsaken Wastes | 5 | The controller life-gain prohibition and targeting life-loss drawback are missing while global upkeep loss remains. |
| Gravebane Zombie | 5 | A dies trigger substitutes for the mandatory graveyard replacement, enabling death/sacrifice triggers and responses before the required top-deck destination. |
| Grinning Totem | 5 | Partial kept (only a shuffle remains); the test doc comment named another oracle id. **HELD-BACK candidate for the PM: the card does nothing a player can use** |
| Hall of Gemstone | 5 | "that player chooses a color" - the card forced green on every land |
| Illicit Auction | 5 | The bidding is the drawback; the card was a free permanent Control Magic |
| Jabari's Influence | 5 | Target filter includes creatures that attacked other players or planeswalkers rather than the caster. |
| Kaervek's Torch | 6 | The omitted targeting-spell tax also applies to the caster own copy spells, so retaining damage while omitting that cost can be stronger. |
| Lure of Prey | 6 | "Cast this spell only if an opponent cast a creature spell this turn" → casting restriction missing: an instant-speed, any-time "put a green creature from hand onto the battlefield" for {2}{G}{G} |
| Malignant Growth | 6 | Damage uses growth counters rather than cards actually drawn, burning through skipped or replaced draws. |
| Mangara's Equity | 6 | "choose black or red" → `ChooseColor` offered all five colors; the damage trigger then punished any color the player picked, and it only watched damage to you (not to a white creature you control) |
| Mangara's Tome | 6 | Only shuffle remains while mandatory five-card exile and the meaningful draw replacement are missing. |
| Maro | 6 | The hand-size characteristic ability is missing; a 0/0 that dies immediately is not useful playable coverage. |
| Meddle | 6 | "If target spell has only one target and that target is a creature, change … to another creature" → `ChangeTarget` ran for any single-target spell (a Bolt aimed at a player or a planeswalker could be moved to a creature); its own reason admitted the missing check. The only ability is the one that must come off, so no rules are left |
| Natural Balance | 6 | "Each player who controls six or more lands chooses five and sacrifices the rest … four or fewer … search for up to X (five minus lands)" → every player searches for ONE basic land whatever they control (a six-land player is never made to sacrifice; a one-land player gets one land, not four). The only ability is wrong, so no rules are left |
| Null Chamber | 6 | The opponent required card-name choice is omitted, creating a one-sided restriction. |
| Polymorph | 6 | "Its controller reveals cards from the top of their library until a creature card, puts it onto the battlefield, shuffles the rest" → half of the spell missing (the card's own reason admits it): a plain {3}{U} "destroy target creature, can't be regenerated", stronger than printed |
| Preferred Selection | 6 | Mandatory bottoming on declined payment is omitted, allowing both desirable cards to stay on top. |
| Prismatic Boon | 6 | One target is allowed even for X zero, granting protection without paying for a target. |
| Psychic Transfer | 6 | "**If the difference** between your life total and target player's life total is 5 or less, exchange life totals" → no condition at all: the exchange happens at any gap (20 vs 1 included), the reason admits it. The only ability is the one that must come off, so no rules are left |
| Purgatory | 6 | Life loss substitutes for paying two life; an unaffordable payment can still return a creature before state-based actions. |
| Ravenous Vampire | 6 | "you may sacrifice a nonartifact creature. If you do, +1/+1 counter. **If you don't, tap this creature**" → the "tap" branch is missing (the card's own reason admits it): a flier that never has to feed or tap, stronger than printed |
| Razor Pendulum | 6 | "each player's end step, **if that player has 5 or less life**, … 2 damage to that player" → no condition at all: it hit the active player at 20 life every end step (the reason blamed a wrong-player check that was not even used) |
| Reflect Damage | 6 | Damage from the spell substitutes for redirection of the original source and leaves original damage, changing protection, source identity and lifelink. |
| Reign of Chaos | 6 | A mandatory creature target is kept but not destroyed, allowing land destruction while sparing the caster only eligible creature. |
| Reparations | 6 | "Whenever an opponent casts a spell **that targets you or a creature you control**, you may draw" → `SpellCast(ControlledByOpponent)`: any spell an opponent casts draws a card (the reason admits the missing filter); `Filter::TargetsPermanentYouControl` is wider than "a creature you control" and cannot say "targets you", so no filter fits. The only ability is the one that must come off, so no rules are left |
| Sand Golem | 6 | The return follows any matching named graveyard card after the discarded incarnation is removed, instead of the exact discarded card. |
| Sapphire Charm | 6 | The draw mode targets any player but always gives the caster the delayed draw, mis-binding the recipient. |
| Sealed Fate | 6 | It looks at one card even for X zero, revealing unprinted information. |
| Seeds of Innocence | 6 | "The controller of each of those artifacts gains life equal to its mana value" → missing: the life the opponent gets back for each lost artifact is the spell's price, so the card was a free mass artifact kill |
| Shallow Grave | 6 | "Return the **top** creature card of your graveyard … haste … **Exile it at the beginning of the next end step**" → the card returned any creature card of the graveyard (a choice, not the top one), gave no haste and never exiled it: a {1}{B} instant that reanimates the best creature for good |
| Soul Echo | 6 | "target opponent may choose that for each 1 damage that would be dealt to you …, you remove an echo counter from this enchantment instead" → the reason says that choice is missing: no echo counter ever leaves, so with X >= 1 the enchantment never goes away and "you don't lose the game for having 0 or less life" holds for good (the opponent's only way to wear it down is gone) |
| Anvil of Bogardan | 7 | No maximum hand size applies only to the controller; the printed instruction applies to every player, so the symmetric restriction is incomplete. |
| Breathstealer's Crypt | 7 | "If a player would draw a card, instead they draw a card and reveal it. If it's a creature card, that player discards it unless they pay 3 life": blank enchantment |
| City of Solitude | 7 | "Players can cast spells and activate abilities only during their own turns": a blank enchantment (its test only casts it) |
| Corrosion | 7 | Rust counters affect every opponent rather than target opponent, and the mandatory leave-battlefield counter removal is missing. |
| Desolation | 7 | "At the beginning of each end step, each player who tapped a land for mana this turn sacrifices a land ... 2 damage to each player who sacrificed a Plains": a blank enchantment (its test only casts it) |
| Elkin Lair | 7 | "At the beginning of each player's upkeep, that player exiles a card at random from their hand. The player may play that card this turn. At the beginning of the next end step, if the player hasn't played the card, they put it into their graveyard.": a blank world enchantment (the test only casts it) |
| Equipoise | 7 | "for each land target player controls in excess of the number you control, choose a land that player controls, then the chosen permanents phase out. Repeat for artifacts and creatures": blank enchantment |
| Eye of Singularity | 7 | "When this enters, destroy each permanent with the same name as another permanent, except basic lands. ... Whenever a permanent other than a basic land enters, destroy all other permanents with that name. They can't be regenerated.": a blank world enchantment (the test only casts it) |
| Infernal Harvest | 7 | "return X Swamps ... deals X damage": the card dealt a flat 1 damage even when there was no Swamp to return (X=0 deals 0): stronger than printed |
| Kaervek's Spite | 7 | "As an additional cost ... sacrifice all permanents you control and discard your hand": the hand is a real cost, but the sacrifice runs on resolution, so a countered or fizzled Spite sacrifices nothing and the permanents stay usable in response: stronger than printed in that case, which the reason did not say. The test asserted "no permanent is controlled by p0", which also passes on an empty battlefield, and never looked at the opponent's side, the life totals or the graveyard |
| Katabatic Winds | 7 | The mandatory flying-creature tap-ability restriction is omitted while retaining its symmetric attack/block lock. |
| Spatial Binding | 7 | "Until your next upkeep, target permanent can't phase out": the ability is `AddKeyword(EMPTY)` on the Aura-less source for `UntilYourNextTurn`, a no-op that pays 1 life for nothing; nothing else on the card |
| Superior Numbers | 7 | The required target opponent is omitted, bypassing player targeting restrictions and target-trigger interactions. |
| Teferi's Imp | 7 | Phasing remains while its mandatory phase-out discard is omitted; retaining the body bypasses a printed drawback. |
| Tidal Wave | 7 | "Sacrifice it at the beginning of the next end step" missing: the 5/5 Wall defender token stayed forever |
| Tombstone Stairwell | 7 | Each opponent receives the sum of all opponents graveyard counts, and cleanup destroys copied Tombspawn not created by this enchantment. |
| Torrent of Lava | 7 | "As long as this is on the stack, each creature has '{T}: Prevent the next 1 damage … by Torrent of Lava'" skipped: a printed drawback of the spell, so X damage to every non-flier is stronger than printed |
| Waiting in the Weeds | 7 | Each opponent receives tokens counted from all opponents Forests, rather than that recipient own untapped Forests. |
| Zombie Mob | 7 | "exile all creature cards from your graveyard": the card exiled every card (instants and lands too), the Partial reason admitted it; stripping the exile instead would skip the drawback |
| Alms | 8 | Printed: exile the TOP card of your graveyard. Card: exile any card (`ExileFromGraveyard(Any)`): a free choice of which card to keep, stronger than printed |
| Barrow Ghoul | 8 | "unless you exile the top creature card of your graveyard" → the player picked ANY creature card in the graveyard (`ExileFromGraveyard` is a choice): a different card than printed, so the 4/4 for {1}{B} pays an easier price |
| Bösium Strip | 8 | The activation costs {3},{T} and does nothing (empty effect list): a dead ability, the card pays for nothing |
| Lichenthrope | 8 | "If damage would be dealt to this creature, put that many -1/-1 counters on it instead" → missing (a drawback): took damage as damage, never shrank, only removed counters it never gets; a 5/5 for 5 stronger than printed |
| Necromancy | 8 | flash with no "sacrifice at next cleanup if cast when a sorcery could not have been cast", and no Aura link ("when this leaves, that creature's controller sacrifices it") → permanent instant-speed reanimation, stronger than printed |
| Phyrexian Marauder | 8 | "can't attack unless you pay {1} for each +1/+1 counter on it" → missing (a drawback): the creature attacked for free with X counters, stronger than printed |
| Rowen | 8 | No ability at all: a 4-mana enchantment that does nothing (reason: "enchantment on the battlefield") |
| Sands of Time | 8 | Printed: skip untap, and each upkeep flip tapped/untapped artifacts, creatures and lands. The card keeps only the skip, so nothing ever untaps: a harsher lock than printed (wrong, not partial). Stripping it leaves a blank artifact |
| Shimmering Efreet | 8 | Mandatory phase-in creature targeting/phasing is omitted; as the sole creature it must target and phase itself out. |
| Squandered Resources | 8 | Printed: mana of a type the sacrificed land could produce. Card: any color from any land, so a Forest makes {U} (wider, wrong not partial) |
| Teferi's Puzzle Box | 8 | Printed: each player, whole hand to the bottom, draw that many. Card: only the controller's draw step, any number, optional; one-sided card selection the opponent does not get |
| Teferi's Realm | 8 | No ability at all: a 3-mana World enchantment that does nothing |
| Three Wishes | 8 | The face-down exile and mandatory next-upkeep graveyard destination of unplayed cards are missing, leaking information and preserving an unprinted exile state. |
| Time and Tide | 8 | The simultaneous phase-in half is missing, so phased-out opposing creatures do not return while others are removed. |
| Circling Vultures | 9 | "sacrifice this creature unless you exile the **top** creature card of your graveyard" → the cost lets the player pick any creature card, so the best card stays safe (a different card than printed; "do less" does not cover it); header lacked `// PARTIAL`. I first kept it Partial and wrote tests, then applied the brief's "different card than printed is wrong" rule |
| Cone of Flame | 9 | Three distinct mandatory targets and damage amounts 1/2/3 are reduced to one 3-damage target, permitting casts that avoid required self-damage or lack three legal targets. |
| Desperate Gambit | 9 | "Choose a source you control and flip a coin; won: its next damage this turn is doubled; lost: its next damage is prevented" → won flip does nothing; lost flip lets the caster pick ANY source (opponent's too) and prevents ALL its damage for the turn, a pure benefit the lost flip never gives |
| Doomsday | 9 | "Search your library and graveyard for five cards and exile the rest. Put the chosen cards on top … You lose half your life" → only the life loss is implemented: casting it is a pure loss with none of the card |
| Empyrial Armor | 9 | "+1/+1 for each card in your hand" missing → Aura that gives nothing (blank card). Test is the "cast, does nothing" kind |
| Ertai's Familiar | 9 | Mandatory mill three whenever it phases out is missing; removing phasing temporarily also differs from "cannot phase out". Missing mill is a drawback, so a playable Partial can be stronger than printed |
| Familiar Ground | 9 | "Each creature you control can't be blocked by more than one creature" missing → blank enchantment (no rule left) |
| Harvest Wurm | 9 | "sacrifice it unless you return a basic land card from your graveyard to your hand" → the card only offered the return as optional and never sacrificed itself: a free 3/2 for {1}{G} (header admitted the missing sacrifice) |
| Heart of Bogardan | 9 | "When a player doesn't pay this enchantment's cumulative upkeep, it deals X damage to target player or planeswalker and each creature that player controls" → missing; the card was a cumulative upkeep that sacrifices itself and nothing else (pure drawback, test only declined the upkeep) |
| Heat Stroke | 9 | Destroys only blockers, sparing the controller's blocked attackers: removes the printed symmetric destruction drawback |
| Liege of the Hollows | 9 | Body-only creature omits symmetric death instruction allowing opponents to pay mana for Squirrels, a downside of the body; omission is not an honest Partial |
| Mana Web | 9 | "tap all lands that player controls **that could produce any type of mana that land could produce**" → taps ALL of that player's lands, so a Forest tapped for mana also taps their Mountains (more than printed; the Partial reason calls it a missing filter but the card hits more objects). Test only used a Plains-only board |
| Nature's Kiss | 9 | "Exile the **top** card of your graveyard" → cost exiles any card of the graveyard (a different card than printed; with the pump free of the top-of-graveyard order) |
| Nature's Resurgence | 9 | Each opponent draws for ALL opponent creature graveyards summed, so it overdraws at 3+ seats; retaining only caster draw would remove the printed symmetric opponent benefit |
| Noble Benefactor | 9 | "Each player may search" becomes mandatory search and shuffle. Taking a card is not always beneficial; forcing an opponent's search changes the rules. Body-only omission would remove opponents' search benefit |
| Paradigm Shift | 9 | "**Exile all cards from your library.** Then shuffle your graveyard into your library" → the exile step is missing: the card only adds the graveyard to the library (a different effect that keeps the library; stronger than printed in nearly every use) |
| Psychic Vortex | 9 | Declining cumulative-upkeep draw keeps the permanent instead of sacrificing it, skipping the printed upkeep payment while preserving later uses |
| Tariff | 9 | "Each player sacrifices the creature **they control with the greatest mana value** unless they pay that creature's mana cost; tied: that player chooses" → each player sacrifices a creature of their own choice (their cheapest), and there is no payment option |
| Teferi's Veil | 9 | Grants creatures an attack trigger instead of triggering from Veil; ability removal can erase mandatory end-of-combat phasing and spare the creature that printed Veil would phase out |
| Thran Tome | 9 | "Reveal the top three cards. **Target opponent chooses** one of those cards. Put that card into your graveyard, then draw two" → mills the top card at random, no reveal and no opponent choice, so the controller keeps the best of three (the opponent's choice is the cost of the ability) |
| Urborg Justice | 9 | Condition counts every creature death, including opponent-only deaths; printed count is creatures put into YOUR graveyard from battlefield this turn. Also only sacrifices one |
| Veteran Explorer | 9 | "Each player may search" forces inspection and shuffle for everyone even if a player finds zero; optional before searching matters (including search punishments) |
| Wave of Terror | 9 | Mana-value sweep omits "They can't be regenerated", allowing controller to save own matching creatures. Removing sweep leaves pure upkeep drawback |
| Abandon Hope | 10 | "discard X cards … choose X cards" → the card always made the opponent discard a card, even for X = 0 (stronger than printed: a 1-for-1 for {1}{B}, where printed X = 1 costs {2}{B}); the additional discard is one card, not X |
| Aluren | 10 | Flash was unbounded; remaining generic3 discount can erase added taxes on MV0 creatures whereas free casting still pays them |
| Bellowing Fiend | 10 | Partial reason accurate (combat damage only). Test used `index::SERRA_ANGEL` and only a blocked attack |
| Booby Trap | 10 | "When the chosen player draws a card with the chosen name, sacrifice this artifact. If you do, 10 damage" → trigger had no name check, so the first card any opponent draws sacrificed it and dealt 10 (an instant kill on the next draw step); "reveals each card they draw" and "other than a basic land name" also absent |
| Corpse Dance | 10 | "Return the **top** creature card of your graveyard … gains haste … Exile it at the beginning of the next end step" → the player picks ANY creature card and it stays for good: a permanent reanimation at instant speed (the drawback is the whole price). Same shape as Shallow Grave |
| Dracoplasm | 10 | Counting sacrificed creatures can exceed actual total power/toughness for 0-power creatures |
| Echo Chamber | 10 | Sacrificing the copied token instead of exiling it can trigger dies benefits; the earlier fair-Partial verdict was unsafe |
| Ertai's Meddling | 10 | Permanent exile drops the return-after-X-upkeeps restriction, making it stronger |
| Extinction | 10 | Destroy-all ignored chosen creature type and killed extra creatures |
| Furnace of Rath | 10 | Unprinted +1/+0 anthem substituted for damage doubling |
| Legacy's Allure | 11 | The target power bound is absent; steals an arbitrary creature with no counters. |
| Living Death | 11 | Reanimates directly and omits exile and sacrifice of all current creatures. |
| Lobotomy | 11 | Discards a chosen card instead of exiling that name from three zones. |
| Mana Severance | 11 | Searched lands go to graveyard rather than exile. |
| Mogg Conscripts | 11 | Any spell incorrectly unlocks the attack requirement; no creature-cast condition. |
| No Quarter | 11 | Granted triggers give creatures abilities and put the source/controller on the wrong permanent. |
| Phyrexian Grimoire | 11 | Random retrieval skips opponent choosing the top two and exiling one. |
| Propaganda | 11 | Attack tax protects other players and planeswalkers as well as you. |
| Reap | 11 | Returns a card even with zero black opposing permanents and omits opponent target. |
| Recycle | 11 | Omits skipped draw step and hand limit, leaving only free draws. |
| Scorched Earth | 11 | Pays one discarded land for arbitrary X, instead of X lands. |
| Scroll Rack | 11 | Draw then choose a card can take the newly drawn card; printed cards must first be exiled. |
| Spinal Graft | 11 | Granted trigger puts its source/controller on the enchanted creature; cannot safely omit drawback. |
| Torture Chamber | 11 | Removes counters on resolution instead of as cost; countered ability retains them. |
| Volrath's Curse | 11 | Omits enchanted controller sacrificing a permanent to ignore the restrictions. |
| Whim of Volrath | 11 | Text change is permanent instead of until end of turn. |
| Cannibalize | 12 | Exiles one creature without the required second same-controller target or counter benefit. |
| Crovax the Cursed | 12 | Declining upkeep sacrifice never removes a counter. |
| Dream Halls | 12 | Blank enchantment offered as playable. |
| Flame Wave | 12 | Damages every opponents creatures rather than target players creatures; self and multiplayer cases are wrong. |
| Hidden Retreat | 12 | Blank enchantment offered as playable. |
| Hornet Cannon | 12 | Creates permanent Hornets; printed next-end-step destruction missing. |
| Invasion Plans | 12 | Defender chooses blocks; attacking player must choose. |
| Mask of the Mimic | 12 | Searches any creature instead of same-name card. |
| Mogg Flunkies | 12 | 3/3 attacks and blocks alone despite mandatory restriction. |
| Mogg Infestation | 12 | Destroys all opponents creatures despite chosen player and grants no replacement Goblins. |
| Portcullis | 12 | Its per-exile delayed return is incorrectly implemented as a removable printed leaves-battlefield ability; removing abilities can permanently strand exiled creatures. |
| Rebound | 12 | Retargets spells aimed at creatures and may retarget to creatures. |
| Reins of Power | 12 | Untaps all opponents; skips control exchange and lets caster keep creatures. |
| Sacred Ground | 12 | Returns lands destroyed by your own spells and sacrifices; opponent-caused requirement missing. |
| Samite Blessing | 12 | Next-one-damage shield has no chosen-source restriction. |
| Spined Sliver | 12 | A granted trigger changes the printed global trigger source/controller and can be removed from the affected Sliver; removing it entirely would omit a mandatory opponent benefit. |
| Volrath's Laboratory | 12 | Creates colorless typeless tokens instead of chosen color and type. |
| Aether Tide | 13 | X creature discard cost absent |
| Cataclysm | 13 | Blank spell; per-type keep choices absent |
| Coat of Arms | 13 | Blank artifact; shared-type count absent |
| Dizzying Gaze | 13 | Aura deals damage instead of enchanted creature |
| Entropic Specter | 13 | Fixed 1/1 is stronger than chosen hand size 0 |
| Kor Chant | 13 | Prevention replaces source-limited redirection |
| Limited Resources | 13 | Blank enchantment; land-sacrifice ETB absent |
| Manabond | 13 | Up-to-one land permits withholding land cards from printed all-land instruction |
| Memory Crystal | 13 | Blank artifact; buyback cost reduction absent |
| Oath of Druids | 13 | No relative-count target; wrong player; wrong reveal destination |
| Oath of Ghouls | 13 | No relative-count target; graveyard choice incorrectly targets |
| Oath of Lieges | 13 | No relative-count target; other players omitted |
| Oath of Mages | 13 | No relative-life target; other players omitted |
| Oath of Scholars | 13 | No relative-hand-size target; other players omitted |
| Pandemonium | 13 | Opponent creature triggers are absent from a symmetric enchantment |
| Paroxysm | 13 | Unconditional pump skips top-card land destruction |
| Penance | 13 | Blank enchantment; hand-to-library cost absent |
| Skyshroud War Beast | 13 | CDA sums all opponents instead of chosen player |
| Soltari Visionary | 13 | The omitted mandatory damage trigger can require destruction of the controller enchantment after damage to that controller; it is not a purely optional benefit. |
| Sonic Burst | 13 | Mandatory random discard cost absent |
| Spike Cannibal | 13 | The omitted mandatory entry transfer also removes counters from allied creatures; retaining its own counter spares those printed losses. |
| Abundance | 14 | Draw replacement absent; test only casts a blank enchantment. |
| Angelic Chorus | 14 | Creature ETB toughness life gain absent; test only casts a blank enchantment. |
| Antagonism | 14 | Unless an opponent of that player was damaged → unconditional end-step damage. |
| Argothian Wurm | 14 | Any player may sacrifice a land → only one opponent asked; other players cannot pay the drawback. |
| Contamination | 14 | Tapped land produces one black instead → normal mana untouched; own mana never restricted. |
| Defensive Formation | 14 | Defender assigns attacker damage absent; test only casts blank enchantment. |
| Diabolic Servitude | 14 | Linked creature must exile on death/source departure → unrestricted permanent reanimation. |
| Disorder | 14 | Each white creature and their controllers damaged → omits damage to own player as well as opponents. |
| Energy Field | 14 | Card to your graveyard from anywhere → only controlled deaths/discards; death tokens falsely count. |
| Exhume | 14 | One creature card per player → every creature reanimated. |
| Fertile Ground | 14 | Its controller adds mana → Aura controller chooses/receives it. |
| Flesh Reaver | 14 | Damage to creature or opponent → opponent damage only; 4/4 body evades creature-damage drawback. |
| Fluctuator | 14 | Cycling-only cost reduction absent; blank artifact cast test. |
| Greener Pastures | 14 | Active player strictly most lands → unconditional token on your upkeep only. |
| Hidden Herd | 14 | Opponent plays nonbasic land → triggers on basic lands too; test explicitly uses Forest. |
| Hidden Stag | 14 | Becomes an enchantment → AddType/RemoveType preserves extra card types and Elk/Beast subtypes. |
| Ill-Gotten Gains | 14 | Discard then each player recovers up to three → opponents lose entire hand without recovery. |
| Lifeline | 15 | Immediately returned dead creature instead of scheduling its exact graveyard incarnation at next end step |
| Lurking Evil | 15 | Paid 1 life instead of half life rounded up |
| No Rest for the Wicked | 15 | Returned milled/discarded creature cards, not only battlefield deaths this turn |
| Noetic Scales | 15 | Blank artifact; cannot compare each active-player creature power with that player hand size |
| Okk | 15 | Missing both mandatory greater-power joint attack/block restrictions |
| Outmaneuver | 15 | Trample substituted for assigning combat damage as though unblocked |
| Pendrell Flux | 15 | Generic {1} substituted for the enchanted creature exact mana cost |
| Persecute | 15 | Arbitrary single discard could take colorless cards; omitted chosen-color mass discard |
| Phyrexian Processor | 15 | Unpaid entry life and unrelated fixed Zombie token replaced linked Phyrexian Minion X/X |
| Planar Birth | 15 | Returned lands untapped instead of tapped |
| Planar Void | 15 | Opponents-only graveyard replacement substituted for all-player graveyard-entry trigger |
| Purging Scythe | 15 | Hit every creature, ignoring least-toughness choice |
| Remembrance | 15 | Searched any creature, not the dead creature name |
| Serra Avatar | 15 | 0/0 died automatically; missing life CDA and all-zone graveyard trigger |
| Show and Tell | 15 | Repeated placement allowed multiple cards per player |
| Smokestack | 15 | Sacrificed one at zero soot and omitted larger mandatory sacrifices |
| Sneak Attack | 15 | Omitted mandatory sacrifice of the creature put into play |
| Sporogenesis | 15 | Fixed one-token output and missing fungus-counter cleanup |
| Sulfuric Vapors | 15 | Unrelated red-creature anthem substituted for red-spell damage replacement |
| Telepathy | 15 | Revealed the controller hand as well as opponents |
| Urza's Armor | 15 | Capped damage at one rather than preventing one |
| Angel's Trumpet | 16 | Omitted damage for idle creatures while granting vigilance. |
| Crawlspace | 16 | Global cap restricted controller attacks and attacks at other players. |
| Damping Engine | 16 | Blank artifact, missing restrictions and sacrifice special action. |
| Harmonic Convergence | 16 | Returned enchantments to hand instead of library top. |
| Intervene | 16 | Countered any spell, ignoring creature target restriction. |
| Last-Ditch Effort | 16 | Sacrificed all creatures, not any chosen number; counted deaths rather than sacrifices. |
| Lurking Skirge | 16 | Not owned by you includes teammate-owned creatures; oracle requires opponent graveyard. |
| Memory Jar | 16 | Discard replaced linked face-down exile and delayed restoration. |
| Multani's Presence | 16 | Blank enchantment, missing spell-countered trigger. |
| Multani, Maro-Sorcerer | 16 | Missing all-hand CDA leaves a dead 0/0 that immediately dies. |
| Pyromancy | 16 | Fixed one damage hits even if a land was discarded. |
| Vile Requiem | 16 | Zero counters could destroy one creature; fixed count replaced counter-derived target limit. |
| Waylay | 16 | Temporary Knights became permanent; cleanup scheduler cannot retain the created group. |
| Archery Training | 17 | fixed one damage at zero arrow counters; cannot refer to granting Aura counters |
| Compost | 17 | controller substituted destination graveyard owner; tokens treated as cards |
| Donate | 17 | recipient ignored targeted player and always named an opponent |
| Fatigue | 17 | spell targeted a player but did nothing |
| Rivalry | 17 | unconditional damage omitted active-player land comparison |
| Treacherous Link | 17 | damage prevention replaced redirection and omitted controller damage |
| Viashino Bey | 17 | body omitted conditional mandatory attack drawback |

## Review coverage and engine support

The six independent Astra audits cover packages 1–3 (238 cards), 4–6 (236),
7–9 (257), 10–12 (244), 13–14 (168), and 15–17 (207). Confirmed findings
were resolved by an exact correction, removal of a whole optional positive
ability, or a canonical hold. A second diff review checked the 71 retained
changed card-test files in the runtime-repair commit, including the unchanged
printed-rule assertions. Static review and executed tests are separate evidence.

Supporting fixes retain permanent-damage amount/recipient and the exact object
incarnation used by delayed effects; honor combat assignment restrictions for
blockers and reveal previews; count separately granted flanking instances;
keep resolution-local chosen names after source departure; and bind “your next
untap step” to the resolving ability's controller. The untap fix uses existing
DSL durations and updates Samite Alchemist, Reveka, Spectral Bears, Deep Spawn
and Homarid Warrior. Its 44-test related suite passes; restoring the old
consumer fails seven positive regressions while the affected-controller
negative control still passes.

Explicit `TargetReq::Player(You)` requirements now participate in target
enumeration, reachability and resolution rechecking, while implicit self-effects
stay implicit. Sacrifice costs retain the original source incarnation and its
last battlefield characteristics through both stacked and immediate mana
abilities. Refreshing characteristics between completed cost parts includes
counters actually remaining at sacrifice; the simultaneous-sacrifice control
keeps a lord's bonus for the entire event. The 64-test related suite is green.

The mana sweep passes all nine tests, including per-card evidence for each
population change. Both its census and executed sweep reach 379 readable
faces, with 213 promises and no skipped boards. The existing reader's omission
of “adds” wording is documented separately as observed-faults #65; agreement
between counters sharing that reader does not establish coverage of those
lines. AI census expectations retain exact membership and negative controls.

The selective hold-back generator validates pinned printing identity and plans
all selected writes before mutation. The line mapper now recognizes narrowed
self triggers, paid activations granted by resolving spells, and unambiguous
retained modal subsets without shifting labels across omitted bullets. Full codegen
was not run without the external corpus.

## Measured baseline

Native `site card-status --no-fetch` reads committed sources with `git archive`
and validates ledger/pool/core-index/exclusions before counting. Original PR
head `896bdff1a0fd` had 10,693 definitions: 10,014 Implemented, 635 Partial and
44 Unimplemented. Candidate `4975f5442` has the same 10,693 definitions:
10,058 Implemented, 294 Partial and 341 Unimplemented. No Generated cards
remain. Corpus identity/exclusion totals are unchanged. These are whole-pool
counts, distinct from the 1,350 reviewed identities above.

## Validation and landing

`cargo lint-rules` passes. The broad `cargo test-rules --no-fail-fast`
population contains 20,744 tests, with 17 existing skips. Its 20,286 passing
tests include all 14,816 engine tests, 357 codegen tests, 151 card-pool tests,
274 AI tests and 264 seat tests. The remaining 458 failures were all tests
requiring PostgreSQL: this invocation omitted `DATABASE_URL`. Repeating all
four affected packages with the documented local database passes **848/848**
tests, with six existing skips. This closes every failure from that run; it
does not describe an unconfigured command as green.

Both `codegen --tables` passes, `scryfall-cache`, `xtask validate`, the required
card-test guard and native `site card-status-fresh` pass. No assertions were
weakened to accept a missing printed cost, target, drawback or payoff.

The earlier engine-only diagnostic run passed 14,756 tests and exposed 46
failures; the subsequent broad run exposed eight card/census/provenance/fixture
failures. Those are repair history, not accepted exceptions. All retained
card scenarios, exact census lists, modal provenance, basic-land guards and
narrator fixtures now pass. Canonical held cards have their obsolete played
card tests removed by codegen.

The first full workspace gate reached 22,949 tests: 22,947 passed and two
client checks failed (25 existing skips). Witch Engine's targeted activation
was incorrectly classified as a mana line because its first instruction adds
mana; the reader now checks all immediate sentences for targets while keeping
later triggered riders separate. Its CardDef and coverage are unchanged.
The other failure was the client target preflight's coarse blind-spot budget.
An exact nine-member census now replaces that budget, preserving the structural
negative controls and population floor. New boundaries distinguish unknown
history/X for an existing creature from a genuinely empty creature population.
No hidden state or unannounced X is inferred from the public view. These known
preflight gaps remain documented rather than being disguised as readable rules.
The diagnostic landing run was stopped after the red full gate; the repaired
commit must pass all three local gates afresh before it can land.

The targeted repair checks pass: **360 codegen tests, 39 client targeting
tests, the full written-row sentence check, and Clippy for both affected
crates with all targets**. The tables were regenerated twice and change only
Witch Engine's missing ability line. Both fixes and the supporting docs were
independently reviewed by Astra.

The final landing command is `DATABASE_URL=… cargo dev land`. It fetches and
incorporates current main under the shared cargo lock, runs **all three**
required local gates (`gate`, `gate features`, `gate wasm`) and pushes only
the exact checked commit if all three pass. A failed gate prevents landing;
CI is not used as a substitute for these local gates. This report, the native
coverage JSON and the continuation brief are included in that checked commit.
