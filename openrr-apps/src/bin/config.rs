use std::{fs, path::PathBuf};

use anyhow::Result;
use schemars::schema_for;
use serde::Deserialize;
use structopt::{clap::arg_enum, StructOpt};
use tracing::debug;

#[derive(Debug, StructOpt)]
#[structopt(name = env!("CARGO_BIN_NAME"))]
struct Args {
    #[structopt(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, StructOpt)]
enum Subcommand {
    /// Generate JSON schema for the specified config file.
    Schema {
        /// Kind of config file.
        #[structopt(possible_values = &ConfigKind::variants(), case_insensitive = true)]
        kind: ConfigKind,
    },
    Merge {
        /// Path to the setting file.
        #[structopt(long, parse(from_os_str))]
        config_path: PathBuf,
        /// Config to overwrite
        #[structopt(long)]
        config: String,
    },
}

arg_enum! {
    #[derive(Debug)]
    enum ConfigKind {
        RobotConfig,
        RobotTeleopConfig,
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Config {
    RobotConfig(openrr_apps::RobotConfig),
    RobotTeleopConfig(openrr_apps::RobotTeleopConfig),
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    let args = Args::from_args();
    debug!(?args);

    match args.subcommand {
        Subcommand::Schema { kind } => {
            let schema = match kind {
                ConfigKind::RobotConfig => schema_for!(openrr_apps::RobotConfig),
                ConfigKind::RobotTeleopConfig => schema_for!(openrr_apps::RobotTeleopConfig),
            };
            println!("{}", serde_json::to_string_pretty(&schema).unwrap());
        }
        Subcommand::Merge {
            config_path,
            config: overwrite,
        } => {
            let s = &fs::read_to_string(&config_path)?;
            // check if the input is valid config.
            let _base: Config = toml::from_str(s)?;
            let mut edit: toml::Value = toml::from_str(s)?;
            openrr_config::overwrite(&mut edit, &overwrite)?;
            println!("{}", toml::to_string(&edit)?);
        }
    }

    Ok(())
}
