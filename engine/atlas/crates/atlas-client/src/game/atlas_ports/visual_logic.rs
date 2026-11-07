//! Source-derived calculations/state machines. Intentionally independent of the game and serde
//! so boundary fixtures can run without linking the renderer.
use std::collections::{BTreeMap, VecDeque};

pub(super) fn strip_formatting(input: &str, upper: bool) -> String {
    let mut chars = input.chars().peekable();
    let mut out = String::new();
    while let Some(c) = chars.next() {
        if c == '§'
            && chars.peek().is_some_and(|c| {
                "0123456789abcdefklmnor".contains(*c) || (upper && "ABCDEFKLMNOR".contains(*c))
            })
        {
            chars.next();
        } else {
            out.push(c)
        }
    }
    out
}

pub(super) fn direction(dx: f64, dz: f64, yaw: f32) -> &'static str {
    let angle = ((-dx).atan2(dz).to_degrees() - yaw as f64 + 180.0).rem_euclid(360.0) - 180.0;
    if (-22.5..22.5).contains(&angle) {
        "↑"
    } else if (22.5..67.5).contains(&angle) {
        "↗"
    } else if (67.5..112.5).contains(&angle) {
        "→"
    } else if (112.5..157.5).contains(&angle) {
        "↘"
    } else if (-67.5..-22.5).contains(&angle) {
        "↖"
    } else if (-112.5..-67.5).contains(&angle) {
        "←"
    } else if (-157.5..-112.5).contains(&angle) {
        "↙"
    } else {
        "↓"
    }
}
pub(super) fn health_color(hp: i32) -> u32 {
    if hp >= 15 {
        0x55ff55
    } else if hp >= 8 {
        0xffaa00
    } else {
        0xff4444
    }
}
pub(super) fn totem_color(count: u32) -> u32 {
    match count {
        0 | 1 => 0x55ff55,
        2 => 0xffff55,
        3 => 0xffaa00,
        4 => 0xff5555,
        _ => 0xaa0000,
    }
}


#[derive(Default)]
pub(super) struct Pops {
    pub tick: u64,
    pub entries: BTreeMap<u128, (u32, u64)>,
}
impl Pops {
    pub fn pop(&mut self, id: u128) -> u32 {
        let e = self.entries.entry(id).or_default();
        e.0 += 1;
        e.1 = self.tick;
        e.0
    }
    pub fn touch(&mut self, id: u128) {
        if let Some(e) = self.entries.get_mut(&id) {
            e.1 = self.tick
        }
    }
    pub fn expire(&mut self, timeout: u64) {
        self.entries
            .retain(|_, e| self.tick.saturating_sub(e.1) <= timeout)
    }
    pub fn count(&self, id: u128) -> u32 {
        self.entries.get(&id).map_or(0, |e| e.0)
    }
}

