//! Lossless parser for Valve KeyValues 1 / VGUI resource files.
//!
//! Every byte of the input is represented by exactly one token, so any file can be rebuilt byte-for-byte.
//! On top of the token stream this crate offers:
//! * a flat list of blocks ([`Panel`]) with direct properties — what the inspector edits,
//! * a full tree ([`Tree`]) with `#base` / `#include` directives — what the resolver merges,
//! * surgical edit operations that only touch the bytes they must change.
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TokenKind {
    Whitespace,
    LineComment,
    BlockComment,
    Quoted,
    Bare,
    Open,
    Close,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub value: String,
    pub span: Span,
    pub condition: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Panel {
    pub id: String,
    pub name: String,
    pub control: Option<String>,
    pub properties: Vec<Property>,
    pub span: Span,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub tokens: Vec<Token>,
    pub panels: Vec<Panel>,
    pub diagnostics: Vec<Diagnostic>,
}

/// `#base "file"` or `#include "file"` at the top level of a file.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Directive {
    pub kind: String,
    pub path: String,
    pub span: Span,
    pub condition: Option<String>,
}

/// One key in the KeyValues tree: either a string value or a block of children.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KvNode {
    pub key: String,
    /// Span of the whole entry: from the key's first byte to the closing brace / value's last byte.
    pub span: Span,
    pub condition: Option<String>,
    /// Stable block id (same scheme as [`Panel::id`]); empty for string values.
    pub id: String,
    pub value: Option<String>,
    /// Span of the value token (string values only).
    pub value_span: Option<Span>,
    pub children: Vec<KvNode>,
}

impl KvNode {
    pub fn is_block(&self) -> bool {
        self.value.is_none()
    }
    /// First child whose key matches (case-insensitive), like KeyValues::FindKey.
    pub fn find(&self, key: &str) -> Option<&KvNode> {
        self.children.iter().find(|c| c.key.eq_ignore_ascii_case(key))
    }
    /// String value of the first matching child whose platform condition holds.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.children
            .iter()
            .find(|c| c.key.eq_ignore_ascii_case(key) && c.value.is_some() && c.condition.as_deref().is_none_or(condition_met))
            .and_then(|c| c.value.as_deref())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Tree {
    pub roots: Vec<KvNode>,
    pub directives: Vec<Directive>,
    pub diagnostics: Vec<Diagnostic>,
}

const BOM: &str = "\u{feff}";

fn text(source: &str, token: &Token) -> String {
    let raw = &source[token.span.start..token.span.end];
    if token.kind == TokenKind::Quoted && raw.len() >= 2 && raw.ends_with('"') {
        unescape(&raw[1..raw.len() - 1])
    } else if token.kind == TokenKind::Quoted {
        unescape(&raw[1..])
    } else {
        raw.to_string()
    }
}

/// KeyValues only treats `\"` and `\\` (plus \n, \t) specially inside quoted strings.
fn unescape(raw: &str) -> String {
    if !raw.contains('\\') {
        return raw.to_string();
    }
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some(other) => {
                    out.push('\\');
                    out.push(other)
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c)
        }
    }
    out
}

fn lex(source: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    let b = source.as_bytes();
    let mut i = 0;
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    if source.starts_with(BOM) {
        tokens.push(Token { kind: TokenKind::Whitespace, span: Span { start: 0, end: BOM.len() } });
        i = BOM.len();
    }
    while i < b.len() {
        let start = i;
        let kind = if b[i].is_ascii_whitespace() {
            while i < b.len() && b[i].is_ascii_whitespace() {
                i += 1
            }
            TokenKind::Whitespace
        } else if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1
            }
            TokenKind::LineComment
        } else if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
            i += 2;
            let mut closed = false;
            while i + 1 < b.len() {
                if b[i] == b'*' && b[i + 1] == b'/' {
                    i += 2;
                    closed = true;
                    break;
                }
                i += 1
            }
            if !closed {
                i = b.len();
                diagnostics.push(Diagnostic { message: "Unterminated block comment".into(), span: Span { start, end: i } })
            }
            TokenKind::BlockComment
        } else if b[i] == b'{' {
            i += 1;
            TokenKind::Open
        } else if b[i] == b'}' {
            i += 1;
            TokenKind::Close
        } else if b[i] == b'"' {
            i += 1;
            let mut closed = false;
            while i < b.len() {
                if b[i] == b'\\' && i + 1 < b.len() {
                    i += 2;
                    continue;
                }
                if b[i] == b'"' {
                    i += 1;
                    closed = true;
                    break;
                }
                // A raw newline inside quotes is legal KeyValues but almost always a missing quote; keep going
                // like the engine does, but report it.
                i += 1
            }
            if !closed {
                diagnostics.push(Diagnostic { message: "Unterminated quoted string".into(), span: Span { start, end: i } })
            }
            TokenKind::Quoted
        } else {
            i += 1;
            while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'{' && b[i] != b'}' && b[i] != b'"' {
                if b[i] == b'/' && matches!(b.get(i + 1), Some(b'/') | Some(b'*')) {
                    break;
                }
                i += 1
            }
            // never split a UTF-8 sequence
            while i < b.len() && !source.is_char_boundary(i) {
                i += 1
            }
            TokenKind::Bare
        };
        tokens.push(Token { kind, span: Span { start, end: i } })
    }
    (tokens, diagnostics)
}

