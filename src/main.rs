use std::path::Path;

use anyhow::{Context as _, bail};
use bf_interpreter::BfInterpreter;

fn main() -> anyhow::Result<()> {
    let Some(source_path) = std::env::args().nth(1) else {
        bail!("expected source file path as a commandline argument");
    };
    let source_path = Path::new(&source_path);
    let source = std::fs::read_to_string(source_path).context("source read failed")?;

    let stdin = std::io::stdin();
    let stdin = stdin.lock();
    let stdout = std::io::stdout();
    let interpreter = BfInterpreter::new(&source, stdin, stdout)?;
    interpreter.execute().context("execution failure")?;
    Ok(())
}
