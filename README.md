# [Rust Digger](https://rust-digger.code-maven.com/)

* Analyze Rust Crates, help evaluation and suggest improvements

* Fetch list of [Crates](https://crates.io/)
* Process the data
* Generate static HTML pages


## Local development environment

```
git clone https://github.com/szabgab/rust-digger.git
cd rust-digger
```

Download the data from crates: (I think this is the only code that assumes some non-Rust tools) See issue #1 and #2

```
./download.sh
```

Clone 15 repositories of the crates that were release in the last 10 days:

```
cargo run --bin rust-digger-clone -- --recent 10 --limit 15
```

Collect data from 15 repositories (VCSs) we cloned. (You can use any number there)

```
cargo run --bin rust-digger-vcs -- --limit 10
```

Generate the static html pages for 10 crates.

```
cargo run --bin rust-digger-html -- --limit 10
```

To run a local web server to serve the static files install [ruststatic](https://github.com/szabgab/rustatic) using:

```
cargo install rustatic
```


and then run:

```
rustatic --nice --indexfile index.html --path _site/
```


## Deployment on Ubuntu-based server

Based on https://www.rust-lang.org/tools/install
```
sudo apt install pkg-config
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo build --release
```

There is a cron-job that runs `all.sh` once a day. (As long as we use the dumped data from Crates.io, there is no point in running more frequently.)

## Processing steps

### Fetching data from crates.io

Discussed here: https://crates.io/data-access

As of 2023.06.17

1. The git repository https://github.com/rust-lang/crates.io-index does not contain the meta data, such as the github URL
1. The https://static.crates.io/db-dump.tar.gz is 305 Mb It unzipped to a folder called `2023-06-16-020046` which is 1.1 Gb and contains CSV dumps of a Postgresql database.

The fetching and unzipping is done by `download.sh`.

For each crate (or for each new crate if we start working incrementally) check if it refers to a repo.
For each repo maintain a file called repo-details/github/repo-name.json in this repo we keep all the information we collected about the repository. When generating the HTML files we consult these files. These files are also updated by the stand-alone processes listed below.
The files are mapped with the Details struct.


### Cloning repositories

* `git pull` takes 0.3 sec when it does not need to copy any files.
* There are  123,216 crates
* Assuming all of them will have git repositories and most of them won't change we'll need
  123,000 * 0.3 = 41,000 sec to update all the repos = that is 683 minues = 11.5 hours.


If we fail to clone the repository we add this information to the repo-details file of the repository.

### Analyzing repositories

* Some information is easy and fast to collect. (e.g. checking if there are YAML files in `.github/workflows` to check if GitHub Actions is configured)


* TODO: if there are more than one crates in the repo, should we analyze and report the crates separately?

### cargo fmt

* Running `cargo fmt --check -- --color=never` and capturing the STDOUT and the exit code. We save them together with the current sha of the repository `git rev-parse HEAD` and the date of processing. (We might also want to save the version of rustfmt `cargo fmt --version` and the version of rustc `rustc --version`)

```
cargo run --bin fmt -- --limit 10
```

### cargo fix


### cargo test


### Collect test coverage report



## Related Sites

* https://crates.io/
* https://docs.rs/
* https://lib.rs/