fn significant(k: &TokenKind) -> bool {
    !matches!(k, TokenKind::Whitespace | TokenKind::LineComment | TokenKind::BlockComment)
}

fn is_condition(raw: &str) -> bool {
    raw.len() >= 2 && raw.starts_with('[') && raw.ends_with(']')
}

fn is_directive(key: &str) -> Option<&'static str> {
    let k = key.to_ascii_lowercase();
    if k == "#base" {
        Some("#base")
    } else if k == "#include" {
        Some("#include")
    } else {
        None
    }
}

struct TreeParser<'a> {
    source: &'a str,
    tokens: &'a [Token],
    sig: Vec<usize>,
    diagnostics: Vec<Diagnostic>,
    directives: Vec<Directive>,
}

impl TreeParser<'_> {
    fn tok(&self, cursor: usize) -> &Token {
        &self.tokens[self.sig[cursor]]
    }
    fn take_condition(&self, cursor: &mut usize) -> Option<String> {
        if *cursor < self.sig.len() {
            let t = self.tok(*cursor);
            if matches!(t.kind, TokenKind::Bare | TokenKind::Quoted) {
                let raw = text(self.source, t);
                if is_condition(&raw) {
                    *cursor += 1;
                    return Some(raw[1..raw.len() - 1].to_string());
                }
            }
        }
        None
    }
    /// Parses entries until the matching close brace (when `closes`) or end of input.
    /// Returns the children and the byte offset just after the closing brace.
    fn entries(&mut self, cursor: &mut usize, parent: &str, closes: bool) -> (Vec<KvNode>, usize) {
        let mut occurrences = HashMap::<String, usize>::new();
        let mut out = Vec::new();
        while *cursor < self.sig.len() {
            let key = self.tok(*cursor).clone();
            if key.kind == TokenKind::Close {
                *cursor += 1;
                if !closes {
                    self.diagnostics.push(Diagnostic { message: "Unexpected closing brace".into(), span: key.span.clone() });
                    continue;
                }
                return (out, key.span.end);
            }
            if !matches!(key.kind, TokenKind::Quoted | TokenKind::Bare) {
                self.diagnostics.push(Diagnostic { message: "Expected a key".into(), span: key.span.clone() });
                *cursor += 1;
                continue;
            }
            let name = text(self.source, &key);
            *cursor += 1;
            if *cursor >= self.sig.len() {
                self.diagnostics.push(Diagnostic { message: format!("Missing value or block for '{name}'"), span: key.span.clone() });
                break;
            }
            // `"Key" [$WIN32] { ... }` — a condition may sit between a block's key and its opening brace
            let mut pre_condition = None;
            if *cursor + 1 < self.sig.len() && self.tok(*cursor + 1).kind == TokenKind::Open {
                let t = self.tok(*cursor);
                if matches!(t.kind, TokenKind::Bare | TokenKind::Quoted) {
                    let raw = text(self.source, t);
                    if is_condition(&raw) {
                        pre_condition = Some(raw[1..raw.len() - 1].to_string());
                        *cursor += 1;
                    }
                }
            }
            let next = self.tok(*cursor).clone();
            if next.kind == TokenKind::Open {
                *cursor += 1;
                let n = occurrences.entry(name.to_ascii_lowercase()).or_default();
                let segment = if *n == 0 { name.clone() } else { format!("{name}[{}]", *n + 1) };
                *n += 1;
                let id = if parent.is_empty() { segment } else { format!("{parent}/{segment}") };
                let (children, end) = self.entries(cursor, &id, true);
                let condition = self.take_condition(cursor).or(pre_condition);
                out.push(KvNode { key: name, span: Span { start: key.span.start, end }, condition, id, value: None, value_span: None, children });
            } else if matches!(next.kind, TokenKind::Quoted | TokenKind::Bare) {
                *cursor += 1;
                let condition = self.take_condition(cursor);
                let value = text(self.source, &next);
                if parent.is_empty() {
                    if let Some(kind) = is_directive(&name) {
                        self.directives.push(Directive {
                            kind: kind.into(),
                            path: value.clone(),
                            span: Span { start: key.span.start, end: next.span.end },
                            condition: condition.clone(),
                        });
                    }
                }
                out.push(KvNode {
                    key: name,
                    span: Span { start: key.span.start, end: next.span.end },
                    condition,
                    id: String::new(),
                    value: Some(value),
                    value_span: Some(next.span.clone()),
                    children: Vec::new(),
                });
            } else {
                self.diagnostics.push(Diagnostic { message: format!("Expected value or '{{' after '{name}'"), span: next.span.clone() });
                *cursor += 1
            }
        }
        if closes {
            self.diagnostics.push(Diagnostic { message: "Missing closing brace".into(), span: Span { start: self.source.len(), end: self.source.len() } })
        }
        (out, self.source.len())
    }
}

