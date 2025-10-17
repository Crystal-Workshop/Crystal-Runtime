use assert_cmd::prelude::*;
use predicates::str::contains;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

fn build_archive() -> NamedTempFile {
    let scene = r#"<scene>
  <object>
    <name>Cube</name>
    <type>mesh</type>
  </object>
</scene>
"#;
    let script = r#"
local cube = place.get("Cube")
if cube ~= nil then
  cube:set_color({x = 1, y = 0, z = 0})
end
"#;

    let mut tmp = NamedTempFile::new().expect("temp archive");
    let script_bytes = script.as_bytes();
    let scene_bytes = scene.as_bytes();

    let mut buffer = Vec::new();
    buffer.extend_from_slice(b"CGME");
    buffer.extend_from_slice(&1u32.to_le_bytes());
    buffer.extend_from_slice(&0u64.to_le_bytes());

    let header_len = buffer.len() as u64;
    let script_offset = header_len;
    buffer.extend_from_slice(script_bytes);
    let script_size = script_bytes.len() as u64;

    let scene_offset = header_len + script_size;
    buffer.extend_from_slice(scene_bytes);
    let scene_size = scene_bytes.len() as u64;

    let toc_offset = scene_offset + scene_size;
    buffer.extend_from_slice(&1u32.to_le_bytes());
    buffer.extend_from_slice(&("scripts/init.lua".len() as u32).to_le_bytes());
    buffer.extend_from_slice(b"scripts/init.lua");
    buffer.extend_from_slice(&script_offset.to_le_bytes());
    buffer.extend_from_slice(&script_size.to_le_bytes());
    buffer.extend_from_slice(&scene_offset.to_le_bytes());
    buffer.extend_from_slice(&scene_size.to_le_bytes());

    buffer[8..16].copy_from_slice(&toc_offset.to_le_bytes());

    tmp.write_all(&buffer).expect("write archive");
    tmp
}

#[test]
fn cli_runs_scripts_and_prints_final_state() {
    let archive = build_archive();
    let mut cmd = Command::cargo_bin("crystal-runtime").expect("binary exists");
    cmd.arg(archive.path())
        .arg("--run-scripts")
        .arg("--summary-only");
    cmd.assert()
        .success()
        .stdout(contains("Loaded scene with 1 objects (0 lights)"))
        .stdout(contains(" - Cube (mesh)"))
        .stdout(contains("Launched 1 script(s)"))
        .stdout(contains(
            " - Cube pos=(0.00, 0.00, 0.00) color=(1.00, 0.00, 0.00)",
        ));
}
