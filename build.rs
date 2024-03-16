use std::process::Command;

fn get_git_commit_hash() -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    git_hash
}

fn get_git_commit_date() -> String {
    let output = Command::new("git")
        .args(["show", "-s", "--format=%cd", "--date=short", "HEAD"])
        .output()
        .unwrap();
    let git_date = String::from_utf8(output.stdout).unwrap();
    git_date
}

fn main() {
    let commit_hash = get_git_commit_hash();
    let (commit_hash, _) = commit_hash.split_at(9);
    println!("cargo:rustc-env=COMMIT_HASH={}", commit_hash);

    let commit_date = get_git_commit_date();
    println!("cargo:rustc-env=COMMIT_DATE={}", commit_date);
}
