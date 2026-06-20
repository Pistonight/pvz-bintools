// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Pistonight/pvz-bintools contributors

use std::borrow::Cow;
use std::fmt::Write as _;
use std::io::{Read as _, Write as _};

use cu::pre::*;
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

use crate::util::BinReader;

const MISSING: f32 = -10000.0;
const HEAD_MAGIC: u32 = 0xDEAD_FED4;
const SCHEMA_HASH: u32 = 0xB393_B4C0;
const TRACK_HEADER_SIZE: u32 = 12;
const TRANSFORM_STRUCT_SIZE: u32 = 44;
const EMPTY_STRING_PTR: u32 = 0x00B8_AE3C;

#[derive(Debug)]
pub struct ReanimCompiledStream {
    pub header: ReanimHeader,
    pub body: Vec<u8>,
}
impl ReanimCompiledStream {
    #[cu::context("failed to read compiled reanim stream")]
    pub fn read_compiled(bytes: &[u8]) -> cu::Result<Self> {
        let mut r = BinReader::new(bytes);
        let mut header = cu::check!(ReanimHeader::read(&mut r), "failed to read header")?;
        let declared_len = header.uncompressed_len as usize;

        let mut uncompressed_bytes = Vec::with_capacity(declared_len);
        cu::check!(
            ZlibDecoder::new(r.remaining()).read_to_end(&mut uncompressed_bytes),
            "failed to decompress compiled reanim"
        )?;
        let actual_len = uncompressed_bytes.len();
        if actual_len != declared_len {
            cu::bail!(
                "bad uncompressed_len field: declared: {declared_len}, actual: {actual_len}; not continuing for security reason"
            );
        }
        let mut r = BinReader::new(&uncompressed_bytes);
        header.schema_hash = cu::check!(r.read_u32(), "failed to read header.schema_hash")?;

        Ok(Self {
            header,
            body: uncompressed_bytes,
        })
    }

    #[cu::context("failed to read decompressed reanim stream")]
    pub fn read(&self) -> cu::Result<ReanimData<'static, '_>> {
        let mut r = BinReader::new(&self.body);
        let _ = r.read_u32(); //skip schema hash which we already read;
        let body = cu::check!(ReanimDefinition::read(&mut r), "failed to read reanim body")?;
        Ok(ReanimData {
            header: self.header.clone(),
            body,
        })
    }

    #[cu::context("failed to write compiled reanim stream")]
    pub fn write<W: std::io::Write>(&self, mut w: W) -> cu::Result<()> {
        w.write_all(&self.header.magic.to_le_bytes())?;
        w.write_all(&self.header.uncompressed_len.to_le_bytes())?;
        w.write_all(&self.body)?;
        Ok(())
    }
}

#[derive(Debug)]
pub struct ReanimData<'a, 'b> {
    pub header: ReanimHeader,
    pub body: ReanimDefinition<'a, 'b>,
}

