//! Bounds-checked PE32/PE32+ resource traversal; never executes a PE.
use anyhow::{Context, Result, bail, ensure};
use std::{collections::BTreeMap, fs, path::Path};

const SIZES: [u16; 7] = [16, 24, 32, 48, 64, 128, 256];
struct Bytes<'a>(&'a [u8]);
impl<'a> Bytes<'a> {
    fn at(&self, offset: usize, size: usize) -> Result<&'a [u8]> {
        let end = offset.checked_add(size).context("PE offset overflow")?;
        self.0.get(offset..end).context("truncated PE resource")
    }
    fn u16(&self, offset: usize) -> Result<u16> {
        Ok(u16::from_le_bytes(self.at(offset, 2)?.try_into()?))
    }
    fn u32(&self, offset: usize) -> Result<u32> {
        Ok(u32::from_le_bytes(self.at(offset, 4)?.try_into()?))
    }
    fn number(&self, offset: usize) -> Result<usize> {
        Ok(usize::try_from(self.u32(offset)?)?)
    }
}
struct Section {
    rva: usize,
    raw_size: usize,
    raw: usize,
}
fn offset(rva: usize, size: usize, sections: &[Section]) -> Result<usize> {
    for section in sections {
        if let Some(delta) = rva.checked_sub(section.rva)
            && delta
                .checked_add(size)
                .is_some_and(|end| end <= section.raw_size)
        {
            return section.raw.checked_add(delta).context("PE offset overflow");
        }
    }
    bail!("RVA {rva:#x} is outside file-backed sections")
}
fn entries(data: &Bytes<'_>, base: usize, at: usize) -> Result<Vec<(u32, usize, bool)>> {
    let count = usize::from(data.u16(at + 12)?) + usize::from(data.u16(at + 14)?);
    data.at(at + 16, count * 8)?;
    let mut entries = Vec::new();
    for i in 0..count {
        let name = data.u32(at + 16 + i * 8)?;
        let target = data.u32(at + 20 + i * 8)?;
        if name & 0x8000_0000 == 0 {
            entries.push((
                name,
                base.checked_add(usize::try_from(target & 0x7fff_ffff)?)
                    .context("resource offset overflow")?,
                target & 0x8000_0000 != 0,
            ));
        }
    }
    Ok(entries)
}

pub(super) fn check(data: &[u8]) -> Result<()> {
    let bytes = Bytes(data);
    ensure!(bytes.at(0, 2)? == b"MZ", "no MZ header");
    let pe = bytes.number(0x3c)?;
    ensure!(bytes.at(pe, 4)? == b"PE\0\0", "no PE signature");
    let count = usize::from(bytes.u16(pe + 6)?);
    let optional_size = usize::from(bytes.u16(pe + 20)?);
    let optional = pe + 24;
    bytes.at(optional, optional_size)?;
    let directory = match bytes.u16(optional)? {
        0x10b => 96,
        0x20b => 112,
        other => bail!("unknown optional header {other:#x}"),
    };
    ensure!(
        optional_size >= directory + 24,
        "optional header lacks resource directory"
    );
    ensure!(
        bytes.u32(optional + directory - 4)? >= 3,
        "no resource data directory"
    );
    let resource = bytes.number(optional + directory + 16)?;
    ensure!(resource != 0, "no icon resources");
    let table = optional + optional_size;
    bytes.at(table, count * 40)?;
    let mut sections = Vec::new();
    for i in 0..count {
        let at = table + i * 40;
        sections.push(Section {
            rva: bytes.number(at + 12)?,
            raw_size: bytes.number(at + 16)?,
            raw: bytes.number(at + 20)?,
        });
    }
    let base = offset(resource, 16, &sections)?;
    let mut found: BTreeMap<u32, BTreeMap<u32, &[u8]>> = BTreeMap::new();
    for (kind, kind_at, directory) in entries(&bytes, base, base)? {
        if kind != 3 && kind != 14 {
            continue;
        }
        ensure!(directory, "icon resource type is not a directory");
        for (id, id_at, is_dir) in entries(&bytes, base, kind_at)? {
            let leaf = if is_dir {
                let languages = entries(&bytes, base, id_at)?;
                let &(_, at, nested) =
                    languages.first().context("empty icon language directory")?;
                ensure!(!nested, "unexpected nested icon language directory");
                at
            } else {
                id_at
            };
            let rva = bytes.number(leaf)?;
            let size = bytes.number(leaf + 4)?;
            let value = bytes.at(offset(rva, size, &sections)?, size)?;
            ensure!(
                found.entry(kind).or_default().insert(id, value).is_none(),
                "duplicate icon resource ID"
            );
        }
    }
    let group = Bytes(
        found
            .get(&14)
            .and_then(|groups| groups.get(&1))
            .copied()
            .context("no icon group 1")?,
    );
    ensure!(
        group.u16(0)? == 0 && group.u16(2)? == 1,
        "icon group 1 is not an icon directory"
    );
    let count = usize::from(group.u16(4)?);
    group.at(6, count * 14)?;
    let mut sizes = Vec::new();
    for i in 0..count {
        let at = 6 + i * 14;
        let dimensions = group.at(at, 2)?;
        let dimension = |v| if v == 0 { 256 } else { u16::from(v) };
        let (width, height) = (dimension(dimensions[0]), dimension(dimensions[1]));
        let id = u32::from(group.u16(at + 12)?);
        let icon = found
            .get(&3)
            .and_then(|icons| icons.get(&id))
            .context("icon group names a missing RT_ICON")?;
        ensure!(width == height, "icon {id} is {width}x{height}, not square");
        ensure!(
            !icon.is_empty() && group.number(at + 8)? == icon.len(),
            "icon {id} has a truncated or mismatched payload"
        );
        sizes.push(width);
    }
    ensure!(
        SIZES.into_iter().all(|size| sizes.contains(&size)),
        "icon group 1 lacks required sizes: has {sizes:?}"
    );
    Ok(())
}

pub(super) fn check_file(path: &Path) -> Result<()> {
    check(&fs::read(path)?).with_context(|| {
        format!(
            "{}: not a readable PE with the required icon",
            path.display()
        )
    })
}

#[cfg(test)]
mod tests;
