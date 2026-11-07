//! Inventory Layouts 1.01 planner, MIT, Inventory Layouts contributors.
//! Ported from the supplied LayoutPlanner; see THIRD_PARTY_NOTICES.md.
use std::collections::BTreeMap;
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct Stack {
    pub key: String,
    pub identity: String,
    pub count: u32,
    pub max: u32,
    pub split_safe: bool,
}
impl Stack {
    fn empty(&self) -> bool {
        self.count == 0
    }
    fn with_count(&self, count: u32) -> Self {
        if count == 0 {
            Self::default()
        } else {
            Self {
                count,
                ..self.clone()
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Action {
    Swap,
    Pickup,
}
#[derive(Clone, Debug)]
pub(super) struct Click {
    pub slot: usize,
    pub button: i32,
    pub action: Action,
    pub expected: Vec<Stack>,
    pub cursor: Stack,
}
#[derive(Default, Debug)]
pub(super) struct Plan {
    pub clicks: Vec<Click>,
    pub missing: BTreeMap<String, u32>,
    pub blocked: BTreeMap<String, u32>,
}
struct Planner<T, I> {
    stacks: Vec<Stack>,
    wanted: Vec<Option<String>>,
    take: T,
    insert: I,
    clicks: Vec<Click>,
    cursor: Stack,
}
pub(super) fn plan(
    stacks: Vec<Stack>,
    wanted: Vec<Option<String>>,
    take: impl Fn(usize) -> bool,
    insert: impl Fn(usize, &Stack) -> bool,
) -> Option<Plan> {
    if stacks.len() != 41 || wanted.len() != 41 {
        return None;
    }
    Planner {
        stacks,
        wanted,
        take,
        insert,
        clicks: vec![],
        cursor: Stack::default(),
    }
    .build()
}
impl<T: Fn(usize) -> bool, I: Fn(usize, &Stack) -> bool> Planner<T, I> {
    fn matches(&self, slot: usize, key: Option<&str>) -> bool {
        !self.stacks[slot].empty() && Some(self.stacks[slot].key.as_str()) == key
    }
    fn build(mut self) -> Option<Plan> {
        let mut available = BTreeMap::<String, u32>::new();
        for s in &self.stacks {
            if !s.empty() {
                *available.entry(s.key.clone()).or_default() += s.count;
            }
        }
        let order: Vec<_> = (37..41).chain(0..37).collect();
        for _ in 0..2 {
            for target in &order {
                self.fill(*target);
            }
        }
        for armor in 37..41 {
            if self.wanted[armor].is_none() && !self.stacks[armor].empty() {
                self.evacuate(armor);
            }
        }
        let mut required = BTreeMap::<String, u32>::new();
        let mut fulfilled = BTreeMap::<String, u32>::new();
        for i in 0..41 {
            if let Some(key) = &self.wanted[i] {
                *required.entry(key.clone()).or_default() += 1;
                if self.matches(i, Some(key)) {
                    *fulfilled.entry(key.clone()).or_default() += 1;
                }
            }
        }
        let mut missing = BTreeMap::new();
        let mut blocked = BTreeMap::new();
        for (key, count) in required {
            let remaining = count.saturating_sub(*fulfilled.get(&key).unwrap_or(&0));
            let absent = remaining.min(count.saturating_sub(*available.get(&key).unwrap_or(&0)));
            if absent > 0 {
                missing.insert(key.clone(), absent);
            }
            if remaining > absent {
                blocked.insert(key, remaining - absent);
            }
        }
        self.cursor.empty().then_some(Plan {
            clicks: self.clicks,
            missing,
            blocked,
        })
    }
    fn fill(&mut self, target: usize) {
        let Some(key) = self.wanted[target].clone() else {
            return;
        };
        if self.matches(target, Some(&key)) || !(self.take)(target) {
            return;
        }
        for source in 0..41 {
            if source != target
                && self.matches(source, Some(&key))
                && (self.take)(source)
                && self.wanted[source].as_ref() != Some(&key)
            {
                if self.swap(source, target) {
                    return;
                }
                if source >= 37 && self.evacuate(source) {
                    self.fill(target);
                    return;
                }
            }
        }
        for donor in 0..37 {
            let stack = self.stacks[donor].clone();
            if donor != target
                && self.matches(donor, Some(&key))
                && stack.count >= 2
                && stack.split_safe
                && (self.take)(donor)
                && (self.insert)(target, &stack.with_count(1))
                && (self.stacks[target].empty() || self.evacuate(target))
            {
                self.split_one(donor, target);
                return;
            }
        }
    }
    fn accepts(&self, slot: usize, stack: &Stack) -> bool {
        stack.empty() || (stack.count <= stack.max && (self.insert)(slot, stack))
    }
    fn swap(&mut self, source: usize, target: usize) -> bool {
        if source == target {
            return true;
        }
        let from = &self.stacks[source];
        let to = &self.stacks[target];
        if !(self.take)(source)
            || !(self.take)(target)
            || !self.accepts(target, from)
            || !self.accepts(source, to)
        {
            return false;
        }
        if source < 9 || source == 36 {
            self.swap_click(target, source);
        } else if target >= 9 && target != 36 {
            let first = if source < 37 { source } else { target };
            let second = if first == source { target } else { source };
            if first >= 37 {
                return false;
            }
            self.swap_click(first, 0);
            self.swap_click(second, 0);
            self.swap_click(first, 0);
        } else {
            self.swap_click(source, target);
        }
        true
    }
    fn swap_click(&mut self, slot: usize, hand: usize) {
        self.stacks.swap(slot, hand);
        self.clicks.push(Click {
            slot,
            button: if hand == 36 { 40 } else { hand as i32 },
            action: Action::Swap,
            expected: self.stacks.clone(),
            cursor: self.cursor.clone(),
        });
    }
    fn pickup(&mut self, slot: usize, button: i32) {
        self.clicks.push(Click {
            slot,
            button,
            action: Action::Pickup,
            expected: self.stacks.clone(),
            cursor: self.cursor.clone(),
        });
    }
    fn split_one(&mut self, donor: usize, target: usize) {
        self.cursor = self.stacks[donor].clone();
        self.stacks[donor] = Stack::default();
        self.pickup(donor, 0);
        self.stacks[target] = self.cursor.with_count(1);
        self.cursor = self.cursor.with_count(self.cursor.count - 1);
        self.pickup(target, 1);
        self.stacks[donor] = std::mem::take(&mut self.cursor);
        self.pickup(donor, 0);
    }
    fn move_empty(&mut self, slot: usize) -> bool {
        for spare in (9..37).chain(0..9) {
            if spare != slot && self.stacks[spare].empty() && self.swap(slot, spare) {
                return true;
            }
        }
        false
    }
    fn evacuate(&mut self, slot: usize) -> bool {
        if !(self.take)(slot) {
            return false;
        }
        if self.move_empty(slot) {
            return true;
        }
        for donor in 0..37 {
            let from = self.stacks[donor].clone();
            if donor != slot
                && !from.empty()
                && from.split_safe
                && (self.take)(donor)
                && !self.matches(donor, self.wanted[donor].as_deref())
            {
                for receiver in 0..37 {
                    let to = &self.stacks[receiver];
                    if receiver != slot
                        && receiver != donor
                        && !to.empty()
                        && to.split_safe
                        && from.identity == to.identity
                        && from.count + to.count <= to.max
                        && (self.insert)(receiver, &to.with_count(from.count + to.count))
                    {
                        self.merge(donor, receiver);
                        return self.move_empty(slot);
                    }
                }
            }
        }
        false
    }
    fn merge(&mut self, donor: usize, receiver: usize) {
        self.cursor = std::mem::take(&mut self.stacks[donor]);
        self.pickup(donor, 0);
        self.stacks[receiver].count += self.cursor.count;
        self.cursor = Stack::default();
        self.pickup(receiver, 0);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn stack(key: &str, count: u32) -> Stack {
        Stack {
            key: key.into(),
            identity: key.into(),
            count,
            max: 64,
            split_safe: true,
        }
    }
    #[test]
    fn hotbar_swap_and_offhand_use_vanilla_swap_buttons() {
        let mut slots = vec![Stack::default(); 41];
        slots[9] = stack("stone", 32);
        let mut wanted = vec![None; 41];
        wanted[2] = Some("stone".into());
        let p = plan(slots, wanted, |_| true, |_, _| true).unwrap();
        assert_eq!(
            (p.clicks[0].slot, p.clicks[0].button, p.clicks[0].action),
            (9, 2, Action::Swap)
        );
        assert_eq!(p.clicks.last().unwrap().expected[2].count, 32);
    }
    #[test]
    fn duplicates_split_one_and_leave_cursor_empty() {
        let mut slots = vec![Stack::default(); 41];
        slots[0] = stack("stone", 4);
        let mut wanted = vec![None; 41];
        wanted[0] = Some("stone".into());
        wanted[1] = Some("stone".into());
        let p = plan(slots, wanted, |_| true, |_, _| true).unwrap();
        assert_eq!(p.clicks.len(), 3);
        assert_eq!((p.clicks[1].slot, p.clicks[1].button), (1, 1));
        assert_eq!(p.clicks.last().unwrap().expected[0].count, 3);
        assert_eq!(p.clicks.last().unwrap().expected[1].count, 1);
        assert_eq!(p.clicks.last().unwrap().cursor.count, 0);
    }
    #[test]
    fn missing_and_cursed_slots_are_reported_separately() {
        let mut slots = vec![Stack::default(); 41];
        slots[37] = stack("helmet", 1);
        let mut wanted = vec![None; 41];
        wanted[0] = Some("helmet".into());
        wanted[1] = Some("apple".into());
        let p = plan(slots, wanted, |i| i != 37, |_, _| true).unwrap();
        assert_eq!(p.missing.get("apple"), Some(&1));
        assert_eq!(p.blocked.get("helmet"), Some(&1));
        assert!(p.clicks.is_empty());
    }
}
