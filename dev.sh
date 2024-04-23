set -e -u
#-o pipefail
cd /home/gabor/work/rust-digger
/home/gabor/.cargo/bin/cargo build --release

cargo run --bin download-db-dump  > download-db-dump.log 2> download-db-dump.err
cargo run --bin download-crates   > download-crates.log  2> download-crates.err
cargo run --bin clone -- --recent 10 > clone.log         2> clone.err
cargo run --bin analyze-vcs       > vcs.log              2> vcs.err
cargo run --bin analyze-crates    > analyze-crates.log   2> analyze-crates.err
cargo run --bin html              > html.log             2> html.err
