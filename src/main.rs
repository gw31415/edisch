mod args;
mod bulk_edit;
mod error;

use args::{ApplyArgs, Args, IOMode, Work};
use atty::Stream;
use bulk_edit::{Editor, TextEditableItem};
use clap::{CommandFactory, Parser};
use console::pad_str;
use dialoguer::Confirm;
use error::{Error, Result};
use regex::Regex;
use scopeguard::defer;
use serenity::{
    all::{ChannelId, ChannelType, EditChannel, GuildChannel, Http},
    model::id::GuildId,
};
use std::{
    cmp::Ordering,
    collections::HashMap,
    env,
    fmt::Display,
    fs::File,
    io::{self, stdin, stdout, BufReader, BufWriter, Read, Write},
    sync::Arc,
};
use unicode_width::UnicodeWidthStr;

#[derive(Clone)]
struct ChannelItem {
    /// Discord HTTPクライアント
    http: Arc<Http>,

    /// チャンネル情報
    channel: GuildChannel,
    /// チャンネルID
    channel_id: ChannelId,

    /// 親チャンネルの名前
    parent_name: Option<String>,
    /// 所属するカテゴリのposition
    category_position: u16,
}

impl ChannelItem {
    fn is_no_categoryzed_channel(&self) -> bool {
        self.channel.kind != ChannelType::Category && self.parent_name.is_none()
    }
    fn is_voice_like_channel(&self) -> bool {
        self.channel.kind == ChannelType::Voice || self.channel.kind == ChannelType::Stage
    }
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
        // 無カテゴリチャンネルを一番上にする
        if self.is_no_categoryzed_channel() && !other.is_no_categoryzed_channel() {
            return Ordering::Less;
        } else if !self.is_no_categoryzed_channel() && other.is_no_categoryzed_channel() {
            return Ordering::Greater;
        }

        // 同一カテゴリのチャンネルをまとめる
        match self.category_position.cmp(&other.category_position) {
            Ordering::Equal => {}
            other => return other,
        }

        // 同一カテゴリ内なら、カテゴリを表すチャンネルを一番上にする
        if self.parent_name.is_some() && other.parent_name.is_none() {
            return Ordering::Greater;
        } else if self.parent_name.is_none() && other.parent_name.is_some() {
            return Ordering::Less;
        }

        // 同一カテゴリ内なら、ボイス系チャンネルを下にする
        if self.is_voice_like_channel() && !other.is_voice_like_channel() {
            return Ordering::Greater;
        } else if !self.is_voice_like_channel() && other.is_voice_like_channel() {
            return Ordering::Less;
        }

