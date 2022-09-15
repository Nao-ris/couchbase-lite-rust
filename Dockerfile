FROM rust
RUN apt-get update
RUN apt-get -y install clang
RUN mkdir /build
WORKDIR /build
ENV LIBCLANG_PATH=/usr/lib/llvm-11/lib/
ADD Cargo.toml Cargo.toml
ADD build.rs build.rs
ADD libcblite-3.0.2 libcblite-3.0.2
ADD src src