impl ReanimData<'_, '_> {
    #[cu::context("failed to convert reanim data to json")]
    pub fn to_json(&self, show_ptr: bool) -> cu::Result<String> {
        let mut s = String::new();
        self.write_json(show_ptr, &mut s)?;
        Ok(s)
    }
    fn write_json(&self, show_ptr: bool, out: &mut String) -> cu::Result<()> {
        let _ = write!(
            out,
            r##"{{
  "magic": "0x{magic:08x}",
  "uncompressed_len": {uncompressed_len},
  "schema_hash": 0x{schema_hash:08x},
"##,
            magic = self.header.magic,
            uncompressed_len = self.header.uncompressed_len,
            schema_hash = self.header.schema_hash
        );
        self.body.write_json(show_ptr, out)?;
        out.push('}');
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ReanimHeader {
    /// magic number that is supposed to be HEAD_MAGIC
    magic: u32,
    /// Uncompressed length of the payload stream
    uncompressed_len: u32,
    /// Schema hash that is supposed to be SCHEMA_HASH
    schema_hash: u32,
}

impl ReanimHeader {
    pub fn new(uncompressed_len: u32) -> Self {
        Self {
            magic: HEAD_MAGIC,
            uncompressed_len,
            schema_hash: SCHEMA_HASH,
        }
    }
    fn read(r: &mut BinReader<'_>) -> cu::Result<Self> {
        let magic = cu::check!(r.read_u32(), "failed to read .magic")?;
        let uncompressed_len = cu::check!(r.read_u32(), "failed to read .uncompressed_len")?;
        Ok(Self {
            magic,
            uncompressed_len,
            schema_hash: 0,
        })
    }
}

#[derive(Debug)]
pub struct ReanimDefinition<'a, 'b> {
    /// (probably) pointer to tracks array, (probably) unused in parser
    tracks_ptr: u32,
    /// length of tracks array
    tracks_count: u32,
    /// <fps> tag
    pub fps: f32,
    /// (probably) unused in parser
    atlas_ptr: u32,
    /// Size of the track struct
    track_header_size: u32,
    /// <track> tags
    pub tracks: Vec<ReanimTrack<'a, 'b>>,
}
impl<'a, 'b> ReanimDefinition<'a, 'b> {
    pub fn new(fps: f32, tracks: Vec<ReanimTrack<'a, 'b>>) -> Self {
        Self {
            tracks_ptr: 0,
            tracks_count: tracks.len() as u32,
            fps,
            atlas_ptr: 0,
            track_header_size: TRACK_HEADER_SIZE,
            tracks,
        }
    }
}

impl<'b> ReanimDefinition<'_, 'b> {
    fn read(r: &mut BinReader<'b>) -> cu::Result<Self> {
        let tracks_ptr = cu::check!(r.read_u32(), "failed to read tracks_ptr")?;
        let tracks_count = cu::check!(r.read_u32(), "failed to read track_count")?;
        let fps = cu::check!(r.read_f32(), "failed to read fps")?;
        let atlas_ptr = cu::check!(r.read_u32(), "failed to read atlas_ptr")?;
        let track_header_size = cu::check!(r.read_u32(), "failed to read track_header_size")?;

        // tracks header
        let mut tracks: Vec<ReanimTrack<'static, 'b>> = Vec::with_capacity(tracks_count as usize);
        for i in 0..tracks_count {
            let header = cu::check!(
                ReanimTrackHeader::read(r),
                "failed to read tracks[{i}].header"
            )?;
            let transforms_count = header.transforms_count;
            tracks.push(ReanimTrack {
                header,
                name: "",
                transform_struct_size: 0,
                transforms: Vec::with_capacity(transforms_count as usize),
            });
        }

        // tracks data
        for (i, track) in tracks.iter_mut().enumerate() {
            cu::check!(track.read(r), "failed to read tracks[{i}]")?;
        }

        Ok(ReanimDefinition {
            tracks_ptr,
            tracks_count,
            fps,
            atlas_ptr,
            track_header_size,
            tracks,
        })
    }
}
impl<'b> ReanimDefinition<'b, 'b> {
    #[cu::context("failed to encode .reanim.compiled bytes")]
    pub fn compile(&mut self) -> cu::Result<ReanimCompiledStream> {
        for track in &mut self.tracks {
            track.compact_transforms();
        }
        let mut w = ReanimWriter::new();
        w.write_u32(SCHEMA_HASH)?;

        w.write_u32(self.tracks_ptr)?;
        w.write_u32(self.tracks_count)?;
        w.write_f32(self.fps)?;
        w.write_u32(self.atlas_ptr)?;
        w.write_u32(self.track_header_size)?;

        // tracks header
        for track in &self.tracks {
            track.header.write(&mut w)?;
        }
        // tracks data
        for track in &self.tracks {
            track.write_data(&mut w)?;
        }

        let uncompressed_len = w.uncompressed_len;
        let compressed_bytes = w.finish_compressed()?;
        Ok(ReanimCompiledStream {
            header: ReanimHeader::new(uncompressed_len),
            body: compressed_bytes,
        })
    }
}
impl ReanimDefinition<'_, '_> {
    fn write_json(&self, show_ptr: bool, out: &mut String) -> cu::Result<()> {
        if show_ptr {
            let _ = writeln!(out, r##"  "tracks_ptr": "0x{:08x}","##, self.tracks_ptr);
        }
        let _ = writeln!(
            out,
            r##"  "tracks_count": {tracks_count},
  "fps": {fps},"##,
            tracks_count = self.tracks_count,
            fps = self.fps,
        );
        if show_ptr {
            let _ = writeln!(out, r##"  "atlas_ptr": "0x{:08x}","##, self.atlas_ptr);
        }
        let _ = writeln!(
            out,
            r##"  "track_header_size": {},
  "tracks": ["##,
            self.track_header_size
        );
        for (i, track) in self.tracks.iter().enumerate() {
            track.write_json(show_ptr, i, out)?;
            if i != self.tracks.len() - 1 {
                let _ = writeln!(out, ",");
            }
        }
        let _ = writeln!(out, "  ]");
        Ok(())
    }
}

#[derive(Debug)]
pub struct ReanimTrack<'a, 'b> {
    /// Header data
    pub header: ReanimTrackHeader,
    /// Name of the track
    pub name: &'b str,
    /// (probably) Size of the transform struct, (probably) unused in parser
    pub transform_struct_size: u32,
    /// The transforms array
    pub transforms: Vec<ReanimTransform<'a, 'b>>,
}
impl<'b> ReanimTrack<'b, 'b> {
    pub fn new(name: &'b str, transforms: Vec<ReanimTransform<'b, 'b>>) -> Self {
        let transforms_count = transforms.len() as u32;
        Self {
            header: ReanimTrackHeader {
                name_ptr: 0,
                transforms_ptr: 0,
                transforms_count,
            },
            name,
            transform_struct_size: TRANSFORM_STRUCT_SIZE,
            transforms,
        }
    }
    fn compact_transforms(&mut self) {
        let mut state = ReanimTransform {
            values: ReanimTransformValues {
                x: XmlFloat::new(0.0),
                y: XmlFloat::new(0.0),
                kx: XmlFloat::new(0.0),
                ky: XmlFloat::new(0.0),
                sx: XmlFloat::new(1.0),
                sy: XmlFloat::new(1.0),
                frame: XmlFloat::new(0.0),
                alpha: XmlFloat::new(1.0),
                image_ptr: 0,
                font_ptr: 0,
                text_ptr: EMPTY_STRING_PTR,
            },
            strings: ReanimTransformStrings::default(),
        };

        for t in &mut self.transforms {
            t.compact(&mut state);
        }
    }
}
impl<'b> ReanimTrack<'_, 'b> {
    fn read(&mut self, r: &mut BinReader<'b>) -> cu::Result<()> {
        let name = cu::check!(r.read_u32_len_utf8(), "failed to read .name")?;
        self.name = name;
        self.transform_struct_size = cu::check!(
            r.read_u32(),
            "failed to read track<'{name}'>.transform_struct_size"
        )?;
        for i in 0..self.header.transforms_count {
            let values = cu::check!(
                ReanimTransformValues::read(r),
                "failed to read track<'{name}'>.transforms[{i}]"
            )?;
            self.transforms.push(ReanimTransform::<'static, 'b> {
                values,
                strings: Default::default(),
            })
        }
        for (i, transform) in self.transforms.iter_mut().enumerate() {
            transform.strings = cu::check!(
                ReanimTransformStrings::read(r),
                "failed to read track<'{name}'>.transforms[{i}]"
            )?;
        }

        Ok(())
    }
}
impl ReanimTrack<'_, '_> {
    fn write_data(&self, w: &mut ReanimWriter) -> cu::Result<()> {
        w.write_string(self.name)?;
        w.write_u32(self.transform_struct_size)?;
        for transform in &self.transforms {
            transform.values.write(w)?;
        }
        for transform in &self.transforms {
            transform.strings.write(w)?;
        }
        Ok(())
    }
    fn write_json(&self, show_ptr: bool, track_idx: usize, out: &mut String) -> cu::Result<()> {
        let _ = write!(out, "  {{\n    \"_idx\": {track_idx},\n");
        if show_ptr {
            let _ = write!(
                out,
                r##"    "name_ptr": "0x{:08x}",
    "tramsforms_ptr": "0x{:08x}",
"##,
                self.header.name_ptr, self.header.transforms_ptr,
            );
        }
        let _ = write!(
            out,
            r##"    "transforms_count": {transforms_count},
    "name": {name},
    "transform_struct_size": {transform_struct_size},
    "transforms": [
"##,
            transforms_count = self.header.transforms_count,
            name = json::stringify(&self.name)?,
            transform_struct_size = self.transform_struct_size,
        );
        for (i, transform) in self.transforms.iter().enumerate() {
            transform.write_json(show_ptr, i, out)?;
            if i != self.transforms.len() - 1 {
                let _ = writeln!(out, ",");
            }
        }
        let _ = write!(out, "\n    ]\n  }}");
        Ok(())
    }
}

#[derive(Debug)]
pub struct ReanimTrackHeader {
    /// (probably) pointer to the name field of the track, (probably) unused in parser
    name_ptr: u32,
    /// (probably) pointer to the transforms array of the track, (probably) unused in parser
    transforms_ptr: u32,
    /// Size of the transforms array
    transforms_count: u32,
}
static_assertions::assert_eq_size!(ReanimTrackHeader, [u8; TRACK_HEADER_SIZE as usize]);
impl ReanimTrackHeader {
    fn write(&self, w: &mut ReanimWriter) -> cu::Result<()> {
        w.write_u32(self.name_ptr)?;
        w.write_u32(self.transforms_ptr)?;
        w.write_u32(self.transforms_count)?;
        Ok(())
    }
    fn read(r: &mut BinReader<'_>) -> cu::Result<Self> {
        let name_ptr = cu::check!(r.read_u32(), "failed to read .name_ptr")?;
        let transforms_ptr = cu::check!(r.read_u32(), "failed to read .transforms_ptr")?;
        let transforms_count = cu::check!(r.read_u32(), "failed to read .transforms_count")?;
        Ok(Self {
            name_ptr,
            transforms_ptr,
            transforms_count,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ReanimTransform<'a, 'b> {
    pub values: ReanimTransformValues<'a>,
    pub strings: ReanimTransformStrings<'b>,
}
impl<'b> ReanimTransform<'b, 'b> {
    fn compact(&mut self, state: &mut Self) {
        Self::compact_float(&mut self.values.x, &mut state.values.x);
        Self::compact_float(&mut self.values.y, &mut state.values.y);
        Self::compact_float(&mut self.values.kx, &mut state.values.kx);
        Self::compact_float(&mut self.values.ky, &mut state.values.ky);
        Self::compact_float(&mut self.values.sx, &mut state.values.sx);
        Self::compact_float(&mut self.values.sy, &mut state.values.sy);
        Self::compact_float(&mut self.values.frame, &mut state.values.frame);
        Self::compact_float(&mut self.values.alpha, &mut state.values.alpha);
        Self::compact_string(&mut self.strings.image, &mut state.strings.image);
        Self::compact_string(&mut self.strings.font, &mut state.strings.font);
        Self::compact_string(&mut self.strings.text, &mut state.strings.text);
    }
    fn compact_float(f: &mut XmlFloat<'b>, state: &mut XmlFloat<'b>) {
        if f.value == MISSING {
            f.raw = "".into();
            return;
        }
        // Use raw string comparison when available (XML path); fall back to float.
        // (this ensures 55 and 55.0 are not treated as equal)
        let same = if !f.raw.is_empty() && !state.raw.is_empty() {
            f.raw == state.raw
        } else {
            f.value == state.value
        };
        if same {
            f.value = MISSING;
        }
        *state = f.clone();
    }
    fn compact_string(v: &mut Cow<'b, str>, state: &mut Cow<'b, str>) {
        if v.is_empty() {
            return;
        }
        if v == state {
            *v = "".into();
        }
        *state = v.clone();
    }
}
impl ReanimTransform<'_, '_> {
    fn write_json(&self, show_ptr: bool, idx: usize, out: &mut String) -> cu::Result<()> {
        let _ = write!(out, r##"      {{ "_idx": {idx}"##);
        macro_rules! write_float {
            ($key:literal, $value:ident) => {{
                let v = self.values.$value.value;
                if v != MISSING {
                    let _ = write!(out, r##", "{}": {}"##, $key, v);
                }
            }};
        }
        write_float!("x", x);
        write_float!("y", y);
        write_float!("kx", kx);
        write_float!("ky", ky);
        write_float!("sx", sx);
        write_float!("sy", sy);
        write_float!("f", frame);
        write_float!("a", alpha);
        let show_next_line = show_ptr
            || (!self.strings.image.is_empty()
                || !self.strings.font.is_empty()
                || !self.strings.text.is_empty());
        if !show_next_line {
            out.push_str(" }");
            return Ok(());
        }
        out.push_str(",\n        ");
        let mut has_trail = true;
        if show_ptr {
            let _ = write!(
                out,
                r##""image_ptr": "0x{image_ptr}", "font_ptr": "0x{font_ptr}", "text_ptr": "0x{text_ptr}""##,
                image_ptr = self.values.image_ptr,
                font_ptr = self.values.font_ptr,
                text_ptr = self.values.text_ptr,
            );
            has_trail = false;
        }
        if !self.strings.image.is_empty() {
            if !has_trail {
                out.push_str(", ");
            }
            let _ = write!(
                out,
                r##""image": {}"##,
                json::stringify(&self.strings.image)?
            );
            has_trail = false;
        }
        if !self.strings.font.is_empty() {
            if !has_trail {
                out.push_str(", ");
            }
            let _ = write!(out, r##""font": {}"##, json::stringify(&self.strings.font)?);
            has_trail = false;
        }
        if !self.strings.text.is_empty() {
            if !has_trail {
                out.push_str(", ");
            }
            let _ = write!(out, r##""text": {}"##, json::stringify(&self.strings.text)?);
            // has_trail = false;
        }
        out.push_str(" }");
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ReanimTransformValues<'a> {
    pub x: XmlFloat<'a>,
    pub y: XmlFloat<'a>,
    pub kx: XmlFloat<'a>,
    pub ky: XmlFloat<'a>,
    pub sx: XmlFloat<'a>,
    pub sy: XmlFloat<'a>,
    pub frame: XmlFloat<'a>,
    pub alpha: XmlFloat<'a>,
    pub image_ptr: u32,
    pub font_ptr: u32,
    pub text_ptr: u32,
}
impl Default for ReanimTransformValues<'_> {
    fn default() -> Self {
        Self {
            x: Default::default(),
            y: Default::default(),
            kx: Default::default(),
            ky: Default::default(),
            sx: Default::default(),
            sy: Default::default(),
            frame: Default::default(),
            alpha: Default::default(),
            image_ptr: Default::default(),
            font_ptr: Default::default(),
            text_ptr: EMPTY_STRING_PTR,
        }
    }
}
impl ReanimTransformValues<'static> {
    fn read(r: &mut BinReader<'_>) -> cu::Result<Self> {
        let x = cu::check!(r.read_f32(), "failed to read .x")?;
        let y = cu::check!(r.read_f32(), "failed to read .y")?;
        let kx = cu::check!(r.read_f32(), "failed to read .kx")?;
        let ky = cu::check!(r.read_f32(), "failed to read .ky")?;
        let sx = cu::check!(r.read_f32(), "failed to read .sx")?;
        let sy = cu::check!(r.read_f32(), "failed to read .sy")?;
        let frame = cu::check!(r.read_f32(), "failed to read .frame")?;
        let alpha = cu::check!(r.read_f32(), "failed to read .alpha")?;
        let image_ptr = cu::check!(r.read_u32(), "failed to read .image_ptr")?;
        let font_ptr = cu::check!(r.read_u32(), "failed to read .font_ptr")?;
        let text_ptr = cu::check!(r.read_u32(), "failed to read .text_ptr")?;
        Ok(Self {
            x: XmlFloat::new(x),
            y: XmlFloat::new(y),
            kx: XmlFloat::new(kx),
            ky: XmlFloat::new(ky),
            sx: XmlFloat::new(sx),
            sy: XmlFloat::new(sy),
            frame: XmlFloat::new(frame),
            alpha: XmlFloat::new(alpha),
            image_ptr,
            font_ptr,
            text_ptr,
        })
    }
}
impl ReanimTransformValues<'_> {
    fn write(&self, w: &mut ReanimWriter) -> cu::Result<()> {
        w.write_f32(self.x.value)?;
        w.write_f32(self.y.value)?;
        w.write_f32(self.kx.value)?;
        w.write_f32(self.ky.value)?;
        w.write_f32(self.sx.value)?;
        w.write_f32(self.sy.value)?;
        w.write_f32(self.frame.value)?;
        w.write_f32(self.alpha.value)?;
        w.write_u32(self.image_ptr)?;
        w.write_u32(self.font_ptr)?;
        w.write_u32(self.text_ptr)?;
        Ok(())
    }
}
#[derive(Debug, Clone)]
pub struct ReanimTransformStrings<'b> {
    pub image: Cow<'b, str>,
    pub font: Cow<'b, str>,
    pub text: Cow<'b, str>,
}
impl<'b> ReanimTransformStrings<'b> {
    fn read(r: &mut BinReader<'b>) -> cu::Result<Self> {
        let image = cu::check!(r.read_u32_len_utf8(), "failed to read .image")?;
        let font = cu::check!(r.read_u32_len_utf8(), "failed to read .font")?;
        let text = cu::check!(r.read_u32_len_utf8(), "failed to read .text")?;
        Ok(Self {
            image: image.into(),
            font: font.into(),
            text: text.into(),
        })
    }
}
impl Default for ReanimTransformStrings<'_> {
    fn default() -> Self {
        Self {
            image: "".into(),
            font: "".into(),
            text: "".into(),
        }
    }
}
impl ReanimTransformStrings<'_> {
    fn write(&self, w: &mut ReanimWriter) -> cu::Result<()> {
        w.write_string(&self.image)?;
        w.write_string(&self.font)?;
        w.write_string(&self.text)?;
        Ok(())
    }
}

/// A float field that carries both the parsed value and the original source string.
/// `raw` is populated from XML text; it is empty when parsed from binary.
/// Compaction uses `raw` for equality when available, falling back to `value`.
#[derive(Clone, Debug)]
pub struct XmlFloat<'a> {
    pub value: f32,
    pub raw: Cow<'a, str>,
}
impl Default for XmlFloat<'_> {
    fn default() -> Self {
        Self::missing()
    }
}

impl XmlFloat<'_> {
    pub fn missing() -> Self {
        XmlFloat {
            value: MISSING,
            raw: "".into(),
        }
    }
}
impl XmlFloat<'static> {
    pub fn new(value: f32) -> Self {
        Self {
            value,
            raw: value.to_string().into(),
        }
    }
}
impl<'a> XmlFloat<'a> {
    pub fn parse(s: &'a str) -> cu::Result<Self> {
        let s = s.trim();
        let v = cu::check!(s.parse::<f32>(), "failed to parse '{s}' as float")?;
        Ok(XmlFloat {
            value: v,
            raw: s.into(),
        })
    }
}

struct ReanimWriter {
    pub uncompressed_len: u32,
    pub enc: ZlibEncoder<Vec<u8>>,
}

impl ReanimWriter {
    fn new() -> Self {
        // using default level results in the same header as original files
        Self {
            uncompressed_len: 0,
            enc: ZlibEncoder::new(Vec::new(), Compression::default()),
        }
    }
    fn write_u32(&mut self, x: u32) -> cu::Result<()> {
        self.enc.write_all(&x.to_le_bytes())?;
        self.uncompressed_len += 4;
        Ok(())
    }

    fn write_f32(&mut self, x: f32) -> cu::Result<()> {
        self.enc.write_all(&x.to_le_bytes())?;
        self.uncompressed_len += 4;
        Ok(())
    }
    fn write_string(&mut self, s: &str) -> cu::Result<()> {
        let bytes = s.as_bytes();
        self.write_u32(bytes.len() as u32)?;
        self.enc.write_all(bytes)?;
        self.uncompressed_len += s.len() as u32;
        Ok(())
    }
    fn finish_compressed(mut self) -> cu::Result<Vec<u8>> {
        self.enc.flush()?;
        Ok(self.enc.finish()?)
    }
}
