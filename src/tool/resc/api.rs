// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Pistonight/pvz-bintools contributors

use std::path::{Path, PathBuf};

use cu::pre::*;

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
    // fix config paths
    config.paths.input_directory = containing_dir
        .join(&config.paths.input_directory)
        .into_utf8()?;
    config.paths.output_xml = containing_dir.join(&config.paths.output_xml).into_utf8()?;
    config.paths.output_cpp = containing_dir.join(&config.paths.output_cpp).into_utf8()?;
    if let Some(output_h) = config.paths.output_h.as_mut() {
        *output_h = containing_dir.join(&*output_h).into_utf8()?;
    }

    let manifest = cu::check!(compiler::compile(&config), "failed to compile config")?;
    let xml = manifest.to_xml();

    cu::check!(
        cu::fs::write(config.paths.output_xml, &xml),
        "failed to write resources xml manifest"
    )?;
    cu::info!("written resources xml");

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
    cu::check!(
        cu::fs::write(output_cpp, &generated_code.source),
        "failed to write output cpp source"
    )?;
    cu::info!("written cpp source");
    cu::check!(
        cu::fs::write(output_h, &generated_code.header),
        "failed to write output cpp header"
    )?;
    cu::info!("written cpp header");

    Ok(())
}
