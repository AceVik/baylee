//! Which of the card DSL's building blocks the engine's own rule tests run,
//! and which cards wait on the ones they do not (`xtask verify --coverage`).
//!
//! A card is built from DSL variants (`Effect::DealDamage`, `Amount::Fixed`,
//! `Trigger::Dies`, …), which [`baylee_train::cardwalk::mechanics`] names
//! `field>Variant`. The engine handles a variant in `match` arms, `if let`s
//! and `let … else`s. This module reads those sites with `syn`, reads which
//! of them ran from an `llvm-cov` export of the engine's tests *without* the
//! per-card tests (`card_tests/`, `combo_tests/`), and grades each variant:
//! tested where every site naming it ran, partly where some did, untested
//! where none did. A card's own test proves the card; this proves the rules
//! under it. L4's "every mechanic it uses is tested" asks that none of a
//! card's mechanics be untested; partly run ones are listed with the sites
//! that did not run, for the engine's tests to reach.
//!
//! What counts as a site:
//!
//! - Only types a card definition can hold (reachable from `CardDef` and
//!   `TokenDef` through their fields), so an engine enum or an unrelated core
//!   enum sharing a name is not read as the DSL's.
//! - An arm that cannot run by design (`unreachable!`, `panic!`, …) is no
//!   site, and neither is a function that reads a variant without playing it
//!   ([`BOOKKEEPING`]: hashing, formatting).
//! - Where no arm names a variant, a check does: `matches!`, `==`/`!=`
//!   against it, and keyword constants (`contains(KeywordSet::FLYING)`),
//!   which count as run where the check ran, whatever it found.
//! - An arm `A | B => …` that ran shows that one of the two did; it counts
//!   for both and is marked shared.
//!
//! A variant no instrumented engine code names is unsited: listed, neither
//! tested nor untested. The export comes from (`cargo install
//! cargo-llvm-cov`; in no gate):
//!
//! ```text
//! cargo llvm-cov --package baylee-engine --lib --json --output-path <file> \
//!     -- --skip card_tests --skip combo_tests
//! ```

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Context as _;
use baylee_core::ids::CardIndex;
use proc_macro2::{LineColumn, Span, TokenTree};
use syn::parse::ParseStream;
use syn::spanned::Spanned as _;
use syn::visit::{self, Visit};

/// A DSL variant, `(type, variant)`; a keyword is `("KeywordSet", NAME)`.
pub type Mechanic = (String, String);

/// The type keyword constants live on.
const KEYWORDS: &str = "KeywordSet";

/// The types a card definition starts from.
const ROOTS: [&str; 2] = ["CardDef", "TokenDef"];

/// Keys `cardwalk` reads under a name of its own, and the field that is.
const WALK_ROOTS: [(&str, &str); 1] = [("face_abilities", "abilities")];

/// Where the DSL's types are defined, under the workspace root.
const DSL_SOURCES: [&str; 2] = ["crates/baylee-cards-dsl/src", "crates/baylee-core/src"];

/// The engine's source, under the workspace root; coverage names files
/// relative to it.
const ENGINE_SOURCE: &str = "crates/baylee-engine/src";

/// Functions (by a part of their name) that read a variant without playing
/// it; their arms are no evidence either way.
const BOOKKEEPING: [&str; 2] = ["hash", "fmt"];

/// Macros an arm that never runs by design consists of.
const DEAD: [&str; 4] = ["unreachable", "panic", "unimplemented", "todo"];

/// A place in a source file: line and column, both from 1 as LLVM counts.
type Pos = (u32, u32);

fn pos(lc: LineColumn) -> Pos {
    (
        u32::try_from(lc.line).unwrap_or(u32::MAX),
        u32::try_from(lc.column + 1).unwrap_or(u32::MAX),
    )
}

/// `Enum::Variant`.
fn show(m: &Mechanic) -> String {
    format!("{}::{}", m.0, m.1)
}

/// Every `.rs` file under `dir`, sorted, skipping test files and generated
/// tables.
fn sources(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        for entry in fs::read_dir(&d).with_context(|| format!("reading {}", d.display()))? {
            let path = entry?.path();
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if path.is_dir() {
                if !matches!(name, "generated" | "card_tests" | "combo_tests") {
                    todo.push(path);
                }
            } else if path.extension().is_some_and(|x| x == "rs")
                && !name.ends_with("_tests.rs")
                && !matches!(name, "tests.rs" | "testkit.rs")
            {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Whether an item exists only in tests.
fn is_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        if a.path().is_ident("test") {
            return true;
        }
        let syn::Meta::List(list) = &a.meta else {
            return false;
        };
        let tokens = list.tokens.to_string().replace(' ', "");
        list.path.is_ident("cfg") && (tokens == "test" || tokens.starts_with("any(test,"))
    })
}

/// The type names a type mentions, generic arguments included.
#[derive(Default)]
struct TypeNames(BTreeSet<String>);

impl<'ast> Visit<'ast> for TypeNames {
    fn visit_path_segment(&mut self, s: &'ast syn::PathSegment) {
        self.0.insert(s.ident.to_string());
        visit::visit_path_segment(self, s);
    }
}

fn type_names(ty: &syn::Type) -> BTreeSet<String> {
    let mut names = TypeNames::default();
    names.visit_type(ty);
    names.0
}

/// The DSL's types, as `syn` reads them from source.
#[derive(Default, Debug)]
struct Dsl {
    /// Enum → its variants, for every enum read.
    enums: BTreeMap<String, BTreeSet<String>>,
    /// Struct names: a key naming one is structure, not a mechanic.
    structs: BTreeSet<String>,
    /// A type → the type names its fields mention.
    owned: BTreeMap<String, BTreeSet<String>>,
    /// A field (or a tuple variant or struct, whose contents the walk reads
    /// under its own name) → its owner and the type names its type mentions.
    fields: BTreeMap<String, Vec<(String, BTreeSet<String>)>>,
    /// A type alias → the type names it stands for.
    aliases: BTreeMap<String, BTreeSet<String>>,
    /// Keyword bit → its `KeywordSet` constant.
    keywords: BTreeMap<u32, String>,
    /// The types a card definition can hold ([`Dsl::finish`]).
    reachable: BTreeSet<String>,
}

/// What a card's `field>Variant` key names.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Resolved {
    /// DSL variants; more than one only when the field's type allows several.
    Mechanics(Vec<Mechanic>),
    /// A struct or plain data (a subtype id, a colour set): not a mechanic.
    Structure,
    /// Not read, and why.
    Unmapped(String),
}

impl Dsl {
    fn read(&mut self, source: &str) -> syn::Result<()> {
        let file = syn::parse_file(source)?;
        self.visit_file(&file);
        Ok(())
    }

