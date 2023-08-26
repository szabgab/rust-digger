FROM rust:1.72-buster
RUN useradd tester
RUN rustup component add rustfmt