fn build(source: &str) -> (Vec<Token>, Tree) {
    let (tokens, mut diagnostics) = lex(source);
    let sig = tokens.iter().enumerate().filter_map(|(i, t)| significant(&t.kind).then_some(i)).collect();
    let mut p = TreeParser { source, tokens: &tokens, sig, diagnostics: Vec::new(), directives: Vec::new() };
    let mut cursor = 0;
    let (roots, _) = p.entries(&mut cursor, "", false);
    diagnostics.extend(p.diagnostics);
    let tree = Tree { roots, directives: p.directives, diagnostics };
    (tokens, tree)
}

/// Full KeyValues tree with directives.
pub fn tree(source: &str) -> Tree {
    build(source).1
}

fn flatten(nodes: &[KvNode], out: &mut Vec<Panel>) {
    for node in nodes.iter().filter(|n| n.is_block()) {
        let properties: Vec<Property> = node
            .children
            .iter()
            .filter(|c| !c.is_block())
            .map(|c| Property {
                name: c.key.clone(),
                value: c.value.clone().unwrap_or_default(),
                span: c.value_span.clone().unwrap(),
                condition: c.condition.clone(),
            })
            .collect();
        let control = node.children.iter().find(|c| !c.is_block() && c.key.eq_ignore_ascii_case("ControlName")).and_then(|c| c.value.clone());
        out.push(Panel { id: node.id.clone(), name: node.key.clone(), control, properties, span: node.span.clone() });
        flatten(&node.children, out);
    }
}

pub fn parse(source: &str) -> Document {
    let (tokens, tree) = build(source);
    let mut panels = Vec::new();
    flatten(&tree.roots, &mut panels);
    Document { tokens, panels, diagnostics: tree.diagnostics }
}

pub fn panels(source: &str) -> Vec<Panel> {
    parse(source).panels
}

/// Platform whose `[$CONDITION]` flags are treated as true. HUDs are previewed for the game client they target,
/// not for the machine the editor runs on, so this defaults to Windows everywhere.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Platform {
    Windows,
    Linux,
    Mac,
}

static PLATFORM: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);

pub fn set_platform(p: Platform) {
    PLATFORM.store(p as u8, std::sync::atomic::Ordering::Relaxed);
}

pub fn platform() -> Platform {
    match PLATFORM.load(std::sync::atomic::Ordering::Relaxed) {
        1 => Platform::Linux,
        2 => Platform::Mac,
        _ => Platform::Windows,
    }
}

/// Evaluates a bracketed KeyValues condition suffix (e.g. "$WIN32", "!$X360") for the target platform.
pub fn condition_met(condition: &str) -> bool {
    // Compound conditions such as "$WIN32 && !$X360" or "$OSX || $LINUX"
    if condition.contains("||") {
        return condition.split("||").any(|c| condition_met(c.trim()));
    }
    if condition.contains("&&") {
        return condition.split("&&").all(|c| condition_met(c.trim()));
    }
    let condition = condition.trim();
    let (negate, flag) = match condition.strip_prefix('!') {
        Some(f) => (true, f),
        None => (false, condition),
    };
    let p = platform();
    let met = match flag.trim_start_matches('$').to_ascii_uppercase().as_str() {
        "WIN32" | "WINDOWS" => p == Platform::Windows,
        "OSX" => p == Platform::Mac,
        "LINUX" => p == Platform::Linux,
        "POSIX" => p != Platform::Windows,
        "X360" | "PS3" | "GAMECONSOLE" | "DECK" => false,
        _ => return true, // unrecognized flag: keep the property visible/editable rather than guess wrong
    };
    if negate {
        !met
    } else {
        met
    }
}

/// The occurrence the engine reads: KeyValues drops entries whose condition fails at load time and
/// `FindKey` returns the first remaining match. Falls back to the first occurrence of any condition so an
/// edit always lands somewhere, even for conditions this build doesn't recognise.
fn pick_property<'a>(properties: &'a [Property], name: &str) -> Option<&'a Property> {
    properties
        .iter()
        .find(|p| p.name.eq_ignore_ascii_case(name) && p.condition.as_deref().is_none_or(condition_met))
        .or_else(|| properties.iter().find(|p| p.name.eq_ignore_ascii_case(name)))
}

