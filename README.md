# Rust Digger

* Analyze Rust Crates

* Fetch list of [Crates](https://crates.io/)
* Process the data
* Generate static HTML pages


## Fetching data

Discussed here: https://crates.io/data-access

As of 2023.06.17

1. The git repository https://github.com/rust-lang/crates.io-index does not contain the meta data, such as the github URL
1. The https://static.crates.io/db-dump.tar.gz is 305 Mb It unzipped to a folder called `2023-06-16-020046` which is 1.1 Gb and contains CSV dumps of a Postgresql database
