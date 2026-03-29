use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

mod config;
#[cfg(test)]
mod main_tests;
mod output;
mod release;
mod scaffold;
mod scaffold_manifest;
mod source_checkout;
mod sync;

#[cfg(test)]
use config::load_config;
use config::IdentityOverrides;
use output::{print_validation_output, render_version_output};
use release::{publish_repo, PublishRegistry};
use scaffold::{create_scaffold, init_scaffold};
use sync::{sync_repo, validate_repo};

#[cfg(test)]
pub(crate) use scaffold::{
    create_scaffold_from, discover_template_root, ensure_scaffold_source_root, init_scaffold_from,
};

#[cfg(test)]
pub(crate) use sync::{github_blob_url, github_raw_url, issue, render_wrapper_readme};

#[cfg(test)]
use output::VersionOutput;

#[cfg(test)]
use std::fs;

#[cfg(test)]
use std::path::Path;

#[derive(Parser)]
#[command(name = "ossplate")]
#[command(
    author,
    version,
    about = "Validate and sync a multi-registry OSS scaffold"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print tool version information
    Version,
    /// Scaffold a new target directory from the current template
    Create {
        target: PathBuf,
        #[command(flatten)]
        overrides: IdentityOverrides,
    },
    /// Initialize or hydrate an existing directory in place
    Init {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[command(flatten)]
        overrides: IdentityOverrides,
    },
    /// Validate project identity and metadata consistency
    Validate {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        json: bool,
    },
    /// Synchronize owned metadata surfaces
    Sync {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        check: bool,
    },
    /// Publish the current checked-out version from source without mutating git state
    Publish {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, value_enum, default_value_t = PublishRegistry::All)]
        registry: PublishRegistry,
        #[arg(long)]
        skip_existing: bool,
    },
}

fn main() {
    if let Err(error) = run() {
        eprintln!("ossplate: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Version => {
            println!("{}", render_version_output()?);
            Ok(())
        }
        Commands::Create { target, overrides } => create_scaffold(&target, &overrides),
        Commands::Init { path, overrides } => init_scaffold(&path, &overrides),
        Commands::Validate { path, json } => {
            let output = validate_repo(&path)?;
            print_validation_output(&output, json)
        }
        Commands::Sync { path, check } => sync_repo(&path, check),
        Commands::Publish {
            path,
            dry_run,
            registry,
            skip_existing,
        } => publish_repo(&path, dry_run, registry, skip_existing),
    }
}
