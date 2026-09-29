//! `osascript` で音声出力のミュート状態を読み書きする。

use crate::command;

const OSASCRIPT: &str = "/usr/bin/osascript";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MuteState {
    Muted,
    Unmuted,
    Unknown,
}

/// `output muted of (get volume settings)` の出力を判定する。
pub fn parse_output_muted(output: &str) -> MuteState {
    match output.trim() {
        "true" => MuteState::Muted,
        "false" => MuteState::Unmuted,
        _ => MuteState::Unknown,
    }
}

pub fn current_mute_state() -> MuteState {
    match command::run(OSASCRIPT, &["-e", "output muted of (get volume settings)"]) {
        Ok(output) if output.success => parse_output_muted(&output.stdout),
        _ => MuteState::Unknown,
    }
}

pub fn set_muted(muted: bool) -> Result<(), String> {
    let script = if muted {
        "set volume with output muted"
    } else {
        "set volume without output muted"
    };
    let output = command::run(OSASCRIPT, &["-e", script]).map_err(|error| error.to_string())?;

    if output.success {
        Ok(())
    } else {
        Err(command::failure_message(
            &output,
            "音声のミュート設定を変更できませんでした。",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn true_is_muted() {
        assert_eq!(parse_output_muted("true\n"), MuteState::Muted);
    }

    #[test]
    fn false_is_unmuted() {
        assert_eq!(parse_output_muted("false\n"), MuteState::Unmuted);
    }

    #[test]
    fn surrounding_whitespace_is_trimmed() {
        assert_eq!(parse_output_muted("  true  "), MuteState::Muted);
    }

    #[test]
    fn missing_value_is_unknown() {
        assert_eq!(parse_output_muted("missing value\n"), MuteState::Unknown);
    }

    #[test]
    fn empty_is_unknown() {
        assert_eq!(parse_output_muted(""), MuteState::Unknown);
    }
}
