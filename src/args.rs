use clap::{Parser, Subcommand};
use clap_complete::Shell;
use serenity::all::ChannelType;
use std::{ops::BitAnd, path::PathBuf};

/// Tool to change Discord channel names in bulk with your $EDITOR
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, args_conflicts_with_subcommands = true)]
pub struct Args {
    #[clap(subcommand)]
    subcommand: Option<Commands>,
    /// Discord connection arguments
    #[clap(flatten)]
    discord: ConnectionArgs,
    /// Filter text channels
    #[clap(flatten)]
    filter: ChannelFilterArgs,
    /// Apply arguments
    #[clap(flatten)]
    apply: ApplyArgs,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Generate shell completion
    Completion {
        /// Shell to generate completion for
        shell: Shell,
    },
    /// Export all channel names to a file or stdout
    Export {
        /// Discord connection arguments
        #[clap(flatten)]
        discord: ConnectionArgs,
        /// File to export to
        #[clap(short, long)]
        output: Option<PathBuf>,
    },
    /// Apply all channel names from a file or stdin
    Apply {
        /// Discord connection arguments
        #[clap(flatten)]
        discord: ConnectionArgs,
        /// File to apply from
        #[clap(short, long)]
        input: Option<PathBuf>,
        /// Apply arguments
        #[clap(flatten)]
        apply: ApplyArgs,
    },
}

/// Token and Guild ID for Discord connection
#[derive(clap::Args, Debug)]
pub struct ConnectionArgs {
    /// Bot token. If not provided, it will be read from the $DISCORD_TOKEN environment variable
    #[clap(short, long)]
    pub token: Option<String>,
    /// Guild ID. If not provided, it will be read from the $GUILD_ID environment variable
    #[clap(short, long)]
    pub guild_id: Option<u64>,
}

#[derive(clap::Args, Debug, Clone, Default)]
pub struct ChannelFilterArgs {
    /// Edit Text Channels
    #[clap(long)]
    text: bool,
    /// Edit Voice Channels
    #[clap(long)]
    voice: bool,
    /// Edit Forum Channels
    #[clap(long)]
    forum: bool,
    /// Edit Stage Channels
    #[clap(long)]
    stage: bool,
    /// Edit News Channels
    #[clap(long)]
    news: bool,
    /// Edit Category Channels
    #[clap(long)]
    category: bool,
    /// Edit All Channels
    #[clap(long)]
    all: bool,
}

impl ChannelFilterArgs {
    pub fn none(&self) -> bool {
        !self.text
            && !self.voice
            && !self.forum
            && !self.stage
            && !self.news
            && !self.category
            && !self.all
    }
}

impl BitAnd<ChannelType> for &ChannelFilterArgs {
    type Output = bool;

    fn bitand(self, rhs: ChannelType) -> bool {
        if self.all {
            return true;
        }
        match rhs {
            ChannelType::Text if self.text => (),
            ChannelType::Voice if self.voice => (),
            ChannelType::Category if self.category => (),
            ChannelType::News if self.news => (),
            ChannelType::Forum if self.forum => (),
            ChannelType::Stage if self.stage => (),
            _ => {
                return false;
            }
        }
        true
    }
}

#[derive(clap::Args, Debug)]
pub struct ApplyArgs {
    /// Automatically confirm all changes
    #[clap(short, long)]
    pub yes: bool,
}

/// Parsed arguments for program execution
pub enum Work {
    /// Edit channel names
    Edit {
        /// Discord connection arguments
        discord: ConnectionArgs,
        /// Filter channels arguments
        filter: ChannelFilterArgs,
        /// Input file or Output file or Editor
        io: IOMode,
        /// Apply confirmation arguments
        apply: Option<ApplyArgs>,
    },
    /// Generate shell completion
    Completion(Shell),
}

/// Input/Output files or Editor mode
pub enum IOMode {
    /// Some Input file or Stdin
    Input(Option<PathBuf>),
    /// Some Output file or Stdout
    Output(Option<PathBuf>),
    /// Editor mode
    Editor,
}

impl From<Args> for Work {
    fn from(val: Args) -> Self {
        match val {
            Args {
                subcommand: None,
                discord,
                filter,
                apply,
            } => Work::Edit {
                discord,
                filter,
                io: IOMode::Editor,
                apply: Some(apply),
            },
            args => match args.subcommand.unwrap() {
                Commands::Completion { shell } => Work::Completion(shell),
                Commands::Export { discord, output } => Work::Edit {
                    discord,
                    filter: ChannelFilterArgs {
                        all: true,
                        ..Default::default()
                    },
                    io: IOMode::Output(output),
                    apply: None,
                },
                Commands::Apply {
                    discord,
                    input,
                    apply,
                } => Work::Edit {
                    discord,
                    filter: ChannelFilterArgs {
                        all: true,
                        ..Default::default()
                    },
                    io: IOMode::Input(input),
                    apply: Some(apply),
                },
            },
        }
    }
}
