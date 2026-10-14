//! Small nonexecuting HTML tokenizer for the site's offline structural guards.
//! Handles quoted `>`, comments, raw script/style text, void and self-closing tags.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use anyhow::{Context, Result, ensure};
use regex::Regex;

#[derive(Debug)]
pub(super) struct Node {
    pub tag: String,
    pub attrs: BTreeMap<String, String>,
    pub parent: Option<usize>,
    pub text: String,
}
impl Node {
    pub fn attr(&self, name: &str) -> &str {
        self.attrs.get(name).map_or("", String::as_str)
    }
    pub fn class(&self, name: &str) -> bool {
        self.attr("class").split_whitespace().any(|c| c == name)
    }
}

pub(super) struct Tree {
    pub nodes: Vec<Node>,
}
impl Tree {
    pub fn ancestors(&self, mut index: usize) -> Vec<&Node> {
        let mut out = Vec::new();
        while let Some(parent) = self.nodes[index].parent {
            out.push(&self.nodes[parent]);
            index = parent;
        }
        out
    }
    fn contains(&self, ancestor: usize, mut index: usize) -> bool {
        loop {
            if index == ancestor {
                return true;
            }
            let Some(parent) = self.nodes[index].parent else {
                return false;
            };
            index = parent;
        }
    }
    pub fn descendants(&self, index: usize) -> impl Iterator<Item = &Node> {
        self.nodes
            .iter()
            .enumerate()
            .filter(move |(i, _)| self.contains(index, *i))
            .map(|(_, n)| n)
    }
    pub fn text(&self, index: usize) -> String {
        self.descendants(index)
            .map(|n| n.text.as_str())
            .collect::<String>()
            .trim()
            .to_owned()
    }
}

fn entity(text: &str) -> String {
    static ENTITIES: OnceLock<Regex> = OnceLock::new();
    let re = ENTITIES.get_or_init(|| {
        Regex::new(r"&(#x[0-9a-fA-F]+|#[0-9]+|amp|lt|gt|quot|apos|nbsp);")
            .expect("static entity regex")
    });
    re.replace_all(text, |c: &regex::Captures<'_>| {
        let value = match &c[1] {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            "nbsp" => Some('\u{a0}'),
            s if s.starts_with("#x") => u32::from_str_radix(&s[2..], 16)
                .ok()
                .and_then(char::from_u32),
            s if s.starts_with('#') => s[1..].parse().ok().and_then(char::from_u32),
            _ => None,
        };
        value.map_or_else(|| c[0].to_owned(), |v| v.to_string())
    })
    .into_owned()
}

// The tokenizer advances monotonically while maintaining a single open-element stack.
#[allow(clippy::too_many_lines)]
pub(super) fn parse(text: &str) -> Result<Tree> {
    let attrs = Regex::new(r#"([^\s=/>]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s>]+)))?"#)?;
    let mut tree = Tree {
        nodes: vec![Node {
            tag: "#root".into(),
            attrs: BTreeMap::new(),
            parent: None,
            text: String::new(),
        }],
    };
    let mut current = 0;
    let mut at = 0;
    while at < text.len() {
        if matches!(tree.nodes[current].tag.as_str(), "script" | "style") {
            let closing = format!("</{}", tree.nodes[current].tag);
            let end = text[at..]
                .to_ascii_lowercase()
                .find(&closing)
                .map_or(text.len(), |i| at + i);
            tree.nodes[current].text.push_str(&text[at..end]);
            at = end;
            if at == text.len() {
                break;
            }
        }
        if !text[at..].starts_with('<') {
            let end = text[at..].find('<').map_or(text.len(), |i| at + i);
            tree.nodes[current].text.push_str(&entity(&text[at..end]));
            at = end;
            continue;
        }
        if text[at..].starts_with("<!--") {
            at += text[at..]
                .find("-->")
                .context("unterminated HTML comment")?
                + 3;
            continue;
        }
        let mut quote = None;
        let mut end = None;
        for (offset, c) in text[at + 1..].char_indices() {
            if let Some(q) = quote {
                if c == q {
                    quote = None;
                }
            } else if c == '"' || c == '\'' {
                quote = Some(c);
            } else if c == '>' {
                end = Some(at + 1 + offset);
                break;
            }
        }
        let end = end.context("unterminated HTML tag")?;
        let body = text[at + 1..end].trim();
        at = end + 1;
        if body.starts_with('!') || body.starts_with('?') {
            continue;
        }
        if let Some(body) = body.strip_prefix('/') {
            let tag = body.trim().to_ascii_lowercase();
            let mut node = Some(current);
            while let Some(i) = node {
                if tree.nodes[i].tag == tag {
                    current = tree.nodes[i].parent.unwrap_or(0);
                    break;
                }
                node = tree.nodes[i].parent;
            }
            continue;
        }
        let split = body
            .find(|c: char| c.is_whitespace() || c == '/')
            .unwrap_or(body.len());
        let tag = body[..split].to_ascii_lowercase();
        ensure!(!tag.is_empty(), "empty HTML tag");
        let mut attributes = BTreeMap::new();
        for c in attrs.captures_iter(body[split..].trim_end_matches('/')) {
            let value = (2..=4).find_map(|i| c.get(i)).map_or("", |m| m.as_str());
            attributes.insert(c[1].to_ascii_lowercase(), entity(value));
        }
        let void = [
            "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source",
            "track", "wbr",
        ]
        .contains(&tag.as_str());
        tree.nodes.push(Node {
            tag,
            attrs: attributes,
            parent: Some(current),
            text: String::new(),
        });
        if !void && !body.ends_with('/') {
            current = tree.nodes.len() - 1;
        }
    }
    Ok(tree)
}
