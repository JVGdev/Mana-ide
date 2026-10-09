//! Reading .masm files from disk, with the libraries beside them.

use std::path::{Path, PathBuf};

use crate::asm::{AsmError, Program, assemble};

/// The repository's folders: lib/, spells/ and bench/.
pub fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

pub fn lib_dir() -> PathBuf {
    repo().join("lib")
}

pub fn spell_dir() -> PathBuf {
    repo().join("spells")
}

/// Finds a library in the spell's own folder first, then in lib/.
pub fn resolver(dirs: Vec<PathBuf>) -> impl Fn(&str) -> Option<String> {
    move |name: &str| {
        for dir in dirs.iter().cloned().chain(std::iter::once(lib_dir())) {
            let path = dir.join(format!("{name}.masm"));
            if path.exists() {
                return std::fs::read_to_string(path).ok();
            }
        }
        None
    }
}

pub fn assemble_file(path: &Path) -> Result<Program, AsmError> {
    let source = std::fs::read_to_string(path).map_err(|e| AsmError { problems: vec![format!("{}: {e}", path.display())] })?;
    let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    assemble(&source, &name, &resolver(vec![dir]))
}

/// A spell from spells/, by name.
pub fn spell(name: &str) -> Program {
    assemble_file(&spell_dir().join(format!("{name}.masm"))).unwrap_or_else(|e| panic!("{e}"))
}