fn quoted(value: &str) -> Result<String, String> {
    if value.contains(['\n', '\r', '\0']) {
        return Err("Property value cannot contain a line break or NUL".into());
    }
    Ok(format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\"")))
}

fn valid_key(name: &str) -> Result<(), String> {
    if name.trim().is_empty() || name.chars().any(|c| c.is_whitespace() || matches!(c, '"' | '{' | '}')) {
        return Err(format!("Invalid key name '{name}'"));
    }
    Ok(())
}

fn newline_of(source: &str) -> &'static str {
    if source.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

fn indent_unit(source: &str) -> &'static str {
    if source.contains("\n\t") {
        "\t"
    } else {
        "    "
    }
}

/// Whitespace from the start of the line containing `offset` up to `offset`, if that prefix is all whitespace.
fn line_indent(source: &str, offset: usize) -> Option<(usize, &str)> {
    let line_start = source[..offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let prefix = &source[line_start..offset];
    prefix.chars().all(|c| c == ' ' || c == '\t').then_some((line_start, prefix))
}

/// Updates one occurrence, or inserts the property. Untouched bytes remain identical.
/// `at`, when given, is a property's `span.start` (as returned by `parse`) and selects that exact occurrence —
/// needed to edit a specific platform-conditioned duplicate (e.g. the `[$X360]` row, not the active `[$WIN32]` one).
/// Without `at`, the occurrence the engine actually reads is edited.
pub fn set_property(source: &str, panel_id: &str, property: &str, value: &str, at: Option<usize>) -> Result<String, String> {
    valid_key(property)?;
    let doc = parse(source);
    let panel = doc.panels.iter().find(|p| p.id == panel_id).ok_or("Panel not found; refresh document")?;
    let target = match at {
        Some(offset) => panel.properties.iter().find(|p| p.span.start == offset && p.name.eq_ignore_ascii_case(property)),
        None => pick_property(&panel.properties, property),
    };
    if let Some(prop) = target {
        let old = &source[prop.span.start..prop.span.end];
        let replacement = if old.starts_with('"') || value.is_empty() || value.chars().any(|c| c.is_whitespace() || matches!(c, '"' | '{' | '}' | '[')) {
            quoted(value)?
        } else {
            value.to_string()
        };
        let mut out = source.to_owned();
        out.replace_range(prop.span.start..prop.span.end, &replacement);
        return Ok(out);
    }
    insert_entry(source, panel, &format!("\"{property}\"{}{}", align_gap(source, property), quoted(value)?))
}

/// Keeps inserted rows visually aligned with the file's existing style (tab-separated columns vs single space).
fn align_gap(source: &str, key: &str) -> String {
    if source.contains("\"\t\t") {
        let width = key.len() + 2;
        let tabs = if width >= 24 { 1 } else { (24 - width).div_ceil(4) + 1 };
        "\t".repeat(tabs.max(1))
    } else {
        " ".into()
    }
}

/// Inserts `entry` (already formatted, single or multi line without trailing newline) as the last child of `panel`.
fn insert_entry(source: &str, panel: &Panel, entry: &str) -> Result<String, String> {
    let close = panel.span.end.checked_sub(1).filter(|i| source.as_bytes().get(*i) == Some(&b'}')).ok_or("Block has no closing brace")?;
    let newline = newline_of(source);
    let unit = indent_unit(source);
    let mut out = source.to_owned();
    match line_indent(source, close) {
        Some((line_start, indent)) => {
            let body = entry.replace('\n', &format!("{newline}{indent}{unit}"));
            out.insert_str(line_start, &format!("{indent}{unit}{body}{newline}"));
        }
        None => {
            // brace shares a line with other content: `{ "a" "b" }`
            out.insert_str(close, &format!(" {} ", entry.replace('\n', " ")));
        }
    }
    Ok(out)
}

/// Removes one property row (the occurrence at `at`, or the one the engine reads), including its condition and,
/// when it sits alone on its line, the whole line.
pub fn remove_property(source: &str, panel_id: &str, property: &str, at: Option<usize>) -> Result<String, String> {
    let t = tree(source);
    let block = find_node(&t.roots, panel_id).ok_or("Panel not found; refresh document")?;
    let candidates: Vec<&KvNode> = block.children.iter().filter(|c| !c.is_block() && c.key.eq_ignore_ascii_case(property)).collect();
    let node = match at {
        Some(offset) => candidates.into_iter().find(|c| c.value_span.as_ref().map(|s| s.start) == Some(offset)),
        None => candidates.iter().find(|c| c.condition.as_deref().is_none_or(condition_met)).copied().or(candidates.first().copied()),
    }
    .ok_or("Property not found")?;
    Ok(remove_span(source, node_full_end(source, node), node.span.start))
}

/// End of the node including a trailing `[$CONDITION]` token.
fn node_full_end(source: &str, node: &KvNode) -> usize {
    let mut end = node.span.end;
    if node.condition.is_some() {
        if let Some(close) = source[end..].find(']') {
            end += close + 1;
        }
    }
    end
}

/// Deletes `[start, end)`; if the range is the only content on its line(s) the full lines go too,
/// and a trailing line comment on the same line is kept on its own line only if there is other content.
fn remove_span(source: &str, end: usize, start: usize) -> String {
    let mut out = source.to_owned();
    let line_start = source[..start].rfind('\n').map(|i| i + 1).unwrap_or(0);
    let before_blank = source[line_start..start].chars().all(|c| c == ' ' || c == '\t');
    let line_end = source[end..].find('\n').map(|i| end + i + 1).unwrap_or(source.len());
    let after = &source[end..line_end];
    let after_blank = after.trim().is_empty();
    if before_blank && after_blank {
        out.replace_range(line_start..line_end, "");
    } else {
        // trim one separating whitespace run so `"a" "1" "b" "2"` -> `"a" "1"`
        let mut s = start;
        while s > line_start && matches!(source.as_bytes()[s - 1], b' ' | b'\t') {
            s -= 1
        }
        out.replace_range(s..end, "");
    }
    out
}

pub fn find_node<'a>(nodes: &'a [KvNode], id: &str) -> Option<&'a KvNode> {
    for n in nodes.iter().filter(|n| n.is_block()) {
        if n.id == id {
            return Some(n);
        }
        if id.starts_with(&format!("{}/", n.id)) {
            if let Some(found) = find_node(&n.children, id) {
                return Some(found);
            }
        }
    }
    None
}

