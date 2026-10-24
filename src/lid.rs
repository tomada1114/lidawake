//! 蓋の開閉と外部ディスプレイの有無から、画面がすべて消えているかを判定する。
//!
//! 蓋の状態は `ioreg` が表示する IOPMrootDomain の `AppleClamshellState` から読む。
//! 外部ディスプレイは CoreGraphics のオンラインディスプレイ一覧に、内蔵以外があるかで判定する。

use crate::command;

const IOREG: &str = "/usr/sbin/ioreg";
const MAX_DISPLAYS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LidState {
    Closed,
    Open,
    Unknown,
}

/// `ioreg -r -k AppleClamshellState -d 1` の出力から `"AppleClamshellState" = Yes|No` の行を探す。
pub fn parse_clamshell_state(output: &str) -> LidState {
    for line in output.lines() {
        let Some((key, value)) = line.trim_start_matches([' ', '\t', '|']).split_once('=') else {
            continue;
        };
        if key.trim() != "\"AppleClamshellState\"" {
            continue;
        }
        return match value.trim() {
            "Yes" => LidState::Closed,
            "No" => LidState::Open,
            _ => LidState::Unknown,
        };
    }
    LidState::Unknown
}

pub fn current_lid_state() -> LidState {
    match command::run(IOREG, &["-r", "-k", "AppleClamshellState", "-d", "1"]) {
        Ok(output) if output.success => parse_clamshell_state(&output.stdout),
        _ => LidState::Unknown,
    }
}

/// 蓋の状態と外部ディスプレイの有無から、画面がすべて消えているかを判定する。
///
/// 判定できないときは `None` を返す。外部ディスプレイが不明でも、蓋が開いていれば画面は点いている。
pub fn all_screens_off(lid: LidState, has_external_display: Option<bool>) -> Option<bool> {
    match (lid, has_external_display) {
        (LidState::Open, _) => Some(false),
        (LidState::Closed, Some(has_external)) => Some(!has_external),
        (LidState::Closed, None) | (LidState::Unknown, _) => None,
    }
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGGetOnlineDisplayList(max_displays: u32, displays: *mut u32, count: *mut u32) -> i32;
    fn CGDisplayIsBuiltin(display: u32) -> u32;
}

/// 内蔵以外のディスプレイがつながっているか。一覧を取得できなければ `None`。
pub fn has_external_display() -> Option<bool> {
    let mut displays = [0u32; MAX_DISPLAYS];
    let mut count = 0u32;
    // SAFETY: 配列の要素数を上限として渡し、書き込まれた件数だけを読む。
    let error =
        unsafe { CGGetOnlineDisplayList(MAX_DISPLAYS as u32, displays.as_mut_ptr(), &mut count) };
    if error != 0 {
        return None;
    }
    let online = &displays[..(count as usize).min(MAX_DISPLAYS)];
    // SAFETY: 一覧で得たディスプレイIDを渡すだけで、ポインタは扱わない。
    Some(
        online
            .iter()
            .any(|&id| unsafe { CGDisplayIsBuiltin(id) } == 0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "+-o IOPMrootDomain  <class IOPMrootDomain, id 0x100000239>\n  |   \"AppleClamshellCausesSleep\" = Yes\n  |   \"AppleClamshellState\" = No\n";

    #[test]
    fn no_is_open() {
        assert_eq!(parse_clamshell_state(SAMPLE), LidState::Open);
    }

    #[test]
    fn yes_is_closed() {
        let output = "  |   \"AppleClamshellState\" = Yes\n";
        assert_eq!(parse_clamshell_state(output), LidState::Closed);
    }

    #[test]
    fn causes_sleep_is_not_mistaken_for_state() {
        let output = "  |   \"AppleClamshellCausesSleep\" = Yes\n";
        assert_eq!(parse_clamshell_state(output), LidState::Unknown);
    }

    #[test]
    fn missing_key_is_unknown() {
        assert_eq!(parse_clamshell_state(""), LidState::Unknown);
    }

    #[test]
    fn unexpected_value_is_unknown() {
        let output = "  |   \"AppleClamshellState\" = 1\n";
        assert_eq!(parse_clamshell_state(output), LidState::Unknown);
    }

    #[test]
    fn screens_off_cases() {
        let cases = [
            (LidState::Open, Some(false), Some(false)),
            (LidState::Open, Some(true), Some(false)),
            (LidState::Open, None, Some(false)),
            (LidState::Closed, Some(false), Some(true)), // 蓋閉じ・外部ディスプレイなし
            (LidState::Closed, Some(true), Some(false)), // クラムシェルモード
            (LidState::Closed, None, None),
            (LidState::Unknown, Some(false), None),
        ];
        for (lid, external, expected) in cases {
            assert_eq!(
                all_screens_off(lid, external),
                expected,
                "{lid:?} {external:?}"
            );
        }
    }
}
