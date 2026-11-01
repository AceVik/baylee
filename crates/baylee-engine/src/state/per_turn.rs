//! What a turn remembers: the per-turn tallies and records, play permissions, and the previous turn.

use crate::object::CounterKind;
use baylee_core::ids::{ObjectId, PlayerId};
use std::hash::Hash;

/// Per-turn counters for conditional triggers (reset at every turn start).
#[derive(Clone, Hash, Debug)]
#[allow(clippy::struct_excessive_bools)]
pub struct PerTurn {
    /// Untapped lands controlled by this turn's active player at its start,
    /// before untapping or phasing; independent of any source's presence.
    pub untapped_lands_at_start: u32,
    /// Cards in the active player's hand at this turn's boundary.
    pub hand_size_at_start: u32,
    /// Main phases actually begun this turn; the first need not be precombat.
    pub main_phases_begun: u32,
    /// A permanent card actually arrived in each owner’s graveyard this turn.
    pub descended: Vec<bool>,
    /// Whether the inherent speed ability has triggered for each seat this turn.
    pub speed_triggered: Vec<bool>,
    /// Actual sacrifices, counted once per projected subtype and sacrificing seat.
    pub sacrificed_subtypes:
        std::collections::BTreeMap<(PlayerId, baylee_core::ids::SubtypeId), u32>,
    /// Whether a permanent left under each player's control this turn.
    pub revolt: Vec<bool>,
    /// Whether each player put a nontoken permanent onto the battlefield
    /// this turn (Arboria): the player who put it there, not the one it
    /// entered under (the card's 2022-12-08 ruling). That is the putter of
    /// the event (`entry::Entries::putter`: the controller of the resolving
    /// spell or ability, the player playing a land), and the player it
    /// entered under where no event names one.
    pub put_nontoken_permanent: Vec<bool>,
    /// Whether each player's next spell this turn has improvise (Archway of
    /// Innovation, `Effect::NextSpellThisTurnHasImprovise`); spent by that
    /// player's next cast.
    pub improvise_next_spell: Vec<bool>,
    /// Whether each player cast an instant or sorcery this turn.
    pub instant_or_sorcery_cast: Vec<bool>,
    /// Source incarnations and players they actually damaged this turn.
    pub player_damage: std::collections::BTreeSet<(baylee_core::ids::DamageSourceRef, PlayerId)>,
    /// Noncreature spells cast this turn, per player.
    pub noncreature_spells: Vec<u32>,
    /// Cards drawn this turn, per player.
    pub draws: Vec<u32>,
    /// Whether the active player has drawn a card in this turn's draw step.
    /// Only the active player has a draw step (CR 504.1), and a turn has one,
    /// so the turn's reset starts each draw step clean. Only a draw made in
    /// that step sets it: a card drawn in the upkeep does not.
    pub drew_in_draw_step: bool,
    /// All spells cast this turn, per player (second-spell triggers).
    pub spells_cast: Vec<u32>,
    /// Completed instant casts per player, independent of spell resolution.
    pub instants_cast: Vec<u32>,
    /// Union of subtypes controlled at actual entry during this turn.
    pub entered_subtypes: Vec<baylee_core::types::SubtypeSet>,
    /// Creature arrivals under each player's control, including later departures.
    pub creatures_entered: Vec<u32>,
    /// Arrivals awaiting the complete simultaneous battlefield projection.
    pub(crate) pending_entry_history: Vec<baylee_core::ids::DamageSourceRef>,
    /// Exact graveyard successors of actual discards, paired with their discarder.
    pub discarded_to_graveyard: Vec<(baylee_core::ids::DamageSourceRef, PlayerId)>,
    /// Players who may cast no further spells this turn (Conduit of
    /// Worlds: "If you do, you can't cast additional spells this turn").
    /// Read by `casting::may_begin_casting`, written by the cast that
    /// set it.
    pub no_more_spells: Vec<bool>,
    /// Whether each player lost life this turn (Luminarch Ascension).
    /// Written by [`GameState::change_life`] and nothing else.
    pub life_lost: Vec<bool>,
    /// Cumulative loss, independent of later life gains, for counted costs.
    pub life_lost_amount: Vec<u32>,
    /// Cumulative life actually gained this turn; later losses do not subtract.
    pub life_gained_amount: Vec<u32>,
    /// The damage dealt to each player this turn, per seat ("the damage
    /// dealt to you this turn", Simulacrum). Written by
    /// [`GameState::damage_player`] and nothing else.
    pub damage_dealt_to: Vec<u32>,
    /// Actual positive permanent damage, deduplicated by both incarnations.
    /// This historical fact survives cleanup; reset only as the turn ends.
    pub(crate) permanent_damage: Vec<crate::damage_history::DamageRecord>,
    /// Exact source and graveyard successor for creatures that died after damage.
    pub(crate) damaged_creature_deaths: Vec<crate::damage_history::DamagedCreatureDeath>,
    /// Creatures that died this turn, all players (Emeritus of Woe's
    /// re-prepare condition).
    pub creatures_died: u32,
    /// Positive counter placements, keyed by exact recipient and kind.
    pub counter_placements:
        std::collections::BTreeSet<(baylee_core::ids::DamageSourceRef, CounterKind)>,
    /// What entered the battlefield this turn, in arrival order
    /// (`Filter::EnteredThisTurn`). Written where a `ZoneChanged` into the
    /// battlefield is journaled: [`GameState::move_object`] and a token's
    /// arrival.
    pub entered_battlefield: Vec<ObjectId>,
    /// Every card drawn this turn, as the object it became in its drawer's
    /// hand: the id and the version it arrived with. Sylvan Library's
    /// "cards in your hand drawn this turn" reads it; an entry whose version
    /// no longer matches the object's is a card that has left the hand since
    /// (CR 400.7) and is no longer one of them. Written by
    /// [`GameState::draw_cards`] and nothing else.
    pub drawn: Vec<(ObjectId, u32)>,
    /// Draws performed by the original instruction, excluding replacement draws (CR 614.11b).
    pub instruction_drawn: Vec<(ObjectId, u32)>,
    /// How many times each ability of each object has resolved this turn
    /// (Nissa, Resurgent Animist's "the second time this ability has
    /// resolved this turn"). Written by the stack's resolution of an
    /// ability and nothing else; see [`AbilityResolved`].
    pub resolved: Vec<AbilityResolved>,
    /// What each Muldrotha-style allowance has let its controller play from
    /// the graveyard this turn ([`GraveyardPlay`]).
    pub graveyard_plays: Vec<GraveyardPlay>,
    /// Successful casts under a particular once-per-turn graveyard allowance.
    pub graveyard_allowances_used: Vec<(PlayerId, baylee_core::ids::EffectId)>,
    /// What was put into a graveyard this turn, from anywhere, in arrival
    /// order (`Filter::PutIntoGraveyardThisTurn`). Written by
    /// [`GameState::move_object`], beside `entered_battlefield`.
    pub entered_graveyard: Vec<ObjectId>,
    /// The objects that are exiled if they would die this turn, each with
    /// its version (`Effect::ExileIfDiesThisTurn`, CR 400.7): read by
    /// `replacement::graveyard_destination`.
    pub exile_if_dies: Vec<(ObjectId, u32)>,
    /// Camouflage resolved this turn: "instead of declaring blockers, each
    /// defending player … divides them into a number of piles" (read by
    /// `Engine::ask_blockers`).
    pub camouflage: bool,
    /// The permanents no regeneration shield is applied to this turn, each
    /// with its version (`Effect::CantBeRegeneratedThisTurn`, CR 701.19c):
    /// read by `sba::destroy`.
    pub cant_regenerate: Vec<(ObjectId, u32)>,
    /// The creatures declared as attackers this turn, each with its
    /// version (`Filter::AttackedThisTurn`, CR 508.1): written by
    /// `Engine::declare_attackers` and nothing else, so a creature put onto
    /// the battlefield attacking is not in it (CR 508.4).
    pub attacked: Vec<(ObjectId, u32, PlayerId)>,
    /// Every block this turn: blocker and attacker as the objects they were
    /// (CR 400.7), and who controlled the attacker as it became blocked
    /// (Glyph of Reincarnation's "the player who controlled that creature
    /// the last time it became blocked by that Wall"). Written by
    /// `GameState::declare_block` and nothing else.
    pub blocks: Vec<BlockRecord>,
    /// Every positive amount of damage dealt this turn, after prevention,
    /// with its source as it was when it dealt it (`Amount::DamageThisTurn`,
    /// `Filter::DealtDamageToSourceThisTurn`, Backdraft).
    pub damage_log: Vec<DamageLogEntry>,
    /// Creatures that could have attacked during this turn's declare attackers
    /// step (Season of the Witch).
    pub could_have_attacked: Vec<(ObjectId, u32)>,
    /// `(target_player, drawn_card)` pairs watched by Wand of Ith this turn.
    pub wand_of_ith_targets: Vec<(PlayerId, ObjectId)>,
    /// `(source, version)` pairs whose damage to creatures prevents them from
    /// regenerating this turn.
    pub damage_cant_regenerate_sources: Vec<(ObjectId, u32)>,
    /// `(source, version)` pairs whose damage to creatures causes them to be
    /// exiled instead if they die this turn.
    pub damage_exile_if_dies_sources: Vec<(ObjectId, u32)>,
    /// `(ability_source, tapped_object)` pairs recording permanents tapped this
    /// turn to pay an activation cost of `ability_source` (Vodalian War Machine).
    pub tapped_for_ability: Vec<(
        baylee_core::ids::DamageSourceRef,
        baylee_core::ids::DamageSourceRef,
    )>,
    /// Active player skips their draw step this turn (Fatespinner).
    pub skip_draw_step: bool,
    /// Active player skips their main phases this turn (Fatespinner).
    pub skip_main_phases: bool,
    /// Active player skips their combat phases this turn (Fatespinner).
    pub skip_combat_phases: bool,
}

