use super::*;
impl Title {
    pub(super) fn draw_atlas_main(
        &self,
        atlas: &HudAtlas,
        batch: &mut HudBatch,
        cursor: (f32, f32),
    ) {
        let buttons = self.buttons(batch.width, batch.height);
        crate::atlas::menu::draw(atlas, batch, cursor, &buttons);
    }
}
