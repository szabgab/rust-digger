set -e -u -o pipefail
cd /home/gabor/work/rust-digger
./download.sh;
/home/gabor/.cargo/bin/cargo build --release > /tmp/rust-digger-build.log 2> /tmp/rust-digger-build.err
./target/release/vcs > /tmp/rust-digger-vcs.log 2> /tmp/rust-digger-vcs.err
./target/release/html > /tmp/rust-digger-html.log 2> /tmp/rust-digger-html.err
