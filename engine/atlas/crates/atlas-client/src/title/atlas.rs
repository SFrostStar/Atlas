use super::*;
impl Title {
    pub(super) fn draw_atlas_main(
        &self,
        atlas: &HudAtlas,
        batch: &mut HudBatch,
        cursor: (f32, f32),
    ) {
        let buttons = self.buttons(batch.width, batch.height);
        if crate::widgets::narrating()
            && let Some(info) = self.narration()
        {
            crate::widgets::begin_screen(|| info);
        }
        crate::atlas::menu::draw(atlas, batch, cursor, &buttons, self.focused_button);
    }
}
