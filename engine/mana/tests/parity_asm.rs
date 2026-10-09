//! Port check (PLAN step R): every .masm file assembles to what the TypeScript assembler made of it. See engine/parity.

use std::path::Path;

use mana::asm::{assemble, listing};
use mana::load::{repo, resolver};
use serde_json::Value;

#[test]
fn every_file_assembles_as_typescript_did() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/parity/programs.json");
    let Ok(text) = std::fs::read_to_string(&path) else {
        eprintln!("no {}: run `npm run parity` first", path.display());
        return;
    };
    let programs: Vec<Value> = serde_json::from_str(&text).unwrap();
    assert!(programs.len() > 20);
    let mut checked = 0;
    for want in &programs {
        let file = want["path"].as_str().unwrap();
        let got = if let Some(source) = want.get("source") {
            assemble(source.as_str().unwrap(), "test.masm", &resolver(vec![]))
        } else {
            let (dir, name) = file.split_once('/').unwrap();
            let source = std::fs::read_to_string(repo().join(file)).unwrap();
            assemble(&source, name, &resolver(vec![repo().join(dir)]))
        };
        checked += 1;
        if let Some(problems) = want.get("problems") {
            let want: Vec<String> = serde_json::from_value(problems.clone()).unwrap();
            assert_eq!(got.unwrap_err().problems, want, "{file}");
            continue;
        }
        let p = got.unwrap_or_else(|e| panic!("{file}: {e}"));
        let hex: String = p.bytes.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(hex, want["bytes"].as_str().unwrap(), "{file}: bytes");
        let labels: Vec<(String, usize)> = p.labels.iter().map(|(k, v)| (k.clone(), *v)).collect();
        let want_labels: Vec<(String, usize)> = serde_json::from_value(want["labels"].clone()).unwrap();
        assert_eq!(labels, want_labels, "{file}: labels");
        let consts: Vec<(String, f64)> = p.consts.iter().map(|(k, v)| (k.clone(), *v)).collect();
        let want_consts: Vec<(String, f64)> = serde_json::from_value(want["consts"].clone()).unwrap();
        assert_eq!(consts, want_consts, "{file}: consts");
        let lines: Vec<(usize, String, usize, String)> =
            p.lines.iter().map(|(a, l)| (*a, l.file.clone(), l.line, l.text.clone())).collect();
        let want_lines: Vec<(usize, String, usize, String)> = serde_json::from_value(want["lines"].clone()).unwrap();
        assert_eq!(lines, want_lines, "{file}: lines");
        assert_eq!(listing(&p.bytes, Some(&p.labels)).unwrap(), want["listing"].as_str().unwrap(), "{file}: listing");
    }
    assert_eq!(checked, programs.len());
    eprintln!("{checked} programs assemble as TypeScript did");
}
