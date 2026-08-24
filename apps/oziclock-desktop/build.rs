#[cfg(target_os = "windows")]
use std::{env, fs, path::PathBuf, process::Command};

fn main() {
    slint_build::compile("ui/main.slint").expect("failed to compile Slint UI");
    embed_windows_icon();
}

#[cfg(target_os = "windows")]
fn embed_windows_icon() {
    println!("cargo:rerun-if-changed=app_icon.rc");
    println!("cargo:rerun-if-changed=../../legacy/dotnet-wpf/Ozi.Clock/Assets/clock.ico");

    let Some(resource_compiler) = find_resource_compiler() else {
        println!(
            "cargo:warning=Windows resource compiler was not found; executable icon was not embedded"
        );
        return;
    };
    let output = PathBuf::from(env::var("OUT_DIR").expect("missing OUT_DIR")).join("app_icon.res");
    let manifest_directory = env::var("CARGO_MANIFEST_DIR").expect("missing CARGO_MANIFEST_DIR");
    let status = Command::new(resource_compiler)
        .current_dir(manifest_directory)
        .args(["/nologo", "/fo"])
        .arg(&output)
        .arg("app_icon.rc")
        .status()
        .expect("failed to invoke Windows resource compiler");
    assert!(status.success(), "Windows resource compiler failed");
    println!("cargo:rustc-link-arg={}", output.display());
}

#[cfg(target_os = "windows")]
fn find_resource_compiler() -> Option<PathBuf> {
    if let Some(path) = env::var_os("RC") {
        return Some(path.into());
    }
    let root = PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10\bin");
    let mut versions: Vec<PathBuf> = fs::read_dir(root)
        .ok()?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect();
    versions.sort();
    versions.reverse();
    versions
        .into_iter()
        .map(|version| version.join("x64").join("rc.exe"))
        .find(|path| path.is_file())
}

#[cfg(not(target_os = "windows"))]
fn embed_windows_icon() {}
