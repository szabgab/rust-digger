use std::env;
use std::path::Path;
use std::process::Command;

use clap::Parser;

#[derive(Parser, Debug)]
#[command(version)]
struct Cli {
    #[arg(
        long,
        default_value_t = 0,
        help = "Number of git repositories to try to run cargo fmt on."
    )]
    limit: u32,
}

/// For each repo run cargo fmt
///
/// For each repo load the details (if they already exist)
///    If we have not ran fmt on the given repo then
///          run fmt
///          save the results back to the details
fn main() {
    let args = Cli::parse();
    simple_logger::init_with_level(log::Level::Info).unwrap();
    log::info!("start updating git repositories");
    run_cargo_fmt(args.limit)
}

fn run_cargo_fmt(limit: u32) {
    log::info!("start update repositories. limit {}.", limit);

    build_docker_image();
    let mut count: u32 = 0;
    let path = Path::new("repos/github");
    let root_dir = env::current_dir().unwrap();
    for user in path.read_dir().expect("read_dir call failed") {
        if let Ok(user) = user {
            log::info!("user: {:?}", user.path());
            for repo in user.path().read_dir().expect("read_dir call failed") {
                if limit > 0 && count >= limit {
                    return;
                }

                if let Ok(repo) = repo {
                    // TODO load details

                    if !repo.path().join("Cargo.toml").exists() {
                        continue;
                    }

                    count += 1;
                    log::info!("repo {}: {:?}", count, repo.path());

                    env::set_current_dir(repo.path()).unwrap();
                    // TODO measure elapsed time
                    let stdout = run_cargo_in_docker();
                    log::info!("stdout: {}", stdout);
                    // TODO save to details
                }
            }

            env::set_current_dir(&root_dir).unwrap();
        }
    }
}

/// docker build -t rust-test .
fn build_docker_image() {
    log::info!("build_docker_image");
    let result = Command::new("docker")
        .arg("build")
        .arg("-t")
        .arg("rust-test")
        .arg(".")
        .output()
        .expect("Could not run");
    log::info!("build_docker_image {:?}", result.status.code());
    if result.status.code() != Some(0) {
        log::warn!("{}", std::str::from_utf8(&result.stdout).unwrap());
        log::warn!("{}", std::str::from_utf8(&result.stderr).unwrap());
    }
}

/// docker run --rm --workdir /opt -v$(pwd):/opt -it --user tester rust-test cargo fmt --check -- --color=never
fn run_cargo_in_docker() -> String {
    log::info!("run_cargo_in_docker");
    let cwd = env::current_dir().unwrap();
    log::info!("cwd: {}", cwd.display());
    let result = Command::new("docker")
        .arg("run")
        .arg("--rm")
        .arg("--workdir")
        .arg("/crate")
        .arg(format!("-v{}:/crate", cwd.display()))
        .arg("--user")
        .arg("tester")
        .arg("rust-test")
        .arg("bash")
        .arg("/opt/fmt.sh")
        // .arg("cargo")
        // .arg("fmt")
        // .arg("--check")
        // .arg("--")
        // .arg("--color=never")
        .output()
        .expect("Could not run");
    log::info!("run_cargo_in_docker {:?}", result.status.code());
    if result.status.code() != Some(0) {
        log::warn!("stdout: {}", std::str::from_utf8(&result.stdout).unwrap());
        log::warn!("stderr: {}", std::str::from_utf8(&result.stderr).unwrap());
    }

    std::str::from_utf8(&result.stdout).unwrap().to_string()
}

//git status --porcelain
// fn git_status() -> String {
//     log::info!("git_status");
//     let result = Command::new("git")
//         .arg("status")
//         .arg("--porcelain")
//         .output()
//         .expect("Could not run");
//     log::info!("git_status {:?}", result.status.code());
//     if result.status.code() != Some(0) {
//         log::warn!("{}", std::str::from_utf8(&result.stdout).unwrap());
//         log::warn!("{}", std::str::from_utf8(&result.stderr).unwrap());
//     }
//     let stdout = std::str::from_utf8(&result.stdout).unwrap();
//     stdout.to_string()
// }

// fn git_checkout() {
//     log::info!("git_checkout");
//     let result = Command::new("git")
//         .arg("checkout")
//         .arg(".")
//         .output()
//         .expect("Could not run");
//     log::info!("git_checkout {:?}", result.status.code());
//     if result.status.code() != Some(0) {
//         log::warn!("{}", std::str::from_utf8(&result.stdout).unwrap());
//         log::warn!("{}", std::str::from_utf8(&result.stderr).unwrap());
//     }
// }
