//! `hudanimations_*.txt` parser (the AnimationController script language) and manifest loader.
use crate::resolve::resolve;
use crate::vfs::Vfs;
use hud_syntax::condition_met;
use serde::Serialize;

#[derive(Clone, Debug, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AnimCommand {
    /// Animate, RunEvent, StopEvent, StopAnimation, StopPanelAnimations, SetVisible, SetFont, SetTexture,
    /// SetString, SetInputEnabled, RunEventChild, FireCommand, PlaySound
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub panel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub property: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interpolator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub param: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    pub start: f32,
    pub duration: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    pub line: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimEvent {
    pub name: String,
    pub file: String,
    pub editable: bool,
    pub line: usize,
    /// Byte range of the whole `event Name { ... }` text in `file`.
    pub start: usize,
    pub end: usize,
    pub commands: Vec<AnimCommand>,
    /// Set when an earlier manifest file already defined this event (the engine keeps the first one).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shadowed_by: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnimProblem {
    pub file: String,
    pub line: usize,
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Animations {
    pub files: Vec<String>,
    pub events: Vec<AnimEvent>,
    pub problems: Vec<AnimProblem>,
}

#[derive(Debug, Clone)]
struct Tok {
    text: String,
    line: usize,
    start: usize,
    end: usize,
    quoted: bool,
}

fn tokenize(src: &str) -> Vec<Tok> {
    let b = src.as_bytes();
    let mut i = 0;
    let mut line = 1;
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i];
        if c == b'\n' {
            line += 1;
            i += 1;
        } else if c.is_ascii_whitespace() || (i == 0 && src.starts_with('\u{feff}')) {
            i += if c.is_ascii_whitespace() { 1 } else { 3 };
        } else if c == b'/' && b.get(i + 1) == Some(&b'/') {
            while i < b.len() && b[i] != b'\n' {
                i += 1
            }
        } else if c == b'/' && b.get(i + 1) == Some(&b'*') {
            i += 2;
            while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                if b[i] == b'\n' {
                    line += 1
                }
                i += 1
            }
            i = (i + 2).min(b.len());
        } else if c == b'{' || c == b'}' {
            out.push(Tok { text: (c as char).to_string(), line, start: i, end: i + 1, quoted: false });
            i += 1;
        } else if c == b'"' {
            let start = i;
            i += 1;
            let s = i;
            while i < b.len() && b[i] != b'"' && b[i] != b'\n' {
                i += 1
            }
            out.push(Tok { text: src[s..i].to_string(), line, start, end: (i + 1).min(b.len()), quoted: true });
            i = (i + 1).min(b.len());
        } else {
            let s = i;
            while i < b.len() && !b[i].is_ascii_whitespace() && b[i] != b'{' && b[i] != b'}' && b[i] != b'"' {
                if b[i] == b'/' && b.get(i + 1) == Some(&b'/') {
                    break;
                }
                i += 1
            }
            while i < b.len() && !src.is_char_boundary(i) {
                i += 1
            }
            out.push(Tok { text: src[s..i].to_string(), line, start: s, end: i, quoted: false });
        }
    }
    out
}

fn arg_count(kind: &str) -> Option<usize> {
    Some(match kind {
        "animate" => 6, // panel prop value interp start duration (+1 for parametric interpolators)
        "runevent" | "stopevent" => 2,
        "stopanimation" => 3,
        "stoppanelanimations" => 2,
        "setfont" | "settexture" | "setstring" => 4,
        "setvisible" | "setinputenabled" => 3,
        "runeventchild" => 3,
        "firecommand" | "playsound" => 2,
        _ => return None,
    })
}

fn canonical(kind: &str) -> &'static str {
    match kind {
        "animate" => "Animate",
        "runevent" => "RunEvent",
        "stopevent" => "StopEvent",
        "stopanimation" => "StopAnimation",
        "stoppanelanimations" => "StopPanelAnimations",
        "setfont" => "SetFont",
        "settexture" => "SetTexture",
        "setstring" => "SetString",
        "setvisible" => "SetVisible",
        "setinputenabled" => "SetInputEnabled",
        "runeventchild" => "RunEventChild",
        "firecommand" => "FireCommand",
        _ => "PlaySound",
    }
}

