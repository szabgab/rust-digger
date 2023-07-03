rm -f db-dump.tar.gz
rm -rd data/
time wget --quiet https://static.crates.io/db-dump.tar.gz
time tar xzf db-dump.tar.gz
DIR=$(ls -d1 2023-* | head -n 1 | cut -d'/' -f1)
mv "$DIR" "data"