    /// Marks the types reachable from [`ROOTS`] through fields and aliases.
    fn finish(&mut self) {
        let mut todo: Vec<String> = ROOTS.iter().map(|r| (*r).to_owned()).collect();
        while let Some(t) = todo.pop() {
            if !self.reachable.insert(t.clone()) {
                continue;
            }
            for next in self.owned.get(&t).into_iter().chain(self.aliases.get(&t)) {
                todo.extend(
                    next.iter()
                        .filter(|n| !self.reachable.contains(*n))
                        .cloned(),
                );
            }
        }
    }

    fn add_fields(&mut self, owner: &str, unnamed: &str, fields: &syn::Fields) {
        for f in fields {
            let names = type_names(&f.ty);
            self.owned
                .entry(owner.to_owned())
                .or_default()
                .extend(names.iter().cloned());
            let field = f
                .ident
                .as_ref()
                .map_or_else(|| unnamed.to_owned(), ToString::to_string);
            self.fields
                .entry(field)
                .or_default()
                .push((owner.to_owned(), names));
        }
    }

    fn is_enum(&self, ty: &str) -> bool {
        self.reachable.contains(ty) && self.enums.contains_key(ty)
    }

    fn is_variant(&self, ty: &str, variant: &str) -> bool {
        if ty == KEYWORDS {
            return self.keywords.values().any(|k| k == variant);
        }
        self.is_enum(ty) && self.enums[ty].contains(variant)
    }

    /// The type names `field`'s type mentions where a card-reachable type
    /// owns it, through aliases.
    fn field_types(&self, field: &str) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut todo: Vec<String> = self
            .fields
            .get(field)
            .into_iter()
            .flatten()
            .filter(|(owner, _)| self.reachable.contains(owner))
            .flat_map(|(_, names)| names.iter().cloned())
            .collect();
        while let Some(t) = todo.pop() {
            if let Some(alias) = self.aliases.get(&t) {
                todo.extend(alias.iter().filter(|a| !out.contains(*a)).cloned());
            }
            out.insert(t);
        }
        out
    }

    /// What a `field>Variant` key names: the variant of the field's own type
    /// where one has it, else the one DSL enum with such a variant.
    fn resolve(&self, key: &str) -> Resolved {
        let Some((field, variant)) = key.split_once('>') else {
            return Resolved::Unmapped("not a field>Variant key".into());
        };
        if field == "keyword" {
            return variant
                .parse::<u32>()
                .ok()
                .and_then(|b| self.keywords.get(&b))
                .map_or_else(
                    || Resolved::Unmapped(format!("keyword bit {variant} has no name")),
                    |k| Resolved::Mechanics(vec![(KEYWORDS.to_owned(), k.clone())]),
                );
        }
        let field = WALK_ROOTS
            .iter()
            .find(|(root, _)| *root == field)
            .map_or(field, |(_, real)| *real);
        let types = self.field_types(field);
        let by_field: Vec<Mechanic> = types
            .iter()
            .filter(|t| self.is_variant(t, variant))
            .map(|t| (t.clone(), variant.to_owned()))
            .collect();
        if !by_field.is_empty() {
            return Resolved::Mechanics(by_field);
        }
        // A struct's name, or a field holding no enum at all (a subtype id,
        // a colour set): data.
        if self.structs.contains(variant)
            || (!types.is_empty() && !types.iter().any(|t| self.is_enum(t)))
        {
            return Resolved::Structure;
        }
        let by_name: Vec<Mechanic> = self
            .enums
            .iter()
            .filter(|(e, v)| self.reachable.contains(*e) && v.contains(variant))
            .map(|(e, _)| (e.clone(), variant.to_owned()))
            .collect();
        match by_name.len() {
            0 => Resolved::Unmapped(format!("`{variant}` is no DSL variant")),
            1 => Resolved::Mechanics(by_name),
            _ => Resolved::Unmapped(format!(
                "`{field}` names no DSL field and `{variant}` is a variant of {}",
                by_name
                    .iter()
                    .map(|m| m.0.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
        }
    }
}

impl<'ast> Visit<'ast> for Dsl {
    fn visit_item_enum(&mut self, e: &'ast syn::ItemEnum) {
        let name = e.ident.to_string();
        let variants = self.enums.entry(name.clone()).or_default();
        variants.extend(e.variants.iter().map(|v| v.ident.to_string()));
        for v in &e.variants {
            self.add_fields(&name, &v.ident.to_string(), &v.fields);
        }
        visit::visit_item_enum(self, e);
    }

    fn visit_item_struct(&mut self, s: &'ast syn::ItemStruct) {
        let name = s.ident.to_string();
        self.structs.insert(name.clone());
        self.add_fields(&name, &name, &s.fields);
        visit::visit_item_struct(self, s);
    }

    fn visit_item_type(&mut self, t: &'ast syn::ItemType) {
        self.aliases.insert(t.ident.to_string(), type_names(&t.ty));
        visit::visit_item_type(self, t);
    }

    /// `keywords! { FLYING = 0, "Flying."; … }`.
    fn visit_item_macro(&mut self, m: &'ast syn::ItemMacro) {
        if m.mac.path.is_ident("keywords") {
            let tokens: Vec<TokenTree> = m.mac.tokens.clone().into_iter().collect();
            for w in tokens.windows(3) {
                if let [
                    TokenTree::Ident(name),
                    TokenTree::Punct(eq),
                    TokenTree::Literal(bit),
                ] = w
                    && eq.as_char() == '='
                    && let Ok(bit) = bit.to_string().parse::<u32>()
                {
                    self.keywords.insert(bit, name.to_string());
                }
            }
        }
        visit::visit_item_macro(self, m);
    }
}

/// How a site shows a variant ran.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// A `match` arm, an `if let` body, the rest of a block after a
    /// `let … else`: it runs only for that variant.
    Arm,
    /// A check naming it (`matches!`, `==`, a keyword constant): the check
    /// ran, whatever it found.
    Check,
}

/// One place in the engine that names DSL variants.
#[derive(Clone, Debug)]
struct Site {
    file: String,
    /// The function it is in.
    func: String,
    kind: Kind,
    /// More than one variant could have made it run (`A | B`).
    shared: bool,
    names: Vec<Mechanic>,
    start: Pos,
    end: Pos,
}

/// The names a file's `use` items bring into scope.
struct Scope<'a> {
    dsl: &'a Dsl,
    /// A local name for a DSL type (`use …::Effect as E`).
    renames: BTreeMap<String, String>,
    /// DSL enums whose variants are in scope by glob.
    globs: Vec<String>,
    /// A variant imported by name.
    imported: BTreeMap<String, Mechanic>,
}