fn num(s: &str) -> f32 {
    s.trim().parse().unwrap_or(0.0)
}

pub fn parse(src: &str, file: &str, editable: bool) -> (Vec<AnimEvent>, Vec<AnimProblem>) {
    let toks = tokenize(src);
    let mut events = Vec::new();
    let mut problems = Vec::new();
    let mut i = 0;
    let problem = |problems: &mut Vec<AnimProblem>, line, message: String| problems.push(AnimProblem { file: file.into(), line, message });
    while i < toks.len() {
        if !toks[i].text.eq_ignore_ascii_case("event") {
            problem(&mut problems, toks[i].line, format!("Expected 'event', found '{}'", toks[i].text));
            i += 1;
            continue;
        }
        let ev_start = toks[i].start;
        let ev_line = toks[i].line;
        let Some(name_tok) = toks.get(i + 1) else { break };
        let name = name_tok.text.clone();
        i += 2;
        // optional condition on the event itself
        let mut ev_enabled = true;
        if let Some(t) = toks.get(i) {
            if t.text.starts_with('[') && t.text.ends_with(']') {
                ev_enabled = condition_met(&t.text[1..t.text.len() - 1]);
                i += 1;
            }
        }
        if toks.get(i).map(|t| t.text.as_str()) != Some("{") {
            problem(&mut problems, ev_line, format!("Event '{name}' is missing '{{'"));
            continue;
        }
        i += 1;
        let mut commands = Vec::new();
        let mut end = src.len();
        while i < toks.len() {
            let t = &toks[i];
            if t.text == "}" && !t.quoted {
                end = t.end;
                i += 1;
                break;
            }
            let kind = t.text.to_ascii_lowercase();
            let Some(n) = arg_count(&kind) else {
                problem(&mut problems, t.line, format!("Unknown command '{}' in event '{name}'", t.text));
                let line = t.line;
                while i < toks.len() && toks[i].line == line && toks[i].text != "}" {
                    i += 1
                }
                continue;
            };
            let line = t.line;
            let mut args: Vec<String> = Vec::new();
            let mut j = i + 1;
            let mut needed = n;
            while args.len() < needed && j < toks.len() && !(toks[j].text == "}" && !toks[j].quoted) {
                args.push(toks[j].text.clone());
                if kind == "animate" && args.len() == 4 {
                    let interp = args[3].to_ascii_lowercase();
                    if matches!(interp.as_str(), "pulse" | "flicker" | "gain" | "bias") {
                        needed = 7;
                    }
                }
                j += 1;
            }
            if args.len() < needed {
                problem(&mut problems, line, format!("'{}' needs {} arguments", canonical(&kind), needed));
                i = j;
                continue;
            }
            let mut condition = None;
            if let Some(c) = toks.get(j) {
                if c.text.starts_with('[') && c.text.ends_with(']') && c.line == line {
                    condition = Some(c.text[1..c.text.len() - 1].to_string());
                    j += 1;
                }
            }
            i = j;
            let mut cmd = AnimCommand {
                kind: canonical(&kind).into(),
                panel: None,
                property: None,
                value: None,
                interpolator: None,
                param: None,
                event: None,
                start: 0.0,
                duration: 0.0,
                condition,
                line,
            };
            match kind.as_str() {
                "animate" => {
                    cmd.panel = Some(args[0].clone());
                    cmd.property = Some(args[1].clone());
                    cmd.value = Some(args[2].clone());
                    cmd.interpolator = Some(args[3].clone());
                    let rest = if needed == 7 {
                        cmd.param = Some(num(&args[4]));
                        &args[5..]
                    } else {
                        &args[4..]
                    };
                    cmd.start = num(&rest[0]);
                    cmd.duration = num(&rest[1]);
                }
                "runevent" | "stopevent" => {
                    cmd.event = Some(args[0].clone());
                    cmd.start = num(&args[1]);
                }
                "stopanimation" => {
                    cmd.panel = Some(args[0].clone());
                    cmd.property = Some(args[1].clone());
                    cmd.start = num(&args[2]);
                }
                "stoppanelanimations" => {
                    cmd.panel = Some(args[0].clone());
                    cmd.start = num(&args[1]);
                }
                "setfont" | "settexture" | "setstring" => {
                    cmd.panel = Some(args[0].clone());
                    cmd.property = Some(args[1].clone());
                    cmd.value = Some(args[2].clone());
                    cmd.start = num(&args[3]);
                }
                "setvisible" | "setinputenabled" => {
                    cmd.panel = Some(args[0].clone());
                    cmd.value = Some(args[1].clone());
                    cmd.start = num(&args[2]);
                }
                "runeventchild" => {
                    cmd.panel = Some(args[0].clone());
                    cmd.event = Some(args[1].clone());
                    cmd.start = num(&args[2]);
                }
                _ => {
                    cmd.start = num(&args[0]);
                    cmd.value = Some(args[1].clone());
                }
            }
            if cmd.condition.as_deref().is_none_or(condition_met) {
                commands.push(cmd);
            }
        }
        if ev_enabled {
            events.push(AnimEvent { name, file: file.into(), editable, line: ev_line, start: ev_start, end, commands, shadowed_by: None });
        }
    }
    (events, problems)
}

