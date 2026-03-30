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
mod test_support;

use config::IdentityOverrides;
use output::{print_validation_output, render_version_output};
use release::{publish_repo, render_publish_plan, PublishRegistry};
use scaffold::{create_scaffold, create_scaffold_json, init_scaffold, init_scaffold_json};
use sync::{
    inspect_repo_json, sync_apply_json, sync_check_json, sync_plan_json, sync_repo, validate_repo,
};

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
        #[arg(long)]
        json: bool,
        #[command(flatten)]
        overrides: IdentityOverrides,
    },
    /// Initialize or hydrate an existing directory in place
    Init {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        json: bool,
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
        #[arg(long)]
        plan: bool,
        #[arg(long)]
        json: bool,
    },
    /// Inspect effective repo contracts and owned metadata surfaces
    Inspect {
        #[arg(long, default_value = ".")]
        path: PathBuf,
        #[arg(long)]
        json: bool,
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
        #[arg(long)]
        plan: bool,
        #[arg(long)]
        json: bool,
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
        Commands::Create {
            target,
            json,
            overrides,
        } => {
            if json {
                println!("{}", create_scaffold_json(&target, &overrides)?);
                Ok(())
            } else {
                create_scaffold(&target, &overrides)
            }
        }
        Commands::Init {
            path,
            json,
            overrides,
        } => {
            if json {
                println!("{}", init_scaffold_json(&path, &overrides)?);
                Ok(())
            } else {
                init_scaffold(&path, &overrides)
            }
        }
        Commands::Validate { path, json } => {
            let output = validate_repo(&path)?;
            print_validation_output(&output, json)
        }
        Commands::Sync {
            path,
            check,
            plan,
            json,
        } => {
            if check && plan {
                anyhow::bail!("sync --check and --plan are mutually exclusive");
            }
            if plan && !json {
                anyhow::bail!("sync --plan currently requires --json");
            }
            if plan {
                println!("{}", sync_plan_json(&path)?);
                Ok(())
            } else if check && json {
                println!("{}", sync_check_json(&path)?);
                Ok(())
            } else if json {
                println!("{}", sync_apply_json(&path)?);
                Ok(())
            } else {
                sync_repo(&path, check)
            }
        }
        Commands::Inspect { path, json } => {
            if !json {
                anyhow::bail!("inspect currently requires --json");
            }
            println!("{}", inspect_repo_json(&path)?);
            Ok(())
        }
        Commands::Publish {
            path,
            dry_run,
            registry,
            skip_existing,
            plan,
            json,
        } => {
            if plan {
                if !json {
                    anyhow::bail!("publish --plan currently requires --json");
                }
                println!(
                    "{}",
                    render_publish_plan(&path, dry_run, registry, skip_existing)?
                );
                Ok(())
            } else {
                publish_repo(&path, dry_run, registry, skip_existing)
            }
        }
    }
}
