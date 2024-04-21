set -e -u
#-o pipefail
cd /home/gabor/work/rust-digger
/home/gabor/.cargo/bin/cargo build --release > /tmp/rust-digger-build.log 2> /tmp/rust-digger-build.err
./target/release/download-db-dump > /tmp/rust-digger-download-db-dump.log 2> /tmp/rust-digger-download-db-dump.err
./target/release/download-crates > /tmp/rust-digger-download-crates.log 2> /tmp/rust-digger-download-crates.err
./target/release/clone --recent 10 > /tmp/rust-digger-clone.log 2> /tmp/rust-digger-clone.err
./target/release/analyze-vcs > /tmp/rust-digger-vcs.log 2> /tmp/rust-digger-vcs.err
./target/release/analyze-crates > /tmp/rust-digger-analyze-crates.out 2> /tmp/rust-digger-analyze-crates.err
./target/release/html > /tmp/rust-digger-html.log 2> /tmp/rust-digger-html.err