/// One block declared this turn ([`PerTurn::blocks`]).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct BlockRecord {
    /// The blocking creature as it was.
    pub blocker: baylee_core::ids::DamageSourceRef,
    /// The creature it blocked as it was.
    pub attacker: baylee_core::ids::DamageSourceRef,
    /// Who controlled the attacker as it became blocked.
    pub attacker_controller: PlayerId,
}

/// One amount of damage dealt this turn ([`PerTurn::damage_log`]).
#[derive(Clone, Debug)]
pub struct DamageLogEntry {
    /// The source as the object it was.
    pub source: baylee_core::ids::DamageSourceRef,
    /// The source as it was when it dealt the damage: its characteristics
    /// and controller then, which "by artifacts" and "red sources you
    /// controlled" are asked of.
    pub snapshot: std::sync::Arc<crate::object::GameObject>,
    /// What was dealt it.
    pub to: crate::event::DamageTarget,
    /// The recipient's incarnation, when it is an object.
    pub to_version: Option<u32>,
    /// How much.
    pub amount: u32,
    /// Combat damage.
    pub combat: bool,
}

impl std::hash::Hash for DamageLogEntry {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (
            self.source,
            self.to,
            self.to_version,
            self.amount,
            self.combat,
        )
            .hash(state);
    }
}

