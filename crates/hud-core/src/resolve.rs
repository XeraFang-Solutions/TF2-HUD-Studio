//! `#base` / `#include` resolution with per-value provenance, following KeyValues::MergeBaseKeys:
//! the including file always keeps its own values; keys only present in a base file are appended.
use crate::vfs::{normalize, relative_to, Origin, Vfs};
use hud_syntax::{condition_met, tree, KvNode};
use serde::Serialize;
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RNode {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<RNode>,
    /// File the entry came from (project path, `tf/...` or `stock:...`).
    pub file: String,
    pub editable: bool,
    /// Block id inside `file` (for blocks: the block itself; for values: the containing block).
    pub id: String,
    /// Byte offset of the value token inside `file` (values only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
}

impl RNode {
    pub fn is_block(&self) -> bool {
        self.value.is_none()
    }
    pub fn get(&self, key: &str) -> Option<&str> {
        self.children.iter().find(|c| c.key.eq_ignore_ascii_case(key) && c.value.is_some()).and_then(|c| c.value.as_deref())
    }
    pub fn child(&self, key: &str) -> Option<&RNode> {
        self.children.iter().find(|c| c.key.eq_ignore_ascii_case(key) && c.is_block())
    }
    pub fn blocks(&self) -> impl Iterator<Item = &RNode> {
        self.children.iter().filter(|c| c.is_block())
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MissingBase {
    pub from: String,
    pub path: String,
    pub offset: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Resolved {
    pub path: String,
    pub origin: Origin,
    pub roots: Vec<RNode>,
    /// Every file that contributed, including the file itself, in load order.
    pub chain: Vec<String>,
    pub missing: Vec<MissingBase>,
}

impl Resolved {
    pub fn root(&self) -> Option<&RNode> {
        self.roots.iter().find(|r| r.is_block())
    }
}

fn convert(node: &KvNode, file: &str, editable: bool, parent_id: &str) -> Option<RNode> {
    if !node.condition.as_deref().is_none_or(condition_met) {
        return None;
    }
    Some(if node.is_block() {
        RNode {
            key: node.key.clone(),
            value: None,
            children: node.children.iter().filter_map(|c| convert(c, file, editable, &node.id)).collect(),
            file: file.into(),
            editable,
            id: node.id.clone(),
            offset: None,
        }
    } else {
        RNode {
            key: node.key.clone(),
            value: node.value.clone(),
            children: Vec::new(),
            file: file.into(),
            editable,
            id: parent_id.into(),
            offset: node.value_span.as_ref().map(|s| s.start),
        }
    })
}

pub fn merge(target: &mut RNode, base: &RNode) {
    for bc in &base.children {
        match target.children.iter_mut().find(|c| c.key.eq_ignore_ascii_case(&bc.key)) {
            Some(existing) => {
                if existing.is_block() && bc.is_block() {
                    merge(existing, bc)
                }
            }
            None => target.children.push(bc.clone()),
        }
    }
}

pub fn resolve(vfs: &Vfs, rel: &str) -> Option<Resolved> {
    let mut seen = HashSet::new();
    let mut chain = Vec::new();
    let mut missing = Vec::new();
    let (roots, origin) = load(vfs, rel, &mut seen, &mut chain, &mut missing, 0)?;
    Some(Resolved { path: normalize(rel), origin, roots, chain, missing })
}

fn load(
    vfs: &Vfs,
    rel: &str,
    seen: &mut HashSet<String>,
    chain: &mut Vec<String>,
    missing: &mut Vec<MissingBase>,
    depth: usize,
) -> Option<(Vec<RNode>, Origin)> {
    let norm = normalize(rel);
    if depth > 16 || !seen.insert(norm.clone()) {
        return None;
    }
    let (text, origin) = vfs.read_text(&norm)?;
    let label = origin.label();
    let editable = matches!(origin, Origin::Project(_));
    chain.push(label.clone());
    let t = tree(&text);
    let mut roots: Vec<RNode> = t.roots.iter().filter(|r| !r.key.starts_with('#')).filter_map(|r| convert(r, &label, editable, "")).collect();
    for d in &t.directives {
        if !d.condition.as_deref().is_none_or(condition_met) {
            continue;
        }
        let target = relative_to(&norm, &d.path);
        match load(vfs, &target, seen, chain, missing, depth + 1) {
            Some((base_roots, _)) => {
                if d.kind == "#include" {
                    roots.extend(base_roots);
                    continue;
                }
                match roots.iter_mut().find(|r| r.is_block()) {
                    Some(root) => {
                        for b in base_roots.iter().filter(|b| b.is_block()) {
                            merge(root, b)
                        }
                    }
                    None => roots = base_roots,
                }
            }
            None => {
                if !vfs.exists(&target) {
                    missing.push(MissingBase { from: label.clone(), path: d.path.clone(), offset: d.span.start })
                }
            }
        }
    }
    Some((roots, origin))
}

/// `.res` files a hudlayout element loads (TF2 hard-codes these in the client; most follow `resource/ui/<name>.res`).
pub fn element_files(vfs: &Vfs, element: &str) -> Vec<String> {
    let e = element.to_ascii_lowercase();
    let known: &[&str] = match e.as_str() {
        "hudplayerstatus" => &["resource/ui/hudplayerclass.res", "resource/ui/hudplayerhealth.res"],
        "hudweaponammo" => &["resource/ui/hudammoweapons.res"],
        "hudchat" => &["resource/ui/basechat.res"],
        "hudobjectivestatus" => &["resource/ui/hudobjectivetimepanel.res", "resource/ui/hudobjectiveflagpanel.res"],
        "hudkothtimestatus" => &["resource/ui/hudobjectivekothtimepanel.res"],
        "chudaccountpanel" => &["resource/ui/hudaccountpanel.res"],
        "chealthaccountpanel" => &["resource/ui/hudhealthaccount.res"],
        "cdamageaccountpanel" => &["resource/ui/huddamageaccount.res"],
        "cmainttargetid" | "cmaintargetid" | "cspectatortargetid" | "csecondarytargetid" | "targetid" => &["resource/ui/targetid.res"],
        "hudmenuengybuild" => &["resource/ui/build_menu/hudmenuengybuild.res"],
        "hudmenuengydestroy" => &["resource/ui/destroy_menu/hudmenuengydestroy.res"],
        "hudmenuspydisguise" => &["resource/ui/disguise_menu/hudmenuspydisguise.res"],
        "hudmenueurekaeffect" => &["resource/ui/build_menu/hudmenueurekaeffect.res"],
        "freezepanel" => &["resource/ui/freezepanel_basic.res"],
        "hudspellmenu" => &["resource/ui/hudspellselection.res"],
        "chuditemattributetracker" | "itemattributetracker" => &["resource/ui/huditemattributetracker.res"],
        "disguisestatus" | "disguisestatuspanel" => &["resource/ui/disguisestatuspanel.res"],
        "chudkillstreaknotice" | "killstreaknotice" => &["resource/ui/hudkillstreaknotice.res"],
        _ => &[],
    };
    let mut out: Vec<String> = known.iter().filter(|p| vfs.exists(p)).map(|p| p.to_string()).collect();
    if out.is_empty() {
        let mut candidates = vec![format!("resource/ui/{e}.res")];
        if let Some(stripped) = e.strip_prefix('c') {
            candidates.push(format!("resource/ui/{stripped}.res"));
        }
        if let Some(c) = candidates.into_iter().find(|c| vfs.exists(c)) {
            out.push(c)
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vfs::index_project;
    use std::path::Path;

    #[test]
    fn base_merge_keeps_own_values_and_appends_base_keys() {
        let files = vec!["resource/ui/a.res".to_string(), "resource/ui/base/b.res".to_string()];
        let index = index_project(&files);
        let overlay = |p: &str| -> Option<String> {
            match p {
                "resource/ui/a.res" => Some("#base \"base/b.res\"\n\"resource/ui/a.res\"\n{\n\t\"P\"\n\t{\n\t\t\"xpos\"\t\"5\"\n\t}\n}\n".into()),
                "resource/ui/base/b.res" => {
                    Some("\"other\"\n{\n\t\"P\"\n\t{\n\t\t\"xpos\"\t\"1\"\n\t\t\"ypos\"\t\"2\"\n\t}\n\t\"Q\" { \"wide\" \"9\" }\n}\n".into())
                }
                _ => None,
            }
        };
        let vfs = Vfs { root: Path::new("/nonexistent"), index: &index, overlay: &overlay, game: None };
        let r = resolve(&vfs, "Resource/UI/A.res").unwrap();
        let root = r.root().unwrap();
        let p = root.child("P").unwrap();
        assert_eq!(p.get("xpos"), Some("5"));
        assert_eq!(p.get("ypos"), Some("2"));
        assert_eq!(p.children.iter().find(|c| c.key == "ypos").unwrap().file, "resource/ui/base/b.res");
        assert_eq!(root.child("Q").unwrap().get("wide"), Some("9"));
        assert_eq!(r.chain.len(), 2);
    }

    #[test]
    fn reports_missing_base() {
        let files = vec!["a.res".to_string()];
        let index = index_project(&files);
        let overlay = |_: &str| Some("#base \"nope.res\"\n\"a\" { }".to_string());
        let vfs = Vfs { root: Path::new("/nonexistent"), index: &index, overlay: &overlay, game: None };
        let r = resolve(&vfs, "a.res").unwrap();
        assert_eq!(r.missing.len(), 1);
    }
}