pub(super) fn pvp_end(message: &str, custom: &[String]) -> bool {
    let s = strip_formatting(message, false).trim().to_lowercase();
    if !s.is_empty()
        && custom
            .iter()
            .any(|k| !k.is_empty() && s.contains(&k.to_lowercase()))
    {
        return true;
    }
    if [
        "вы вышли из боя",
        "вы вышли из режима боя",
        "вы вышли из пвп",
        "вы вышли из pvp",
        "режим боя окончен",
        "режим боя завершен",
        "режим пвп окончен",
        "бой окончен",
        "бой завершен",
        "покинули режим боя",
        "покинули pvp",
        "покинули пвп",
        "больше не в бою",
        "больше не находитесь в бою",
        "ты выжил",
        "вы выжили",
        "пережили бой",
        "вы успешно вышли",
        "безопасная зона",
        "no longer in combat",
        "left combat",
        "combat ended",
        "pvp ended",
    ]
    .iter()
    .any(|w| s.contains(w))
    {
        return true;
    }
    [
        "выш",
        "покинул",
        "окончен",
        "закончи",
        "завершен",
        "больше не",
        "пережил",
        "спаслись",
        "safe",
        "escaped",
    ]
    .iter()
    .any(|w| s.contains(w))
        && ["боя", "бою", "бой", "пвп", "pvp", "combat"]
            .iter()
            .any(|w| s.contains(w))
}
pub(super) fn pvp_boss(message: &str, custom: &[String]) -> bool {
    let s = strip_formatting(message, false).trim().to_lowercase();
    !s.is_empty()
        && !pvp_end(&s, custom)
        && (custom
            .iter()
            .any(|w| !w.is_empty() && s.contains(&w.to_lowercase()))
            || ["бой", "боя", "пвп", "pvp", "combat", "антирелог", "сражен"]
                .iter()
                .any(|w| s.contains(w)))
}
#[derive(Default)]
pub(super) struct PvpState {
    pub was_in_pvp: bool,
    pub duration_ticks: u32,
    pub display_start: Option<u64>,
    pub last_trigger: u64,
}
impl PvpState {
    pub fn trigger(&mut self, now: u64) -> bool {
        if now.saturating_sub(self.last_trigger) < 3500 {
            return false;
        }
        self.last_trigger = now;
        self.display_start = Some(now);
        true
    }
    pub fn bosses(&mut self, titles: &[String], custom: &[String], now: u64) -> bool {
        if titles.iter().any(|s| pvp_end(s, custom)) {
            self.was_in_pvp = false;
            self.duration_ticks = 0;
            return self.trigger(now);
        }
        if titles.iter().any(|s| pvp_boss(s, custom)) {
            self.was_in_pvp = true;
            self.duration_ticks += 1;
            false
        } else {
            let trigger = self.was_in_pvp && self.duration_ticks >= 4;
            self.was_in_pvp = false;
            self.duration_ticks = 0;
            trigger && self.trigger(now)
        }
    }
    pub fn alpha(&self, now: u64, duration: u64, fade: bool) -> Option<u8> {
        let elapsed = now.checked_sub(self.display_start?)?;
        if elapsed > duration {
            return None;
        }
        Some(if fade && elapsed as i128 > duration as i128 - 400 {
            ((duration - elapsed) as f32 / 400.0 * 255.0) as u8
        } else {
            255
        })
    }
}

#[derive(Default)]
pub(super) struct Kills {
    attacks: BTreeMap<u128, (String, u64)>,
    killed_ids: BTreeMap<u128, u64>,
    killed_names: BTreeMap<String, u64>,
    queue: VecDeque<String>,
    last_sent: u64,
}
impl Kills {
    pub fn attack(&mut self, id: u128, name: &str, now: u64) {
        self.attacks.insert(id, (name.into(), now));
    }
    pub fn death(&mut self, id: u128, name: &str, now: u64, local: &str, enabled: bool) {
        if self
            .attacks
            .get(&id)
            .is_some_and(|(_, t)| now.saturating_sub(*t) <= 6500)
        {
            self.trigger(Some(id), name, now, local, enabled)
        }
    }
    pub fn trigger(&mut self, id: Option<u128>, name: &str, now: u64, local: &str, enabled: bool) {
        if !enabled {
            return;
        }
        let name = sanitize_name(name);
        if name.is_empty() || name.eq_ignore_ascii_case(local) {
            return;
        }
        if let Some(id) = id {
            if self
                .killed_ids
                .get(&id)
                .is_some_and(|t| now.saturating_sub(*t) < 5000)
            {
                return;
            }
            self.killed_ids.insert(id, now);
            self.attacks.remove(&id);
        }
        let key = name.to_lowercase();
        if self
            .killed_names
            .get(&key)
            .is_some_and(|t| now.saturating_sub(*t) < 5000)
        {
            return;
        }
        self.killed_names.insert(key, now);
        self.queue.push_back(format!("!-{name}"));
    }
    pub fn chat(&mut self, message: &str, local: &str, now: u64, enabled: bool) {
        let s = strip_formatting(message, true).trim().to_owned();
        if local.is_empty() || !s.contains(local) {
            return;
        }
        let lower = s.to_lowercase();
        let me = local.to_lowercase();
        // Same three grammars as KillTracker: vanilla English and the two Russian forms.
        for phrase in [
            " was slain by ",
            " was shot by ",
            " was killed by ",
            " was blown up by ",
            " was pummeled by ",
            " was doomed to fall by ",
            " was fireballed by ",
            " был убит ",
            " погиб от руки ",
            " пал от руки ",
            " пал в бою против ",
            " был застрелен ",
            " погиб в бою с ",
        ] {
            if let Some(index) = lower.find(phrase) {
                let remainder = lower[index + phrase.len()..].trim_start();
                let remainder = remainder
                    .strip_prefix("игроком ")
                    .or_else(|| remainder.strip_prefix("игрока "))
                    .unwrap_or(remainder);
                let matches = if phrase.starts_with(" was ") {
                    remainder == me
                        || remainder
                            .strip_prefix(&me)
                            .is_some_and(|s| s.starts_with(" using "))
                } else {
                    remainder == me
                        || remainder
                            .strip_prefix(&me)
                            .is_some_and(|s| s.starts_with(' '))
                };
                if matches {
                    let victim = s[..index]
                        .trim()
                        .strip_prefix("Игрок ")
                        .unwrap_or(s[..index].trim());
                    self.trigger(None, victim, now, local, enabled);
                    break;
                }
            }
        }
    }
    pub fn tick(&mut self, now: u64, connected: bool) -> Option<String> {
        if now % 2000 < 50 {
            self.attacks
                .retain(|_, (_, t)| now.saturating_sub(*t) <= 13000);
            self.killed_ids
                .retain(|_, t| now.saturating_sub(*t) <= 10000);
            self.killed_names
                .retain(|_, t| now.saturating_sub(*t) <= 10000)
        }
        if connected && now.saturating_sub(self.last_sent) >= 400 {
            let message = self.queue.pop_front();
            if message.is_some() {
                self.last_sent = now
            }
            message
        } else {
            None
        }
    }
}
pub(super) fn sanitize_name(name: &str) -> String {
    let s = strip_formatting(name, true);
    let mut s = s.trim();
    if s.starts_with('[') {
        if let Some(i) = s.find(']') {
            s = s[i + 1..].trim_start()
        }
    }
    s.rsplit(' ').next().unwrap_or(s).trim().into()
}