impl Scope<'_> {
    fn is_type(&self, name: &str) -> bool {
        name == KEYWORDS || self.dsl.is_enum(name)
    }

    fn real<'n>(&'n self, name: &'n str) -> &'n str {
        self.renames.get(name).map_or(name, String::as_str)
    }

    fn tree(&mut self, prefix: &mut Vec<String>, tree: &syn::UseTree) {
        match tree {
            syn::UseTree::Path(p) => {
                prefix.push(p.ident.to_string());
                self.tree(prefix, &p.tree);
                prefix.pop();
            }
            syn::UseTree::Name(n) => {
                let name = n.ident.to_string();
                self.name(prefix, &name, &name);
            }
            syn::UseTree::Rename(r) => {
                self.name(prefix, &r.ident.to_string(), &r.rename.to_string());
            }
            syn::UseTree::Glob(_) => {
                if let Some(last) = prefix.last() {
                    let ty = self.real(last).to_owned();
                    if self.dsl.is_enum(&ty) {
                        self.globs.push(ty);
                    }
                }
            }
            syn::UseTree::Group(g) => {
                for item in &g.items {
                    self.tree(prefix, item);
                }
            }
        }
    }

    fn name(&mut self, prefix: &[String], name: &str, local: &str) {
        let (name, local) = if name == "self" {
            let Some(last) = prefix.last() else { return };
            (
                last.as_str(),
                if local == "self" {
                    last.as_str()
                } else {
                    local
                },
            )
        } else {
            (name, local)
        };
        if self.is_type(name) {
            self.renames.insert(local.to_owned(), name.to_owned());
            return;
        }
        if let Some(last) = prefix.last() {
            let ty = self.real(last).to_owned();
            if self.dsl.is_variant(&ty, name) {
                self.imported
                    .insert(local.to_owned(), (ty, name.to_owned()));
            }
        }
    }

    /// The variant a path names, if a DSL enum's.
    fn path(&self, path: &syn::Path) -> Option<Mechanic> {
        let segs: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        match segs.as_slice() {
            [] => None,
            [one] => self.single(one),
            [.., ty, variant] => {
                let ty = self.real(ty);
                (ty != KEYWORDS && self.dsl.is_variant(ty, variant))
                    .then(|| (ty.to_owned(), variant.clone()))
            }
        }
    }

    fn single(&self, name: &str) -> Option<Mechanic> {
        self.imported.get(name).cloned().or_else(|| {
            self.globs
                .iter()
                .find(|g| self.dsl.is_variant(g, name))
                .map(|g| (g.clone(), name.to_owned()))
        })
    }
}

impl<'ast> Visit<'ast> for Scope<'_> {
    fn visit_item_use(&mut self, u: &'ast syn::ItemUse) {
        self.tree(&mut Vec::new(), &u.tree);
    }
}

/// Reads one engine file's sites.
struct EngineReader<'a> {
    scope: Scope<'a>,
    file: String,
    /// The functions the visit is in, innermost last.
    funcs: Vec<String>,
    sites: Vec<Site>,
    /// Enums the engine defines itself, reported where one shares a DSL
    /// enum's name.
    own_enums: BTreeSet<String>,
}

/// Whether an arm's body cannot run by design.
fn is_dead(body: &syn::Expr) -> bool {
    let dead = |m: &syn::Macro| {
        m.path
            .segments
            .last()
            .is_some_and(|s| DEAD.iter().any(|d| s.ident == d))
    };
    match body {
        syn::Expr::Macro(m) => dead(&m.mac),
        syn::Expr::Block(b) => match b.block.stmts.as_slice() {
            [syn::Stmt::Macro(m)] => dead(&m.mac),
            [syn::Stmt::Expr(e, _)] => is_dead(e),
            _ => false,
        },
        _ => false,
    }
}

/// An expression with references, parentheses and derefs peeled off.
fn peeled(e: &syn::Expr) -> &syn::Expr {
    match e {
        syn::Expr::Reference(r) => peeled(&r.expr),
        syn::Expr::Paren(p) => peeled(&p.expr),
        syn::Expr::Unary(u) if matches!(u.op, syn::UnOp::Deref(_)) => peeled(&u.expr),
        _ => e,
    }
}

impl EngineReader<'_> {
    fn pat_names(&self, pat: &syn::Pat, out: &mut Vec<Mechanic>, shared: &mut bool) {
        match pat {
            syn::Pat::Ident(p) => match &p.subpat {
                Some((_, sub)) => self.pat_names(sub, out, shared),
                None => out.extend(self.scope.single(&p.ident.to_string())),
            },
            syn::Pat::Path(p) => out.extend(self.scope.path(&p.path)),
            syn::Pat::TupleStruct(p) => {
                out.extend(self.scope.path(&p.path));
                for e in &p.elems {
                    self.pat_names(e, out, shared);
                }
            }
            syn::Pat::Struct(p) => {
                out.extend(self.scope.path(&p.path));
                for f in &p.fields {
                    self.pat_names(&f.pat, out, shared);
                }
            }
            syn::Pat::Or(p) => {
                let mut cases = Vec::new();
                for c in &p.cases {
                    self.pat_names(c, &mut cases, shared);
                }
                cases.sort();
                cases.dedup();
                *shared |= cases.len() > 1;
                out.extend(cases);
            }
            syn::Pat::Tuple(p) => {
                for e in &p.elems {
                    self.pat_names(e, out, shared);
                }
            }
            syn::Pat::Slice(p) => {
                for e in &p.elems {
                    self.pat_names(e, out, shared);
                }
            }
            syn::Pat::Reference(p) => self.pat_names(&p.pat, out, shared),
            syn::Pat::Paren(p) => self.pat_names(&p.pat, out, shared),
            syn::Pat::Type(p) => self.pat_names(&p.pat, out, shared),
            _ => {}
        }
    }

    fn site(&mut self, kind: Kind, pat: &syn::Pat, start: Pos, end: Pos) {
        let mut names = Vec::new();
        let mut shared = false;
        self.pat_names(pat, &mut names, &mut shared);
        self.push(kind, names, shared, start, end);
    }

    fn push(&mut self, kind: Kind, mut names: Vec<Mechanic>, shared: bool, start: Pos, end: Pos) {
        if names.is_empty() {
            return;
        }
        names.sort();
        names.dedup();
        self.sites.push(Site {
            file: self.file.clone(),
            func: self.funcs.last().cloned().unwrap_or_default(),
            kind,
            shared,
            names,
            start,
            end,
        });
    }

    fn in_fn(&mut self, name: &syn::Ident, visit: impl FnOnce(&mut Self)) {
        self.funcs.push(name.to_string());
        visit(self);
        self.funcs.pop();
    }
}

fn span(s: Span) -> (Pos, Pos) {
    (pos(s.start()), pos(s.end()))
}

/// Every `let` in an `if`/`while` condition, through `&&` chains.
fn lets_in<'e>(cond: &'e syn::Expr, out: &mut Vec<&'e syn::ExprLet>) {
    match cond {
        syn::Expr::Let(l) => out.push(l),
        syn::Expr::Binary(b) if matches!(b.op, syn::BinOp::And(_)) => {
            lets_in(&b.left, out);
            lets_in(&b.right, out);
        }
        syn::Expr::Paren(p) => lets_in(&p.expr, out),
        _ => {}
    }
}

