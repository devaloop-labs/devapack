use crate::{
    builder::{bank as bank_builder, plugin as plugin_builder},
    utils::{signature::get_signature, version::get_version},
};
use clap::CommandFactory;
use clap::FromArgMatches;
use clap::{Parser, Subcommand};
use std::env;
use tokio::io;

mod addon;
mod builder;
mod publisher;
mod types;
mod utils;

#[derive(Parser)]
#[command(name = "devapack")]
#[command(author = "Devaloop")]
#[command(about = "A tool to create and build banks/plugins/presets/templates for Devalang")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Manage Banks
    Bank {
        #[command(subcommand)]
        command: BankCommands,
    },

    /// Manage Plugins
    Plugin {
        #[command(subcommand)]
        command: PluginCommands,
    },

    /// Submit an addon to the official Devalang repository
    Submit {},

    /// Update an existing addon in the official Devalang repository
    Update {},

    /// Manage Publishers
    Publisher {
        #[command(subcommand)]
        command: PublisherCommands,
    },
}

#[derive(Subcommand)]
enum BankCommands {
    /// Scaffold a new bank
    Create {},

    /// Build banks
    Build {
        /// Relative path OR alias bank.<bankId>. Leave empty to build all.
        path: Option<String>,
    },

    /// List available banks
    List {},

    /// Bump bank version
    Version {
        /// Bank identifier: <publisher>.<name>
        id: String,
        /// Bump type: major | minor | patch
        bump: String,
    },

    /// Delete a generated bank
    Delete {
        /// Bank identifier: <publisher>.<name>
        id: String,
    },
}

#[derive(Subcommand)]
enum PluginCommands {
    /// Scaffold a new plugin
    Create {},

    /// Build plugins
    Build {
        /// Relative path OR alias plugin.<pluginId>. Leave empty to build all.
        path: Option<String>,
        #[arg(short, long, default_value_t = false)]
        /// Whether to build the release version
        release: bool,
        #[arg(long, default_value_t = false)]
        /// Require artifact to be signed (will error if no signature produced)
        require_signature: bool,
    },

    /// List available plugins
    List {},

    /// Manage Plugin Versions
    Version {
        /// Plugin identifier: <publisher>.<name>
        id: String,
        /// Bump type: major | minor | patch
        bump: String,
    },
}

#[derive(Subcommand)]
enum PublisherCommands {
    /// Create a new publisher
    Create {},

    /// Update publisher details
    Update { name: Option<String> },

    /// List your publishers
    List {},
}

