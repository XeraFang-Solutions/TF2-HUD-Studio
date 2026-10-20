//! Valve Texture Format decoder (v7.0–7.5): returns the largest mip of the first frame/face as RGBA8.
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VtfInfo {
    pub width: u32,
    pub height: u32,
    pub version: String,
    pub format: String,
    pub frames: u16,
    pub mipmaps: u8,
    pub flags: u32,
}

pub struct Image {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

fn rd16(b: &[u8], p: usize) -> Option<u16> {
    b.get(p..p + 2).map(|s| u16::from_le_bytes([s[0], s[1]]))
}
fn rd32(b: &[u8], p: usize) -> Option<u32> {
    b.get(p..p + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

const FORMAT_NAMES: [&str; 27] = [
    "RGBA8888",
    "ABGR8888",
    "RGB888",
    "BGR888",
    "RGB565",
    "I8",
    "IA88",
    "P8",
    "A8",
    "RGB888_BLUESCREEN",
    "BGR888_BLUESCREEN",
    "ARGB8888",
    "BGRA8888",
    "DXT1",
    "DXT3",
    "DXT5",
    "BGRX8888",
    "BGR565",
    "BGRX5551",
    "BGRA4444",
    "DXT1_ONEBITALPHA",
    "BGRA5551",
    "UV88",
    "UVWQ8888",
    "RGBA16161616F",
    "RGBA16161616",
    "UVLX8888",
];

/// Bytes needed for one `w`×`h` image in `format`.
fn image_size(format: u32, w: u32, h: u32) -> Option<usize> {
    let (w, h) = (w.max(1) as usize, h.max(1) as usize);
    let blocks = w.div_ceil(4) * h.div_ceil(4);
    Some(match format {
        13 | 20 => blocks * 8,
        14 | 15 => blocks * 16,
        0 | 1 | 11 | 12 | 16 | 23 | 26 => w * h * 4,
        2 | 3 | 9 | 10 => w * h * 3,
        4 | 6 | 17 | 18 | 19 | 21 | 22 => w * h * 2,
        5 | 7 | 8 => w * h,
        24 | 25 => w * h * 8,
        _ => return None,
    })
}

fn rgb565(c: u16) -> [u8; 3] {
    let r = ((c >> 11) & 31) as u32;
    let g = ((c >> 5) & 63) as u32;
    let b = (c & 31) as u32;
    [(r * 255 / 31) as u8, (g * 255 / 63) as u8, (b * 255 / 31) as u8]
}

fn dxt_colors(block: &[u8], dxt1: bool) -> [[u8; 4]; 4] {
    let c0 = u16::from_le_bytes([block[0], block[1]]);
    let c1 = u16::from_le_bytes([block[2], block[3]]);
    let (a, b) = (rgb565(c0), rgb565(c1));
    let mix = |x: u8, y: u8, wx: u32, wy: u32, d: u32| ((x as u32 * wx + y as u32 * wy) / d) as u8;
    let mut out = [[a[0], a[1], a[2], 255], [b[0], b[1], b[2], 255], [0; 4], [0; 4]];
    if !dxt1 || c0 > c1 {
        out[2] = [mix(a[0], b[0], 2, 1, 3), mix(a[1], b[1], 2, 1, 3), mix(a[2], b[2], 2, 1, 3), 255];
        out[3] = [mix(a[0], b[0], 1, 2, 3), mix(a[1], b[1], 1, 2, 3), mix(a[2], b[2], 1, 2, 3), 255];
    } else {
        out[2] = [mix(a[0], b[0], 1, 1, 2), mix(a[1], b[1], 1, 1, 2), mix(a[2], b[2], 1, 1, 2), 255];
        out[3] = [0, 0, 0, 0];
    }
    out
}

fn decode_dxt(data: &[u8], w: u32, h: u32, format: u32) -> Vec<u8> {
    let (w, h) = (w as usize, h as usize);
    let mut out = vec![0u8; w * h * 4];
    let block_size = if format == 13 || format == 20 { 8 } else { 16 };
    let bw = w.div_ceil(4);
    for by in 0..h.div_ceil(4) {
        for bx in 0..bw {
            let off = (by * bw + bx) * block_size;
            let Some(block) = data.get(off..off + block_size) else { return out };
            let (alpha_part, color_part) = if block_size == 16 { (&block[..8], &block[8..]) } else { (&block[..0], block) };
            let colors = dxt_colors(color_part, block_size == 8);
            let indices = u32::from_le_bytes([color_part[4], color_part[5], color_part[6], color_part[7]]);
            let mut alphas = [255u8; 16];
            if format == 14 {
                for i in 0..16 {
                    let nib = (alpha_part[i / 2] >> ((i % 2) * 4)) & 15;
                    alphas[i] = nib * 17;
                }
            } else if format == 15 {
                let a0 = alpha_part[0] as u32;
                let a1 = alpha_part[1] as u32;
                let mut table = [a0, a1, 0, 0, 0, 0, 0, 0];
                if a0 > a1 {
                    for (i, t) in table.iter_mut().enumerate().skip(2) {
                        *t = ((8 - i as u32) * a0 + (i as u32 - 1) * a1) / 7;
                    }
                } else {
                    for (i, t) in table.iter_mut().enumerate().take(6).skip(2) {
                        *t = ((6 - i as u32) * a0 + (i as u32 - 1) * a1) / 5;
                    }
                    table[6] = 0;
                    table[7] = 255;
                }
                let bits = alpha_part[2..8].iter().enumerate().fold(0u64, |acc, (i, b)| acc | ((*b as u64) << (8 * i)));
                for (i, a) in alphas.iter_mut().enumerate() {
                    *a = table[((bits >> (3 * i)) & 7) as usize] as u8;
                }
            }
            for py in 0..4 {
                for px in 0..4 {
                    let (x, y) = (bx * 4 + px, by * 4 + py);
                    if x >= w || y >= h {
                        continue;
                    }
                    let i = py * 4 + px;
                    let mut c = colors[((indices >> (2 * i)) & 3) as usize];
                    if block_size == 16 {
                        c[3] = alphas[i];
                    }
                    let o = (y * w + x) * 4;
                    out[o..o + 4].copy_from_slice(&c);
                }
            }
        }
    }
    out
}

fn half_to_f32(h: u16) -> f32 {
    let sign = if h & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exp = ((h >> 10) & 0x1f) as i32;
    let frac = (h & 0x3ff) as f32;
    match exp {
        0 => sign * frac * 2f32.powi(-24),
        31 => sign * f32::INFINITY,
        e => sign * (1.0 + frac / 1024.0) * 2f32.powi(e - 15),
    }
}

fn decode_pixels(data: &[u8], w: u32, h: u32, format: u32) -> Option<Vec<u8>> {
    if matches!(format, 13 | 14 | 15 | 20) {
        return Some(decode_dxt(data, w, h, format));
    }
    let n = (w * h) as usize;
    let mut out = Vec::with_capacity(n * 4);
    let bpp = image_size(format, 1, 1)?;
    for i in 0..n {
        let p = data.get(i * bpp..i * bpp + bpp)?;
        let px: [u8; 4] = match format {
            0 => [p[0], p[1], p[2], p[3]],
            1 => [p[3], p[2], p[1], p[0]],
            2 => [p[0], p[1], p[2], 255],
            3 => [p[2], p[1], p[0], 255],
            9 | 10 => {
                let (r, g, b) = if format == 9 { (p[0], p[1], p[2]) } else { (p[2], p[1], p[0]) };
                [r, g, b, if r == 0 && g == 0 && b == 255 { 0 } else { 255 }]
            }
            4 => {
                let c = rgb565(u16::from_le_bytes([p[0], p[1]]));
                [c[0], c[1], c[2], 255]
            }
            17 => {
                let c = rgb565(u16::from_le_bytes([p[0], p[1]]));
                [c[2], c[1], c[0], 255]
            }
            5 => [p[0], p[0], p[0], 255],
            6 => [p[0], p[0], p[0], p[1]],
            7 => [p[0], p[0], p[0], 255],
            8 => [255, 255, 255, p[0]],
            11 => [p[1], p[2], p[3], p[0]],
            12 => [p[2], p[1], p[0], p[3]],
            16 => [p[2], p[1], p[0], 255],
            18 | 21 => {
                let c = u16::from_le_bytes([p[0], p[1]]);
                let b = ((c & 31) * 255 / 31) as u8;
                let g = (((c >> 5) & 31) * 255 / 31) as u8;
                let r = (((c >> 10) & 31) * 255 / 31) as u8;
                [r, g, b, if format == 21 && c & 0x8000 == 0 { 0 } else { 255 }]
            }
            19 => {
                let c = u16::from_le_bytes([p[0], p[1]]);
                let n = |s: u16| (((c >> s) & 15) * 17) as u8;
                [n(8), n(4), n(0), n(12)]
            }
            22 => [p[0], p[1], 0, 255],
            23 | 26 => [p[0], p[1], p[2], p[3]],
            24 => {
                let v = |k: usize| half_to_f32(u16::from_le_bytes([p[k * 2], p[k * 2 + 1]]));
                let t = |f: f32| ((f / (1.0 + f)).clamp(0.0, 1.0).powf(1.0 / 2.2) * 255.0) as u8;
                [t(v(0)), t(v(1)), t(v(2)), (v(3).clamp(0.0, 1.0) * 255.0) as u8]
            }
            25 => {
                let v = |k: usize| (u16::from_le_bytes([p[k * 2], p[k * 2 + 1]]) >> 8) as u8;
                [v(0), v(1), v(2), v(3)]
            }
            _ => return None,
        };
        out.extend_from_slice(&px);
    }
    Some(out)
}

pub fn info(bytes: &[u8]) -> Result<VtfInfo, String> {
    if bytes.get(0..4) != Some(b"VTF\0") {
        return Err("Not a VTF file".into());
    }
    let major = rd32(bytes, 4).ok_or("Truncated VTF")?;
    let minor = rd32(bytes, 8).ok_or("Truncated VTF")?;
    let format = rd32(bytes, 52).ok_or("Truncated VTF")?;
    Ok(VtfInfo {
        width: rd16(bytes, 16).ok_or("Truncated VTF")? as u32,
        height: rd16(bytes, 18).ok_or("Truncated VTF")? as u32,
        version: format!("{major}.{minor}"),
        format: FORMAT_NAMES.get(format as usize).unwrap_or(&"Unknown").to_string(),
        frames: rd16(bytes, 24).unwrap_or(1),
        mipmaps: *bytes.get(56).unwrap_or(&1),
        flags: rd32(bytes, 20).unwrap_or(0),
    })
}

pub fn decode(bytes: &[u8]) -> Result<Image, String> {
    let i = info(bytes)?;
    let minor: u32 = i.version.split('.').nth(1).and_then(|m| m.parse().ok()).unwrap_or(0);
    let header_size = rd32(bytes, 12).ok_or("Truncated VTF")? as usize;
    let format = rd32(bytes, 52).ok_or("Truncated VTF")?;
    let low_format = rd32(bytes, 57).unwrap_or(0xFFFF_FFFF);
    let (low_w, low_h) = (*bytes.get(61).unwrap_or(&0) as u32, *bytes.get(62).unwrap_or(&0) as u32);
    let depth = if minor >= 2 { rd16(bytes, 63).unwrap_or(1).max(1) as usize } else { 1 };
    let frames = i.frames.max(1) as usize;
    let mipmaps = i.mipmaps.max(1) as u32;
    // Environment maps: 6 faces, plus a spheremap face in pre-7.5 files whose start frame isn't 0xFFFF (VTFLib rule).
    let faces = if i.flags & 0x4000 != 0 {
        if minor < 5 && rd16(bytes, 26).unwrap_or(0xFFFF) != 0xFFFF {
            7
        } else {
            6
        }
    } else {
        1
    };
    image_size(format, 1, 1).ok_or_else(|| format!("Unsupported VTF format {}", i.format))?;

    let high_start = if minor >= 3 {
        let count = rd32(bytes, 68).ok_or("Truncated VTF")? as usize;
        let mut found = None;
        for r in 0..count {
            let p = 80 + r * 8;
            if bytes.get(p..p + 3) == Some(&[0x30, 0, 0]) {
                found = rd32(bytes, p + 4).map(|v| v as usize);
            }
        }
        found.ok_or("VTF has no high-resolution image resource")?
    } else {
        let low_size = if low_format == 0xFFFF_FFFF || low_w == 0 { 0 } else { image_size(low_format, low_w, low_h).unwrap_or(0) };
        header_size + low_size
    };
    // Mips are stored smallest first; mip 0 (full size) is last.
    let mut offset = high_start;
    for mip in (1..mipmaps).rev() {
        let (w, h) = ((i.width >> mip).max(1), (i.height >> mip).max(1));
        offset += image_size(format, w, h).unwrap() * frames * faces * depth;
    }
    let size = image_size(format, i.width, i.height).unwrap();
    let data = bytes.get(offset..offset + size).ok_or("VTF image data is truncated")?;
    let rgba = decode_pixels(data, i.width, i.height, format).ok_or("Could not decode VTF pixels")?;
    Ok(Image { width: i.width, height: i.height, rgba })
}

pub fn to_png(img: &Image) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, img.width, img.height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(|e| e.to_string())?;
        w.write_image_data(&img.rgba).map_err(|e| e.to_string())?;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn vtf72(format: u32, w: u16, h: u16, data: &[u8]) -> Vec<u8> {
        let mut b = vec![0u8; 80];
        b[0..4].copy_from_slice(b"VTF\0");
        b[4..8].copy_from_slice(&7u32.to_le_bytes());
        b[8..12].copy_from_slice(&2u32.to_le_bytes());
        b[12..16].copy_from_slice(&80u32.to_le_bytes());
        b[16..18].copy_from_slice(&w.to_le_bytes());
        b[18..20].copy_from_slice(&h.to_le_bytes());
        b[24..26].copy_from_slice(&1u16.to_le_bytes());
        b[52..56].copy_from_slice(&format.to_le_bytes());
        b[56] = 1;
        b[57..61].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        b[63..65].copy_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(data);
        b
    }
    #[test]
    fn decodes_bgra() {
        let v = vtf72(12, 1, 1, &[10, 20, 30, 40]);
        let img = decode(&v).unwrap();
        assert_eq!(img.rgba, vec![30, 20, 10, 40]);
        assert!(!to_png(&img).unwrap().is_empty());
    }
    #[test]
    fn decodes_dxt1_solid_block() {
        // c0 = pure red (0xF800), c1 = black, all indices 0 -> red
        let block = [0x00, 0xF8, 0x00, 0x00, 0, 0, 0, 0];
        let img = decode(&vtf72(13, 4, 4, &block)).unwrap();
        assert_eq!(&img.rgba[0..4], &[255, 0, 0, 255]);
        assert_eq!(img.rgba.len(), 64);
    }
    #[test]
    fn decodes_dxt5_alpha() {
        let mut block = [0u8; 16];
        block[0] = 128; // a0
        block[1] = 128; // a1, indices all 0 -> 128
        block[8..10].copy_from_slice(&0x07E0u16.to_le_bytes()); // green
        let img = decode(&vtf72(15, 4, 4, &block)).unwrap();
        assert_eq!(&img.rgba[0..4], &[0, 255, 0, 128]);
    }
}
