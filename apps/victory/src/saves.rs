//! `victory saves`: list, create, copy and delete save slots (see
//! `victory_core::saves`). Deleting moves a slot to the title's `trash`.

use std::path::PathBuf;

use anyhow::{Result, bail};
use clap::Subcommand;
use victory_core::saves;

#[derive(Subcommand)]
pub enum Saves {
    /// List the slots of each title.
    List,
    /// Create an empty slot (a fresh game).
    New { name: String },
    /// Copy a slot to a new one.
    Copy { from: String, to: String },
    /// Move a slot to the trash folder (nothing is erased).
    Delete { name: String },
}

/// Title folders to act on: `--title` (8 hex digits) or every one present.
fn targets(data: &std::path::Path, title: Option<&str>) -> Result<Vec<PathBuf>> {
    if let Some(t) = title {
        return Ok(vec![data.join(t.to_uppercase())]);
    }
    let all = saves::titles(data);
    if all.is_empty() {
        bail!("no title data under {}; run the game once or pass --title <ID>", data.display());
    }
    Ok(all)
}

pub fn run(cmd: Saves, title: Option<String>) -> Result<()> {
    let cfg = victory_core::Config::load()?;
    let titles = targets(&cfg.data_dir, title.as_deref())?;
    for t in titles {
        let id = t.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        match &cmd {
            Saves::List => {
                println!("title {id}:");
                for s in saves::list(&t) {
                    let p = saves::resolve(&t, &s)?;
                    let note = if p.legacy { " (pre-slot location)" } else { "" };
                    println!("  {s}{note}");
                }
            }
            Saves::New { name } => {
                saves::create(&t, name)?;
                println!("{id}: created slot {name}");
            }
            Saves::Copy { from, to } => {
                saves::copy(&t, from, to)?;
                println!("{id}: copied {from} to {to}");
            }
            Saves::Delete { name } => {
                let secs = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs());
                let dest = saves::delete(&t, name, secs)?;
                println!("{id}: moved slot {name} to {}", dest.display());
            }
        }
    }
    Ok(())
}