/// The pattern of `matches!(expr, pattern if guard)`.
fn matches_pattern(input: ParseStream) -> syn::Result<syn::Pat> {
    let _: syn::Expr = input.parse()?;
    let _: syn::Token![,] = input.parse()?;
    let pat = syn::Pat::parse_multi_with_leading_vert(input)?;
    if input.peek(syn::Token![if]) {
        let _: syn::Token![if] = input.parse()?;
        let _: syn::Expr = input.parse()?;
    }
    let _: Option<syn::Token![,]> = input.parse()?;
    Ok(pat)
}

impl<'ast> Visit<'ast> for EngineReader<'_> {
    fn visit_item_mod(&mut self, m: &'ast syn::ItemMod) {
        if !is_test(&m.attrs) {
            visit::visit_item_mod(self, m);
        }
    }

    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        if !is_test(&f.attrs) {
            self.in_fn(&f.sig.ident, |r| visit::visit_item_fn(r, f));
        }
    }

    fn visit_item_impl(&mut self, i: &'ast syn::ItemImpl) {
        if !is_test(&i.attrs) {
            visit::visit_item_impl(self, i);
        }
    }

    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        if !is_test(&f.attrs) {
            self.in_fn(&f.sig.ident, |r| visit::visit_impl_item_fn(r, f));
        }
    }

    fn visit_item_enum(&mut self, e: &'ast syn::ItemEnum) {
        self.own_enums.insert(e.ident.to_string());
        visit::visit_item_enum(self, e);
    }

    fn visit_expr_match(&mut self, m: &'ast syn::ExprMatch) {
        for arm in &m.arms {
            if !is_dead(&arm.body) {
                let (start, end) = span(arm.body.span());
                self.site(Kind::Arm, &arm.pat, start, end);
            }
        }
        visit::visit_expr_match(self, m);
    }

    fn visit_expr_if(&mut self, i: &'ast syn::ExprIf) {
        let mut lets = Vec::new();
        lets_in(&i.cond, &mut lets);
        let (start, end) = span(i.then_branch.span());
        for l in lets {
            self.site(Kind::Arm, &l.pat, start, end);
        }
        visit::visit_expr_if(self, i);
    }

    fn visit_expr_while(&mut self, w: &'ast syn::ExprWhile) {
        let mut lets = Vec::new();
        lets_in(&w.cond, &mut lets);
        let (start, end) = span(w.body.span());
        for l in lets {
            self.site(Kind::Arm, &l.pat, start, end);
        }
        visit::visit_expr_while(self, w);
    }

    /// `let Pattern = x else { … };` guards the rest of its block.
    fn visit_block(&mut self, b: &'ast syn::Block) {
        for (i, stmt) in b.stmts.iter().enumerate() {
            if let syn::Stmt::Local(local) = stmt
                && local
                    .init
                    .as_ref()
                    .is_some_and(|init| init.diverge.is_some())
                && let (Some(next), Some(last)) = (b.stmts.get(i + 1), b.stmts.last())
            {
                let start = pos(next.span().start());
                let end = pos(last.span().end());
                self.site(Kind::Arm, &local.pat, start, end);
            }
        }
        visit::visit_block(self, b);
    }

    fn visit_expr_binary(&mut self, b: &'ast syn::ExprBinary) {
        if matches!(b.op, syn::BinOp::Eq(_) | syn::BinOp::Ne(_)) {
            let names: Vec<Mechanic> = [&*b.left, &*b.right]
                .into_iter()
                .filter_map(|side| match peeled(side) {
                    syn::Expr::Path(p) => self.scope.path(&p.path),
                    _ => None,
                })
                .collect();
            let (start, end) = span(b.span());
            self.push(Kind::Check, names, false, start, end);
        }
        visit::visit_expr_binary(self, b);
    }

    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        if m.path.segments.last().is_some_and(|s| s.ident == "matches")
            && let Ok(pat) = m.parse_body_with(matches_pattern)
        {
            let (start, end) = span(m.span());
            self.site(Kind::Check, &pat, start, end);
        }
        visit::visit_macro(self, m);
    }

    fn visit_expr_path(&mut self, p: &'ast syn::ExprPath) {
        let segs: Vec<String> = p
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect();
        if let [.., ty, name] = segs.as_slice()
            && self.scope.real(ty) == KEYWORDS
            && self.scope.dsl.is_variant(KEYWORDS, name)
        {
            let names = vec![(KEYWORDS.to_owned(), name.clone())];
            let (start, end) = span(p.span());
            self.push(Kind::Check, names, false, start, end);
        }
        visit::visit_expr_path(self, p);
    }
}

/// The sites of one engine file, and the enums it defines.
fn read_engine_file(
    dsl: &Dsl,
    file: &str,
    source: &str,
) -> syn::Result<(Vec<Site>, BTreeSet<String>)> {
    let ast = syn::parse_file(source)?;
    let mut scope = Scope {
        dsl,
        renames: BTreeMap::new(),
        globs: Vec::new(),
        imported: BTreeMap::new(),
    };
    scope.visit_file(&ast);
    let mut reader = EngineReader {
        scope,
        file: file.to_owned(),
        funcs: Vec::new(),
        sites: Vec::new(),
        own_enums: BTreeSet::new(),
    };
    reader.visit_file(&ast);
    Ok((reader.sites, reader.own_enums))
}

/// An `llvm-cov` export's code regions, by file under [`ENGINE_SOURCE`]:
/// start, end, how often it ran.
#[derive(Default)]
struct Coverage {
    files: BTreeMap<String, Vec<(Pos, Pos, u64)>>,
}

impl Coverage {
    fn load(path: &Path) -> anyhow::Result<Self> {
        let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        let json: serde_json::Value = serde_json::from_slice(&bytes)
            .with_context(|| format!("{} is no JSON", path.display()))?;
        let functions = json["data"][0]["functions"]
            .as_array()
            .context("no `data[0].functions`: not an llvm-cov JSON export")?;
        let marker = format!("{ENGINE_SOURCE}/");
        let mut cov = Self::default();
        for f in functions {
            let names: Vec<Option<String>> = f["filenames"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|n| {
                    n.as_str()
                        .and_then(|n| n.split_once(&marker))
                        .map(|(_, rel)| rel.to_owned())
                })
                .collect();
            for r in f["regions"].as_array().into_iter().flatten() {
                let r: Vec<u64> = r
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(serde_json::Value::as_u64)
                    .collect();
                // [line, col, end line, end col, count, file, expanded file, kind];
                // kind 0 is a code region.
                let [ls, cs, le, ce, count, file, _, 0] = r.as_slice() else {
                    continue;
                };
                let Some(Some(name)) = usize::try_from(*file).ok().and_then(|i| names.get(i))
                else {
                    continue;
                };
                let p = |a: u64| u32::try_from(a).unwrap_or(u32::MAX);
                cov.files.entry(name.clone()).or_default().push((
                    (p(*ls), p(*cs)),
                    (p(*le), p(*ce)),
                    *count,
                ));
            }
        }
        for regions in cov.files.values_mut() {
            regions.sort_unstable();
            // One region per span: a generic function's instances merged.
            regions.dedup_by(|later, kept| {
                let same = later.0 == kept.0 && later.1 == kept.1;
                if same {
                    kept.2 = kept.2.max(later.2);
                }
                same
            });
        }
        Ok(cov)
    }

