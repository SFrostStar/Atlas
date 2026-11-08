use crate::{
    hud::{HudAtlas, HudBatch, rgba},
    lang,
    text::Text,
};
pub struct Button {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub label: String,
    pub enabled: bool,
    pub id: u8,
    pub icon: Option<(&'static str, f32)>,
    pub trailing: Option<(&'static str, f32)>,
    pub action: Option<crate::widgets::Action>,
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
        trailing: None,
        action: None,
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
    let config = super::mod_menu::config::current();
    if config.modify_title_screen {
        use super::mod_menu::config::TitleStyle;
        let label = super::mod_menu::button_text();
        match config.mods_button_style {
            TitleStyle::Icon => {
                let row_y = buttons.iter().find(|b| b.id == 5).map_or(y + 72.0, |b| b.y);
                for button in buttons.iter_mut().filter(|b| matches!(b.id, 5 | 6 | 7)) { button.x -= 12.0; }
                let mut mods = b(cx + 26.0, row_y, 20.0, &label, true, 19);
                mods.icon = Some(("modmenu/mods/enabled",20.0));
                buttons.push(mods);
            }
            TitleStyle::ReplaceRealms if buttons.iter().any(|b| b.id == 0) => {
                if let Some(button) = buttons.iter_mut().find(|b| b.id == 0) { button.label=label;button.id=19;button.enabled=true; }
            }
            TitleStyle::Shrink if buttons.iter().any(|b| b.id == 0) => {
                let button = buttons.iter_mut().find(|b|b.id==0).unwrap();button.w=98.0;
                let row_y=button.y;buttons.push(b(cx+2.0,row_y,98.0,&label,true,19));
            }
            _ => {
                let row_y=buttons.iter().find(|b|b.id==5).map_or(y+72.0,|b|b.y);
                for button in &mut buttons { if button.y>=row_y { button.y+=24.0; } }
                buttons.push(b(cx-100.0,row_y,200.0,&label,true,19));
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
                &format!("gui/sprites/{}", if icon == "modmenu/mods/enabled" && hover { "modmenu/mods/focused" } else { icon }),
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
        &format!("{}{}", super::version_label(), if super::mod_menu::config::current().mod_count_location.title() { super::mod_menu::count_text() } else { String::new() }),
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

impl Button {
    pub fn does(mut self, action: crate::widgets::Action) -> Self {
        self.action = Some(action);
        self
    }
    /// `AbstractButton.extractDefaultLabel`: the label centred with a 2 px
    /// margin, and slid inside the button when it does not fit
    /// (`AbstractWidget.extractScrollingStringOverContents`), instead of being
    /// drawn past it over the buttons beside it (so_so's 590 px World Type
    /// label on Create World, hi_in's Join Server in the Multiplayer footer).
    /// A button with a `trailing` icon is `SpriteIconButton.TextAndIcon`, whose
    /// label keeps clear of the icon: see [`Button::draw_label_at`].
    pub fn draw_label(&self, batch: &mut HudBatch, atlas: &HudAtlas, color: u32) {
        self.draw_label_at(batch, atlas, color, crate::hud_overlays::now_ms());
    }

    /// [`Button::draw_label`] at `now_ms`. `SpriteIconButton.TextAndIcon`
    /// (`extractContents`) hands the label to `acceptScrolling` with `left`
    /// at the button's x plus 2, `right` at its x plus width minus the
    /// sprite's width minus 4, and `centerX` at its x plus half its width: the
    /// label slides inside that box when wider, and one that fits is moved
    /// left until it ends where the icon's room begins. A button without an
    /// icon has `right` 2 short of its edge, where `centerX` is the middle of
    /// the box and nothing moves.
    pub fn draw_label_at(&self, batch: &mut HudBatch, atlas: &HudAtlas, color: u32, now_ms: i64) {
        let right = match self.trailing {
            Some((_, size)) => self.x + self.w - size - 4.0,
            None => self.x + self.w - 2.0,
        };
        let room = if self.action.is_some() && self.enabled {
            crate::widgets::action_glyph_room()
        } else {
            0.0
        };
        crate::ui::scrolling_text_around_at(
            batch,
            atlas,
            &Text::literal(&self.label),
            self.x + (self.w / 2.0).floor(),
            self.x + 2.0 + room,
            right,
            self.y,
            self.y + 20.0,
            self.y + 6.0,
            now_ms,
            color,
        );
    }
}