        // 同一カテゴリ内なら、positionでソート
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
    async fn apply(&mut self, content: String) -> Result<()> {
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
            ChannelType::News => '📣',
            ChannelType::Forum => '💬',
            ChannelType::Stage => '🎭',
            _ => '❓',
        }
        .to_string();
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
    fn validate(&self, new: &str) -> Result<()> {
        let len = new.chars().count();
        if !(2..=100).contains(&len) {
            return Err(Error::InvalidChannelName {
                name: new.to_string(),
                message: "Channel name must be between 2 and 100 characters",
            });
        }

        let err = Err(Error::InvalidChannelName {
            name: new.to_string(),
            message: "Contains characters or patterns that cannot be used",
        });

        // TODO: 文字種やルールの制限が不十分。
        let re = if self.channel.kind == ChannelType::Category {
            Regex::new(r"^[\-\w]*|[^\x00-\x7F ]*$").unwrap()
        } else {
            Regex::new(r"^[\-\w]*|[^\x00-\x7F]*$").unwrap()
        };
        if !re.is_match(new) || new.contains("--") {
            return err;
        }

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    let is_tty = atty::is(Stream::Stderr);

    if let Err(e) = run(is_tty).await {
        let prompt = if e.unknown() {
            let mut p = console::style("UNKNOWN ERROR");
            if is_tty {
                p = p.on_red().bold();
            }
            p
        } else {
            let mut p = console::style("error:");
            if is_tty {
                p = p.red().bold();
            }
            p
        };
        eprint!("{} ", prompt);
        eprintln!("{}", e);
        std::process::exit(1);
    }
}

async fn run(is_tty: bool) -> Result<()> {
    let work: Work = Args::parse().into();

    let (discord, filter, io, apply) = match work {
        Work::Completion(shell) => {
            shell_completion(shell);
            return Ok(());
        }
        Work::Edit {
            discord,
            filter,
            io,
            apply,
        } => (discord, filter, io, apply),
    };

    let (http, guild_id) = {
        // 設定したいGuild ID
        let guild_id = GuildId::new(discord.guild_id.unwrap_or({
            let Ok(id) = env::var("GUILD_ID") else {
                return Err(Error::MissingArgument("GUILD_ID".into()));
            };
            let Ok(id) = id.parse() else {
                return Err(Error::ParseArgument("GUILD_ID".into()));
            };
            id
        }));

        let token = discord
            .token
            .clone()
            .unwrap_or(env::var("DISCORD_TOKEN").unwrap_or_default());
        if token.is_empty() {
            return Err(Error::MissingArgument("DISCORD_TOKEN".into()));
        }

        // 接続
        let http = Arc::new(Http::new(&token));
        (http, guild_id)
    };

    // 指定したGuildのチャンネル一覧を取得
    {
        // チャンネル一覧取得中の表示
        let mut msg = console::style("Fetching channels...");
        if is_tty {
            msg = msg.dim();
        }
        eprintln!("{msg}");
        stdout().flush().unwrap();
    }

    let channels = {
        defer! {
            if is_tty {
                // チャンネル一覧取得中の表示を消す
                eprint!("\x1B[1A\x1B[2K");
                stdout().flush().unwrap();
            }
        }

        if filter.none() {
            HashMap::new()
        } else {
            guild_id.channels(&http).await?
        }
    };

    // フィルタリングとパース、ソート
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
                let category_position = if let Some(parent_id) = channel.parent_id {
                    channels
                        .get(&parent_id)
                        .map(|p| p.position)
                        .unwrap_or(channel.position)
                } else {
                    channel.position
                };
                if (&filter) & kind {
                    Some(ChannelItem {
                        http: http.clone(),
                        channel,
                        channel_id,
                        parent_name,
                        category_position,
                    })
                } else {
                    None
                }
            })
            .collect();
        if items.is_empty() {
            eprintln!("No channels found");
            return Ok(());
        }
        items.sort();
        items
    };

    // チャンネル名の一括編集
    let mut editor = Editor::new(items.into_iter())?;

    let diffs: Vec<_> = {
        match io {
            IOMode::Output(output) => {
                match output {
                    Some(file) => {
                        let mut output = BufWriter::new(File::create(file)?);
                        writeln!(output, "{}", editor)?;
                    }
                    None => {
                        let mut output = BufWriter::new(stdout());
                        writeln!(output, "{}", editor)?;
                    }
                }
                return Ok(());
            }
            IOMode::Editor => {
                editor.edit()?;
            }
            IOMode::Input(input) => {
                let text = {
                    let mut text = String::new();
                    match input {
                        Some(ref p) => {
                            BufReader::new(File::open(p)?).read_to_string(&mut text)?;
                        }
                        None => {
                            BufReader::new(stdin()).read_to_string(&mut text)?;
                        }
                    }
                    text
                };
                editor.set_text(text)?;
            }
        }
        editor.try_into()?
    };

    if let Some(ApplyArgs { yes, .. }) = apply {
        if diffs.is_empty() {
            eprintln!("No changes to apply");
            return Ok(());
        }

        // OldとNewの表示文字列の幅を揃えるための計算
        let old_width = {
            let max_strwidth = diffs
                .iter()
                .map(|diff| UnicodeWidthStr::width(diff.old.as_str()))
                .max()
                .unwrap_or(0);
            max_strwidth
        };
        let new_width = {
            let max_strwidth = diffs
                .iter()
                .map(|diff| UnicodeWidthStr::width(diff.new.as_str()))
                .max()
                .unwrap_or(0);
            max_strwidth
        };

        if !yes {
            // 変更予定表の表示
            for diff in &diffs {
                let mut old = console::style(pad_str(
                    &diff.old,
                    old_width,
                    console::Alignment::Left,
                    None,
                ));
                let mut new = console::style(pad_str(
                    &diff.new,
                    new_width,
                    console::Alignment::Left,
                    None,
                ));
                let mut id = console::style(format!("({})", diff.item));
                let split = " -> ".to_string();
                if is_tty {
                    old = old.green();
                    new = new.green();
                    id = id.dim().italic();
                }
                eprintln!("{old}{split}{new}  {id}");
            }

            if !Confirm::new()
                .with_prompt("Do you want to apply these changes?")
                .default(false)
                .interact()?
            {
                return Ok(());
            }
        }

        // 変更状況の表示と適用
        for diff in diffs {
            let mut prompt = console::style("Applying:");
            let mut old = console::style(pad_str(
                &diff.old,
                old_width,
                console::Alignment::Left,
                None,
            ));
            let mut new = console::style(pad_str(
                &diff.new,
                new_width,
                console::Alignment::Left,
                None,
            ));
            let mut id = console::style(format!("({})", diff.item));
            let split = " -> ".to_string();
            if is_tty {
                prompt = prompt.blue().bold();
                old = old.green();
                new = new.green();
                id = id.dim().italic();
            }

            eprintln!("{prompt} {old}{split}{new}  {id}");
            diff.apply().await?;
        }
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