#[derive(Clone, Debug)]
pub(super) struct MenuItem {
    pub id: String,
    pub name: String,
    pub lore: Vec<String>,
    pub count: i32,
}
impl MenuItem {
    fn text(&self) -> String {
        format!("{} {}", self.name, self.lore.join(" ")).to_lowercase()
    }
    pub fn empty(&self) -> bool {
        self.count <= 0 || self.id == "minecraft:air"
    }
}
fn numbers_after<'a>(input: &'a str, prefix: &'a str) -> Option<(i32, &'a str)> {
    all_numbers_after(input, prefix).next()
}
fn all_numbers_after<'a>(input: &'a str, prefix: &'a str) -> impl Iterator<Item = (i32, &'a str)> {
    input.match_indices(prefix).filter_map(move |(index, _)| {
        let rest = input[index + prefix.len()..].trim_start();
        let rest = if prefix == "гриф" {
            rest.strip_prefix('-')
                .or_else(|| rest.strip_prefix('#'))
                .unwrap_or(rest)
                .trim_start()
        } else {
            rest
        };
        let end = rest
            .find(|c: char| !c.is_ascii_digit())
            .unwrap_or(rest.len());
        (end > 0)
            .then(|| rest[..end].parse().ok().map(|n| (n, &rest[end..])))
            .flatten()
    })
}
pub(super) fn current_grief(board: &str) -> i32 {
    let text = board.to_lowercase();
    if (text.contains("хаб") || text.contains("lobby"))
        && !text.contains("гриф-")
        && !text.contains("гриф #")
    {
        return 0;
    }
    numbers_after(&text, "гриф").map_or(-1, |(n, _)| n)
}
pub(super) fn target_slot(items: &[MenuItem], target: i32) -> Option<usize> {
    for (i, item) in items.iter().enumerate() {
        if item.empty() {
            continue;
        }
        // These four materials are excluded by the Java version unless their NAME contains 11.
        if [
            "minecraft:crafter",
            "minecraft:anvil",
            "minecraft:blast_furnace",
            "minecraft:smoker",
        ]
        .contains(&item.id.as_str())
            && !item.name.to_lowercase().contains("11")
        {
            continue;
        }
        let t = item.text();
        let matches = ["гриф", "#"].iter().any(|prefix| {
            all_numbers_after(&t, prefix).any(|(n, tail)| {
                n == target
                    && tail
                        .chars()
                        .next()
                        .is_none_or(|c| !c.is_alphanumeric() && c != '_')
            })
        });
        let reverse = t.match_indices(&target.to_string()).any(|(i, _)| {
            let before = &t[..i];
            let rest = t[i + target.to_string().len()..].trim_start();
            before
                .chars()
                .last()
                .is_none_or(|c| !c.is_alphanumeric() && c != '_')
                && rest.starts_with("гриф")
        });
        if matches
            || reverse
            || (item.count == target
                && ["гриф", "сервер", "онлайн", "игроков"]
                    .iter()
                    .any(|k| t.contains(k)))
        {
            return Some(i);
        }
    }
    None
}
pub(super) fn crafter_slot(items: &[MenuItem]) -> Option<usize> {
    let crafter =
        |i: &MenuItem| ["minecraft:crafter", "minecraft:crafting_table"].contains(&i.id.as_str());
    if items.len() >= 27 && !items[22].empty() {
        let i = &items[22];
        if crafter(i)
            || ["1.21", "верстак", "крафтер", "гриф"]
                .iter()
                .any(|s| i.name.to_lowercase().contains(s))
        {
            return Some(22);
        }
    }
    if let Some(i) = items.iter().position(|i| !i.empty() && crafter(i)) {
        return Some(i);
    }
    if let Some(i) = items.iter().position(|i| {
        let t = i.text();
        !i.empty()
            && ["1.21", "верстак", "крафтер"].iter().any(|s| t.contains(s))
            && ["гриф", "выживание", "сервер"]
                .iter()
                .any(|s| t.contains(s))
            && numbers_after(&t, "гриф").is_none()
    }) {
        return Some(i);
    }
    (items.len() >= 27 && !items[22].empty()).then_some(22)
}
#[derive(Default)]
pub(super) struct Joiner {
    pub phase: JoinPhase,
    pub attempts: u32,
    pub last_action: u64,
    pub last_menu: u64,
    pub previous_grief: i32,
    pub status: String,
}
#[derive(Default, Copy, Clone, PartialEq, Debug)]
pub(super) enum JoinPhase {
    #[default]
    Disabled,
    Checking,
    Opening,
    Waiting,
    Navigating,
    Clicking,
    Connected,
}
impl JoinPhase {
    pub fn description(self) -> &'static str {
        match self {
            Self::Disabled => "Выключен",
            Self::Checking => "Проверка сервера...",
            Self::Opening => "Открытие меню (компас)...",
            Self::Waiting => "Ожидание открытия меню...",
            Self::Navigating => "Переход в меню 1.21...",
            Self::Clicking => "Клик по грифу...",
            Self::Connected => "Успешно зашли на гриф!",
        }
    }
}
#[derive(Debug, PartialEq)]
pub(super) enum JoinAction {
    Rejoin(i32),
    Connected,
    Click(usize),
    Close,
    Open,
}
impl Joiner {
    pub fn toggle(&mut self, enabled: bool, target: i32) {
        self.phase = if enabled {
            self.attempts = 0;
            self.status = format!("Запуск захода на Гриф {target}...");
            JoinPhase::Checking
        } else {
            self.status = "Выключен".into();
            JoinPhase::Disabled
        }
    }
    pub fn tick(
        &mut self,
        now: u64,
        enabled: bool,
        target: i32,
        auto: bool,
        delay: u64,
        board: &str,
        menu: Option<&[MenuItem]>,
    ) -> Option<JoinAction> {
        let grief = current_grief(board);
        if auto && self.previous_grief > 0 && grief == 0 && !enabled {
            return Some(JoinAction::Rejoin(self.previous_grief));
        }
        if grief > 0 {
            self.previous_grief = grief
        }
        if !enabled {
            self.phase = JoinPhase::Disabled;
            return None;
        }
        if grief == target {
            if self.phase != JoinPhase::Connected {
                self.phase = JoinPhase::Connected;
                self.status = format!("Успешно на Гриф {target}!");
                return Some(JoinAction::Connected);
            }
            return None;
        }
        if now.saturating_sub(self.last_action) < delay {
            return None;
        }
        if let Some(items) = menu {
            if let Some(i) = target_slot(items, target) {
                self.phase = JoinPhase::Clicking;
                self.status = format!("Клик по слоту #{i} (Гриф {target})");
                self.attempts += 1;
                self.last_action = now;
                Some(JoinAction::Click(i))
            } else if let Some(i) = crafter_slot(items) {
                self.phase = JoinPhase::Navigating;
                self.status = format!("Клик по верстаку 1.21 (слот #{i})");
                self.last_action = now.saturating_sub(delay / 2);
                Some(JoinAction::Click(i))
            } else if now.saturating_sub(self.last_menu) > 2500 {
                self.last_action = now;
                Some(JoinAction::Close)
            } else {
                None
            }
        } else if now.saturating_sub(self.last_menu) >= delay {
            self.phase = JoinPhase::Waiting;
            self.status = "Открытие меню (компас)...".into();
            self.last_menu = now;
            self.last_action = now;
            Some(JoinAction::Open)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(in crate::game) struct BeamVertex {
    pub position: [f32; 3],
    pub alpha: f32,
}
/// Exactly four intersecting gradient quads, at 0/45/90/135 degrees.
pub(in crate::game) fn beam_vertices(
    width: f32,
    height: f32,
    alpha: f32,
    rotation_degrees: f32,
) -> [BeamVertex; 16] {
    std::array::from_fn(|index| {
        let i = index / 4;
        let vertex = index % 4;
        let a = (f64::from(rotation_degrees) + i as f64 * 45.0).to_radians() as f32;
        let half = width / 2.0;
        let sign = if vertex == 0 || vertex == 3 {
            -1.0
        } else {
            1.0
        };
        let top = vertex >= 2;
        BeamVertex {
            position: [
                half * sign * f64::from(a).cos() as f32,
                if top { height } else { 0.0 },
                half * sign * f64::from(a).sin() as f32,
            ],
            alpha: if top { 0.0 } else { alpha },
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Expected values were obtained by compiling and running the supplied Java methods.
    #[test]
    fn supplied_java_oracle_phrases_and_names() {
        for (message, end, boss) in [
            ("§aВы вышли из режима боя", true, false),
            ("Идёт бой", false, true),
            ("Антирелог 5", false, true),
            ("pvp ended", true, false),
            ("Arena #3", false, false),
            ("", false, false),
        ] {
            assert_eq!(pvp_end(message, &[]), end, "{message}");
            assert_eq!(pvp_boss(message, &[]), boss, "{message}");
        }
        for (input, expected) in [
            ("§cVictim", "Victim"),
            ("[VIP] Player", "Player"),
            ("[VIP] Игрок Bad", "Bad"),
            ("  Bad  ", "Bad"),
        ] {
            assert_eq!(sanitize_name(input), expected);
        }
        assert!(!pvp_end("", &[String::new()]));
        assert!(!pvp_end("ordinary text", &[String::new()]));
        let mut kills = Kills::default();
        kills.chat("Wrong was slain by Me unrelated suffix", "Me", 20000, true);
        assert!(kills.tick(20000, true).is_none());
        kills.chat("Correct was slain by Me using Sword", "Me", 21000, true);
        assert_eq!(kills.tick(21000, true).as_deref(), Some("!-Correct"));
    }
    #[test]
    fn radar_octants_match_java_half_open_boundaries() {
        assert_eq!(direction(0., 10., 0.), "↑");
        assert_eq!(direction(-10., 0., 0.), "→");
        assert_eq!(direction(10., 0., 0.), "←");
        assert_eq!(direction(0., 10., -22.5), "↗");
        assert_eq!(direction(0., 10., 22.5), "↑");
    }
    #[test]
    fn pop_timeout_is_strict_and_hurt_refreshes_it() {
        let mut p = Pops::default();
        assert_eq!(p.pop(7), 1);
        p.tick = 600;
        p.expire(600);
        assert_eq!(p.count(7), 1);
        p.touch(7);
        p.tick = 1200;
        p.expire(600);
        assert_eq!(p.count(7), 1);
        p.tick = 1201;
        p.expire(600);
        assert_eq!(p.count(7), 0);
        assert_eq!(
            (1..=5).map(totem_color).collect::<Vec<_>>(),
            [0x55ff55, 0xffff55, 0xffaa00, 0xff5555, 0xaa0000]
        );
    }
    #[test]
    fn pvp_requires_four_ticks_and_exact_fade_window() {
        let mut p = PvpState::default();
        for _ in 0..3 {
            assert!(!p.bosses(&["PvP 00:04".into()], &[], 10000))
        }
        assert!(!p.bosses(&[], &[], 10050));
        for _ in 0..4 {
            p.bosses(&["Антирелог 5".into()], &[], 11000);
        }
        assert!(p.bosses(&[], &[], 11050));
        assert_eq!(p.alpha(12650, 2000, true), Some(255));
        assert_eq!(p.alpha(12850, 2000, true), Some(127));
        assert_eq!(p.alpha(13050, 2000, true), Some(0));
        assert_eq!(p.alpha(13051, 2000, true), None);
        assert!(!p.trigger(14000));
        assert!(p.trigger(14550));
    }
    #[test]
    fn custom_pvp_keywords_are_end_messages_even_on_bossbars() {
        assert!(pvp_end("Arena #3", &["arena".into()]));
        assert!(!pvp_boss("Arena #3", &["arena".into()]));
        assert!(pvp_end("§aВы вышли из режима боя", &[]));
        assert!(!pvp_end("Идёт бой", &[]));
    }
    #[test]
    fn kill_windows_and_chat_throttling_match_java() {
        let mut k = Kills::default();
        k.attack(1, "Victim", 10000);
        k.death(1, "Victim", 16500, "Me", true);
        assert_eq!(k.tick(16500, true).as_deref(), Some("!-Victim"));
        k.trigger(None, "[VIP] Victim", 17000, "Me", true);
        assert!(k.tick(17000, true).is_none());
        k.trigger(None, "Other", 17100, "Me", true);
        assert_eq!(k.tick(17100, true).as_deref(), Some("!-Other"));
        k.chat("Target was slain by Me using Sword", "Me", 18000, true);
        assert_eq!(k.tick(18000, true).as_deref(), Some("!-Target"));
        k.chat("Игрок Bad был убит игроком Me", "Me", 19000, true);
        assert_eq!(k.tick(19000, true).as_deref(), Some("!-Bad"));
    }
    #[test]
    fn crystal_beam_has_original_four_gradient_quads() {
        let v = beam_vertices(0.25, 60., 0.5, 0.);
        assert_eq!(v[0].position, [-0.125, 0., 0.]);
        assert_eq!(v[1].position, [0.125, 0., 0.]);
        assert_eq!(v[2].position, [0.125, 60., 0.]);
        assert_eq!(v[0].alpha, 0.5);
        assert_eq!(v[2].alpha, 0.);
        assert!((v[4].position[0] + 0.08838835).abs() < 1e-7);
    }
    #[test]
    fn joiner_detects_grief_and_hub_and_uses_menu_lore() {
        assert_eq!(current_grief("ReallyWorld | Гриф-11"), 11);
        assert_eq!(current_grief("Сервер: Хаб"), 0);
        let mut slots = vec![
            MenuItem {
                id: "minecraft:air".into(),
                name: String::new(),
                lore: vec![],
                count: 0
            };
            27
        ];
        slots[22] = MenuItem {
            id: "minecraft:crafting_table".into(),
            name: "Верстак 1.21".into(),
            lore: vec![],
            count: 1,
        };
        assert_eq!(crafter_slot(&slots), Some(22));
        slots[7] = MenuItem {
            id: "minecraft:diamond".into(),
            name: "Сервер".into(),
            lore: vec!["Гриф #11 (онлайн 80)".into()],
            count: 3,
        };
        assert_eq!(target_slot(&slots, 11), Some(7));
        assert_eq!(target_slot(&slots, 1), None);
        let mut j = Joiner::default();
        j.toggle(true, 11);
        assert_eq!(
            j.tick(10000, true, 11, true, 900, "Хаб", None),
            Some(JoinAction::Open)
        );
        assert_eq!(
            j.tick(10899, true, 11, true, 900, "Хаб", Some(&slots)),
            None
        );
        assert_eq!(
            j.tick(10900, true, 11, true, 900, "Хаб", Some(&slots)),
            Some(JoinAction::Click(7))
        );
        assert_eq!(j.attempts, 1);
    }
}