/// Loads every file listed in scripts/hudanimations_manifest.txt (project first, stock as fallback).
pub fn load(vfs: &Vfs) -> Animations {
    let mut files = Vec::new();
    if let Some(manifest) = resolve(vfs, "scripts/hudanimations_manifest.txt") {
        for root in manifest.roots.iter().filter(|r| r.is_block()) {
            for c in root.children.iter().filter(|c| c.key.eq_ignore_ascii_case("file")) {
                if let Some(v) = &c.value {
                    files.push(v.clone())
                }
            }
        }
    }
    if files.is_empty() {
        files = vec!["scripts/hudanimations.txt".into(), "scripts/hudanimations_tf.txt".into()];
    }
    let mut events: Vec<AnimEvent> = Vec::new();
    let mut problems = Vec::new();
    let mut loaded = Vec::new();
    for f in &files {
        let Some((text, origin)) = vfs.read_text(f) else {
            // stock files only resolve once the TF2 folder is known; don't report them as missing before that
            let stock = matches!(f.to_ascii_lowercase().replace('\\', "/").as_str(), "scripts/hudanimations.txt" | "scripts/hudanimations_tf.txt");
            if stock && vfs.game.is_none() {
                continue;
            }
            problems.push(AnimProblem { file: "scripts/hudanimations_manifest.txt".into(), line: 0, message: format!("Animation file '{f}' not found") });
            continue;
        };
        let label = origin.label();
        let (evs, probs) = parse(&text, &label, origin.project_path().is_some());
        problems.extend(probs);
        for mut e in evs {
            if let Some(first) = events.iter().find(|x| x.name.eq_ignore_ascii_case(&e.name)) {
                e.shadowed_by = Some(first.file.clone());
            }
            events.push(e);
        }
        loaded.push(label);
    }
    Animations { files: loaded, events, problems }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_commands() {
        let src = "// c\nevent HudHealthDyingPulse\n{\n\tAnimate\tPlayerStatusHealthBonusImage \tAlpha\t\t\"255\"\t\tLinear 0.0 0.075\n\tAnimate X FgColor \"Red\" Pulse 3 0.1 1.5\n\tRunEvent HudHealthDyingPulseLoop\t0.3\n\tSetVisible Foo 0 0.0 [$X360]\n\tFireCommand 0.5 \"playsound ui/x.wav\"\n}\nevent Other { StopEvent HudHealthDyingPulse 0.0 }\n";
        let (evs, probs) = parse(src, "f", true);
        assert!(probs.is_empty(), "{probs:?}");
        assert_eq!(evs.len(), 2);
        let c = &evs[0].commands;
        assert_eq!(c.len(), 4, "X360 line filtered");
        assert_eq!(c[0].panel.as_deref(), Some("PlayerStatusHealthBonusImage"));
        assert_eq!(c[0].duration, 0.075);
        assert_eq!(c[1].param, Some(3.0));
        assert_eq!(c[1].start, 0.1);
        assert_eq!(c[2].event.as_deref(), Some("HudHealthDyingPulseLoop"));
        assert_eq!(c[3].value.as_deref(), Some("playsound ui/x.wav"));
        assert_eq!(&src[evs[1].start..evs[1].end], "event Other { StopEvent HudHealthDyingPulse 0.0 }");
    }
}
