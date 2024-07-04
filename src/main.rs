use bulk_edit::{bulk_edit, TextEditableItem};
use serenity::{
    all::{ChannelId, EditChannel, GuildChannel, Http},
    model::id::GuildId,
};
use std::{fmt::Display, io, sync::Arc};
mod bulk_edit;

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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = "token";

    // 設定したいGuild ID
    let guild_id = GuildId::new(1234567890);

    // クライアントを初期化
    let http = Arc::new(Http::new(token));

    // 指定したGuildのチャンネル一覧を取得
    let mut items = vec![];
    match guild_id.channels(&http).await {
        Ok(channels) => {
            for (channel_id, channel) in channels {
                let item = ChannelItem {
                    channel,
                    channel_id,
                    http: http.clone(),
                };
                items.push(item);
            }
        }
        Err(why) => {
            println!("Error fetching channels: {:?}", why);
        }
    }
    let diffs = bulk_edit(items.into_iter())?;
    for diff in diffs {
        println!("Applying {}", diff);
        diff.apply().await?;
    }
    Ok(())
}
