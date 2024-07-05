use std::env::{self, temp_dir};
use std::fmt::Display;
use std::fs::File;
use std::io::{self, Read, Write};
use std::process::Command;

/// テキストエディタを起動し、指定された内容を編集する
fn edit(contents: impl Display) -> Result<String, io::Error> {
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
        return Err(io::Error::new(io::ErrorKind::Other, "EDITOR failed"));
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
    async fn apply(&mut self, content: String) -> Result<(), io::Error>;
    /// コメント
    fn comment(&self) -> String {
        String::new()
    }
    /// バリデーション
    fn validate(&self, _new: &str) -> Result<(), io::Error> {
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
    pub async fn apply(self) -> Result<(), io::Error> {
        let Diff { new, mut item, .. } = self;
        item.apply(new).await
    }
}

/// テキストエディタで一気に変更する
pub fn bulk_edit<T: TextEditableItem>(
    items: impl ExactSizeIterator<Item = T> + Clone,
) -> Result<Vec<Diff<T>>, io::Error> {
    let len = items.len();
    let text = items
        .clone()
        .map(|item| {
            let mut line = item.content();
            if line.contains('\t') {
                panic!("tab character is not allowed in content");
            }
            if !item.comment().is_empty() {
                line.push_str(&format!("\t{}", item.comment()));
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n");
    if len != text.lines().count() {
        return Err(io::Error::new(io::ErrorKind::Other, "item count mismatch"));
    }
    let text = {
        let mut text = edit(text)?;
        // 最後の文字が改行の場合削除
        if text.ends_with('\n') {
            text.pop();
        }
        if len != text.lines().count() {
            return Err(io::Error::new(io::ErrorKind::Other, "item count mismatch"));
        }
        text
    };
    let mut diffs = Vec::new();
    for (item, line) in items.into_iter().zip(text.lines()) {
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