/// Inserts a new child block under `parent_id` with the given properties; returns the new source and the new block id.
pub fn insert_block(source: &str, parent_id: &str, name: &str, properties: &[(String, String)]) -> Result<(String, String), String> {
    valid_key(name)?;
    for (k, _) in properties {
        valid_key(k)?
    }
    let doc = parse(source);
    let parent = doc.panels.iter().find(|p| p.id == parent_id).ok_or("Parent block not found; refresh document")?;
    let unit = indent_unit(source);
    let mut entry = format!("\"{name}\"\n{{");
    for (k, v) in properties {
        entry.push_str(&format!("\n{unit}\"{k}\"{}{}", align_gap(source, k), quoted(v)?));
    }
    entry.push_str("\n}");
    let out = insert_entry(source, parent, &entry)?;
    // the new block is the last block named `name` under the parent
    let new_id = parse(&out)
        .panels
        .iter()
        .rfind(|p| p.name.eq_ignore_ascii_case(name) && p.id.rsplit_once('/').map(|(head, _)| head) == Some(parent_id))
        .map(|p| p.id.clone())
        .ok_or("Inserted block could not be located")?;
    Ok((out, new_id))
}

/// Ensures a chain of nested blocks exists below `parent_id` (e.g. `["HudPlayerHealth", "PlayerStatusHealthValue"]`),
/// creating missing ones empty. Returns the new source and the id of the deepest block.
pub fn ensure_blocks(source: &str, parent_id: &str, path: &[String]) -> Result<(String, String), String> {
    let mut src = source.to_owned();
    let mut current = parent_id.to_owned();
    for segment in path {
        let t = tree(&src);
        let parent = find_node(&t.roots, &current).ok_or("Parent block not found")?;
        if let Some(existing) = parent.children.iter().find(|c| c.is_block() && c.key.eq_ignore_ascii_case(segment)) {
            current = existing.id.clone();
        } else {
            let (next, id) = insert_block(&src, &current, segment, &[])?;
            src = next;
            current = id;
        }
    }
    Ok((src, current))
}

/// Removes a block (and its condition suffix) entirely.
pub fn remove_block(source: &str, id: &str) -> Result<String, String> {
    let t = tree(source);
    let node = find_node(&t.roots, id).ok_or("Block not found; refresh document")?;
    Ok(remove_span(source, node_full_end(source, node), node.span.start))
}

