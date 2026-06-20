// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Pistonight/pvz-bintools contributors

use cu::pre::*;

use crate::util::Pattern;

#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub paths: Config_paths,
    pub codegen: Config_codegen,
    pub resource_groups: Vec<Config_resource_group>,
}

/// Path configurations (relative to the directory containing the config file)
#[allow(non_camel_case_types)]
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config_paths {
    /// Input directory to discover the resources
    pub input_directory: String,

    /// Output location for resources.xml
    ///
    /// Can be part of the input directory (will be automatically ignored)
    pub output_xml: String,

    /// Output location for the resource loader implementation .cpp file
    pub output_cpp: String,
    /// Output location for the resource loader header .h file
    ///
    /// Default is the cpp file with the extension changed to .h
    pub output_h: Option<String>,

    /// Path patterns to exclude from adding to the resource manifest
    ///
    /// Note that directories are also checked and if the directory paths
    /// match, the directory will be skipped entirely
    #[serde(default)]
    pub excludes: Vec<Pattern>,
}

#[allow(non_camel_case_types)]
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config_codegen {
    /// Include prefix to include SexyAppFramework. Default is "SexyAppFramework
    #[serde(default = "default_include_prefix_sexy")]
    pub include_prefix_sexy: String,
    /// Include prefix to include the generate resources header.
    pub include_prefix: Option<String>,
    /// Namespace to put the generated code, Default is "Sexy"
    #[serde(default = "default_namespace")]
    pub namespace: String,
}
fn default_include_prefix_sexy() -> String {
    "SexyAppFramework".to_owned()
}
fn default_namespace() -> String {
    "Sexy".to_owned()
}

#[allow(non_camel_case_types)]
#[derive(Deserialize)]
pub struct Config_resource_group {
    /// Name of the group
    pub name: String,
    /// Contents of the group
    pub contents: Vec<Config_resource_group_content>,
}

#[allow(non_camel_case_types)]
#[derive(Deserialize)]
#[serde(tag = "type")]
pub enum Config_resource_group_content {
    Image(Config_resource_content_image),
    Font(Config_resource_content_font),
    Sound(Config_resource_content_sound),
}

#[allow(non_camel_case_types)]
#[derive(Deserialize)]
pub struct Config_resource_content_image {
    #[serde(flatten)]
    pub common: Config_resource_content_common,
    /// Default attributes to put on every `<Image>` of this content group
    #[serde(default)]
    pub attrs: Config_image_attrs,
    /// Items in this content group
    pub items: Vec<Config_image_item>,
}

#[allow(non_camel_case_types)]
#[derive(Deserialize)]
pub struct Config_resource_content_common {
    /// Corresponds to the `<SetDefaults>` element
    pub defaults: Option<Config_set_defaults>,
}

#[allow(non_camel_case_types)]
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config_set_defaults {
    /// Common path prefix
    ///
    /// Corresponds to `<SetDefaults path="">`
    pub path: String,

    /// Common ID prefix
    ///
    /// Corresponds to `<SetDefaults idprefix="">`
    pub id_prefix: String,
}

#[allow(non_camel_case_types)]
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config_image_item {
    #[serde(flatten)]
    pub path: Config_item_path,
    /// Attribute to override the group one
    #[serde(default)]
    pub attrs: Config_image_individual_attrs,
}

#[allow(non_camel_case_types)]
#[derive(Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Config_image_individual_attrs {
    nopal: Option<bool>,
    ddsurface: Option<bool>,
    a4r4g4b4: Option<bool>,
    a8r8g8b8: Option<bool>,
    rows: Option<u32>,
    cols: Option<u32>,
    minsubdivide: Option<bool>,

    /// Path to an image to use as alpha channel.
    ///
    /// Can use `[name]` as the placeholder for the file stem name
    /// (the part without the extension)
    pub alphaimage: Option<String>,

    /// what??
    pub alphagrid: Option<String>,
}

impl Config_image_individual_attrs {
    pub fn apply(&self, defaults: &Config_image_attrs) -> Config_image_attrs {
        Config_image_attrs {
            nopal: self.nopal.unwrap_or(defaults.nopal),
            ddsurface: self.ddsurface.unwrap_or(defaults.ddsurface),
            a4r4g4b4: self.a4r4g4b4.unwrap_or(defaults.a4r4g4b4),
            a8r8g8b8: self.a8r8g8b8.unwrap_or(defaults.a8r8g8b8),
            rows: self.rows.unwrap_or(defaults.rows),
            cols: self.cols.unwrap_or(defaults.cols),
            minsubdivide: self.minsubdivide.unwrap_or(defaults.minsubdivide),
        }
    }
}

#[allow(non_camel_case_types)]
#[derive(Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Config_image_attrs {
    /// Try to not palletize the image
    #[serde(default)]
    pub nopal: bool,
    /// Store the image as DirectDraw
    #[serde(default)]
    pub ddsurface: bool,
    /// In 3D acceleration, use A4R4G4B4 to store image
    /// which saves video memory (probably not that useful with modern hardware?)
    #[serde(default)]
    pub a4r4g4b4: bool,
    #[serde(default)]
    pub a8r8g8b8: bool,
    /// Num of rows if the image is a matrix
    #[serde(default)]
    pub rows: u32,
    /// Num of cols if the image is a matrix
    #[serde(default)]
    pub cols: u32,
    /// what
    #[serde(default)]
    pub minsubdivide: bool,
}

#[allow(non_camel_case_types)]
#[derive(Deserialize)]
pub struct Config_resource_content_font {
    #[serde(flatten)]
    pub common: Config_resource_content_common,
    /// Items in this content group
    pub items: Vec<Config_font_item>,
}

#[allow(non_camel_case_types)]
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config_font_item {
    #[serde(flatten)]
    pub path: Config_item_path,
    /// Treat the font as sys font
    #[serde(default)]
    pub sys: bool,
    /// Attributes for sys font (must be external)
    #[serde(default)]
    pub attrs: Option<Config_font_attrs>,
}

/// These can only be specified for sys font
#[allow(non_camel_case_types)]
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config_font_attrs {
    #[serde(default)]
    pub bold: bool,
    #[serde(default)]
    pub italic: bool,
    #[serde(default)]
    pub shadow: bool,
    #[serde(default)]
    pub underline: bool,
}

#[allow(non_camel_case_types)]
#[derive(Deserialize)]
pub struct Config_resource_content_sound {
    #[serde(flatten)]
    pub common: Config_resource_content_common,
    /// Items in this content group
    pub items: Vec<Config_sound_item>,
}

#[allow(non_camel_case_types)]
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config_sound_item {
    #[serde(flatten)]
    pub path: Config_item_path,
}

#[allow(non_camel_case_types)]
#[derive(Deserialize)]
#[serde(rename_all = "kebab-case", untagged)]
pub enum Config_item_path {
    Raw {
        /// Write the raw content to the resources.xml output
        raw: String,
    },
    Exact {
        /// If true, then the resource won't be searched inside the input
        /// directory but rather put into the output manifest directly
        #[serde(default)]
        external: bool,
        /// ID of the resource, default is the same behavior as pattern matching
        id: Option<String>,
        /// Path of the resource, no extensions (this is what will end up in the manifest)
        path: String,
    },
    Pattern {
        /// Pattern to match the resource
        ///
        /// The ID will automatically be the file stem (the file name without extension)
        /// with anything non-alphanumeric converted to `_`, then converted to upper case.
        pattern: Pattern,
    },
}
