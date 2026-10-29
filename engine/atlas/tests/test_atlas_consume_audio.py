"""Exercise the real consumable emitter's sound routing without a window.

Vanilla Player.playSound excludes that player from the server broadcast;
LocalPlayer.playSound plays the predicted bite locally. Both copies must not
reach the eater. This probes the actual adapter method, not a duplicate rule.
"""
from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


def method(source, name):
    start = source.index(f"fn {name}(")
    opening = source.index("{", start)
    depth, end = 1, opening + 1
    while depth:
        depth += (source[end] == "{") - (source[end] == "}")
        end += 1
    return source[start:end]


class ConsumeAudioTests(unittest.TestCase):
    def compile_and_run(self, fixture):
        with tempfile.TemporaryDirectory(prefix="atlas-consume-audio-") as folder:
            folder = Path(folder)
            rust = folder / "consume.rs"
            binary = folder / ("consume.exe" if os.name == "nt" else "consume")
            rust.write_text(fixture, encoding="utf-8")
            built = subprocess.run(["rustc", "--edition=2024", str(rust), "-o", str(binary)],
                                   capture_output=True, text=True, timeout=60)
            self.assertEqual(built.returncode, 0, built.stderr)
            run = subprocess.run([str(binary)], capture_output=True, text=True, timeout=10)
            self.assertEqual(run.returncode, 0, run.stderr)

    @unittest.skipUnless(shutil.which("rustc"), "requires Rust")
    def test_q_menu_drop_predicts_one_swing_only_when_item_is_removed(self):
        source = (ROOT / "crates/atlas-client/src/game/remote.rs").read_text(encoding="utf-8")
        fixture = r'''
#[derive(Clone, Copy, PartialEq)] enum Input { Throw, Swap }
#[derive(Clone)] struct Stack;
type Predicted = (i16, i32, Input, Vec<(i16, Option<Stack>)>, Option<Stack>);
enum Hand { Main }
struct SwingAnimation;
impl SwingAnimation { const DEFAULT: Self = Self; }
#[derive(Default)] struct Menu { count: usize, dropped: Vec<u8>, carried: Option<Stack> }
impl Menu {
    fn click(&mut self, _: &(), _: i32, button: i32, input: Input) -> Result<(), ()> {
        if input == Input::Throw && self.count > 0 {
            let n = if button == 0 { 1 } else { self.count };
            self.count -= n;
            self.dropped.extend(vec![0; n]);
        }
        Ok(())
    }
}
fn menu_items(_: &Menu) -> Vec<Option<Stack>> { vec![] }
fn changed_slots(_: &[Option<Stack>], _: &Menu) -> Vec<(i16, Option<Stack>)> { vec![] }
#[derive(Default)] struct Game { menu: Menu, carried: Option<Stack>, swings: usize }
impl Game {
    fn crafter_slot_clicked(&mut self, _: usize, _: i32, _: Input) {}
    fn remote_menu(&self) -> (i32, String) { (0, "inventory".into()) }
    fn menu_slot_index(_: &str, i: usize) -> Option<usize> { Some(i) }
    fn menu_session(&mut self, f: impl FnOnce(&mut Menu, &(), &()) -> Option<bool>) -> bool {
        f(&mut self.menu, &(), &()).unwrap_or(false)
    }
    fn swing_hand(&mut self, _: Hand, _: SwingAnimation) { self.swings += 1; }
    fn send_clicks(&mut self, _: i32, _: Vec<Predicted>) {}
    METHOD
}
fn main() {
    let mut game = Game::default();
    game.menu.count = 4;
    assert!(game.remote_menu_ops(&[(Some(0), 0, Input::Throw)]));
    assert_eq!(game.menu.count, 3);
    assert_eq!(game.swings, 1, "Q in the open inventory must animate the hand");
    game.remote_menu_ops(&[(Some(0), 1, Input::Throw)]);
    assert_eq!(game.menu.count, 0);
    assert_eq!(game.swings, 2);
    game.remote_menu_ops(&[(Some(0), 0, Input::Throw)]);
    game.remote_menu_ops(&[(Some(0), 0, Input::Swap)]);
    assert_eq!(game.swings, 2, "empty slots and swaps must not swing");
}
'''.replace("METHOD", method(source, "remote_menu_ops"))
        self.compile_and_run(fixture)

    @unittest.skipUnless(shutil.which("rustc"), "requires Rust")
    def test_server_bites_exclude_eater_but_local_bites_still_play(self):
        source = (ROOT / "crates/atlas-client/src/game/consume.rs").read_text(encoding="utf-8")
        fixture = r'''
use std::convert::Infallible;
#[derive(Default)]
struct Audio { shared: usize, excluded: Vec<u32> }
impl Audio {
    fn play(&mut self, _: &str, _: Option<[f64; 3]>, _: f32, _: f32) { self.shared += 1; }
    fn play_except(&mut self, conn: u32, _: &str, _: Option<[f64; 3]>, _: f32, _: f32) {
        self.excluded.push(conn);
    }
}
#[derive(Default)]
struct Game { lan_acting: Option<u32>, audio: Audio }
impl Game { fn feet(&self) -> Option<[f64; 3]> { Some([0.0; 3]) } }
struct Emitter<'a>(&'a mut Game);
impl Emitter<'_> { METHOD }
fn main() {
    let mut client = Game::default();
    Emitter(&mut client).play_sound("minecraft:entity.generic.eat".into(), 1.0, 1.0).unwrap();
    assert_eq!(client.audio.shared, 1);
    assert!(client.audio.excluded.is_empty());
    let mut server = Game { lan_acting: Some(42), ..Default::default() };
    for _ in 0..6 {
        Emitter(&mut server).play_sound("minecraft:entity.generic.eat".into(), 1.0, 1.0).unwrap();
    }
    assert_eq!(server.audio.shared, 0, "predicted bites must not be broadcast to the eater again");
    assert_eq!(server.audio.excluded, vec![42; 6], "observers still hear every bite");
}
'''.replace("METHOD", method(source, "play_sound"))
        self.compile_and_run(fixture)


if __name__ == "__main__":
    unittest.main()
