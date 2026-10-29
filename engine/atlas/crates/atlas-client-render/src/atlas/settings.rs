//! Atlas-owned page content; the options adapter translates actions into engine widgets.
#[derive(Clone, Copy)]
pub enum Page {
    Graphics,
    AdvancedGraphics,
    ShaderPacks,
    Extras,
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
    vec![
        (
            Open("Graphics", Page::Graphics),
            Some(Open("Advanced Graphics", Page::AdvancedGraphics)),
        ),
        (
            Open("Shader Packs", Page::ShaderPacks),
            Some(Open("Extras", Page::Extras)),
        ),
        (Open("Mods", Page::Mods), Some(Switch(Toggle::NoAi))),
        (
            Switch(Toggle::DiscordPresence),
            (!no_ai).then_some(Open("AI Agent", Page::Agent)),
        ),
    ]
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
            assert!(matches!(
                menu[2],
                (
                    Entry::Page(_, Page::Mods),
                    Some(Entry::Toggle(Toggle::NoAi))
                )
            ));
            assert_eq!(menu[3].1.is_none(), disabled);
        }
    }
}