/// Copies a block directly after itself under a new key; a `fieldName` inside is renamed too.
pub fn duplicate_block(source: &str, id: &str, new_name: &str) -> Result<(String, String), String> {
    valid_key(new_name)?;
    let t = tree(source);
    let node = find_node(&t.roots, id).ok_or("Block not found; refresh document")?;
    let end = node_full_end(source, node);
    let original = &source[node.span.start..end];
    // replace the key token (first token of the block text)
    let key_end = if let Some(rest) = original.strip_prefix('"') {
        rest.find('"').map(|i| i + 2).unwrap_or(original.len())
    } else {
        original.find(|c: char| c.is_whitespace() || c == '{').unwrap_or(original.len())
    };
    let copy = format!("\"{new_name}\"{}", &original[key_end..]);
    let newline = newline_of(source);
    let indent = line_indent(source, node.span.start).map(|(_, i)| i.to_string()).unwrap_or_default();
    let mut out = source.to_owned();
    out.insert_str(end, &format!("{newline}{indent}{copy}"));
    let parent = id.rsplit_once('/').map(|(h, _)| h.to_string()).unwrap_or_default();
    let new_id = parse(&out)
        .panels
        .iter()
        .rfind(|p| p.name.eq_ignore_ascii_case(new_name) && p.id.rsplit_once('/').map(|(h, _)| h.to_string()).unwrap_or_default() == parent)
        .map(|p| p.id.clone())
        .ok_or("Duplicated block could not be located")?;
    let has_field_name = find_node(&tree(&out).roots, &new_id).is_some_and(|n| n.children.iter().any(|c| c.key.eq_ignore_ascii_case("fieldName")));
    if has_field_name {
        out = set_property(&out, &new_id, "fieldName", new_name, None)?;
    }
    Ok((out, new_id))
}

/// Renames a block key (and its fieldName when they matched).
pub fn rename_block(source: &str, id: &str, new_name: &str) -> Result<(String, String), String> {
    valid_key(new_name)?;
    let t = tree(source);
    let node = find_node(&t.roots, id).ok_or("Block not found; refresh document")?;
    let (tokens, _) = lex(source);
    let key_token = tokens.iter().find(|tk| tk.span.start == node.span.start).ok_or("Key token missing")?;
    let field_matches = node.get("fieldName").is_some_and(|f| f.eq_ignore_ascii_case(&node.key));
    let mut out = source.to_owned();
    out.replace_range(key_token.span.clone().start..key_token.span.end, &quoted(new_name)?);
    let parent = id.rsplit_once('/').map(|(h, _)| h.to_string());
    let new_id = parse(&out)
        .panels
        .iter()
        .filter(|p| p.name == new_name && p.id.rsplit_once('/').map(|(h, _)| h.to_string()) == parent)
        .find(|p| p.span.start == node.span.start)
        .map(|p| p.id.clone())
        .ok_or("Renamed block could not be located")?;
    if field_matches {
        out = set_property(&out, &new_id, "fieldName", new_name, None)?;
    }
    Ok((out, new_id))
}

pub fn replace_property(source: &str, panel_id: &str, property: &str, value: &str, at: Option<usize>) -> Result<String, String> {
    set_property(source, panel_id, property, value, at)
}

/// Text encodings found in TF2 resource files. Localization files are commonly UTF-16 LE with a BOM.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Encoding {
    Utf8,
    Utf16Le,
    Utf16Be,
}

/// Decodes bytes into text, reporting the encoding so a save can write the same bytes back.
/// UTF-8 BOMs stay in the text (the lexer treats them as whitespace) so round trips stay exact.
pub fn decode(bytes: &[u8]) -> Result<(String, Encoding), String> {
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        let units: Vec<u16> = bytes[2..].as_chunks::<2>().0.iter().map(|&c| u16::from_le_bytes(c)).collect();
        return String::from_utf16(&units).map(|s| (s, Encoding::Utf16Le)).map_err(|_| "Invalid UTF-16 text".into());
    }
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        let units: Vec<u16> = bytes[2..].as_chunks::<2>().0.iter().map(|&c| u16::from_be_bytes(c)).collect();
        return String::from_utf16(&units).map(|s| (s, Encoding::Utf16Be)).map_err(|_| "Invalid UTF-16 text".into());
    }
    match std::str::from_utf8(bytes) {
        Ok(s) => Ok((s.to_string(), Encoding::Utf8)),
        // Many old HUD files are Windows-1252; map bytes 1:1 (Latin-1) so they stay viewable. Saving re-encodes as UTF-8.
        Err(_) => Err("File is neither UTF-8 nor UTF-16 text".into()),
    }
}

