//! 夜間の自動ミュート区間を判定する。
//!
//! 区間は毎晩21:00〜翌6:00（日をまたぐ）。開始時刻は含み、終了時刻は含まない（[21:00, 6:00)）。

use chrono::{Local, Timelike};

pub const START_HOUR: u32 = 21;
pub const END_HOUR: u32 = 6;

/// 時・分から、その時刻が自動ミュート対象かを判定する。
pub fn should_mute(hour: u32, minute: u32) -> bool {
    // 日中の区間 [6:00, 21:00) に入っていなければミュート対象。
    let minute_of_day = hour * 60 + minute;
    !(END_HOUR * 60..START_HOUR * 60).contains(&minute_of_day)
}

/// 現在のローカル時刻が自動ミュート対象かを判定する。
pub fn should_mute_now() -> bool {
    let now = Local::now();
    should_mute(now.hour(), now.minute())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boundaries_and_inside() {
        let cases = [
            (20, 59, false), // 開始直前
            (21, 0, true),   // 開始ちょうど
            (21, 1, true),   // 開始直後
            (23, 59, true),  // 日付またぎ直前
            (0, 0, true),    // 深夜0時
            (3, 0, true),    // 区間の内側
            (5, 59, true),   // 終了直前
            (6, 0, false),   // 終了ちょうど
            (6, 1, false),   // 終了直後
            (12, 0, false),  // 昼間
        ];
        for (hour, minute, expected) in cases {
            assert_eq!(should_mute(hour, minute), expected, "{hour:02}:{minute:02}");
        }
    }
}
