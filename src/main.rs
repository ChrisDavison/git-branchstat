use anyhow::{anyhow, Result};
use rayon::prelude::*;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const VERSION: &'static str = "0.1.0";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() > 0 && args[0] == "version" {
        println!("git-branchstat {}", VERSION);
        std::process::exit(0);
    }

    let path = &PathBuf::from(".").canonicalize().unwrap();
    if !is_git_repo() {
        println!("Not a git repo.");
        std::process::exit(1);
    }
    let stat = branchstat(&path);
    if let Ok(Some(status)) = stat {
        println!("{}", status);
    }
}

// Run a git command and return the lines of the output
fn command_output(dir: &Path, args: &[&str]) -> Result<Vec<String>> {
    let out = Command::new("git")
        .current_dir(dir.to_path_buf())
        .args(args)
        .output()?;
    Ok(std::str::from_utf8(&out.stdout)?
        .lines()
        .map(|x| x.to_string())
        .collect())
}

pub fn branchstat(p: &Path) -> Result<Option<String>> {
    let outputs = vec![ahead_behind(p)?, modified(p)?, status(p)?, untracked(p)?]
        .par_iter()
        .filter(|&x| x.is_some())
        .map(|x| x.as_ref().unwrap().as_str())
        .collect::<Vec<&str>>()
        .join(", ");

    if outputs.is_empty() {
        Ok(None)
    } else {
        let out = format!(
            "{:20} | {}",
            p.file_name().unwrap().to_string_lossy(),
            outputs
        );
        Ok(Some(out))
    }
}

fn ahead_behind(p: &Path) -> Result<Option<String>> {
    let response: String = command_output(
        p,
        &[
            "for-each-ref",
            "--format='%(refname:short) %(upstream:track)'",
            "refs/heads",
        ],
    )?
    .par_iter()
    .map(|x| x.trim_matches('\'').trim())
    .filter(|x| {
        let splits: Vec<&str> = x.split(' ').collect();
        splits.get(1).is_some()
    })
    .collect();
    if !response.is_empty() {
        Ok(Some(response))
    } else {
        Ok(None)
    }
}

fn modified(p: &Path) -> Result<Option<String>> {
    let modified = command_output(p, &["diff", "--shortstat"])?.join("\n");
    if modified.contains("changed") {
        let num = modified.trim_start().split(' ').collect::<Vec<&str>>()[0];
        Ok(Some(format!("{}±", num)))
    } else {
        Ok(None)
    }
}

fn status(p: &Path) -> Result<Option<String>> {
    let response = command_output(p, &["diff", "--stat", "--cached"])?;
    if !response.is_empty() {
        Ok(Some(format!("Staged {}", response.len())))
    } else {
        Ok(None)
    }
}

fn untracked(p: &Path) -> Result<Option<String>> {
    let untracked = command_output(p, &["ls-files", "--others", "--exclude-standard"])?;
    if !untracked.is_empty() {
        Ok(Some(format!("{}?", untracked.len())))
    } else {
        Ok(None)
    }
}

pub fn branches(p: &Path) -> Result<Option<String>> {
    let branches: String = command_output(p, &["branch"])?
        .par_iter()
        .map(|x| x.trim())
        .filter(|x| x.starts_with('*'))
        .map(|x| &x[2..])
        .collect();
    let parentpath = p.parent().ok_or(anyhow!("No parent for dir"))?;
    let parentname = parentpath
        .file_stem()
        .ok_or(anyhow!("No stem for parent"))?
        .to_string_lossy();
    let dirname = p
        .file_stem()
        .ok_or(anyhow!("No stem for dir"))?
        .to_string_lossy();
    let dirstr = format!("{}/{}", parentname, dirname);
    Ok(Some(format!("{:40}\t{}", dirstr, branches)))
}

fn is_git_repo() -> bool {
    let status = Command::new("git")
        .arg("branch")
        .stdout(Stdio::null())
        .status()
        .expect("Failed to check if valid git repo");
    match status.code() {
        Some(128) => false,
        _ => true,
    }
}
