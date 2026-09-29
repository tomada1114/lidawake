//! 外部コマンドを実行して、終了状態と標準出力・標準エラーを受け取る。

use std::io;
use std::process::Command;

pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

pub fn run(program: &str, args: &[&str]) -> io::Result<CommandOutput> {
    let output = Command::new(program).args(args).output()?;

    Ok(CommandOutput {
        success: output.status.success(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

/// 失敗時のメッセージを作る。標準エラーが空なら `fallback` を使う。
pub fn failure_message(output: &CommandOutput, fallback: &str) -> String {
    let detail = output.stderr.trim();
    if detail.is_empty() {
        fallback.to_string()
    } else {
        detail.to_string()
    }
}
