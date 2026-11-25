//! Before a run: the title's shaders translated into the shader cache, and
//! the host pipelines that need no run made and kept in the pipeline
//! library. A run with the same build then reads translations instead of
//! making them, and loads those pipelines instead of compiling them while
//! the title draws.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context, Result};

use crate::shader::{MAX_FILE, sources};

/// The files shaders are looked for in: `paths` (folders walked whole), or
/// the files directly in the configured game folder (its shader archive,
/// executable and libraries are there).
fn files(paths: Vec<PathBuf>) -> Result<Vec<PathBuf>> {
    let (roots, depth) = if paths.is_empty() {
        let cfg = victory_core::Config::load()?;
        let game = cfg
            .game_dir
            .context("no paths given and no game folder configured (see victory.example.toml)")?;
        (vec![game], 1)
    } else {
        (paths, usize::MAX)
    };
    let mut files = Vec::new();
    for root in &roots {
        for entry in walkdir::WalkDir::new(root).max_depth(depth).into_iter().filter_map(Result::ok) {
            if entry.file_type().is_file() && entry.metadata().is_ok_and(|m| m.len() <= MAX_FILE) {
                files.push(entry.into_path());
            }
        }
    }
    Ok(files)
}

/// Every distinct shader container of `file`, cut to its own length (the
/// bytes a title passes when it makes a pipeline).
fn containers_of(file: &Path, seen: &mut HashSet<[u8; 32]>, out: &mut Vec<Vec<u8>>) {
    let Ok(data) = std::fs::read(file) else {
        return;
    };
    for source in sources(&data) {
        let Some(bytes) = source.bytes() else {
            continue;
        };
        // The container's total size is in its header.
        let Some(total) = bytes.get(0x18..0x1c).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize)
        else {
            continue;
        };
        let Some(container) = bytes.get(..total) else {
            continue;
        };
        if seen.insert(*blake3::hash(container).as_bytes()) {
            out.push(container.to_vec());
        }
    }
}

pub fn run(paths: Vec<PathBuf>, compute: bool, mesh: bool, jobs: Option<usize>) -> Result<()> {
    let cfg = victory_core::Config::load()?;
    victory_hle::modules::d3d12x::use_shader_cache(cfg.cache_subdir("shaders")?);
    let started = Instant::now();
    let files = files(paths)?;
    let (mut seen, mut containers) = (HashSet::new(), Vec::new());
    for file in &files {
        containers_of(file, &mut seen, &mut containers);
    }
    let bytes: usize = containers.iter().map(Vec::len).sum();
    println!(
        "found          {} shader containers ({} MB) in {} files, in {:.1} s",
        containers.len(),
        bytes >> 20,
        files.len(),
        started.elapsed().as_secs_f64()
    );
    anyhow::ensure!(!containers.is_empty(), "no shader container found");
    // The translation and the host's compile are both work for a
    // processor; two are left to the rest of the PC.
    let jobs = jobs.unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get().saturating_sub(2).max(1)));
    println!(
        "preparing      translations{}{} on {jobs} threads",
        if compute { ", compute pipelines" } else { "" },
        if mesh { ", mesh programs' compute pipelines" } else { "" }
    );
    let total = containers.len();
    let views: Vec<&[u8]> = containers.iter().map(Vec::as_slice).collect();
    let last = std::sync::Mutex::new(Instant::now());
    let progress = |done: usize| {
        let mut last = last.lock().unwrap_or_else(|e| e.into_inner());
        if last.elapsed().as_secs() >= 10 {
            *last = Instant::now();
            println!("               {done} of {total} ({} %)", done * 100 / total);
        }
    };
    #[cfg(windows)]
    {
        let work = Instant::now();
        let done = victory_gpu::pipeline::prepare(&views, compute, mesh, jobs, &progress).map_err(anyhow::Error::msg)?;
        let seconds = work.elapsed().as_secs_f64();
        println!(
            "shaders        {} of the stages a run translates: {} translate ({} were in the cache already), {} do not",
            done.shaders, done.translated, done.cached, done.untranslated
        );
        if compute || mesh {
            println!(
                "pipelines      {} in the library ({} were there already), {} refused by the host",
                done.pipelines, done.loaded, done.refused
            );
        }
        println!("time           {seconds:.1} s ({:.1} shaders a second)", done.shaders as f64 / seconds.max(0.001));
        println!("cache          {}", cfg.cache_dir.display());
    }
    #[cfg(not(windows))]
    {
        let _ = (views, progress, compute, mesh, jobs);
        anyhow::bail!("prepare needs the host GPU (Windows)");
    }
    Ok(())
}
