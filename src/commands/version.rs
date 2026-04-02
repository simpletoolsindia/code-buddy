//! Version command

use anyhow::Result;

pub fn run() -> Result<i32> {
    println!("code-buddy {}", env!("CARGO_PKG_VERSION"));
    Ok(0)
}
