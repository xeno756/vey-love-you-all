//! The compute shaders a run leaves out, chosen by name.
//!
//! `VICTORY_SKIP_SHADERS` names one or more list files (`;` between them).
//! A line of a list is the 16 digits logs print for a shader, or a part of
//! shader names as the title's files have them (any case): every shader
//! whose name contains it is meant. `#` starts a comment. Names are read
//! from the list a preparation writes beside the cache
//! (`victory prepare --names-only`).
//!
//! Only dispatches are left out: a name that also matches shaders of draws
//! leaves out none of them (`victory_gpu::pipeline::leave_out`).

use std::path::Path;

/// What lists ask for, against the names a preparation listed.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Chosen {
    /// The hashes of the shaders meant, each once.
    pub(crate) hashes: Vec<String>,
    /// Each entry of the lists, and how many shaders it names.
    pub(crate) entries: Vec<(String, usize)>,
}

/// The shaders `lists` (the text of the list files) names among `names`
/// (lines of a hash, a space and a name).
pub(crate) fn choose(lists: &str, names: &str) -> Chosen {
    let named: Vec<(&str, String)> = names
        .lines()
        .filter_map(|line| {
            let (hash, name) = line.split_once(' ')?;
            Some((hash, name.to_ascii_lowercase()))
        })
        .collect();
    let mut chosen = Chosen::default();
    for line in lists.lines() {
        let entry = line.split('#').next().unwrap_or("").trim();
        if entry.is_empty() {
            continue;
        }
        let before = chosen.hashes.len();
        if entry.len() == 16 && entry.bytes().all(|b| b.is_ascii_hexdigit()) {
            chosen.hashes.push(entry.to_ascii_lowercase());
        } else {
            let wanted = entry.to_ascii_lowercase();
            chosen.hashes.extend(
                named
                    .iter()
                    .filter(|(_, name)| name.contains(&wanted))
                    .map(|(hash, _)| (*hash).to_owned()),
            );
        }
        chosen.entries.push((entry.to_owned(), chosen.hashes.len() - before));
    }
    chosen.hashes.sort_unstable();
    chosen.hashes.dedup();
    chosen
}

/// Reads the lists `VICTORY_SKIP_SHADERS` names and tells the GPU layer
/// which compute shaders to leave out. Before the title starts.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn apply(cache_dir: &Path) {
    let Some(value) = std::env::var_os("VICTORY_SKIP_SHADERS").filter(|v| !v.is_empty()) else {
        return;
    };
    let mut lists = String::new();
    for path in std::env::split_paths(&value) {
        match std::fs::read_to_string(&path) {
            Ok(text) => {
                lists.push_str(&text);
                lists.push('\n');
            }
            Err(e) => {
                // On the error stream as well: that is the log of a run.
                println!("left out       list {} not read: {e}", path.display());
                eprintln!("left out: list {} not read: {e}", path.display());
            }
        }
    }
    let names_path = cache_dir.join(crate::prepare::NAMES_FILE);
    let names = std::fs::read_to_string(&names_path).unwrap_or_default();
    let chosen = choose(&lists, &names);
    if names.is_empty() {
        println!(
            "left out       no shader names at {}: `victory prepare --names-only` writes them",
            names_path.display()
        );
        eprintln!(
            "left out: no shader names at {}: entries that are names leave nothing out",
            names_path.display()
        );
    }
    for (entry, shaders) in &chosen.entries {
        eprintln!("left out: \"{entry}\" names {shaders} shaders");
    }
    println!(
        "left out       the dispatches of the compute shaders among {} named by {} entries",
        chosen.hashes.len(),
        chosen.entries.len()
    );
    #[cfg(windows)]
    victory_gpu::pipeline::leave_out(chosen.hashes);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An entry is a hash as logs print it, or a part of names in any
    /// case; comments and empty lines are nothing; an entry that names
    /// nothing is counted as such.
    #[test]
    fn entries_name_shaders_by_hash_or_by_part_of_their_name() {
        let names = "00000000000000aa effects:CS_Blur_X\n00000000000000bb effects:CS_Blur_Y\n00000000000000cc sky:CS_Main\n";
        let lists = "# the blur\nblur  # both passes\n\nEFFECTS:cs_blur_x\n00000000000000CC\nnothing_like_it\n";
        let chosen = choose(lists, names);
        assert_eq!(chosen.hashes, ["00000000000000aa", "00000000000000bb", "00000000000000cc"]);
        assert_eq!(
            chosen.entries,
            [
                ("blur".to_owned(), 2),
                ("EFFECTS:cs_blur_x".to_owned(), 1),
                ("00000000000000CC".to_owned(), 1),
                ("nothing_like_it".to_owned(), 0)
            ]
        );
        assert_eq!(choose("", names), Chosen::default());
    }
}
