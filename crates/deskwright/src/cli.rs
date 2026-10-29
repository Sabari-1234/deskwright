use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use deskwright::DeskwrightConfiguration;

/// Deskwright: drive desktop apps through their accessibility tree, like Playwright drives
/// browsers (macOS today). With no command it serves MCP over stdio for AI clients.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(flatten)]
    pub configuration: DeskwrightConfiguration,

    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

#[derive(Debug, Subcommand)]
pub enum CliCommand {
    /// Serve the tools over MCP on stdin/stdout (the default).
    Serve,
    /// List the available tools.
    Tools,
    /// Run one tool and print its output; exits with 1 if the tool failed.
    Call {
        /// The tool's name, e.g. desk_list_apps.
        tool: String,
        /// The tool's arguments as a JSON object.
        #[arg(default_value = "{}")]
        arguments: String,
        /// Save images the tool returns (screenshots) as PNG files in this folder.
        #[arg(long)]
        image_dir: Option<PathBuf>,
    },
    /// Run flow files (JSON lists of tool calls) and report each step with its timing;
    /// exits with 1 if any flow failed.
    Run {
        /// One or more flow files.
        #[arg(required = true)]
        flows: Vec<PathBuf>,
        /// How to print the results.
        #[arg(long, value_enum, default_value_t = ReportFormat::Text)]
        reporter: ReportFormat,
        /// Write an HTML trace (with a screenshot after every step) to this file.
        #[arg(long)]
        trace: Option<PathBuf>,
        /// Run every flow this many times, e.g. to benchmark a flow.
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..=100))]
        repeat: u32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ReportFormat {
    Text,
    Json,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(arguments: &[&str]) -> Cli {
        Cli::try_parse_from(std::iter::once("deskwright").chain(arguments.iter().copied())).unwrap()
    }

    #[test]
    fn serves_by_default_and_keeps_global_flags() {
        let cli = parse(&["--enable-script"]);

        assert!(cli.command.is_none());
        assert!(cli.configuration.enable_script);
    }

    #[test]
    fn parses_call_with_default_arguments() {
        let Some(CliCommand::Call {
            tool, arguments, ..
        }) = parse(&["call", "desk_list_apps"]).command
        else {
            panic!("expected call");
        };

        assert_eq!(tool, "desk_list_apps");
        assert_eq!(arguments, "{}");
    }

    #[test]
    fn parses_run_options_and_flags_after_the_command() {
        let cli = parse(&[
            "run",
            "a.json",
            "b.json",
            "--reporter",
            "json",
            "--repeat",
            "3",
            "--enable-script",
        ]);

        let Some(CliCommand::Run {
            flows,
            reporter,
            repeat,
            trace,
        }) = cli.command
        else {
            panic!("expected run");
        };
        assert_eq!(flows.len(), 2);
        assert_eq!(reporter, ReportFormat::Json);
        assert_eq!(repeat, 3);
        assert!(trace.is_none());
        assert!(cli.configuration.enable_script);
    }

    #[test]
    fn rejects_run_without_flows_or_with_too_many_repeats() {
        let parse = |arguments: &[&str]| {
            Cli::try_parse_from(std::iter::once("deskwright").chain(arguments.iter().copied()))
        };

        assert!(parse(&["run"]).is_err());
        assert!(parse(&["run", "a.json", "--repeat", "0"]).is_err());
    }
}
