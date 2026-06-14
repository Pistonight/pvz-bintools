// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Pistonight/pvz-bintools contributors

use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
use std::ffi::{OsStr, OsString};
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};

use cu::pre::*;

use crate::tool::reanimc::{data, xml};

#[derive(Debug, clap::Parser, AsRef)]
pub struct Cli {
    /// Input .reanim file(s), use `-` to read from stdin, can use glob.
    pub input: String,

    /// Run in dump mode instead of compile mode. Dump a .compiled file to JSON for inspection
    #[clap(short, long)]
    pub dump: bool,

    /// Also dump pointer fields which are unused
    #[clap(short, long, requires = "dump")]
    pub ptr: bool,

    /// Output location. Must be the target file location if input is not a glob, and must be a directory
    /// when input is a glob. Use `-` to write output to stdout (only if input is not a glob)
    ///
    /// The output files will always be in a flat list in the output directory even if the input
    /// files have nested directories. It will be an error if multiple input files have the same
    /// name.
    #[clap(short, long)]
    pub output: Option<String>,

    #[clap(flatten)]
    #[as_ref]
    flags: cu::cli::Flags,
}

impl Cli {
    pub fn read_inputs(&self) -> cu::Result<Inputs> {
        if self.input.is_empty() {
            if self.dump {
                cu::bail!("input .compiled file to dump required");
            }
            cu::bail!("input .reanim file(s) to compile required");
        }

        let input_is_glob = self.input.contains('*');

        if input_is_glob {
            let mut has_error = false;
            let mut output_map = BTreeMap::new();

            cu::debug!("using glob input mode: '{}'", self.input);

            let output = match &self.output {
                None => ".",
                Some(output) => {
                    if output.trim() == "-" {
                        cu::bail!("writing to stdout is not allowed when input is a glob");
                    }
                    output
                }
            };

            cu::debug!("output dir will be: '{output}'");

            let glob = cu::check!(
                cu::fs::glob(&self.input),
                "failed to process input file glob"
            )?;
            for entry in glob {
                let entry = match entry {
                    Ok(x) => x,
                    Err(e) => {
                        cu::warn!("ignoring file failed to read: {e:?}");
                        continue;
                    }
                };

                let Some(name) = entry.file_name() else {
                    cu::warn!("ignoring file without a name: '{}'", entry.display());
                    continue;
                };

                let output_name = derive_output_name(name, self.dump);

                let output_path = cu::path!(&output / output_name);
                let bytes = match cu::fs::read(&entry) {
                    Err(e) => {
                        cu::error!("error reading input: {e:?}");
                        has_error = true;
                        continue;
                    }
                    Ok(v) => v,
                };
                match output_map.entry(output_path) {
                    Entry::Vacant(e) => {
                        e.insert(InputInfo {
                            path: Some(entry),
                            bytes,
                        });
                    }
                    Entry::Occupied(existing) => {
                        let existing_path = existing
                            .get()
                            .path
                            .as_deref()
                            .unwrap_or(Path::new("<stdin>"));
                        cu::bail!(
                            "name collision on input files: '{}' and '{}' -> output: '{}'",
                            existing_path.display(),
                            entry.display(),
                            existing.key().display()
                        );
                    }
                }
            }

            if has_error {
                cu::bail!("there were error(s) reading input glob");
            }
            if output_map.is_empty() {
                cu::bail!("no input found matching the glob pattern");
            }
            return Ok(Inputs::Files(output_map));
        }

        let input_is_stdin = self.input.trim() == "-";
        if input_is_stdin {
            cu::debug!("using stdin input");
            let mut bytes = vec![];
            cu::check!(
                std::io::stdin().read_to_end(&mut bytes),
                "failed to read input from stdin"
            )?;
            let info = match &self.output {
                None => {
                    // no output when input is stdin, write to stdout
                    cu::debug!("using stdout output");
                    Inputs::Stdout(bytes)
                }
                Some(output) => {
                    if output.trim() == "-" {
                        cu::debug!("using stdout output");
                        Inputs::Stdout(bytes)
                    } else {
                        cu::debug!("output file will be: '{output}'");
                        Inputs::file(output.clone().into(), None, bytes)
                    }
                }
            };
            return Ok(info);
        }

        let input_file = Path::new(&self.input);
        cu::debug!("using single-file input: '{}'", input_file.display());
        let name = cu::check!(
            input_file.file_name(),
            "input file must have a name: '{}'",
            input_file.display()
        )?;
        let bytes = cu::check!(cu::fs::read(input_file), "failed to read input file")?;
        let output: PathBuf = match &self.output {
            None => derive_output_name(name, self.dump).into(),
            Some(path) => path.clone().into(),
        };
        cu::debug!("output file will be: '{}'", output.display());
        Ok(Inputs::file(output, Some(input_file.to_owned()), bytes))
    }
}

