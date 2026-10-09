// mas: assemble a spell. Prints the listing; writes the bytes with -o.
//
//   mas spells/Fireball.masm
//   mas spells/Fireball.masm -o Fireball.mbc

use std::path::Path;
use std::process::exit;

use mana::asm::listing;
use mana::load::assemble_file;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let file =
        args.iter().enumerate().find(|(i, a)| !a.starts_with('-') && (*i == 0 || args[i - 1] != "-o")).map(|(_, a)| a.clone());
    let out = args.iter().position(|a| a == "-o").and_then(|i| args.get(i + 1)).cloned();
    let Some(file) = file else {
        eprintln!("usage: mas <spell.masm> [-o out.mbc]");
        exit(2);
    };
    match assemble_file(Path::new(&file)) {
        Ok(program) => {
            println!("{}", listing(&program.bytes, Some(&program.labels)).unwrap());
            println!("\n{} bytes", program.bytes.len());
            if let Some(out) = out {
                std::fs::write(out, &program.bytes).unwrap();
            }
        }
        Err(e) => {
            for p in e.problems {
                eprintln!("{p}");
            }
            exit(1);
        }
    }
}
