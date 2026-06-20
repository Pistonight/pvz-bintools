// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2026 Pistonight/pvz-bintools contributors

use cu::pre::*;

use crate::tool::{pakc, reanimc, resc};

/// Binary tools for PVZ
#[derive(clap::Parser, AsRef)]
pub struct Cli {
    /// Command (tool) to run
    #[clap(subcommand)]
    pub command: Option<CliCommand>,

    /// Print the version
    #[clap(short = 'V', long)]
    version: bool,

    #[clap(flatten)]
    #[as_ref]
    flags: cu::cli::Flags,
}

impl Cli {
    pub fn preprocess(&mut self) {
        if let Some(command) = &self.command {
            self.flags.merge(command.as_ref());
        }
    }
}

#[derive(clap::Subcommand)]
pub enum CliCommand {
    /// .pak tool
    Pakc(pakc::api::Cli),
    /// .reanim tool
    Reanimc(reanimc::api::Cli),
    /// resource tool
    Resc(resc::api::Cli),
}

impl AsRef<cu::cli::Flags> for CliCommand {
    fn as_ref(&self) -> &cu::cli::Flags {
        match self {
            CliCommand::Pakc(cli) => cli.as_ref(),
            CliCommand::Reanimc(cli) => cli.as_ref(),
            CliCommand::Resc(cli) => cli.as_ref(),
        }
    }
}

pub fn run(cli: Cli) -> cu::Result<()> {
    if cli.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        cu::lv::disable_print_time();
        return Ok(());
    }
    let Some(cmd) = cli.command else {
        cu::cli::print_help::<Cli>(false);
        cu::lv::disable_print_time();
        return Ok(());
    };
    match cmd {
        CliCommand::Pakc(cli) => pakc::api::run(cli),
        CliCommand::Reanimc(cli) => reanimc::api::run(cli),
        CliCommand::Resc(cli) => resc::api::run(cli),
    }
}
