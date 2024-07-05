use std::borrow::Cow;
use thiserror::Error;

/// Short-hand for `Result<T, Error>`
pub type Result<T> = std::result::Result<T, Error>;

/// edischのエラー型
#[derive(Debug, Error)]
pub enum Error {
    /// 必要な引数が足りない場合
    #[error("Missing argument: {0}")]
    MissingArgument(Cow<'static, str>),
    /// 引数のパースに失敗した場合
    #[error("Failed to parse argument: {0}")]
    ParseArgument(Cow<'static, str>),

    /// 編集結果が不正な場合
    #[error("Invalid edit result: {0}")]
    InvalidEditResult(Cow<'static, str>),

    /// ファイルの読み書きに失敗した場合 (一時ファイルなど)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// テキストエディタが正常に終了しなかった場合
    #[error("EDITOR failed{}", if let Some(code) = .0 { format!(" with code {}", code) } else { String::new() })]
    Command(Option<i32>),

    /// チャンネル名が不正な場合
    #[error("Invalid channel name: {:?} ({})", name, message)]
    InvalidChannelName { name: String, message: &'static str },

    // 以下はキャッチされていないエラー
    #[error("{0}")]
    Serenity(#[from] serenity::Error),
    #[error("{0}")]
    Dialoguer(#[from] dialoguer::Error),
}

impl Error {
    /// 特にキャッチすることを想定していないエラー
    pub fn unknown(&self) -> bool {
        use Error::*;
        matches!(self, Serenity(_) | Dialoguer(_))
    }
}
