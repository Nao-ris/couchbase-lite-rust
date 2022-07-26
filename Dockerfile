FROM rust
RUN apt-get update
RUN apt-get -y install clang
RUN mkdir /build
WORKDIR /build
ENV LIBCLANG_PATH=/usr/lib/llvm-11/lib/
ENV LD_LIBRARY_PATH=$LD_LIBRARY_PATH:/build/libcblite-3.0.1/lib 
ADD Cargo.toml Cargo.toml
ADD build.rs build.rs
ADD libcblite-3.0.1 libcblite-3.0.1
ADD src src
RUN cargo c
RUN cargo test -- --test-threads=1