#[tokio::main]
async fn main() -> io::Result<()> {
    let version = get_version();
    let signature = get_signature(&version);

    let version_static: &'static str = Box::leak(format!("v{}", version).into_boxed_str());
    let signature_static: &'static str = Box::leak(signature.into_boxed_str());

    let mut cmd = Cli::command();
    cmd = cmd.version(version_static).before_help(signature_static);

    let raw_args: Vec<String> = std::env::args().collect();
    if raw_args.iter().any(|a| a == "--version" || a == "-V") {
        println!("{}", signature_static);
        return Ok(());
    }

    let matches = cmd.get_matches();
    let cli: Cli = Cli::from_arg_matches(&matches).expect("failed to parse cli args");

    let cwd: String = env::current_dir()
        .map_err(|e| std::io::Error::other(format!("Failed to get current dir: {}", e)))?
        .into_os_string()
        .into_string()
        .map_err(|_| std::io::Error::other("Current directory contains invalid UTF-8"))?;

    match cli.command {
        Commands::Submit {} => {
            if let Err(e) = addon::submit::prompt::prompt_submit_addon(&cwd).await {
                return Err(io::Error::other(e));
            }

            Ok(())
        }

        Commands::Update {} => {
            if let Err(e) = addon::update::prompt::prompt_update_addon(&cwd).await {
                return Err(io::Error::other(e));
            }

            Ok(())
        }

        Commands::Bank { command } => match command {
            BankCommands::Create {} => {
                if let Err(e) = addon::bank::prompt::prompt_bank_addon(&cwd).await {
                    return Err(io::Error::other(e));
                }

                Ok(())
            }

            BankCommands::Build { path } => {
                match path {
                    Some(p) => {
                        let cwd_clone = cwd.clone();
                        let p_clone = p.clone();
                        let res = tokio::task::spawn_blocking(move || {
                            bank_builder::build_bank(&p_clone, &cwd_clone)
                        })
                        .await
                        .map_err(|e| io::Error::other(format!("Join error: {}", e)))?;
                        if let Err(e) = res {
                            return Err(io::Error::other(e));
                        }
                    }
                    None => {
                        let cwd_clone = cwd.clone();
                        let res = tokio::task::spawn_blocking(move || {
                            bank_builder::build_all_banks(&cwd_clone)
                        })
                        .await
                        .map_err(|e| io::Error::other(format!("Join error: {}", e)))?;
                        if let Err(e) = res {
                            return Err(io::Error::other(e));
                        }
                    }
                }

                Ok(())
            }

            BankCommands::List {} => {
                if let Err(e) = addon::bank::manage::list_banks(&cwd) {
                    return Err(io::Error::other(e));
                }

                Ok(())
            }

            BankCommands::Version { id, bump } => {
                if let Err(e) = addon::bank::manage::bump_version(&cwd, &id, &bump) {
                    return Err(io::Error::other(e));
                }

                Ok(())
            }

            BankCommands::Delete { id } => {
                if let Err(e) = addon::bank::manage::delete_bank(&cwd, &id) {
                    return Err(io::Error::other(e));
                }

                Ok(())
            }
        },

        Commands::Plugin { command } => match command {
            PluginCommands::Create {} => {
                if let Err(e) = addon::plugin::prompt::prompt_plugin_addon(&cwd).await {
                    return Err(io::Error::other(e));
                }

                Ok(())
            }

            PluginCommands::Build {
                path,
                release,
                require_signature,
            } => {
                match path {
                    Some(p) => {
                        let cwd_clone = cwd.clone();
                        let p_clone = p.clone();
                        let rel = release;
                        let req_sig = require_signature;
                        let res = tokio::task::spawn_blocking(move || {
                            plugin_builder::build_plugin(&p_clone, &rel, &cwd_clone, req_sig, true)
                        })
                        .await
                        .map_err(|e| io::Error::other(format!("Join error: {}", e)))?;
                        if let Err(e) = res {
                            return Err(io::Error::other(e));
                        }
                    }
                    None => {
                        let cwd_clone = cwd.clone();
                        let rel = release;
                        let req_sig = require_signature;
                        let res = tokio::task::spawn_blocking(move || {
                            plugin_builder::build_all_plugins(&rel, &cwd_clone, req_sig)
                        })
                        .await
                        .map_err(|e| io::Error::other(format!("Join error: {}", e)))?;
                        if let Err(e) = res {
                            return Err(io::Error::other(e));
                        }
                    }
                }

                Ok(())
            }
            PluginCommands::List {} => {
                if let Err(e) = addon::plugin::manage::list_plugins(&cwd) {
                    eprintln!("Error listing plugins: {}", e);
                }

                Ok(())
            }
            PluginCommands::Version { id, bump } => {
                if let Err(e) = addon::plugin::manage::bump_version(&cwd, &id, &bump) {
                    eprintln!("Error bumping version: {}", e);
                }

                Ok(())
            }
        },

        Commands::Publisher { command } => match command {
            PublisherCommands::Create {} => {
                if let Err(e) = publisher::create::prompt_create_publisher().await {
                    return Err(io::Error::other(e));
                }

                Ok(())
            }
            PublisherCommands::Update { name } => {
                if let Err(e) = publisher::update::prompt_update_publisher(name).await {
                    return Err(io::Error::other(e));
                }

                Ok(())
            }

            PublisherCommands::List {} => {
                if let Err(e) = publisher::list::list_publishers().await {
                    return Err(io::Error::other(e));
                }

                Ok(())
            }
        },
    }
}
