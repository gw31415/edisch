use crate::error::{Error, Result};
use std::{
    borrow::Cow::Borrowed,
    env::{self, temp_dir},
    fmt::Display,
    fs::File,
    io::{Read, Write},
    process::Command,
};

/// テキストエディタを起動し、指定された内容を編集する
fn edit(contents: &impl Display) -> Result<String> {
    // 一時ファイルを作成し、パスとファイルハンドルを返す
    let tempfile = {
        let mut path = temp_dir();
        path.push("channels.edisch");
        let mut file = File::create(&path)?;
        writeln!(file, "{contents}")?;
        path
    };

    // コマンドの実行
    let status = Command::new(env::var("EDITOR").unwrap_or("vi".to_string()))
        .arg(&tempfile)
        .status()?;
    if !status.success() {
        return Err(Error::Command(status.code()));
    }

    // 編集結果の取得
    let contents = {
        let mut contents = String::new();
        File::open(tempfile)?.read_to_string(&mut contents)?;
        contents
    };
    Ok(contents)
}

/// 一括変更することができるアイテム
pub trait TextEditableItem {
    /// テキスト部分の抽出
    fn content(&self) -> String;
    /// テキストを適用する
    async fn apply(&mut self, content: String) -> Result<()>;
    /// コメント
    fn comment(&self) -> String {
        String::new()
    }
    /// バリデーション
    fn validate(&self, _new: &str) -> Result<()> {
        Ok(())
    }
}

/// 変更を表す
pub struct Diff<T: TextEditableItem> {
    /// 変更前のテキスト
    pub old: String,
    /// 変更後のテキスト
    pub new: String,
    /// 変更前のアイテム
    pub item: T,
}

impl<T: TextEditableItem> Diff<T> {
    pub async fn apply(self) -> Result<()> {
        let Diff { new, mut item, .. } = self;
        item.apply(new).await
    }
}

pub struct Editor<T> {
    items: Vec<T>,
    lines: Vec<String>,
}

impl<T: TextEditableItem> Editor<T> {
    pub fn new(items: impl ExactSizeIterator<Item = T> + Clone) -> Result<Self> {
        let items = items.into_iter();
        let len = items.len();
        let mut lines = Vec::new();
        for item in items.clone() {
            let mut line = item.content();
            if item.content().contains('\t') {
                return Err(Error::NotEditableItem(Borrowed(
                    "tab character is not allowed in content",
                )));
            }
            if !item.comment().is_empty() {
                line.push_str(&format!("\t{}", item.comment()));
            }
            if line.contains('\n') {
                return Err(Error::NotEditableItem(Borrowed(
                    "newline character is not allowed in content",
                )));
            }
            lines.push(line);
        }
        if len != lines.len() {
            return Err(Error::NotEditableItem(Borrowed("item count mismatch")));
        }
        Ok(Self {
            items: items.collect(),
            lines,
        })
    }
    pub fn edit(&mut self) -> Result<()> {
        let mut text = edit(self)?;
        // 最後の文字が改行の場合削除
        if text.ends_with('\n') {
            text.pop();
        }
        if self.items.len() != text.lines().count() {
            return Err(Error::InvalidEditResult(Borrowed("item count mismatch")));
        }
        self.lines = text.lines().map(str::to_string).collect();
        Ok(())
    }
}

impl<T: TextEditableItem> TryInto<Vec<Diff<T>>> for Editor<T> {
    type Error = Error;
    fn try_into(self) -> Result<Vec<Diff<T>>> {
        let mut diffs = Vec::new();
        for (item, line) in self.items.into_iter().zip(self.lines.into_iter()) {
            let new = if let Some(pos) = line.find('\t') {
                line[..pos].to_string()
            } else {
                line.to_string()
            };
            item.validate(&new)?;
            let old = item.content();
            if old != new {
                diffs.push(Diff { old, new, item });
            }
        }
        Ok(diffs)
    }
}

impl<T: TextEditableItem> Display for Editor<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut count = self.lines.len();
        for line in &self.lines {
            count -= 1;
            if count > 0 {
                writeln!(f, "{}", line)?;
            } else {
                write!(f, "{}", line)?;
            }
        }
        Ok(())
    }
}
