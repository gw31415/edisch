mod bulk_edit;

use bulk_edit::{bulk_edit, TextEditableItem};
use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use dialoguer::Confirm;
use serenity::{
    all::{ChannelId, ChannelType, EditChannel, GuildChannel, Http},
    model::id::GuildId,
};
use std::{cmp::Ordering, io::BufWriter};
use std::{env, fmt::Display, io, sync::Arc};

#[derive(Clone)]
struct ChannelItem {
    channel: GuildChannel,
    channel_id: ChannelId,
    http: Arc<Http>,
    parent_name: Option<String>,
    parent_position: u16,
}

impl PartialEq for ChannelItem {
    fn eq(&self, other: &Self) -> bool {
        self.channel_id == other.channel_id
    }
}

impl Eq for ChannelItem {}

impl PartialOrd for ChannelItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ChannelItem {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.parent_position.cmp(&other.parent_position) {
            Ordering::Equal => {}
            other => return other,
        }
        if self.parent_name.is_some() && other.parent_name.is_none() {
            return Ordering::Greater;
        } else if self.parent_name.is_none() && other.parent_name.is_some() {
            return Ordering::Less;
        }
        match self.channel.parent_id.cmp(&other.channel.parent_id) {
            Ordering::Equal => self.channel.position.cmp(&other.channel.position),
            other => other,
        }
    }
}

impl Display for ChannelItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.channel_id)
    }
}

impl TextEditableItem for ChannelItem {
    async fn apply(&mut self, content: String) -> Result<(), io::Error> {
        let editchannel = EditChannel::new().name(content);
        self.channel_id
            .edit(self.http.clone(), editchannel)
            .await
            .or(Err(io::Error::new(
                io::ErrorKind::Other,
                "failed to edit channel",
            )))?;
        Ok(())
    }
    fn content(&self) -> String {
        self.channel.name.clone()
    }
    fn comment(&self) -> String {
        let mut comment = match self.channel.kind {
            ChannelType::Text => '📝',
            ChannelType::Voice => '🔊',
            ChannelType::Category => '📁',
            ChannelType::News => '📰',
            ChannelType::Forum => '📣',
            ChannelType::Stage => '🎭',
            _ => '❓',
        }.to_string();
        let parent_name = self.parent_name.clone();
        if let Some(parent_name) = parent_name {
            comment.push_str(" in ");
            comment.push_str(&parent_name);
        }
        comment.push_str(" (");
        comment.push_str(&self.channel_id.to_string());
        comment.push(')');
        comment
    }
    fn validate(&self, new: &str) -> Result<(), io::Error> {
        let len = new.chars().count();
        if !(2..=100).contains(&len) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Channel name must be between 2 and 100 characters",
            ));
        }

        // let err = Err(io::Error::new(
        //     io::ErrorKind::InvalidInput,
        //     format!("Invalid channel name: {}", new),
        // ));

        // if self.channel.kind == ChannelType::Category {
        //     let re = Regex::new(r"^[\p{L}\p{N}_ -]+$").unwrap();
        //
        //     if !re.is_match(new) {
        //         return err;
        //     }
        // } else {
        //     let re = Regex::new(r"^[\p{L}\p{N}_-]*$").unwrap();
        //
        //     if !re.is_match(new) {
        //         return err;
        //     }
        // }

        Ok(())
    }
}

/// Tool to change Discord channel names in bulk
#[derive(Parser, Debug)]
struct Args {
    /// Bot token
    #[clap(short, long)]
    token: Option<String>,
    /// Guild ID
    #[clap(short, long)]
    guild_id: Option<u64>,
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
    /// Edit all channel types
    #[clap(long)]
    all: bool,
    /// Generate shell completion
    #[arg(long, value_name = "SHELL")]
    completion: Option<Shell>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    // Shell completion
    if let Some(shell) = args.completion {
        shell_completion(shell);
        return Ok(());
    }

    let token = args.token.unwrap_or(env::var("DISCORD_TOKEN").unwrap());

    // 設定したいGuild ID
    let guild_id = GuildId::new(args.guild_id.unwrap_or(env::var("GUILD_ID")?.parse()?));

    // クライアントを初期化
    let http = Arc::new(Http::new(&token));

    // 指定したGuildのチャンネル一覧を取得
    let channels = guild_id.channels(&http).await?;
    let items = {
        let mut items: Vec<_> = channels
            .clone()
            .into_iter()
            .filter_map(|(channel_id, channel)| {
                let kind = channel.kind;
                let parent_name = 'p: {
                    let Some(id) = channel.parent_id else {
                        break 'p None;
                    };
                    let Some(parent) = channels.get(&id) else {
                        break 'p None;
                    };
                    Some(parent.name.clone())
                };
                let parent_position = if let Some(parent_id) = channel.parent_id {
                    channels
                        .get(&parent_id)
                        .map(|p| p.position)
                        .unwrap_or(channel.position)
                } else {
                    channel.position
                };
                let item = Some(ChannelItem {
                    channel,
                    channel_id,
                    parent_name,
                    parent_position,
                    http: http.clone(),
                });
                if args.all {
                    return item;
                }
                match kind {
                    ChannelType::Text if args.text => item,
                    ChannelType::Voice if args.voice => item,
                    ChannelType::Category if args.category => item,
                    ChannelType::News if args.news => item,
                    ChannelType::Forum if args.forum => item,
                    ChannelType::Stage if args.stage => item,
                    _ => None,
                }
            })
            .collect();
        items.sort();
        items
    };
    if items.is_empty() {
        println!("No channels found");
        return Ok(());
    }
    let diffs = bulk_edit(items.into_iter())?;
    if diffs.is_empty() {
        println!("No changes to apply");
        return Ok(());
    }
    for diff in &diffs {
        println!("{}", diff);
    }

    if !Confirm::new()
        .with_prompt("Do you want to apply these changes?")
        .default(false)
        .interact()?
    {
        return Ok(());
    }
    for diff in diffs {
        println!("Applying {}", diff);
        diff.apply().await?;
    }
    Ok(())
}

#[cold]
fn shell_completion(shell: clap_complete::Shell) {
    let mut stdout = BufWriter::new(io::stdout());
    let mut cmd = Args::command();
    let name = cmd.get_name().to_string();
    clap_complete::generate(shell, &mut cmd, name, &mut stdout);
}
