# [Rust Digger](https://rust-digger.code-maven.com/)

* Analyze Rust Crates

* Fetch list of [Crates](https://crates.io/)
* Process the data
* Generate static HTML pages

## Sites

* https://crates.io/
* https://docs.rs/
* https://lib.rs/

## Fetching data

Discussed here: https://crates.io/data-access

As of 2023.06.17

1. The git repository https://github.com/rust-lang/crates.io-index does not contain the meta data, such as the github URL
1. The https://static.crates.io/db-dump.tar.gz is 305 Mb It unzipped to a folder called `2023-06-16-020046` which is 1.1 Gb and contains CSV dumps of a Postgresql database


## Local development environment

```
git clone https://github.com/szabgab/rust-digger.git
cd rust-digger
cargo run 200
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
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
cargo run
```