    /// How often the code in `start..=end` ran, and where that was read:
    /// the most any region starting in it ran; else (an expression with no
    /// code of its own, such as a constant) what the nearest region before
    /// it on its line ran; else what the innermost region around its start
    /// ran. `None` where nothing there was instrumented.
    fn count(&self, file: &str, start: Pos, end: Pos) -> Option<(u64, Via)> {
        let regions = self.files.get(file)?;
        let from = regions.partition_point(|r| r.0 < start);
        let inside = regions[from..]
            .iter()
            .take_while(|r| r.0 <= end)
            .map(|r| (r.2, Via::Inside))
            .max();
        inside
            .or_else(|| {
                regions[..from]
                    .last()
                    .filter(|r| r.0.0 == start.0)
                    .map(|r| (r.2, Via::SameLine))
            })
            .or_else(|| {
                regions[..from]
                    .iter()
                    .rev()
                    .find(|r| r.1 >= start)
                    .map(|r| (r.2, Via::Enclosing))
            })
    }
}

/// Where a site's count was read ([`Coverage::count`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Via {
    /// A region starting in the site.
    Inside,
    /// The nearest region before it on its line.
    SameLine,
    /// The innermost region around it.
    Enclosing,
}

impl Via {
    fn name(self) -> &'static str {
        match self {
            Self::Inside => "inside",
            Self::SameLine => "same_line",
            Self::Enclosing => "enclosing",
        }
    }
}

/// How far the engine's rule tests run one mechanic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    /// No engine code names it where coverage can see.
    Unsited,
    /// No site naming it ran.
    Untested,
    /// Some sites ran, some did not.
    Partly,
    /// Every site naming it ran.
    Tested,
}

impl Status {
    fn name(self) -> &'static str {
        match self {
            Self::Untested => "untested",
            Self::Partly => "partly",
            Self::Tested => "tested",
            Self::Unsited => "unsited",
        }
    }
}

/// One site as the report shows it.
#[derive(Clone, Debug)]
pub struct Seen {
    /// `file:line`.
    pub at: String,
    /// The function it is in.
    pub func: String,
    /// How often it ran; `None` where no region covers it.
    pub count: Option<u64>,
    /// Where the count was read.
    pub via: Option<Via>,
    /// Shared with another variant (`A | B`).
    pub shared: bool,
}

/// One mechanic's evidence.
#[derive(Clone, Debug)]
pub struct Evidence {
    /// How far it is tested.
    pub status: Status,
    /// Whether the sites read are checks, not arms (no arm names it).
    pub by_checks: bool,
    /// The sites read: arms where it has any, else checks; bookkeeping left
    /// out.
    pub sites: Vec<Seen>,
}

impl Evidence {
    fn counted(&self) -> impl Iterator<Item = &Seen> {
        self.sites.iter().filter(|s| s.count.is_some())
    }

    fn ran(&self) -> usize {
        self.counted().filter(|s| s.count > Some(0)).count()
    }

    fn unrun(&self) -> Vec<&Seen> {
        self.counted().filter(|s| s.count == Some(0)).collect()
    }
}

fn evidence(sites: &[&Site], cov: &Coverage) -> Evidence {
    let rules: Vec<&&Site> = sites
        .iter()
        .filter(|s| !BOOKKEEPING.iter().any(|b| s.func.contains(b)))
        .collect();
    let arms: Vec<&&Site> = rules
        .iter()
        .copied()
        .filter(|s| s.kind == Kind::Arm)
        .collect();
    let by_checks = arms.is_empty();
    let read = if by_checks { rules } else { arms };
    let sites: Vec<Seen> = read
        .iter()
        .map(|s| {
            let count = cov.count(&s.file, s.start, s.end);
            Seen {
                at: format!("{}:{}", s.file, s.start.0),
                func: s.func.clone(),
                count: count.map(|c| c.0),
                via: count.map(|c| c.1),
                shared: s.shared,
            }
        })
        .collect();
    let mut e = Evidence {
        status: Status::Unsited,
        by_checks,
        sites,
    };
    let counted = e.counted().count();
    e.status = match (counted, e.ran()) {
        (0, _) => Status::Unsited,
        (_, 0) => Status::Untested,
        (n, r) if r == n => Status::Tested,
        _ => Status::Partly,
    };
    e
}

/// A function whose sites did not run ([`Analysis::cold_functions`]).
struct Cold<'a> {
    /// Its first such site, `file:line`.
    at: &'a str,
    func: &'a str,
    /// The partly run mechanics it names at those sites.
    mechanics: BTreeSet<&'a Mechanic>,
    /// Ranked (L3) and house cards using one of them.
    l3: usize,
    house: usize,
}

/// What the mechanics analysis found.
pub struct Analysis {
    /// Per mechanic a pool card's key resolves to, its evidence.
    pub mechanics: BTreeMap<Mechanic, Evidence>,
    /// Per implemented card, the mechanics it uses and the status of each
    /// (the best, where a key could name several).
    pub uses: BTreeMap<CardIndex, BTreeMap<Mechanic, Status>>,
    /// Card keys that name no DSL variant: key → (why, cards using it).
    pub unmapped: BTreeMap<String, (String, usize)>,
    /// Enums the engine defines under a DSL enum's name.
    pub collisions: BTreeSet<String>,
    /// Sites read in the engine.
    pub engine_sites: usize,
    /// Engine files read.
    pub engine_files: usize,
    /// The commit the engine's source was read at ([`engine_head`]).
    pub engine_head: String,
    /// Why the export may not match the source, if it may not.
    pub stale: Option<String>,
}

