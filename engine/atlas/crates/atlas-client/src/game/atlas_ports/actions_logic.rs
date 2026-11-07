//! Deterministic behavior of the supplied client utility mods.
//! Rust adaptation of StreamerMode (CC0-1.0, supplied license), SortMod
//! (MIT, Developer), MiddleClickPearl (CC0), and ChunkAnimator (MIT, Donyka).
//! See THIRD_PARTY_NOTICES.md for the supplied versions and notices.
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub(super) struct PasswordOptions {
    pub enabled: bool,
    pub mask: String,
    pub preserve_spaces: bool,
    pub protected_commands: Vec<String>,
}
impl Default for PasswordOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            mask: "*".into(),
            preserve_spaces: true,
            protected_commands: [
                "/login",
                "/l",
                "/reg",
                "/register",
                "/changepassword",
                "/cp",
                "/authme:login",
                "/authme:register",
                "/authme:reg",
                "/authme:l",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        }
    }
}
// Character.isWhitespace differs from Unicode White_Space (notably NBSP).
fn java_space(c: char) -> bool {
    matches!(c, '\t'..='\r' | '\u{001c}'..='\u{0020}' | '\u{1680}' | '\u{2000}'..='\u{2006}' | '\u{2008}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{205f}' | '\u{3000}')
}
pub(super) fn password_prefix(input: &str, options: &PasswordOptions) -> Option<usize> {
    if !options.enabled {
        return None;
    }
    let start = input.char_indices().find(|(_, c)| !java_space(*c))?.0;
    if !input[start..].starts_with('/') {
        return None;
    }
    let end = input[start..]
        .char_indices()
        .find(|(_, c)| java_space(*c))
        .map(|(i, _)| start + i)?;
    let command = &input[start..end];
    options
        .protected_commands
        .iter()
        .any(|p| p.eq_ignore_ascii_case(command))
        .then_some(end)
}
pub(super) fn mask_password(input: &str, options: &PasswordOptions) -> String {
    let Some(end) = password_prefix(input, options) else {
        return input.into();
    };
    let first = options
        .mask
        .encode_utf16()
        .next()
        .unwrap_or(u16::from(b'*'));
    let mut result: Vec<u16> = input[..end].encode_utf16().collect();
    for c in input[end..].chars() {
        if options.preserve_spaces && java_space(c) {
            result.extend(c.encode_utf16(&mut [0; 2]).iter().copied());
        } else {
            result.extend(std::iter::repeat_n(first, c.len_utf16()));
        }
    }
    String::from_utf16_lossy(&result)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct SortStack {
    pub item: String,
    pub name: String,
    pub identity: String,
    pub count: i32,
    pub max: i32,
}
pub(super) fn chest_sort_plan(input: &[Option<SortStack>]) -> VecDeque<usize> {
    let mut packed: Vec<SortStack> = Vec::new();
    for stack in input.iter().flatten() {
        let mut rest = stack.count;
        for target in &mut packed {
            if target.identity == stack.identity && target.count < target.max {
                let n = rest.min(target.max - target.count);
                target.count += n;
                rest -= n;
                if rest == 0 {
                    break;
                }
            }
        }
        if rest > 0 {
            let mut stack = stack.clone();
            stack.count = rest;
            packed.push(stack);
        }
    }
    packed.sort_by(|a, b| {
        b.item
            .cmp(&a.item)
            .then_with(|| b.name.cmp(&a.name))
            .then_with(|| b.count.cmp(&a.count))
    });
    let desired: Vec<_> = packed
        .into_iter()
        .map(Some)
        .chain(std::iter::repeat(None))
        .take(input.len())
        .collect();
    let mut current = input.to_vec();
    let mut plan = VecDeque::new();
    for (target, wanted) in desired.iter().enumerate() {
        let same = |a: &Option<SortStack>, b: &Option<SortStack>| match (a, b) {
            (None, None) => true,
            (Some(a), Some(b)) => a.identity == b.identity && a.count == b.count,
            _ => false,
        };
        if same(&current[target], wanted) {
            continue;
        }
        if let Some(source) = ((target + 1)..current.len()).find(|i| same(&current[*i], wanted)) {
            plan.extend([source, target, source]);
            current.swap(source, target);
        }
    }
    plan
}

pub(super) fn drop_inventory_slots() -> impl Iterator<Item = usize> {
    (9..36).chain(0..9).chain([504])
}
pub(super) fn pearl_slot(items: &[Option<String>]) -> Option<usize> {
    items
        .iter()
        .take(9)
        .position(|i| i.as_deref() == Some("minecraft:ender_pearl"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn auth_password_never_masks_command_or_normal_chat() {
        let o = PasswordOptions::default();
        assert_eq!(
            mask_password("  /LoGiN hello мир", &o),
            "  /LoGiN ***** ***"
        );
        assert_eq!(mask_password("/login 😀", &o), "/login **");
        assert_eq!(mask_password("/login", &o), "/login");
        assert_eq!(mask_password("/loginx secret", &o), "/loginx secret");
        assert_eq!(
            mask_password("hello /login secret", &o),
            "hello /login secret"
        );
        assert!(password_prefix("/authme:reg a b", &o).is_some());
        assert!(password_prefix("/login\u{a0}secret", &o).is_none());
    }
    #[test]
    fn sort_uses_exact_original_swap_queue_and_keeps_components() {
        let stack = |item: &str, count| {
            Some(SortStack {
                item: item.into(),
                name: item.into(),
                identity: item.into(),
                count,
                max: 64,
            })
        };
        let input = [stack("apple", 64), stack("stone", 64), None];
        assert_eq!(
            chest_sort_plan(&input).into_iter().collect::<Vec<_>>(),
            vec![1, 0, 1]
        );
        // The supplied Java planner cannot realize a merged count absent in a source stack.
        assert!(chest_sort_plan(&[stack("stone", 16), stack("stone", 16)]).is_empty());
    }
    #[test]
    fn full_inventory_drop_skips_armor_and_includes_offhand() {
        let s: Vec<_> = drop_inventory_slots().collect();
        assert_eq!(s.len(), 37);
        assert_eq!(s[0], 9);
        assert_eq!(s[27], 0);
        assert_eq!(s[36], 504);
    }
}