/// One card played or cast from a graveyard under a
/// `Modifier::PermanentOfEachTypeFromGraveyard` allowance.
///
/// The allowance is the source's, as the object it is (CR 400.7): a Muldrotha
/// that left and came back gives a fresh one.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GraveyardPlay {
    /// Who played it.
    pub player: PlayerId,
    /// The permission's source.
    pub source: ObjectId,
    /// The source's version.
    pub version: u32,
    /// The card's permanent types: `LAND` for a land played, the rest for a
    /// spell cast, of which it used one.
    pub types: baylee_core::types::TypeSet,
}

/// One ability of one object, and how many times it has resolved this turn.
///
/// The object is its id **and** version: an object that changed zones is a
/// new object (CR 400.7), and its abilities have resolved no times yet.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct AbilityResolved {
    /// The ability's source.
    pub source: ObjectId,
    /// The source's version when the ability resolved.
    pub version: u32,
    /// Index into the source's ability list.
    pub index: u32,
    /// Resolutions this turn.
    pub times: u32,
}

/// Runtime lifetime of a permission to play an exact object.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum PlayLifetime {
    /// Expires with the benefiting player's actual turns.
    Turns(baylee_cards_dsl::PlayWindow),
    /// The exact permanent must remain present and prepared (CR 722.3c).
    Prepared(baylee_core::ids::DamageSourceRef),
}

impl From<baylee_cards_dsl::PlayWindow> for PlayLifetime {
    fn from(window: baylee_cards_dsl::PlayWindow) -> Self {
        Self::Turns(window)
    }
}

