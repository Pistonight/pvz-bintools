// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Pistonight/pvz-bintools contributors

use std::fmt::Write as _;

use cu::pre::*;

use crate::tool::resc::{Manifest, ManifestItemTag};

#[derive(Default)]
pub struct Codegen {
    pub source: String,
    pub header: String,
}

pub struct CodegenConfig {
    pub sexy_namespace: String,
    pub namespace: String,
    pub header_name: String,
    pub sexy_include: String,
    pub header_include: String,
}

static HEADER: &str = r"// GENERATED FILE - do not edit manually!
// clang-format off
";

pub fn generate(manifest: &Manifest, config: &CodegenConfig) -> cu::Result<Codegen> {
    let mut out = Codegen::default();

    let CodegenConfig {
        sexy_namespace,
        namespace,
        header_name,
        sexy_include,
        header_include,
    } = config;

    let ids = cu::check!(
        collect_ids(manifest),
        "there are invalid IDs in the manifest"
    )?;

    let _ = writeln!(out.source, "{HEADER}");
    let _ = writeln!(out.header, "{HEADER}");

    let _ = writeln!(out.header, "#pragma once");
    let _ = writeln!(out.header, "namespace {sexy_namespace} {{ ");
    let _ = writeln!(out.header, "class ResourceManager;");
    let _ = writeln!(out.header, "class Image;");
    let _ = writeln!(out.header, "class Font;");
    let _ = writeln!(out.header, "}} // namespace {sexy_namespace}");
    let _ = writeln!(out.header, "namespace {namespace} {{ ");

    // includes
    let _ = writeln!(out.source, "#include <atomic>");
    let _ = writeln!(out.source, "#include <cstdint>");
    let _ = writeln!(out.source, "#include <mutex>");
    let _ = writeln!(out.source, "#include <unordered_map>");
    if header_include.is_empty() {
        let _ = writeln!(out.source, "#include \"{header_name}\"");
    } else {
        if header_include.starts_with('.') {
            let _ = writeln!(out.source, "#include \"{header_include}/{header_name}\"");
        } else {
            let _ = writeln!(out.source, "#include <{header_include}/{header_name}>");
        }
    }
    if sexy_include.starts_with('.') {
        let _ = writeln!(out.source, "#include \"{sexy_include}/ResourceManager.h\"");
    } else {
        let _ = writeln!(out.source, "#include <{sexy_include}/ResourceManager.h>");
    }

    let _ = writeln!(out.source, "namespace {namespace} {{ ");

    // idmap
    let _ = writeln!(out.source, "static void* gResources[] = {{");
    let _ = writeln!(out.header, "// resource ids");
    let _ = writeln!(out.header, "enum class ResourceId {{");
    // FONT -> IMAGE -> SOUND
    let mut stage = "";
    let mut prev = "";
    for id in &ids {
        let _ = writeln!(out.source, "    &{id},");
        if stage.is_empty() || !id.starts_with(stage) {
            if !prev.is_empty() {
                let _ = writeln!(out.header, "    _{stage}LAST = {prev}_ID,");
            }
            stage = match stage {
                "" => "FONT_",
                "FONT_" => "IMAGE_",
                "IMAGE_" => "SOUND_",
                _ => cu::bail!("invalid id stage: '{stage}'"),
            };
            let _ = writeln!(out.header, "    {id}_ID,");
            let _ = writeln!(out.header, "    _{stage}FIRST = {id}_ID,");
        } else {
            let _ = writeln!(out.header, "    {id}_ID,");
        }
        prev = id;
    }
    let _ = writeln!(out.source, "    nullptr");
    let _ = writeln!(out.source, "}};");
    let _ = writeln!(out.header, "    _{stage}LAST = {prev}_ID,");
    let _ = writeln!(out.header, "    LENGTH");
    let _ = writeln!(out.header, "}};");

    // helpers
    let _ = writeln!(
        out.header,
        "Sexy::Image* LoadImageById(Sexy::ResourceManager* manager, ResourceId id);"
    );
    // we deliberately don't generate ReplaceImageById, it stores a random pointer
    // globally that could be freed anytime which easily causes UAF if not careful
    let _ = writeln!(out.header, "Sexy::Image* GetImageById(ResourceId id);");
    let _ = writeln!(out.header, "Sexy::Font* GetFontById(ResourceId id);");
    let _ = writeln!(out.header, "int GetSoundById(ResourceId id);");
    let _ = writeln!(
        out.header,
        "ResourceId GetIdByImage(Sexy::Image* theImage);"
    );
    let _ = writeln!(out.header, "ResourceId GetIdByFont(Sexy::Font* theFont);");
    let _ = writeln!(out.header, "ResourceId GetIdBySound(int theSound);");
    // we deliberately don't generate GetRef functions since they are foot guns
    let _ = writeln!(out.header, "const char* IdToString(ResourceId id);");
    // we deliberately don't generate IdFromString since it's foot gun
    for line in include_str!("impl.cpp").lines() {
        let _ = writeln!(out.source, "{line}");
    }
    let _ = writeln!(out.source, "const char* IdToString(ResourceId id) {{");
    let _ = writeln!(out.source, "    switch (id) {{");
    for id in &ids {
        let _ = writeln!(
            out.source,
            "        case ResourceId::{id}_ID: return \"{id}\";"
        );
    }
    let _ = writeln!(out.source, "        default: return \"\";");
    let _ = writeln!(out.source, "    }}");
    let _ = writeln!(out.source, "}}");

    // extractor for each group
    for group in &manifest.resource_groups {
        if group.is_empty() {
            let _ = writeln!(
                out.header,
                "inline bool Extract{}Resources(Sexy::ResourceManager*) {{ return true; }}",
                group.id
            );
            continue;
        }
        let _ = writeln!(
            out.header,
            "bool Extract{}Resources(Sexy::ResourceManager* theMgr);",
            group.id
        );
        let _ = writeln!(
            out.source,
            "bool Extract{}Resources(Sexy::ResourceManager* theMgr) {{",
            group.id
        );
        let _ = writeln!(out.source, "    if (!theMgr) {{ return false; }}",);
        let _ = writeln!(out.source, "    try {{",);
        let mut globals = vec![];
        for item in group.iter() {
            let id = item.full_id;
            let (typ, getter) = match item.item.tag {
                ManifestItemTag::Raw => {
                    cu::bail!("unexpected raw tag: manifest needs to be re-parsed");
                }
                ManifestItemTag::Image => ("Sexy::Image*", "GetImageThrow"),
                ManifestItemTag::Font => ("Sexy::Font*", "GetFontThrow"),
                ManifestItemTag::Sound => ("int", "GetSoundThrow"),
            };
            let _ = writeln!(out.source, "        {id} = theMgr->{getter}(\"{id}\");");
            globals.push((typ, id));
        }
        let _ = writeln!(
            out.source,
            "    }} catch(Sexy::ResourceManagerException&) {{ return false; }}",
        );
        let _ = writeln!(out.source, "    return true;",);
        let _ = writeln!(out.source, "}}",);

        for (typ, id) in globals {
            let _ = writeln!(out.header, "extern {typ} {id};");
            let _ = writeln!(out.source, "{typ} {id};");
        }
    }

    // loader
    let _ = writeln!(
        out.header,
        "bool ExtractResourcesByName(Sexy::ResourceManager* theManager, const char* theName);"
    );
    let _ = writeln!(
        out.source,
        "bool ExtractResourcesByName(Sexy::ResourceManager* theManager, const char* theName) {{"
    );
    let _ = writeln!(out.source, "    if (!theManager) {{ return false; }}",);
    let _ = writeln!(out.source, "    if (!theName) {{ return false; }}",);
    for group in &manifest.resource_groups {
        let name = &group.id;
        let _ = writeln!(
            out.source,
            "    if (std::strcmp(theName, \"{name}\") == 0) {{ return Extract{name}Resources(theManager); }}",
        );
    }
    let _ = writeln!(out.source, "    return false;");
    let _ = writeln!(out.source, "}}");

    let _ = writeln!(out.source, "}} // namespace {namespace}");
    let _ = writeln!(out.header, "}} // namespace {namespace}");

    Ok(out)
}

fn collect_ids(manifest: &Manifest) -> cu::Result<Vec<String>> {
    let mut ids = vec![];
    for group in &manifest.resource_groups {
        for item in group.iter() {
            let full_id = item.full_id;
            let tag = item.item.tag;
            if full_id.starts_with("IMAGE_") {
                if tag != ManifestItemTag::Image {
                    cu::bail!(
                        "invalid entry: expect item ID={full_id} to be an Image, got {}",
                        tag.to_str()
                    );
                }
            } else if full_id.starts_with("FONT_") {
                if tag != ManifestItemTag::Font {
                    cu::bail!(
                        "invalid entry: expect item ID={full_id} to be a Font, got {}",
                        tag.to_str()
                    );
                }
            } else if full_id.starts_with("SOUND_") {
                if tag != ManifestItemTag::Sound {
                    cu::bail!(
                        "invalid entry: expect item ID={full_id} to be a Sound, got {}",
                        tag.to_str()
                    );
                }
            } else {
                cu::bail!(
                    "invalid entry: item ID={full_id}; ID must start with IMAGE_, FONT_ or SOUND_"
                );
            }
            ids.push(full_id);
        }
    }
    ids.sort();
    Ok(ids)
}
