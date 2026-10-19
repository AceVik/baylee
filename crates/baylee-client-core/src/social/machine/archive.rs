//! Authenticated archive import, without copying another device's protocol state.
use super::{
    BTreeMap, BTreeSet, ChatDisk, Conv, DeviceSecrets, Effect, Entropy, LineKey, LineKind,
    LineState, Record, RestKind, STATE_FILE, STATE_VERSION, Social, StoredConv, Zeroizing,
    at_rest_open, at_rest_seal, history,
};
use serde::Deserialize;

const MAX_ARCHIVE_BYTES: usize = 512 * 1024 * 1024;
const MAX_STATE_BYTES: usize = 8 * 1024 * 1024;
const MAX_RECORD_BYTES: usize = 256 * 1024;
const MAX_CONVERSATIONS: usize = 16_384;

/// An archive was malformed, unauthenticated, ambiguous or exceeded import limits.
/// No social state or disk effects are committed on this error.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArchiveError;

impl std::fmt::Display for ArchiveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("chat archive could not be authenticated or merged")
    }
}
impl std::error::Error for ArchiveError {}

// Unknown protocol-state fields are deliberately ignored, not deserialized into
// the live StoredState (registration, replay suppression, keys, ACKs, preferences).
#[derive(Deserialize)]
struct ArchiveState {
    v: u32,
    device_id: String,
    convs: Vec<StoredConv>,
}

struct ImportedConversation {
    stored: StoredConv,
    id: Conv,
    visible: Vec<Record>,
}

impl Social {
    /// Validate a complete local state before startup can replace it with defaults.
    /// Archive import separately ignores another device's protocol-state fields.
    ///
    /// # Errors
    /// Rejects authentication, JSON, version, device or conversation-index errors,
    /// including malformed protocol fields that the normal startup parser rejects.
    pub fn validate_stored_state(device: &DeviceSecrets, bytes: &[u8]) -> Result<(), ArchiveError> {
        let plain = open_state(device, bytes)?;
        parse_state(device, &plain)?;
        serde_json::from_slice::<super::StoredState>(&plain).map_err(|_| ArchiveError)?;
        Ok(())
    }

    /// Authenticate and merge one device's complete archive into the current device.
    /// `current` must contain the complete local logs, including lines outside the UI
    /// window. Apply the returned disk effects before importing the next archive.
    /// Only local state/history effects are returned; no registration, send or ACK.
    ///
    /// # Errors
    /// Rejects damaged/truncated records, unknown files, mismatched keys/device IDs,
    /// conflicting duplicate message IDs or bounded-size violations before mutation.
    pub fn restore_archive(
        &mut self,
        archived_device: &DeviceSecrets,
        archive: &dyn ChatDisk,
        current: &dyn ChatDisk,
        now_ms: u64,
        rng: &mut dyn Entropy,
    ) -> Result<Vec<Effect>, ArchiveError> {
        if self.removed {
            return Err(ArchiveError);
        }
        let state = read_state(archived_device, archive)?;
        let Some((state, state_bytes)) = state else {
            return Ok(Vec::new());
        };
        let mut remaining = MAX_ARCHIVE_BYTES - state_bytes;
        let mut output_remaining = MAX_ARCHIVE_BYTES;
        let mut imported = Vec::new();
        let mut effects = Vec::new();
        for stored in state.convs {
            let id = stored.id.parse::<Conv>().map_err(|_| ArchiveError)?;
            if matches!(id, Conv::Dm { .. }) && id.partner(&self.me.tag).is_none() {
                return Err(ArchiveError);
            }
            let Some(records) = read_log(archived_device, archive, &id, &mut remaining)? else {
                continue;
            };
            if self.prefs.history_days == 0 {
                continue;
            }
            let (records, changed) = self.merge_archive_records(current, &id, &records, now_ms)?;
            let mut bytes = Vec::new();
            for record in records.iter().filter(|_| changed) {
                bytes.extend(
                    history::seal_record(self.device.vault_key(), &id, record, rng)
                        .ok_or(ArchiveError)?,
                );
                if bytes.len() > output_remaining {
                    return Err(ArchiveError);
                }
            }
            output_remaining -= bytes.len();
            if changed {
                effects.push(Effect::RewriteHistory {
                    file: history::file_path(self.device.vault_key(), &id),
                    bytes,
                });
            }
            let visible = records
                .into_iter()
                .rev()
                .take(super::super::convs::LINES_KEPT)
                .collect();
            imported.push(ImportedConversation {
                stored,
                id,
                visible,
            });
        }
        if imported.is_empty() {
            return Ok(effects);
        }
        // Seal the resulting conversation index before any in-memory mutation.
        // Other state fields remain exactly the current device's values.
        let mut state = self.state.clone();
        for item in &imported {
            if !state.convs.iter().any(|conv| conv.id == item.stored.id) {
                state.convs.push(StoredConv {
                    id: item.stored.id.clone(),
                    title: super::super::text::clean_name(&item.stored.title),
                });
            }
        }
        if state != self.state || self.dirty {
            let json = Zeroizing::new(serde_json::to_vec(&state).map_err(|_| ArchiveError)?);
            let sealed = at_rest_seal(
                self.device.vault_key(),
                RestKind::State,
                STATE_FILE,
                &json,
                rng,
            )
            .map_err(|_| ArchiveError)?;
            effects.push(Effect::SaveState(sealed));
        }
        self.state = state;
        self.install_archive(imported, now_ms);
        self.dirty = false;
        self.bump();
        Ok(effects)
    }