pub fn encode(text: &str, encoding: Encoding) -> Vec<u8> {
    match encoding {
        Encoding::Utf8 => text.as_bytes().to_vec(),
        Encoding::Utf16Le => {
            let mut out = vec![0xFF, 0xFE];
            for u in text.encode_utf16() {
                out.extend_from_slice(&u.to_le_bytes())
            }
            out
        }
        Encoding::Utf16Be => {
            let mut out = vec![0xFE, 0xFF];
            for u in text.encode_utf16() {
                out.extend_from_slice(&u.to_be_bytes())
            }
            out
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const SAMPLE: &str = "// header\r\n\"HudLayout\"\r\n{\r\n  \"HudHealth\" { \"xpos\" \"r95\" // preserve me\r\n    \"ypos\" \"c-30\" \"wide\" \"100\" \"tall\" \"50\" }\r\n}\r\n";

    fn rebuilt(s: &str) -> String {
        parse(s).tokens.iter().map(|t| &s[t.span.start..t.span.end]).collect()
    }

    #[test]
    fn token_stream_is_lossless() {
        let d = parse(SAMPLE);
        assert_eq!(rebuilt(SAMPLE), SAMPLE);
        assert!(d.diagnostics.is_empty())
    }
    #[test]
    fn extracts_hierarchy() {
        let p = panels(SAMPLE);
        let h = p.iter().find(|p| p.name == "HudHealth").unwrap();
        assert_eq!(h.id, "HudLayout/HudHealth");
        assert_eq!(h.properties.len(), 4)
    }
    #[test]
    fn surgical_edit() {
        let out = set_property(SAMPLE, "HudLayout/HudHealth", "xpos", "r80", None).unwrap();
        assert_eq!(out, SAMPLE.replacen("\"r95\"", "\"r80\"", 1))
    }
    #[test]
    fn adds_property() {
        let s = "\"root\"\n{\n  \"Panel\"\n  {\n    \"xpos\" \"10\"\n  }\n}\n";
        let out = set_property(s, "root/Panel", "visible", "1", None).unwrap();
        assert!(out.contains("    \"visible\" \"1\"\n  }"), "{out}")
    }
    #[test]
    fn stable_ids() {
        let a = panels("\"r\"{\"P\"{\"xpos\"\"1\"}}");
        let b = panels("//x\n\"r\" { \"P\" { \"xpos\" \"999\" } }");
        assert_eq!(a[1].id, b[1].id)
    }
    #[test]
    fn duplicate_keys_edit_the_first_like_findkey() {
        let s = "\"root\"{\"Hud\"{\"xpos\" \"10\" \"xpos\" \"20\"}}";
        let out = set_property(s, "root/Hud", "xpos", "30", None).unwrap();
        assert!(out.contains("\"30\" \"xpos\" \"20\""))
    }
    #[test]
    fn recovers_unclosed() {
        assert!(!parse("\"root\" { \"x\" \"1\"").diagnostics.is_empty())
    }
    #[test]
    fn parses_condition_suffix() {
        let p = panels("\"root\"{\"Hud\"{\"xpos\" \"10\" [$WIN32]}}");
        assert_eq!(p[1].properties[0].condition.as_deref(), Some("$WIN32"))
    }
    #[test]
    fn x360_never_matches_this_desktop_build() {
        assert!(!condition_met("$X360"));
        assert!(condition_met("!$X360"))
    }
    #[test]
    fn compound_conditions() {
        assert!(condition_met("!$X360 && !$PS3"));
        assert!(!condition_met("$X360 || $PS3"));
    }
    #[test]
    fn unknown_condition_defaults_to_visible() {
        assert!(condition_met("$SOMENEWFLAG"))
    }
    #[test]
    fn condition_aware_edit_targets_the_active_platform_occurrence() {
        let s = "\"root\"{\"Hud\"{\"xpos\" \"r635\" [$X360] \"xpos\" \"r672\" [$WIN32]}}";
        let out = set_property(s, "root/Hud", "xpos", "r900", None).unwrap();
        assert!(out.contains("\"r900\" [$WIN32]"));
        assert!(out.contains("\"r635\" [$X360]"));
    }
    #[test]
    fn at_offset_targets_one_exact_occurrence() {
        let s = "\"root\"{\"Hud\"{\"xpos\" \"r635\" [$WIN32] \"xpos\" \"r672\" [$X360]}}";
        let x360 = panels(s)[1].properties.iter().find(|p| p.condition.as_deref() == Some("$X360")).unwrap().clone();
        let out = set_property(s, "root/Hud", "xpos", "r999", Some(x360.span.start)).unwrap();
        assert!(out.contains("\"r999\" [$X360]"));
        assert!(out.contains("\"r635\" [$WIN32]"));
    }
    #[test]
    fn synthetic_fixture_round_trips_and_edits() {
        let s = include_str!("../../../fixtures/synthetic/hudlayout.res");
        let d = parse(s);
        assert!(d.diagnostics.is_empty());
        assert_eq!(rebuilt(s), s);
        let out = set_property(s, "HudLayout/HudPlayerHealth", "xpos", "r200", None).unwrap();
        assert!(out.contains("\"r200\" // Keep this comment intact"))
    }
    #[test]
    fn bom_is_whitespace() {
        let s = "\u{feff}\"root\"\n{\n\t\"a\"\t\"1\"\n}\n";
        let d = parse(s);
        assert!(d.diagnostics.is_empty());
        assert_eq!(d.panels[0].name, "root");
        assert_eq!(rebuilt(s), s);
    }
    #[test]
    fn directives_are_collected() {
        let t = tree("#base \"scheme/colors.res\"\n#base \"other.res\" [$WIN32]\n\"Scheme\" { }");
        assert_eq!(t.directives.len(), 2);
        assert_eq!(t.directives[0].path, "scheme/colors.res");
        assert_eq!(t.directives[1].condition.as_deref(), Some("$WIN32"));
        assert_eq!(t.roots.last().unwrap().key, "Scheme");
    }
    #[test]
    fn condition_before_block_brace() {
        let s = "\"r\"\n{\n\t\"Base\"\t[$WIN32]\n\t{\n\t\t\"xpos\" \"1\"\n\t}\n}\n";
        let t = tree(s);
        assert!(t.diagnostics.is_empty(), "{:?}", t.diagnostics);
        let b = &t.roots[0].children[0];
        assert_eq!(b.condition.as_deref(), Some("$WIN32"));
        assert_eq!(b.get("xpos"), Some("1"));
    }
    #[test]
    fn escaped_quotes_unescape() {
        let t = tree("\"r\" { \"labelText\" \"say \\\"hi\\\"\" }");
        assert_eq!(t.roots[0].get("labelText"), Some("say \"hi\""));
    }
    #[test]
    fn insert_remove_duplicate_blocks() {
        let s = "\"root\"\n{\n\t\"A\"\n\t{\n\t\t\"fieldName\"\t\"A\"\n\t\t\"xpos\"\t\"1\"\n\t}\n}\n";
        let (out, id) = insert_block(s, "root", "B", &[("ControlName".into(), "CExLabel".into()), ("xpos".into(), "5".into())]).unwrap();
        assert_eq!(id, "root/B");
        assert_eq!(rebuilt(&out), out);
        assert!(parse(&out).diagnostics.is_empty());
        assert!(out.contains("\t\"B\"\n\t{\n\t\t\"ControlName\""), "{out}");
        let (dup, did) = duplicate_block(&out, "root/A", "A2").unwrap();
        assert_eq!(did, "root/A2");
        let a2 = find_node(&tree(&dup).roots, "root/A2").unwrap().clone();
        assert_eq!(a2.get("fieldName"), Some("A2"));
        assert_eq!(a2.get("xpos"), Some("1"));
        let removed = remove_block(&dup, "root/A").unwrap();
        assert!(find_node(&tree(&removed).roots, "root/A").is_none());
        assert!(parse(&removed).diagnostics.is_empty());
        assert!(!removed.contains("\n\n\n"));
    }
    #[test]
    fn ensure_blocks_creates_chain() {
        let s = "\"root\"\n{\n}\n";
        let (out, id) = ensure_blocks(s, "root", &["A".into(), "B".into()]).unwrap();
        assert_eq!(id, "root/A/B");
        let (again, id2) = ensure_blocks(&out, "root", &["a".into(), "B".into()]).unwrap();
        assert_eq!(again, out);
        assert_eq!(id2, "root/A/B");
    }
    #[test]
    fn remove_property_line() {
        let s = "\"r\"\n{\n\t\"P\"\n\t{\n\t\t\"xpos\"\t\"1\"\t[$WIN32]\n\t\t\"ypos\"\t\"2\"\n\t}\n}\n";
        let out = remove_property(s, "r/P", "xpos", None).unwrap();
        assert_eq!(out, s.replace("\t\t\"xpos\"\t\"1\"\t[$WIN32]\n", ""));
        let inline = "\"r\" { \"P\" { \"a\" \"1\" \"b\" \"2\" } }";
        assert_eq!(remove_property(inline, "r/P", "b", None).unwrap(), "\"r\" { \"P\" { \"a\" \"1\" } }");
    }
    #[test]
    fn rename_updates_field_name() {
        let s = "\"r\" { \"P\" { \"fieldName\" \"P\" } }";
        let (out, id) = rename_block(s, "r/P", "Q").unwrap();
        assert_eq!(id, "r/Q");
        assert_eq!(out, "\"r\" { \"Q\" { \"fieldName\" \"Q\" } }");
    }
    #[test]
    fn utf16_round_trip() {
        let mut bytes = vec![0xFF, 0xFE];
        for u in "\"lang\" { }".encode_utf16() {
            bytes.extend_from_slice(&u.to_le_bytes())
        }
        let (text, enc) = decode(&bytes).unwrap();
        assert_eq!(enc, Encoding::Utf16Le);
        assert_eq!(encode(&text, enc), bytes);
    }
}
