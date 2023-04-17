FROM --platform=amd64 rust AS build
RUN apt-get update
RUN apt-get -y install clang
RUN mkdir /build
WORKDIR /build
ENV LIBCLANG_PATH=/usr/lib/llvm-11/lib/
ADD Cargo.toml Cargo.toml
ADD build.rs build.rs
ADD libcblite libcblite
ADD src src
RUN cargo c
RUN cargo test -- --test-threads=1

FROM --platform=amd64 rust AS strip-stage
RUN apt-get update
RUN apt-get -y install binutils binutils-aarch64-linux-gnu
RUN mkdir /build
WORKDIR /build
ADD libcblite libcblite
RUN strip /build/libcblite/lib/x86_64-linux-android/libcblite.so -o /build/libcblite/lib/x86_64-linux-android/libcblite.stripped.so
RUN strip /build/libcblite/lib/i686-linux-android/libcblite.so -o /build/libcblite/lib/i686-linux-android/libcblite.stripped.so
RUN /usr/aarch64-linux-gnu/bin/strip /build/libcblite/lib/aarch64-linux-android/libcblite.so -o /build/libcblite/lib/aarch64-linux-android/libcblite.stripped.so
RUN /usr/aarch64-linux-gnu/bin/strip /build/libcblite/lib/armv7-linux-androideabi/libcblite.so -o /build/libcblite/lib/armv7-linux-androideabi/libcblite.stripped.so
RUN strip /build/libcblite/lib/x86_64-pc-windows-gnu/cblite.dll -o /build/libcblite/lib/x86_64-pc-windows-gnu/cblite.stripped.dll

FROM scratch AS strip 
COPY --from=strip-stage /build/libcblite/ .
COPY --from=strip-stage /build/libcblite/ .