    fn merge_archive_records(
        &self,
        current: &dyn ChatDisk,
        id: &Conv,
        records: &[Record],
        now_ms: u64,
    ) -> Result<(Vec<Record>, bool), ArchiveError> {
        let mut remaining = MAX_ARCHIVE_BYTES;
        let local = read_log(&self.device, current, id, &mut remaining)?.unwrap_or_default();
        let mut memory = Vec::new();
        if let Some(conv) = self.conversation(id) {
            for line in &conv.lines {
                if let LineKind::Message { from, mine, body } = &line.kind
                    && line.state == LineState::Final
                {
                    memory.push(Record {
                        id: line.key.id.clone(),
                        at: line.at,
                        from: from.clone(),
                        mine: *mine,
                        body: body.clone(),
                        evidence: self.evidence.get(&line.key).cloned().flatten(),
                    });
                }
            }
        }
        let mut merged = BTreeMap::new();
        for record in local.iter().chain(records).chain(&memory) {
            merge_record(&mut merged, record, &self.me.tag)?;
        }
        let keep_after = now_ms.saturating_sub(u64::from(self.prefs.history_days) * 86_400_000);
        let mut records: Vec<_> = merged
            .into_values()
            .filter(|record| record.at >= keep_after)
            .collect();
        records.sort_by(|a, b| (a.at, &a.id).cmp(&(b.at, &b.id)));
        let changed = !records.iter().copied().eq(local.iter());
        Ok((records.into_iter().cloned().collect(), changed))
    }

    fn install_archive(&mut self, imported: Vec<ImportedConversation>, now_ms: u64) {
        for item in imported {
            let existed = self.conversation(&item.id).is_some();
            for record in &item.visible {
                if record.evidence.is_some() {
                    self.evidence
                        .entry(LineKey {
                            conv: item.id.clone(),
                            id: record.id.clone(),
                        })
                        .or_insert_with(|| record.evidence.clone());
                }
            }
            let lines: Vec<_> = item
                .visible
                .iter()
                .map(|record| {
                    let mut line = record.line(&item.id, &self.me.tag);
                    line.hidden = self.state.ignored.contains(&record.from.tag);
                    line
                })
                .collect();
            let conv = self.conv_entry(&item.id, now_ms);
            for line in lines {
                conv.insert(line);
            }
            if !existed {
                conv.open = false;
                conv.title = super::super::text::clean_name(&item.stored.title);
            }
        }
    }
}

