set -x

rm -rf rust-digger-html
rm -f rust-digger.gz

set -e

mv _site/ rust-digger-html
tar czf rust-digger.gz rust-digger-html/
scp rust-digger.gz s7:work/
ssh s7 "cd work && tar xzf rust-digger.gz"