/// Permission to play one exact card incarnation (CR 400.7).
/// Its lifetime is independent of per-turn counters.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PlayPermission {
    /// Who may play it.
    pub player: PlayerId,
    /// The card.
    pub card: ObjectId,
    /// The version of `card` the permission was given for.
    pub version: u32,
    /// "Without paying its mana cost" (Dauthi Voidwalker): a spell is cast
    /// through the free-cast path, and X is 0 (CR 107.3b).
    pub free: bool,
    /// Card-specific spending permission, including colorless symbols.
    pub mana_any_type: bool,
    /// "You may **cast** that card" (Ragavan, Nimble Pilferer) rather than
    /// "play": to play a card is to play it as a land or cast it as a spell
    /// (CR 601.1a), and this permission is the second half only, so a land
    /// exiled under it stays where it is (a land is never cast, CR 305.9).
    pub cast_only: bool,
    /// Type restriction on the spell being cast, not the card's front face.
    pub spell_types: Option<baylee_core::types::TypeSet>,
    /// Turn-based expiry or the prepared permanent granting access.
    pub window: PlayLifetime,
}

impl PerTurn {
    /// Zeroed counters for `players` seats.
    #[must_use]
    pub fn new(players: usize) -> Self {
        Self {
            untapped_lands_at_start: 0,
            hand_size_at_start: 0,
            main_phases_begun: 0,
            descended: vec![false; players],
            speed_triggered: vec![false; players],
            sacrificed_subtypes: std::collections::BTreeMap::new(),
            revolt: vec![false; players],
            put_nontoken_permanent: vec![false; players],
            improvise_next_spell: vec![false; players],
            instant_or_sorcery_cast: vec![false; players],
            player_damage: std::collections::BTreeSet::new(),
            noncreature_spells: vec![0; players],
            spells_cast: vec![0; players],
            instants_cast: vec![0; players],
            entered_subtypes: vec![baylee_core::types::SubtypeSet::EMPTY; players],
            creatures_entered: vec![0; players],
            pending_entry_history: Vec::new(),
            discarded_to_graveyard: Vec::new(),
            no_more_spells: vec![false; players],
            life_lost: vec![false; players],
            life_lost_amount: vec![0; players],
            life_gained_amount: vec![0; players],
            damage_dealt_to: vec![0; players],
            permanent_damage: Vec::new(),
            damaged_creature_deaths: Vec::new(),
            creatures_died: 0,
            counter_placements: std::collections::BTreeSet::new(),
            draws: vec![0; players],
            drew_in_draw_step: false,
            entered_battlefield: Vec::new(),
            drawn: Vec::new(),
            instruction_drawn: Vec::new(),
            resolved: Vec::new(),
            graveyard_plays: Vec::new(),
            graveyard_allowances_used: Vec::new(),
            entered_graveyard: Vec::new(),
            exile_if_dies: Vec::new(),
            camouflage: false,
            cant_regenerate: Vec::new(),
            attacked: Vec::new(),
            blocks: Vec::new(),
            damage_log: Vec::new(),
            could_have_attacked: Vec::new(),
            wand_of_ith_targets: Vec::new(),
            damage_cant_regenerate_sources: Vec::new(),
            damage_exile_if_dies_sources: Vec::new(),
            tapped_for_ability: Vec::new(),
            skip_draw_step: false,
            skip_main_phases: false,
            skip_combat_phases: false,
        }
    }

    /// Counts one more resolution of ability `index` of `source` as it is
    /// at `version`, and returns the count with it included.
    pub fn note_resolution(&mut self, source: ObjectId, version: u32, index: u32) -> u32 {
        if let Some(entry) = self
            .resolved
            .iter_mut()
            .find(|e| e.source == source && e.version == version && e.index == index)
        {
            entry.times += 1;
            return entry.times;
        }
        self.resolved.push(AbilityResolved {
            source,
            version,
            index,
            times: 1,
        });
        1
    }

    /// How many times ability `index` of `source` at `version` has resolved
    /// this turn.
    #[must_use]
    pub fn resolutions(&self, source: ObjectId, version: u32, index: u32) -> u32 {
        self.resolved
            .iter()
            .find(|e| e.source == source && e.version == version && e.index == index)
            .map_or(0, |e| e.times)
    }

    /// Spells `player` has cast this turn.
    #[must_use]
    pub fn spells_cast_by(&self, player: PlayerId) -> u32 {
        self.spells_cast
            .get(player.get() as usize)
            .copied()
            .unwrap_or(0)
    }