fn derive_output_name(input: &OsStr, dump: bool) -> OsString {
    let mut base = input.to_owned();
    if dump {
        base.push(".json");
    } else {
        base.push(".compiled");
    }
    base
}

pub enum Inputs {
    Stdout(Vec<u8>),
    Files(BTreeMap<PathBuf, InputInfo>),
}

pub struct InputInfo {
    pub path: Option<PathBuf>,
    pub bytes: Vec<u8>,
}

impl Inputs {
    fn file(output: PathBuf, input: Option<PathBuf>, bytes: Vec<u8>) -> Self {
        Self::Files(BTreeMap::from_iter(std::iter::once((
            output,
            InputInfo { path: input, bytes },
        ))))
    }
}

pub fn run(cli: Cli) -> cu::Result<()> {
    let inputs = cli.read_inputs()?;
    if cli.dump {
        return dump(inputs, cli.ptr);
    }
    compile(inputs)
}

pub fn dump(inputs: Inputs, show_ptr: bool) -> cu::Result<()> {
    match inputs {
        Inputs::Stdout(bytes) => {
            let stream = data::ReanimCompiledStream::read_compiled(&bytes)?;
            let data = stream.read()?;
            let json = data.to_json(show_ptr)?;
            println!("{json}");
            cu::lv::disable_print_time();
        }
        Inputs::Files(files) => {
            let len = files.len();
            for (output_path, input) in files {
                match input.path {
                    Some(p) => cu::debug!("dumping '{}'", p.display()),
                    None => cu::debug!("dumping <stdin>"),
                }
                let stream = data::ReanimCompiledStream::read_compiled(&input.bytes)?;
                let data = stream.read()?;
                let json = data.to_json(show_ptr)?;
                cu::fs::write(output_path, json)?;
            }
            if len > 1 {
                cu::info!("dumped {len} files");
            }
        }
    }
    Ok(())
}

pub fn compile(inputs: Inputs) -> cu::Result<()> {
    match inputs {
        Inputs::Stdout(bytes) => {
            let xml_src = xml::format_document(&bytes)?;
            let document = xml::ReanimDocument::parse_xml(&xml_src)?;
            let stream = document.parse()?.compile()?;
            let mut stdout = std::io::stdout();
            cu::check!(stream.write(&mut stdout), "failed to write to stdout")?;
            cu::check!(stdout.flush(), "failed to flush to stdout")?;
            cu::lv::disable_print_time();
        }
        Inputs::Files(files) => {
            // compiling is a bit slow (on my machine takes 3 seconds to compile all from the
            // original main.pak), so parallelize to get free perf
            cu::co::run(async move {
                let len = files.len();
                let mut handles = Vec::with_capacity(len);
                let pool = cu::co::pool(-1);
                for (output_path, input) in files {
                    let input_path = input
                        .path
                        .map(|x| format!("'{}'", x.display()))
                        .unwrap_or("<stdin>".into());
                    cu::debug!("compiling {input_path}");
                    let handle = pool.spawn_blocking(move || {
                        let stream = (|| {
                            let xml_src = xml::format_document(&input.bytes)?;
                            let document = xml::ReanimDocument::parse_xml(&xml_src)?;
                            let stream = document.parse()?.compile()?;
                            cu::Ok(stream)
                        })();
                        let stream =
                            cu::check!(stream, "failed to parse input file: '{input_path}'")?;
                        cu::Ok((stream, output_path))
                    });
                    handles.push(handle);
                }
                let mut set = cu::co::set(handles);
                while let Some(result) = set.next().await {
                    let (stream, output_path) = result???;
                    let mut writer = cu::fs::writer(output_path)?;
                    stream.write(&mut writer)?;
                    writer.flush()?;
                }
                if len > 1 {
                    cu::info!("compiled {len} files");
                }
                cu::Ok(())
            })?;
        }
    }
    Ok(())
}
