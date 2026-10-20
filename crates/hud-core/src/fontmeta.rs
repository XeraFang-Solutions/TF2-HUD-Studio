//! Minimal TrueType/OpenType reader for the bits a HUD preview needs: names and vertical metrics.

pub struct FontMeta {
    pub family: Option<String>,
    pub full_name: Option<String>,
    pub typographic_family: Option<String>,
    /// (usWinAscent + usWinDescent) / unitsPerEm — GDI's cell height, which is what VGUI "tall" sets.
    pub cell_ratio: f32,
    pub ascent_ratio: f32,
}

fn be16(b: &[u8], p: usize) -> Option<u16> {
    b.get(p..p + 2).map(|s| u16::from_be_bytes([s[0], s[1]]))
}
fn be32(b: &[u8], p: usize) -> Option<u32> {
    b.get(p..p + 4).map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
}

fn table(b: &[u8], tag: &[u8; 4]) -> Option<usize> {
    let n = be16(b, 4)? as usize;
    (0..n).find_map(|i| {
        let rec = 12 + i * 16;
        (b.get(rec..rec + 4)? == tag).then(|| be32(b, rec + 8).map(|o| o as usize)).flatten()
    })
}

fn name(b: &[u8], name_table: usize, id: u16) -> Option<String> {
    let count = be16(b, name_table + 2)? as usize;
    let storage = name_table + be16(b, name_table + 4)? as usize;
    let mut fallback = None;
    for i in 0..count {
        let r = name_table + 6 + i * 12;
        let (platform, _enc, lang, nid, len, off) =
            (be16(b, r)?, be16(b, r + 2)?, be16(b, r + 4)?, be16(b, r + 6)?, be16(b, r + 8)? as usize, be16(b, r + 10)? as usize);
        if nid != id {
            continue;
        }
        let raw = b.get(storage + off..storage + off + len)?;
        if platform == 3 || platform == 0 {
            let units: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
            let s = String::from_utf16_lossy(&units);
            if platform == 3 && (lang == 0x409 || fallback.is_none()) {
                if lang == 0x409 {
                    return Some(s);
                }
                fallback = Some(s);
            } else if fallback.is_none() {
                fallback = Some(s)
            }
        } else if platform == 1 && fallback.is_none() {
            fallback = Some(raw.iter().map(|&c| c as char).collect());
        }
    }
    fallback
}

pub fn read(bytes: &[u8]) -> Option<FontMeta> {
    let tag = bytes.get(0..4)?;
    if tag != [0, 1, 0, 0] && tag != b"OTTO" && tag != b"true" {
        return None;
    }
    let head = table(bytes, b"head")?;
    let upem = be16(bytes, head + 18)? as f32;
    let (mut ascent, mut descent) = (0f32, 0f32);
    if let Some(os2) = table(bytes, b"OS/2") {
        ascent = be16(bytes, os2 + 74).unwrap_or(0) as f32;
        descent = be16(bytes, os2 + 76).unwrap_or(0) as f32;
    }
    if ascent <= 0.0 {
        if let Some(hhea) = table(bytes, b"hhea") {
            ascent = be16(bytes, hhea + 4)? as i16 as f32;
            descent = -(be16(bytes, hhea + 6)? as i16 as f32);
        }
    }
    let (cell_ratio, ascent_ratio) = if upem > 0.0 && ascent > 0.0 { ((ascent + descent) / upem, ascent / upem) } else { (1.2, 0.95) };
    let names = table(bytes, b"name");
    Some(FontMeta {
        family: names.and_then(|n| name(bytes, n, 1)),
        full_name: names.and_then(|n| name(bytes, n, 4)),
        typographic_family: names.and_then(|n| name(bytes, n, 16)),
        cell_ratio,
        ascent_ratio,
    })
}
