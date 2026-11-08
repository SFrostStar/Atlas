//! Atlas-owned page content; the options adapter translates actions into engine widgets.
#[derive(Clone, Copy)]
pub enum Page {
    Graphics,
    AdvancedGraphics,
    ShaderPacks,
    Mods,
    Agent,
}
#[derive(Clone, Copy)]
pub enum Toggle {
    NoAi,
    DiscordPresence,
    Fullbright,
    DynamicLights,
}
#[derive(Clone, Copy)]
pub enum Entry {
    Page(&'static str, Page),
    Toggle(Toggle),
}
pub fn rows(no_ai: bool) -> Vec<(Entry, Option<Entry>)> {
    use Entry::{Page as Open, Toggle as Switch};
    let mut rows = vec![
        (
            Open("options.video", Page::Graphics),
            Some(Open("atlas.pages.shader_packs", Page::ShaderPacks)),
        ),
        (Switch(Toggle::DiscordPresence), Some(Switch(Toggle::NoAi))),
    ];
    if !no_ai {
        rows.push((Open("atlas.pages.agent", Page::Agent), None));
    }
    rows
}
pub const LIGHTING_TITLE: &str = "Lighting";
pub const LIGHTING: [Toggle; 2] = [Toggle::Fullbright, Toggle::DynamicLights];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn no_ai_keeps_its_position_and_hides_agent_navigation() {
        for disabled in [false, true] {
            let menu = rows(disabled);
            assert!(matches!(menu[0].1, Some(Entry::Page(_, Page::ShaderPacks))));
            assert!(matches!(menu[1].1, Some(Entry::Toggle(Toggle::NoAi))));
            if disabled {
                assert_eq!(menu.len(), 2);
            } else {
                assert!(matches!(menu[2], (Entry::Page(_, Page::Agent), None)));
            }
        }
    }
}
