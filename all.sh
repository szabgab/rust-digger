set -e -u
#-o pipefail
cd /home/gabor/work/rust-digger
/home/gabor/.cargo/bin/cargo build --release > /tmp/rust-digger-build.log 2> /tmp/rust-digger-build.err
./target/release/rust-digger-download-db-dump > /tmp/rust-digger-download-db-dump.log 2> /tmp/rust-digger-download-db-dump.err
./target/release/rust-digger-clone --recent 10 > /tmp/rust-digger-clone.log 2> /tmp/rust-digger-clone.err
./target/release/rust-digger-vcs > /tmp/rust-digger-vcs.log 2> /tmp/rust-digger-vcs.err
./target/release/rust-digger-html > /tmp/rust-digger-html.log 2> /tmp/rust-digger-html.err