/// Reads the DSL, the engine and the coverage export, and grades every
/// mechanic an implemented card uses.
///
/// # Errors
/// When a source file cannot be read or parsed, or the export is not one.
pub fn analyse(root: &Path, coverage: &Path) -> anyhow::Result<Analysis> {
    let dsl = read_dsl(root)?;
    let engine = read_engine(root, &dsl)?;
    let sites = &engine.sites;
    let collisions: BTreeSet<String> = engine
        .own_enums
        .iter()
        .filter(|e| dsl.is_enum(e))
        .cloned()
        .collect();
    let cov = Coverage::load(coverage)?;
    let exported = fs::metadata(coverage)?.modified()?;
    let stale = engine
        .newest
        .as_ref()
        .filter(|(_, changed)| *changed > exported)
        .map(|(file, _)| {
            format!(
                "{ENGINE_SOURCE}/{file} changed after the export was made: its lines may have moved, so make the export again"
            )
        });

    let mut by_mechanic: BTreeMap<&Mechanic, Vec<&Site>> = BTreeMap::new();
    for s in sites {
        for m in &s.names {
            by_mechanic.entry(m).or_default().push(s);
        }
    }

    let mut mechanics: BTreeMap<Mechanic, Evidence> = BTreeMap::new();
    let mut uses = BTreeMap::new();
    let mut unmapped: BTreeMap<String, (String, usize)> = BTreeMap::new();
    for def in baylee_cards::all().filter(|d| d.is_implemented()) {
        let mut card: BTreeMap<Mechanic, Status> = BTreeMap::new();
        for key in baylee_train::cardwalk::mechanics(def) {
            let options = match dsl.resolve(&key) {
                Resolved::Mechanics(m) => m,
                Resolved::Structure => continue,
                Resolved::Unmapped(why) => {
                    unmapped.entry(key).or_insert((why, 0)).1 += 1;
                    continue;
                }
            };
            let best = options
                .into_iter()
                .map(|m| {
                    let e = mechanics.entry(m.clone()).or_insert_with(|| {
                        evidence(by_mechanic.get(&m).map_or(&[][..], Vec::as_slice), &cov)
                    });
                    (e.status, m)
                })
                .max();
            if let Some((status, m)) = best {
                card.insert(m, status);
            }
        }
        uses.insert(def.index, card);
    }
    Ok(Analysis {
        mechanics,
        uses,
        unmapped,
        collisions,
        engine_sites: sites.len(),
        engine_files: engine.files,
        engine_head: engine_head(root),
        stale,
    })
}

/// The DSL's types, read from source.
fn read_dsl(root: &Path) -> anyhow::Result<Dsl> {
    let mut dsl = Dsl::default();
    for dir in DSL_SOURCES {
        for path in sources(&root.join(dir))? {
            let text = fs::read_to_string(&path)?;
            dsl.read(&text)
                .with_context(|| format!("parsing {}", path.display()))?;
        }
    }
    dsl.finish();
    Ok(dsl)
}

/// What the engine's source says.
struct Engine {
    sites: Vec<Site>,
    own_enums: BTreeSet<String>,
    files: usize,
    /// The file changed last, and when.
    newest: Option<(String, std::time::SystemTime)>,
}

fn read_engine(root: &Path, dsl: &Dsl) -> anyhow::Result<Engine> {
    let dir = root.join(ENGINE_SOURCE);
    let files = sources(&dir)?;
    let mut engine = Engine {
        sites: Vec::new(),
        own_enums: BTreeSet::new(),
        files: files.len(),
        newest: None,
    };
    for path in &files {
        let rel = path
            .strip_prefix(&dir)?
            .to_string_lossy()
            .replace('\\', "/");
        let text = fs::read_to_string(path)?;
        let (found, enums) = read_engine_file(dsl, &rel, &text)
            .with_context(|| format!("parsing {}", path.display()))?;
        engine.sites.extend(found);
        engine.own_enums.extend(enums);
        let changed = fs::metadata(path)?.modified()?;
        if engine.newest.as_ref().is_none_or(|(_, t)| changed > *t) {
            engine.newest = Some((rel, changed));
        }
    }
    Ok(engine)
}

/// The commit the engine's source is at, with `+changes` where its working
/// tree differs; empty where git cannot say.
fn engine_head(root: &Path) -> String {
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
    };
    let head = git(&["rev-parse", "--short", "HEAD"]).unwrap_or_default();
    let dirty = git(&["status", "--porcelain", "--", ENGINE_SOURCE]).is_some_and(|s| !s.is_empty());
    if dirty {
        format!("{head}+changes")
    } else {
        head
    }
}

impl Analysis {
    /// The mechanics a card uses at `status`.
    fn at(&self, card: CardIndex, status: Status) -> Vec<&Mechanic> {
        self.uses
            .get(&card)
            .into_iter()
            .flatten()
            .filter(|(_, s)| **s == status)
            .map(|(m, _)| m)
            .collect()
    }

    /// Why an L3 card stops short of L4's mechanics part (a mechanic it uses
    /// is untested), if it does.
    pub fn stop(&self, card: CardIndex) -> Option<String> {
        let ms = self.at(card, Status::Untested);
        if ms.is_empty() {
            return None;
        }
        let mut names: Vec<String> = ms.iter().take(3).map(|m| show(m)).collect();
        if ms.len() > 3 {
            names.push(format!("+{}", ms.len() - 3));
        }
        Some(format!("untested mechanics: {}", names.join(", ")))
    }

    /// Per mechanic at `status`, the `ranked` and `house` cards using it,
    /// most house cards first.
    fn ranked_at(
        &self,
        status: Status,
        ranked: &BTreeSet<CardIndex>,
        house: &BTreeSet<CardIndex>,
    ) -> Vec<(&Mechanic, usize, usize)> {
        let mut by: BTreeMap<&Mechanic, (usize, usize)> = BTreeMap::new();
        for (card, ms) in &self.uses {
            for (m, s) in ms {
                if *s == status {
                    let b = by.entry(m).or_default();
                    b.0 += usize::from(ranked.contains(card));
                    b.1 += usize::from(house.contains(card));
                }
            }
        }
        let mut out: Vec<_> = by.into_iter().map(|(m, (r, h))| (m, r, h)).collect();
        out.sort_by(|a, b| (b.2, b.1).cmp(&(a.2, a.1)).then(a.0.cmp(b.0)));
        out
    }

