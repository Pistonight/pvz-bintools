// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Pistonight/pvz-bintools contributors

use std::path::{Path, PathBuf};

use cu::pre::*;
use itertools::Itertools as _;

use crate::tool::resc::codegen::{self, CodegenConfig};
use crate::tool::resc::{Config, Manifest, compiler};

#[derive(Debug, clap::Parser, AsRef)]
pub struct Cli {
    /// Input file. Note that the paths in the YAML config is relative to the containing directory
    /// of the input
    pub input: String,

    /// Run in dump mode instead of compile mode. Dump a resources.xml file to JSON for inspection
    #[clap(short, long)]
    pub dump_xml: bool,

    #[clap(flatten)]
    #[as_ref]
    flags: cu::cli::Flags,
}

pub fn run(cli: Cli) -> cu::Result<()> {
    let input = cu::fs::read_string(&cli.input)?;
    let containing_dir = PathBuf::from(cli.input).parent_abs()?;

    if cli.dump_xml {
        let manifest = cu::check!(
            Manifest::try_parse_xml(&input),
            "failed to parse input manifest"
        )?;
        let manifest_json = cu::check!(
            json::stringify_pretty(&manifest),
            "failed to dump manifest to json"
        )?;
        println!("{manifest_json}");
        cu::lv::disable_print_time();
        return Ok(());
    }

    let mut config = cu::check!(yaml::parse::<Config>(&input), "failed to parse config")?;
    config.paths.prepend_containing_dir(&containing_dir)?;

    let manifest = cu::check!(compiler::compile(&config), "failed to compile config")?;
    let xml = manifest.to_xml();

    write_if_changed(config.paths.output_xml.as_ref(), &xml)?;

    // must re-parse XML to get the raw tags
    let mut manifest = cu::check!(
        Manifest::try_parse_xml(&xml),
        "unexpected: failed to parse generated resources.xml"
    )?;
    manifest.sort();

    let output_cpp = &config.paths.output_cpp;
    let (output_h, header_name) = match config.paths.output_h {
        Some(output_h) => {
            let p = Path::new(&output_h);
            let p = cu::check!(
                p.file_name(),
                "invalid output-h: must have a name for the header"
            )?;
            let p = cu::check!(p.as_utf8(), "output header path must be UTF-8")?;
            let p = p.to_owned();
            (output_h, p)
        }
        None => {
            let cpp_p = Path::new(&output_cpp);
            let p = cu::check!(
                cpp_p.file_stem(),
                "invalid output-cpp: must have a file name"
            )?;
            let p = cu::check!(p.as_utf8(), "output cpp path must be UTF-8")?;
            let p = format!("{p}.h");
            let output_h = cpp_p
                .parent()
                .unwrap_or(Path::new(""))
                .join(&p)
                .into_utf8()?;
            (output_h, p)
        }
    };

    let codegen_config = CodegenConfig {
        sexy_namespace: "Sexy".to_string(),
        namespace: config.codegen.namespace,
        header_name,
        sexy_include: config.codegen.include_prefix_sexy,
        header_include: config.codegen.include_prefix.unwrap_or("".to_string()),
    };

    let generated_code = cu::check!(
        codegen::generate(&manifest, &codegen_config),
        "codegen failed"
    )?;

    write_if_changed(output_cpp.as_ref(), &generated_code.source)?;
    write_if_changed(output_h.as_ref(), &generated_code.header)?;

    Ok(())
}

fn write_if_changed(path: &Path, new_content: &str) -> cu::Result<()> {
    let mut new_normalized = new_content.lines().join("\n");
    if let Ok(current) = cu::fs::read_string(path) {
        // we have clang-format off but git/other program could still change line end
        let current_normalized = current.lines().join("\n");
        if current_normalized == new_normalized {
            cu::info!("up-to-date: {}", path.try_to_rel().display());
            return Ok(());
        }
    }
    if !new_normalized.ends_with('\n') {
        new_normalized.push('\n');
    }
    cu::fs::write(path, new_normalized)?;
    cu::info!("written {}", path.try_to_rel().display());
    Ok(())
}