fn merge_record<'a>(
    merged: &mut BTreeMap<&'a str, &'a Record>,
    record: &'a Record,
    me: &str,
) -> Result<(), ArchiveError> {
    if record.id.is_empty()
        || record.id.len() > 64
        || record.mine != (record.from.tag == me)
        || !baylee_protocol::social::valid_tag(&record.from.tag)
    {
        return Err(ArchiveError);
    }
    if let Some(existing) = merged.get(record.id.as_str()) {
        if existing.at != record.at
            || existing.from.tag != record.from.tag
            || existing.mine != record.mine
            || existing.body != record.body
        {
            return Err(ArchiveError);
        }
    } else {
        merged.insert(&record.id, record);
    }
    Ok(())
}

fn read_state(
    device: &DeviceSecrets,
    archive: &dyn ChatDisk,
) -> Result<Option<(ArchiveState, usize)>, ArchiveError> {
    let roots = archive.list("");
    let logs = archive.list(super::super::disk::HISTORY_DIR);
    if roots.iter().any(|name| name != STATE_FILE) || logs.len() > MAX_CONVERSATIONS {
        return Err(ArchiveError);
    }
    let Some(bytes) = archive.read(STATE_FILE) else {
        return if logs.is_empty() && roots.is_empty() {
            Ok(None)
        } else {
            Err(ArchiveError)
        };
    };
    let plain = open_state(device, &bytes)?;
    let (state, expected) = parse_state(device, &plain)?;
    if logs.iter().any(|name| !expected.contains(name)) {
        return Err(ArchiveError);
    }
    Ok(Some((state, bytes.len())))
}

fn open_state(device: &DeviceSecrets, bytes: &[u8]) -> Result<Zeroizing<Vec<u8>>, ArchiveError> {
    if bytes.len() > MAX_STATE_BYTES {
        return Err(ArchiveError);
    }
    at_rest_open(device.vault_key(), RestKind::State, STATE_FILE, bytes).map_err(|_| ArchiveError)
}

fn parse_state(
    device: &DeviceSecrets,
    plain: &[u8],
) -> Result<(ArchiveState, BTreeSet<String>), ArchiveError> {
    let state: ArchiveState = serde_json::from_slice(plain).map_err(|_| ArchiveError)?;
    if state.v != STATE_VERSION
        || state.device_id != device.device_id()
        || state.convs.len() > MAX_CONVERSATIONS
    {
        return Err(ArchiveError);
    }
    let mut expected = BTreeSet::new();
    for conv in &state.convs {
        let id = conv.id.parse::<Conv>().map_err(|_| ArchiveError)?;
        if conv.id != id.to_string()
            || !expected.insert(history::file_name(device.vault_key(), &id))
        {
            return Err(ArchiveError);
        }
    }
    Ok((state, expected))
}

fn read_log(
    device: &DeviceSecrets,
    disk: &dyn ChatDisk,
    id: &Conv,
    remaining: &mut usize,
) -> Result<Option<Vec<Record>>, ArchiveError> {
    let name = history::file_name(device.vault_key(), id);
    let Some(bytes) = disk.read(&history::file_path(device.vault_key(), id)) else {
        return Ok(None);
    };
    *remaining = remaining.checked_sub(bytes.len()).ok_or(ArchiveError)?;
    let mut rest = bytes.as_slice();
    let mut records = Vec::new();
    while !rest.is_empty() {
        let prefix: [u8; 4] = rest
            .get(..4)
            .ok_or(ArchiveError)?
            .try_into()
            .map_err(|_| ArchiveError)?;
        let length = usize::try_from(u32::from_be_bytes(prefix)).map_err(|_| ArchiveError)?;
        if length > MAX_RECORD_BYTES {
            return Err(ArchiveError);
        }
        let end = length.checked_add(4).ok_or(ArchiveError)?;
        let frame = rest.get(4..end).ok_or(ArchiveError)?;
        let plain = at_rest_open(device.vault_key(), RestKind::History, &name, frame)
            .map_err(|_| ArchiveError)?;
        records.push(serde_json::from_slice(&plain).map_err(|_| ArchiveError)?);
        rest = &rest[end..];
    }
    Ok(Some(records))
}
