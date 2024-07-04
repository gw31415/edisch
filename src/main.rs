mod bulk_edit;

use bulk_edit::{bulk_edit, TextEditableItem};
use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use dialoguer::Confirm;
use regex::Regex;
use serenity::{
    all::{ChannelId, ChannelType, EditChannel, GuildChannel, Http},
    model::id::GuildId,
};
use std::io::BufWriter;
use std::{env, fmt::Display, io, sync::Arc};

fn is_valid_channel_name(name: &str) -> bool {
    let len = name.chars().count();
    if !(2..=100).contains(&len) {
        return false;
    }

    // チャンネル名に使用可能な文字の正規表現
    let re = Regex::new(r"^[\p{L}\p{N}_-]*$").unwrap();

    re.is_match(name)
}

#[derive(Clone)]
struct ChannelItem {
    channel: GuildChannel,
    channel_id: ChannelId,
    http: Arc<Http>,
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
    fn validate(&self) -> Result<(), io::Error> {
        let name = self.content();
        if !is_valid_channel_name(&name) {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid channel name",
            ))
        } else {
            Ok(())
        }
    }
}

#[derive(Parser, Debug)]
struct Args {
    /// Bot token
    #[clap(short, long)]
    token: Option<String>,
    /// Guild ID
    #[clap(short, long)]
    guild_id: Option<u64>,
    /// テキストチャンネル
    #[clap(long)]
    text: bool,
    /// ボイスチャンネル
    #[clap(long)]
    voice: bool,
    /// フォーラムチャンネル
    #[clap(long)]
    forum: bool,
    /// Stageチャンネル
    #[clap(long)]
    stage: bool,
    /// Newsチャンネル
    #[clap(long)]
    news: bool,
    /// カテゴリチャンネル
    #[clap(long)]
    category: bool,
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
    let items: Vec<_> = channels
        .into_iter()
        .filter_map(|(channel_id, channel)| {
            let kind = channel.kind;
            let item = Some(ChannelItem {
                channel,
                channel_id,
                http: http.clone(),
            });
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
