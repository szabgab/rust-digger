FROM rust:1.72-buster
RUN useradd tester
RUN rustup component add rustfmt
RUN mkdir /crate
COPY fmt.sh /opt/fmt.sh
RUN chmod +x /opt/fmt.sh