    /// The functions whose sites did not run for a partly run mechanic, with
    /// the first such site, the mechanics they leave partly run and the
    /// `ranked` and `house` cards using one of those; most house cards
    /// first. A rule test through one of them makes every mechanic it
    /// names run there.
    fn cold_functions(
        &self,
        ranked: &BTreeSet<CardIndex>,
        house: &BTreeSet<CardIndex>,
    ) -> Vec<Cold<'_>> {
        let mut cold: BTreeMap<(&str, &str), Cold<'_>> = BTreeMap::new();
        for (m, e) in &self.mechanics {
            if e.status != Status::Partly {
                continue;
            }
            for site in e.unrun() {
                let file = site.at.split_once(':').map_or(site.at.as_str(), |(f, _)| f);
                let c = cold.entry((file, &site.func)).or_insert_with(|| Cold {
                    at: &site.at,
                    func: &site.func,
                    mechanics: BTreeSet::new(),
                    l3: 0,
                    house: 0,
                });
                c.at = c.at.min(&site.at);
                c.mechanics.insert(m);
            }
        }
        let mut out: Vec<Cold<'_>> = cold.into_values().collect();
        for c in &mut out {
            for (card, ms) in &self.uses {
                if ms.keys().any(|m| c.mechanics.contains(m)) {
                    c.l3 += usize::from(ranked.contains(card));
                    c.house += usize::from(house.contains(card));
                }
            }
        }
        out.sort_by(|a, b| (b.house, b.l3).cmp(&(a.house, a.l3)).then(a.at.cmp(b.at)));
        out
    }

    /// Prints the gap report (the untested mechanics that keep the most
    /// house-deck and `ranked` cards from L4, the partly run ones, the
    /// unsited and unmapped ones) and returns it as JSON.
    #[allow(clippy::too_many_lines)] // one report: summary, three lists, JSON
    pub fn report(
        &self,
        ranked: &BTreeSet<CardIndex>,
        house: &BTreeSet<CardIndex>,
    ) -> serde_json::Value {
        let count = |s: Status| self.mechanics.values().filter(|e| e.status == s).count();
        println!(
            "mechanics: {} DSL variants the pool uses; engine: {} sites in {} files",
            self.mechanics.len(),
            self.engine_sites,
            self.engine_files
        );
        println!(
            "  tested {}   partly {}   untested {}   unsited {}",
            count(Status::Tested),
            count(Status::Partly),
            count(Status::Untested),
            count(Status::Unsited)
        );
        let clear = ranked
            .iter()
            .filter(|c| self.at(**c, Status::Untested).is_empty())
            .count();
        let full = ranked
            .iter()
            .filter(|c| {
                self.at(**c, Status::Untested).is_empty() && self.at(**c, Status::Partly).is_empty()
            })
            .count();
        println!(
            "  of {} L3 cards: {clear} use no untested mechanic (L4's mechanics part), {full} only fully run ones",
            ranked.len()
        );
        println!("  engine source at {}", self.engine_head);
        if let Some(why) = &self.stale {
            println!("  WARNING: {why}");
        }
        if !self.collisions.is_empty() {
            println!(
                "  the engine defines enums named like DSL ones (their arms may be misread): {}",
                self.collisions
                    .iter()
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }

        let row = |m: &Mechanic, l3: usize, h: usize| {
            let e = &self.mechanics[m];
            let first = e.unrun().first().map(|s| format!("{} ({})", s.at, s.func));
            println!(
                "  {:44} {h:6} {l3:5}  {:>4}/{:<4}{} {}",
                show(m),
                e.ran(),
                e.counted().count(),
                if e.by_checks { "c" } else { " " },
                first.unwrap_or_default()
            );
        };
        let header = || {
            println!(
                "  {:44} {:>6} {:>5}  {:>9}  first site that did not run",
                "mechanic", "house", "L3", "ran"
            );
        };
        let untested = self.ranked_at(Status::Untested, ranked, house);
        println!("untested (keep cards from L4), by house-deck cards then L3 cards using them:");
        header();
        for (m, l3, h) in untested.iter().take(25) {
            row(m, *l3, *h);
        }
        let partly = self.ranked_at(Status::Partly, ranked, house);
        println!(
            "partly run ({}; not blocking, the sites that did not run are the tests to write):",
            partly.len()
        );
        header();
        for (m, l3, h) in partly.iter().take(15) {
            row(m, *l3, *h);
        }
        let cold = self.cold_functions(ranked, house);
        println!(
            "functions those sites are in ({}), by the cards whose mechanics they leave partly run:",
            cold.len()
        );
        println!(
            "  {:44} {:>6} {:>5}  {:>9}  mechanics",
            "first site (function)", "house", "L3", ""
        );
        for c in cold.iter().take(12) {
            let names: Vec<String> = c.mechanics.iter().take(4).map(|m| show(m)).collect();
            let more = c.mechanics.len().saturating_sub(4);
            println!(
                "  {:44} {:6} {:5}  {:>9}  {}{}",
                format!("{} ({})", c.at, c.func),
                c.house,
                c.l3,
                "",
                names.join(", "),
                if more > 0 {
                    format!(" +{more}")
                } else {
                    String::new()
                }
            );
        }
        let unsited = self.ranked_at(Status::Unsited, ranked, house);
        println!(
            "unsited: {} mechanics no instrumented engine code names; most used:",
            unsited.len()
        );
        for (m, l3, h) in unsited.iter().take(10) {
            println!("  {:44} {h:6} {l3:5}", show(m));
        }
        let mut unmapped: Vec<_> = self.unmapped.iter().collect();
        unmapped.sort_by(|a, b| b.1.1.cmp(&a.1.1).then(a.0.cmp(b.0)));
        println!(
            "unmapped: {} card keys name no DSL variant; most used:",
            unmapped.len()
        );
        for (key, (why, n)) in unmapped.iter().take(10) {
            println!("  {key:44} {n:5}  {why}");
        }

        let listed = |list: &[(&Mechanic, usize, usize)]| -> Vec<serde_json::Value> {
            list.iter()
                .map(|(m, l3, h)| serde_json::json!({"mechanic": show(m), "house_cards": h, "l3_cards": l3}))
                .collect()
        };
        serde_json::json!({
            "status_counts": {
                "tested": count(Status::Tested),
                "partly": count(Status::Partly),
                "untested": count(Status::Untested),
                "unsited": count(Status::Unsited),
            },
            "l3_cards": ranked.len(),
            "l3_no_untested": clear,
            "l3_fully_run": full,
            "collisions": self.collisions,
            "engine_head": self.engine_head,
            "stale": self.stale,
            "untested": listed(&untested),
            "cold_functions": cold.iter().map(|c| serde_json::json!({
                "at": c.at, "func": c.func, "house_cards": c.house, "l3_cards": c.l3,
                "mechanics": c.mechanics.iter().map(|m| show(m)).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "partly": listed(&partly),
            "unsited": listed(&unsited),
            "mechanics": self.mechanics.iter().map(|(m, e)| serde_json::json!({
                "mechanic": show(m),
                "status": e.status.name(),
                "by_checks": e.by_checks,
                "sites": e.sites.iter().map(|s| serde_json::json!({
                    "at": s.at, "func": s.func, "count": s.count, "shared": s.shared, "via": s.via.map(Via::name),
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
            "unmapped": unmapped.iter().map(|(k, (why, n))| serde_json::json!({"key": k, "cards": n, "why": why})).collect::<Vec<_>>(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dsl() -> Dsl {
        let mut d = Dsl::default();
        d.read(
            r#"
            pub enum Amount { Fixed(i32), X }
            pub enum ManaSource { Fixed(u8), Any }
            pub enum Unrelated { Any }
            pub type Amounts = &'static [Amount];
            pub enum Effect { DealDamage { amount: Amount }, Gain { amounts: Amounts }, Draw, Mill, Scry, Tag(SubtypeId) }
            pub struct Cost { mana: u8 }
            pub struct CardDef { effects: &'static [Effect], cost: Cost, mana: ManaSource, keywords: KeywordSet }
            pub struct KeywordSet(u128);
            keywords! { FLYING = 0, "Flying."; HASTE = 4, "Haste."; }
            "#,
        )
        .unwrap();
        d.finish();
        d
    }

    fn m(ty: &str, v: &str) -> Mechanic {
        (ty.to_owned(), v.to_owned())
    }

    #[test]
    fn a_key_resolves_through_its_fields_type() {
        let d = dsl();
        assert!(!d.is_enum("Unrelated"), "no card can hold it");
        assert_eq!(
            d.resolve("amount>Fixed"),
            Resolved::Mechanics(vec![m("Amount", "Fixed")])
        );
        assert_eq!(
            d.resolve("amounts>X"),
            Resolved::Mechanics(vec![m("Amount", "X")])
        );
        assert_eq!(
            d.resolve("effects>Draw"),
            Resolved::Mechanics(vec![m("Effect", "Draw")])
        );
        // Not a field: the one DSL enum with that variant.
        assert_eq!(
            d.resolve("face_effects>Mill"),
            Resolved::Mechanics(vec![m("Effect", "Mill")])
        );
        assert_eq!(
            d.resolve("face_mana>Any"),
            Resolved::Mechanics(vec![m("ManaSource", "Any")])
        );
        assert!(matches!(
            d.resolve("face_amount>Fixed"),
            Resolved::Unmapped(_)
        ));
        assert_eq!(d.resolve("cost>Cost"), Resolved::Structure);
        assert_eq!(d.resolve("Tag>SubtypeId"), Resolved::Structure);
        assert_eq!(
            d.resolve("keyword>4"),
            Resolved::Mechanics(vec![m(KEYWORDS, "HASTE")])
        );
        assert!(matches!(d.resolve("keyword>9"), Resolved::Unmapped(_)));
    }

    #[test]
    fn engine_sites_are_read_with_their_imports() {
        let d = dsl();
        let (sites, own) = read_engine_file(
            &d,
            "f.rs",
            r#"
            use dsl::Effect as E;
            use dsl::Amount::*;
            use dsl::{KeywordSet, ManaSource::{self as Src}};
            enum Local { A }
            fn f(e: &E, a: Amount, s: Src) {
                match e {
                    E::DealDamage { amount: Fixed(_) } => {}
                    E::Draw | E::Mill => {}
                    E::Gain { .. } => unreachable!("never"),
                    _ => {}
                }
                if let E::Scry = e && let X = a {}
                if matches!(s, Src::Any) {}
                if k.contains(KeywordSet::FLYING) {}
                if *s == Src::Fixed(1) || s != &Src::Any {}
                let E::Mill = e else { return };
                g();
            }
            fn filter_hash(e: &E) { match e { E::Draw => {} _ => {} } }
            #[cfg(test)]
            mod tests { fn t(e: &E) { match e { E::Gain { .. } => {} _ => {} } } }
            "#,
        )
        .unwrap();
        let got: Vec<(&str, Kind, bool, Vec<Mechanic>)> = sites
            .iter()
            .map(|s| (s.func.as_str(), s.kind, s.shared, s.names.clone()))
            .collect();
        assert_eq!(
            got,
            vec![
                ("f", Kind::Arm, false, vec![m("Effect", "Mill")]),
                (
                    "f",
                    Kind::Arm,
                    false,
                    vec![m("Amount", "Fixed"), m("Effect", "DealDamage")]
                ),
                (
                    "f",
                    Kind::Arm,
                    true,
                    vec![m("Effect", "Draw"), m("Effect", "Mill")]
                ),
                ("f", Kind::Arm, false, vec![m("Effect", "Scry")]),
                ("f", Kind::Arm, false, vec![m("Amount", "X")]),
                ("f", Kind::Check, false, vec![m("ManaSource", "Any")]),
                ("f", Kind::Check, false, vec![m(KEYWORDS, "FLYING")]),
                ("f", Kind::Check, false, vec![m("ManaSource", "Any")]),
                ("filter_hash", Kind::Arm, false, vec![m("Effect", "Draw")]),
            ]
        );
        assert_eq!(sites[1].start.0, 8, "an arm's site is its body");
        assert_eq!(
            sites[0].start.0, 18,
            "a let-else guards the rest of its block"
        );
        assert!(own.contains("Local"));
    }

    #[test]
    fn a_span_ran_as_often_as_its_code() {
        let mut cov = Coverage::default();
        cov.files.insert(
            "f.rs".into(),
            vec![
                ((10, 1), (14, 2), 1),
                ((11, 20), (11, 30), 0),
                ((12, 20), (12, 30), 3),
                ((13, 20), (13, 30), 4),
            ],
        );
        assert_eq!(
            cov.count("f.rs", (12, 20), (12, 30)),
            Some((3, Via::Inside))
        );
        assert_eq!(
            cov.count("f.rs", (11, 20), (11, 30)),
            Some((0, Via::Inside))
        );
        // No region starts inside: the nearest before it on its line, else
        // the innermost one around it.
        assert_eq!(
            cov.count("f.rs", (13, 32), (13, 40)),
            Some((4, Via::SameLine))
        );
        assert_eq!(
            cov.count("f.rs", (13, 5), (13, 9)),
            Some((1, Via::Enclosing))
        );
        assert_eq!(cov.count("f.rs", (20, 1), (20, 9)), None);
        assert_eq!(cov.count("g.rs", (10, 1), (10, 9)), None);
    }

    fn workspace() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("xtask lives in the workspace")
    }

    /// A renamed DSL type or a new `cardwalk` root would otherwise turn
    /// cards' mechanics into unread keys without a word.
    #[test]
    fn every_key_of_an_implemented_card_is_read() {
        let dsl = read_dsl(workspace()).unwrap();
        let unread: BTreeSet<String> = baylee_cards::all()
            .filter(|d| d.is_implemented())
            .flat_map(baylee_train::cardwalk::mechanics)
            .filter(|k| matches!(dsl.resolve(k), Resolved::Unmapped(_)))
            .collect();
        assert!(
            unread.is_empty(),
            "card keys the mechanics report cannot read: {unread:?}"
        );
    }

    /// A textual reader: a misread `use` or a new way of writing arms moves
    /// the count out of these bounds (676 sites on 2026-09-30).
    #[test]
    fn the_engine_is_read_to_a_plausible_number_of_sites() {
        let dsl = read_dsl(workspace()).unwrap();
        let engine = read_engine(workspace(), &dsl).unwrap();
        let n = engine.sites.len();
        assert!((400..=2000).contains(&n), "{n} engine sites");
        assert!(engine.files >= 30, "{} engine files", engine.files);
    }

    #[test]
    fn bookkeeping_is_no_evidence_and_arms_outrank_checks() {
        let site = |func: &str, kind: Kind, line: u32| Site {
            file: "f.rs".into(),
            func: func.into(),
            kind,
            shared: false,
            names: vec![m("Effect", "Draw")],
            start: (line, 1),
            end: (line, 9),
        };
        let mut cov = Coverage::default();
        cov.files.insert(
            "f.rs".into(),
            vec![
                ((1, 1), (1, 9), 0),
                ((2, 1), (2, 9), 5),
                ((3, 1), (3, 9), 0),
            ],
        );
        let (hash, check, arm) = (
            site("filter_hash", Kind::Arm, 1),
            site("f", Kind::Check, 2),
            site("f", Kind::Arm, 3),
        );
        assert_eq!(evidence(&[&hash, &check], &cov).status, Status::Tested);
        assert_eq!(
            evidence(&[&hash, &check, &arm], &cov).status,
            Status::Untested
        );
        assert_eq!(evidence(&[&hash], &cov).status, Status::Unsited);
    }
}
