import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location("privacy", ROOT / "scripts/check_repository_privacy.py")
privacy = importlib.util.module_from_spec(spec)
spec.loader.exec_module(privacy)


class RepositoryPrivacyTests(unittest.TestCase):
    def test_player_and_secret_files_are_blocked_even_when_force_added(self):
        for path in ["data/instances/atlas/saves/World/level.dat", "nested/saves/a/region/r.0.0.mca",
                     "data/accounts.json", "accounts.json", ".env", ".env.production",
                     "work/session.json", "nested/atlas-player.txt", "runtime/atlas-client.exe",
                     "assets/game-icon.png", "tools/cargo/bin/cargo.exe", "nested/credentials.json"]:
            with self.subTest(path=path):
                self.assertTrue(privacy.prohibited(path))

    def test_original_source_and_atlas_artwork_are_allowed(self):
        for path in ["engine/atlas/tools/check_public_tree.py", "engine/atlas/crates/atlas-world/src/level/data.rs",
                     "assets/icon.png", "assets/atlas-landscape.png", "lib/account-manager.cjs",
                     "engine/atlas/LICENSE", ".env.example"]:
            with self.subTest(path=path):
                self.assertFalse(privacy.prohibited(path))


if __name__ == "__main__":
    unittest.main()
