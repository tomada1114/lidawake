//! `pmset` からスリープ可否を読み取り、管理者権限ヘルパー経由で切り替える。

use crate::command;

const PMSET: &str = "/usr/bin/pmset";
const SUDO: &str = "/usr/bin/sudo";
const HELPER: &str = "/usr/local/libexec/lidawake-helper";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepPermission {
    /// `SleepDisabled 1`。蓋を閉じてもスリープしない。
    Prohibited,
    /// `SleepDisabled 0`。通常どおりスリープする。
    Allowed,
    Unknown,
}

/// `pmset -g` の出力から `SleepDisabled 0|1` の行を探す。
///
/// 行全体が「SleepDisabled」と「0」か「1」の2語だけで構成される場合に限り一致とみなす。
pub fn parse_sleep_permission(output: &str) -> SleepPermission {
    for line in output.lines() {
        let words: Vec<&str> = line.split_whitespace().collect();
        match words.as_slice() {
            ["SleepDisabled", "1"] => return SleepPermission::Prohibited,
            ["SleepDisabled", "0"] => return SleepPermission::Allowed,
            _ => {}
        }
    }
    SleepPermission::Unknown
}

pub fn current_permission() -> SleepPermission {
    match command::run(PMSET, &["-g"]) {
        Ok(output) if output.success => parse_sleep_permission(&output.stdout),
        _ => SleepPermission::Unknown,
    }
}

pub fn set_sleep_prohibited(prohibited: bool) -> Result<(), String> {
    let desired = if prohibited { "1" } else { "0" };
    let output = command::run(SUDO, &["-n", HELPER, desired]).map_err(|error| error.to_string())?;

    if !output.success {
        return Err(command::failure_message(
            &output,
            "スリープ設定を変更できませんでした。",
        ));
    }

    let expected = if prohibited {
        SleepPermission::Prohibited
    } else {
        SleepPermission::Allowed
    };
    if current_permission() != expected {
        return Err("設定変更後の状態を確認できませんでした。".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleep_disabled_1_is_prohibited() {
        let output =
            "System-wide power settings:\n SleepDisabled\t\t1\nCurrently in use:\n sleep\t\t0\n";
        assert_eq!(parse_sleep_permission(output), SleepPermission::Prohibited);
    }

    #[test]
    fn sleep_disabled_0_is_allowed() {
        let output = "System-wide power settings:\n SleepDisabled  0\n";
        assert_eq!(parse_sleep_permission(output), SleepPermission::Allowed);
    }

    #[test]
    fn missing_setting_is_unknown() {
        assert_eq!(
            parse_sleep_permission("Currently in use:\n sleep 0"),
            SleepPermission::Unknown
        );
    }

    #[test]
    fn sleep_0_is_not_mistaken_for_sleep_disabled() {
        let output = "Currently in use:\n sleep 0\n displaysleep 0\n";
        assert_eq!(parse_sleep_permission(output), SleepPermission::Unknown);
    }
}
