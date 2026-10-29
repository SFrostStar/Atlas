use crate::{
    hud::{HudAtlas, HudBatch, rgba},
    lang,
};
pub struct Button {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub label: String,
    pub enabled: bool,
    pub id: u8,
    pub icon: Option<(&'static str, f32)>,
}
pub fn buttons(w: f32, h: f32, microsoft_account: bool) -> Vec<Button> {
    let b = |x: f32, y: f32, w: f32, label: &str, enabled: bool, id: u8| Button {
        x,
        y,
        w,
        label: crate::atlas::labels::label_without_ellipsis(label).into(),
        enabled,
        id,
        icon: None,
    };
    let cx = (w / 2.0).floor();
    let y = (h / 4.0).floor() + 48.0;
    let mut buttons = vec![
        b(
            cx - 100.0,
            y,
            200.0,
            &lang::get_or("menu.singleplayer", "Singleplayer"),
            true,
            1,
        ),
        b(
            cx - 100.0,
            y + 24.0,
            200.0,
            &lang::get_or("menu.multiplayer", "Multiplayer"),
            true,
            4,
        ),
        b(
            cx - 100.0,
            y + 48.0,
            200.0,
            &lang::get_or("menu.online", "Minecraft Realms"),
            false,
            0,
        ),
        b(
            cx - 100.0,
            y + 96.0,
            98.0,
            &lang::get_or("menu.options", "Options..."),
            true,
            3,
        ),
        b(
            cx + 2.0,
            y + 96.0,
            98.0,
            &lang::get_or("menu.quit", "Quit Game"),
            true,
            2,
        ),
    ];
    // CommonButtons' icon buttons; Atlas's Join a Friend (an
    // invite code) sits behind vanilla's Friends button.
    for (i, (key, plain, id, icon)) in [
        ("gui.friends.open", "Friends", 5, "friends/friends"),
        ("options.language", "Language...", 6, "icon/language"),
        (
            "options.accessibility",
            "Accessibility Settings...",
            7,
            "icon/accessibility",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let mut button = b(
            cx - 34.0 + i as f32 * 24.0,
            y + 72.0,
            20.0,
            &lang::get_or(key, plain),
            true,
            id,
        );
        button.icon = Some((icon, 15.0));
        buttons.push(button);
    }
    if !microsoft_account {
        buttons.retain(|button| button.id != 0);
        // Close the removed row rather than leave an empty Realms slot.
        for button in &mut buttons {
            if button.y >= y + 72.0 {
                button.y -= 24.0;
            }
        }
    }
    buttons
}
pub fn draw(atlas: &HudAtlas, batch: &mut HudBatch, cursor: (f32, f32), buttons: &[Button]) {
    crate::atlas_menu::background(batch, atlas);
    let (w, h) = (batch.width, batch.height);
    let cx = (w / 2.0).floor();
    let top = (h / 4.0).floor() + 48.0;
    batch.sprite_scaled(
        atlas,
        "atlas/loading-logo",
        cx - 47.0,
        top - 52.0,
        26.0,
        26.0,
        u32::MAX,
    );
    let scale = batch.scale;
    let mut ui = crate::ui_theme::Ui::new(atlas, batch, cursor);
    let k = scale / ui.density;
    use crate::ui_theme::Face;
    ui.text(
        Face::Medium,
        23.0 * k,
        (cx - 15.0) * k,
        (top - 58.0) * k,
        rgba(237, 242, 244, 255),
        super::NAME,
    );
    for b in buttons {
        crate::widgets::record(&b.label, [b.x, b.y, b.w, 20.0], b.enabled);
        let hover = b.enabled
            && cursor.0 >= b.x
            && cursor.0 < b.x + b.w
            && cursor.1 >= b.y
            && cursor.1 < b.y + 20.0;
        let fill = if !b.enabled {
            rgba(24, 32, 40, 210)
        } else if hover {
            rgba(42, 60, 71, 245)
        } else {
            rgba(23, 37, 48, 225)
        };
        ui.rounded(b.x * k, b.y * k, b.w * k, 20.0 * k, 3.0 * k, fill);
        ui.border(
            b.x * k,
            b.y * k,
            b.w * k,
            20.0 * k,
            3.0 * k,
            rgba(93, 128, 145, 65),
        );
        if let Some((icon, size)) = b.icon {
            ui.batch.sprite_scaled(
                atlas,
                &format!("gui/sprites/{icon}"),
                b.x + (b.w - size) / 2.0,
                b.y + (20.0 - size) / 2.0,
                size,
                size,
                u32::MAX,
            );
        } else {
            let color = if !b.enabled {
                rgba(116, 128, 136, 255)
            } else {
                rgba(225, 234, 239, 255)
            };
            let size = 8.0 * k;
            let tw = ui.measure(Face::Medium, size, &b.label);
            ui.text_line(
                Face::Medium,
                size,
                20.0 * k,
                (b.x + b.w / 2.0) * k - tw / 2.0,
                b.y * k,
                color,
                &b.label,
            );
        }
    }
    ui.text(
        Face::Regular,
        4.7 * k,
        4.0 * k,
        (h - 10.0) * k,
        rgba(133, 153, 165, 255),
        &super::version_label(),
    );
    let notice = super::NOTICE;
    let tw = ui.measure(Face::Regular, 4.7 * k, notice);
    ui.text(
        Face::Regular,
        4.7 * k,
        (w - 4.0) * k - tw,
        (h - 10.0) * k,
        rgba(109, 130, 144, 255),
        notice,
    );
    ui.finish();
}
