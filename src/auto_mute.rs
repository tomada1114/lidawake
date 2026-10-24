//! 自動ミュートの理由（夜間・画面がすべて消えている）を合わせて、ミュートの操作を決める。
//!
//! どれかの理由が新たに有効になったとき、鳴っていればミュートし、アプリがミュートしたと記録する。
//! すべての理由が無効になったとき、アプリがミュートしていた場合に限って解除する。
//! 手動でミュートしていた場合は触らない。

use crate::audio::MuteState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MuteReasons {
    pub quiet_hours: bool,
    pub screens_off: bool,
}

impl MuteReasons {
    fn any(self) -> bool {
        self.quiet_hours || self.screens_off
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MuteAction {
    Mute,
    Unmute,
    Keep,
}

/// 前回と今回の理由、アプリがミュートしたか、現在のミュート状態から操作を決める。
///
/// 前回が `None`（起動直後）なら、すべての理由が無効だったものとして扱う。
pub fn decide(
    previous: Option<MuteReasons>,
    current: MuteReasons,
    did_auto_mute: bool,
    state: MuteState,
) -> MuteAction {
    let previous = previous.unwrap_or_default();
    let newly_active = (current.quiet_hours && !previous.quiet_hours)
        || (current.screens_off && !previous.screens_off);

    if newly_active && state == MuteState::Unmuted {
        MuteAction::Mute
    } else if !current.any() && did_auto_mute {
        MuteAction::Unmute
    } else {
        MuteAction::Keep
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NONE: MuteReasons = MuteReasons {
        quiet_hours: false,
        screens_off: false,
    };
    const NIGHT: MuteReasons = MuteReasons {
        quiet_hours: true,
        screens_off: false,
    };
    const LID: MuteReasons = MuteReasons {
        quiet_hours: false,
        screens_off: true,
    };
    const BOTH: MuteReasons = MuteReasons {
        quiet_hours: true,
        screens_off: true,
    };

    use MuteAction::{Keep, Mute, Unmute};
    use MuteState::{Muted, Unknown, Unmuted};

    #[test]
    fn cases() {
        let cases = [
            // (前回, 今回, アプリがミュートしたか, 現在の状態, 期待)
            (None, NONE, false, Unmuted, Keep),  // 起動直後・昼間
            (None, NIGHT, false, Unmuted, Mute), // 起動直後・夜間
            (None, LID, false, Unmuted, Mute),   // 起動直後・蓋閉じ
            (Some(NONE), NIGHT, false, Unmuted, Mute), // 夜間に入った
            (Some(NONE), LID, false, Unmuted, Mute), // 蓋を閉じた
            (Some(NONE), NIGHT, false, Muted, Keep), // 手動ミュート済みなら触らない
            (Some(NONE), LID, false, Unknown, Keep), // 状態不明なら触らない
            (Some(NIGHT), NONE, true, Muted, Unmute), // 夜間が明けた
            (Some(LID), NONE, true, Muted, Unmute), // 蓋を開けた
            (Some(NIGHT), NONE, false, Muted, Keep), // 手動ミュートは解除しない
            (Some(NIGHT), BOTH, true, Muted, Keep), // 夜間中に蓋閉じ
            (Some(BOTH), LID, true, Muted, Keep), // 蓋閉じ中に夜明け
            (Some(BOTH), NIGHT, true, Muted, Keep), // 夜間中に蓋を開けた
            (Some(LID), BOTH, true, Muted, Keep), // 蓋閉じ中に夜間へ
            (Some(NIGHT), BOTH, true, Unmuted, Mute), // 夜間に手動解除した後で蓋閉じ
            (Some(NIGHT), NIGHT, true, Unmuted, Keep), // 夜間に手動解除したら再ミュートしない
            (Some(NONE), NONE, false, Unmuted, Keep), // 変化なし
        ];
        for (previous, current, did_auto_mute, state, expected) in cases {
            assert_eq!(
                decide(previous, current, did_auto_mute, state),
                expected,
                "{previous:?} -> {current:?}, did_auto_mute={did_auto_mute}, {state:?}"
            );
        }
    }
}
