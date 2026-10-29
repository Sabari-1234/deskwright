mod cli;
mod commands;

use std::process::ExitCode;

use clap::Parser;
use deskwright::Deskwright;
use tracing_subscriber::EnvFilter;

use crate::cli::{Cli, CliCommand};
use crate::commands::run_command::RunOptions;
use crate::commands::{call_command, run_command, serve_command, tools_command};

const DEFAULT_LOG_FILTER: &str = "info";

#[tokio::main]
async fn main() -> anyhow::Result<ExitCode> {
    let cli = Cli::parse();
    initialize_logging();
    let deskwright = Deskwright::launch(&cli.configuration)?;

    match cli.command.unwrap_or(CliCommand::Serve) {
        CliCommand::Serve => {
            serve_command::serve(&deskwright, cli.configuration.enable_script).await?;
            Ok(ExitCode::SUCCESS)
        }
        CliCommand::Tools => {
            print!("{}", tools_command::render_tool_list(&deskwright.tools()));
            Ok(ExitCode::SUCCESS)
        }
        CliCommand::Call {
            tool,
            arguments,
            image_dir,
        } => {
            let output =
                call_command::call(&deskwright, &tool, &arguments, image_dir.as_deref()).await?;
            print!("{}", call_command::render_output(&output));
            Ok(exit_code(!output.is_error))
        }
        CliCommand::Run {
            flows,
            reporter,
            trace,
            repeat,
        } => {
            let options = RunOptions {
                reporter,
                trace: trace.as_deref(),
                repeat,
            };
            let (report, has_passed) = run_command::run(&deskwright, &flows, &options).await?;
            print!("{report}");
            Ok(exit_code(has_passed))
        }
    }
}

fn exit_code(has_succeeded: bool) -> ExitCode {
    if has_succeeded {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Logs go to stderr because stdout carries the MCP protocol and the command output.
fn initialize_logging() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_FILTER));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();
}
