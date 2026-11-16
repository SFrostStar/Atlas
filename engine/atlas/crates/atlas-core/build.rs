use std::{fs, path::Path};

fn main() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../package.json");
    match fs::read(&manifest) {
        Ok(bytes) => {
            println!("cargo:rerun-if-changed={}", manifest.display());
            let manifest: serde_json::Value =
                serde_json::from_slice(&bytes).expect("valid Atlas package.json");
            if manifest["name"] == "atlas-launcher" {
                let version = manifest["version"].as_str().expect("Atlas package version");
                println!("cargo:rustc-env=ATLAS_PACKAGE_VERSION={version}");
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!("reading Atlas package.json: {error}"),
    }
}
