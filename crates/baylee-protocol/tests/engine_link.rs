//! The engine link only gains ([`baylee_protocol::ENGINE_LINK`]). A gateway
//! keeps a game on the engine build that started it after an update
//! (`docs/proposals/games-survive-restarts.md`), so what an older engine sends
//! has to read the same here, what this gateway sends has to read the same
//! there, and what it says to a client at that older table too.
//!
//! `engine_link.txt` holds every field, enum value and reservation of the
//! transport schema, one line each, as protoc compiled it. A line there that
//! the schema no longer says is a change an older build would misread, and is
//! refused. A line the schema gained goes into the file in the same change, so
//! that the next change is held to it as well. Lines are never taken out.

use prost::Message as _;
use prost_types::field_descriptor_proto::{Label, Type};
use prost_types::{DescriptorProto, EnumDescriptorProto, FileDescriptorSet};
use std::collections::BTreeSet;

/// The schema this build compiled (`build.rs`).
const SCHEMA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/transport.fds"));

/// Everything the link has said, as far as revision [`HELD_REVISION`].
const HELD: &str = include_str!("engine_link.txt");

/// The revision the file's first line names.
const HELD_REVISION: &str = "# engine link revision 1";

/// The schema as lines, sorted.
fn said() -> BTreeSet<String> {
    let set = FileDescriptorSet::decode(SCHEMA).expect("protoc's descriptor set reads");
    let mut lines = BTreeSet::new();
    for file in &set.file {
        let package = file.package();
        for message in &file.message_type {
            message_lines(
                &format!("{package}.{}", message.name()),
                message,
                &mut lines,
            );
        }
        for kind in &file.enum_type {
            enum_lines(&format!("{package}.{}", kind.name()), kind, &mut lines);
        }
    }
    lines
}

fn message_lines(name: &str, message: &DescriptorProto, lines: &mut BTreeSet<String>) {
    for field in &message.field {
        let kind = match field.r#type() {
            Type::Message | Type::Enum => field.type_name().trim_start_matches('.').to_owned(),
            plain => plain.as_str_name().to_lowercase(),
        };
        let repeated = if field.label() == Label::Repeated {
            " repeated"
        } else {
            ""
        };
        let optional = if field.proto3_optional() {
            " optional"
        } else {
            ""
        };
        let oneof = field
            .oneof_index
            .filter(|_| !field.proto3_optional())
            .and_then(|at| message.oneof_decl.get(usize::try_from(at).ok()?))
            .map_or_else(String::new, |oneof| format!(" in {}", oneof.name()));
        lines.insert(format!(
            "{name}.{} = {}: {kind}{repeated}{optional}{oneof}",
            field.name(),
            field.number()
        ));
    }
    for range in &message.reserved_range {
        lines.insert(format!(
            "{name} reserves {}..{}",
            range.start(),
            range.end()
        ));
    }
    for reserved in &message.reserved_name {
        lines.insert(format!("{name} reserves \"{reserved}\""));
    }
    for nested in &message.nested_type {
        message_lines(&format!("{name}.{}", nested.name()), nested, lines);
    }
    for kind in &message.enum_type {
        enum_lines(&format!("{name}.{}", kind.name()), kind, lines);
    }
}

fn enum_lines(name: &str, kind: &EnumDescriptorProto, lines: &mut BTreeSet<String>) {
    for value in &kind.value {
        lines.insert(format!("{name}.{} = {}", value.name(), value.number()));
    }
    for range in &kind.reserved_range {
        lines.insert(format!(
            "{name} reserves {}..={}",
            range.start(),
            range.end()
        ));
    }
    for reserved in &kind.reserved_name {
        lines.insert(format!("{name} reserves \"{reserved}\""));
    }
}

fn held() -> BTreeSet<String> {
    HELD.lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

fn listed<'a>(lines: impl Iterator<Item = &'a String>) -> String {
    lines.fold(String::new(), |mut all, line| {
        all.push_str(line);
        all.push('\n');
        all
    })
}

/// Everything the schema said, it still says, and the file holds all it says.
#[test]
fn the_link_still_says_all_it_said() {
    let said = said();
    let held = held();
    let changed: Vec<&String> = held.difference(&said).collect();
    assert!(
        changed.is_empty(),
        "the transport schema no longer says these, which an engine, a gateway or a client \
         of an older build reads. Put them back as they were: a field nothing uses any more \
         stays, never renumbered, retyped, renamed or moved into a oneof.\n{}",
        listed(changed.into_iter())
    );
    let gained: Vec<&String> = said.difference(&held).collect();
    assert!(
        gained.is_empty(),
        "the transport schema gained these: add them to crates/baylee-protocol/tests/\
         engine_link.txt in the same change, so the next one is held to them too.\n{}",
        listed(gained.into_iter())
    );
}

/// The file names the revision this build speaks, so raising it is a change
/// to the file as well: what the new revision lets a gateway ask is then
/// written beside the lines it added.
#[test]
fn the_file_names_this_builds_link_revision() {
    assert_eq!(HELD.lines().next(), Some(HELD_REVISION));
    assert_eq!(
        HELD_REVISION.rsplit(' ').next(),
        Some(baylee_protocol::ENGINE_LINK.to_string().as_str())
    );
}