    /// Clears the object and event history that belongs only to this turn.
    fn clear_turn_history(&mut self) {
        self.pending_entry_history.clear();
        self.entered_battlefield.clear();
        self.drawn.clear();
        self.instruction_drawn.clear();
        self.resolved.clear();
        self.graveyard_plays.clear();
        self.graveyard_allowances_used.clear();
        self.entered_graveyard.clear();
        self.exile_if_dies.clear();
        self.cant_regenerate.clear();
        self.attacked.clear();
        self.blocks.clear();
        self.damage_log.clear();
        self.could_have_attacked.clear();
        self.wand_of_ith_targets.clear();
        self.damage_cant_regenerate_sources.clear();
        self.damage_exile_if_dies_sources.clear();
        self.tapped_for_ability.clear();
        self.permanent_damage.clear();
        self.damaged_creature_deaths.clear();
    }

    /// Resets all counters (called at every turn start).
    pub fn reset(&mut self) {
        self.clear_turn_history();
        // Every field is named without `..`, including the history cleared
        // above, so adding a field makes this exhaustive reset fail to compile.
        let Self {
            untapped_lands_at_start,
            hand_size_at_start,
            main_phases_begun,
            descended,
            speed_triggered,
            sacrificed_subtypes,
            revolt,
            put_nontoken_permanent,
            improvise_next_spell,
            instant_or_sorcery_cast,
            player_damage,
            noncreature_spells,
            draws,
            drew_in_draw_step,
            spells_cast,
            instants_cast,
            entered_subtypes,
            creatures_entered,
            discarded_to_graveyard,
            no_more_spells,
            life_lost,
            life_lost_amount,
            life_gained_amount,
            damage_dealt_to,
            creatures_died,
            counter_placements,
            entered_battlefield: _,
            drawn: _,
            instruction_drawn: _,
            resolved: _,
            graveyard_plays: _,
            graveyard_allowances_used: _,
            entered_graveyard: _,
            exile_if_dies: _,
            camouflage,
            cant_regenerate: _,
            attacked: _,
            blocks: _,
            damage_log: _,
            could_have_attacked: _,
            wand_of_ith_targets: _,
            damage_cant_regenerate_sources: _,
            damage_exile_if_dies_sources: _,
            tapped_for_ability: _,
            skip_draw_step,
            skip_main_phases,
            skip_combat_phases,
            pending_entry_history: _,
            permanent_damage: _,
            damaged_creature_deaths: _,
        } = self;
        *untapped_lands_at_start = 0;
        *hand_size_at_start = 0;
        *main_phases_begun = 0;
        descended.fill(false);
        speed_triggered.fill(false);
        sacrificed_subtypes.clear();
        revolt.fill(false);
        put_nontoken_permanent.fill(false);
        improvise_next_spell.fill(false);
        instant_or_sorcery_cast.fill(false);
        player_damage.clear();
        noncreature_spells.fill(0);
        draws.fill(0);
        *drew_in_draw_step = false;
        spells_cast.fill(0);
        instants_cast.fill(0);
        entered_subtypes.fill(baylee_core::types::SubtypeSet::EMPTY);
        creatures_entered.fill(0);
        discarded_to_graveyard.clear();
        no_more_spells.fill(false);
        life_lost.fill(false);
        life_lost_amount.fill(0);
        life_gained_amount.fill(0);
        damage_dealt_to.fill(0);
        *creatures_died = 0;
        counter_placements.clear();
        *camouflage = false;
        *skip_draw_step = false;
        *skip_main_phases = false;
        *skip_combat_phases = false;
    }
}

/// What the turn before this one was, kept for CR 502.2.
///
/// Two numbers rather than a whole `PerTurn` snapshot: the untap step asks
/// exactly one question of the previous turn, and copying every counter to
/// answer it would put a per-seat allocation on every turn boundary.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct PreviousTurn {
    /// Whose turn it was.
    pub active: PlayerId,
    /// How many spells that player cast during it.
    pub spells_cast: u32,
    /// How many spells every player together cast during it — the
    /// werewolves' "if no spells were cast last turn".
    pub spells_by_all: u32,
    /// The most spells any one player cast during it — "if a player cast
    /// two or more spells last turn".
    pub most_by_one: u32,
}
