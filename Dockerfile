FROM rust:1.73-bullseye
RUN adduser --home /home/tester --gecos "Test User" --disabled-password tester
RUN rustup component add rustfmt
RUN apt-get update && \
    apt-get install -y llvm && \
    echo DONE
RUN mkdir /crate
COPY fmt.sh /opt/fmt.sh
RUN chmod +x /opt/fmt.sh

