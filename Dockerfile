FROM rust:1.73-bullseye
RUN useradd tester
RUN rustup component add rustfmt
RUN mkdir /crate
COPY fmt.sh /opt/fmt.sh
RUN chmod +x /opt/fmt.sh

