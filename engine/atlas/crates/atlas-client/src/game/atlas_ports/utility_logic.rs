//! Original policies for the small Atlas client ports; no Java mod code is used.

/// Requests the ordinary sprint key; vanilla still decides whether sprinting is possible.
pub(super) fn auto_sprint(playing: bool, forward: bool, backward: bool, sneaking: bool) -> bool {
    playing && forward && !backward && !sneaking
}

fn nickname_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

/// A complete nickname, ignoring case. Avoids matching Alex inside Alexander.
pub(super) fn mentions(message: &str, nickname: &str) -> bool {
    if nickname.is_empty() || message.len() > 16_384 {
        return false;
    }
    let message = message.to_lowercase();
    let nickname = nickname.to_lowercase();
    let text = message.trim_start();
    // Vanilla's decorated self-chat contains the nickname in the sender prefix.
    // A common server `Name: message` prefix is handled as well.
    if text.starts_with(&format!("<{nickname}>")) || text.starts_with(&format!("{nickname}:")) {
        return false;
    }
    message.match_indices(&nickname).any(|(start, matched)| {
        let before = message[..start].chars().next_back();
        let after = message[start + matched.len()..].chars().next();
        !before.is_some_and(nickname_char) && !after.is_some_and(nickname_char)
    })
}

/// Forty normal client ticks avoid a pile of overlapping sounds from chat bursts.
#[derive(Default)]
pub(super) struct MentionLimiter {
    last: Option<u64>,
}

impl MentionLimiter {
    pub(super) fn take(&mut self, message: &str, nickname: &str, tick: u64) -> bool {
        if !mentions(message, nickname) {
            return false;
        }
        let ready = self
            .last
            .is_none_or(|last| tick < last || tick - last >= 40);
        if ready {
            self.last = Some(tick);
        }
        ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprint_is_only_requested_for_unobstructed_forward_input() {
        assert!(auto_sprint(true, true, false, false));
        assert!(!auto_sprint(false, true, false, false));
        assert!(!auto_sprint(true, false, false, false));
        assert!(!auto_sprint(true, true, true, false));
        assert!(!auto_sprint(true, true, false, true));
    }

    #[test]
    fn mentions_match_whole_nicknames_and_ignore_self_prefixes() {
        assert!(mentions("<Other> hey, @SFrostStar!", "SFrostStar"));
        assert!(mentions("Привет, sfroststar!", "SFrostStar"));
        assert!(mentions("Привет, Игрок!", "игрок"));
        assert!(!mentions("SFrostStars", "SFrostStar"));
        assert!(!mentions("not_SFrostStar", "SFrostStar"));
        assert!(!mentions("<SFrostStar> Hello", "SFrostStar"));
        assert!(!mentions("SFrostStar: Hello", "SFrostStar"));
        assert!(!mentions("anything", ""));
    }

    #[test]
    fn chat_bursts_sound_once_and_clock_reset_does_not_mute_the_next_world() {
        let mut limiter = MentionLimiter::default();
        assert!(limiter.take("hey Alex", "Alex", 100));
        assert!(!limiter.take("Alex again", "Alex", 101));
        assert!(!limiter.take("Alex again", "Alex", 139));
        assert!(limiter.take("Alex again", "Alex", 140));
        assert!(!limiter.take("unrelated", "Alex", 180));
        assert!(limiter.take("Alex", "Alex", 180));
        assert!(limiter.take("Alex", "Alex", 1));
    }
}
