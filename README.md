# EXPERIMENTAL Rust Bindings For Couchbase Lite

This is a Rust API binding of [Couchbase Lite][CBL], an embedded NoSQL document database engine with sync.

## Disclaimer

> This library is **NOT SUPPORTED BY COUCHBASE**. Even if you are a Couchbase customer, our otherwise awesome support team cannot help you with using this library.

As of September 2021, this library is still incomplete and has been tested only partially and informally, mostly on one platform (macOS). Also the author is a novice Rustacean and may not be doing things the Rust Way.

That said, we would like to maintain and improve this library as time permits. We welcome bug reports, fixes and improvements!

## Building

**_"Some assembly required..."_**

### 1. Install LLVM/Clang

In addition to [Rust][RUST], you'll need to install LLVM and Clang, which are required by the
[bindgen][BINDGEN] tool that generates Rust FFI APIs from C headers.
Installation instructions are [here][BINDGEN_INSTALL].

### 2. Get Couchbase Lite For C

Next you need the Couchbase Lite For C shared library and headers. You can download them from Couchbase, or build them yourself from the [Git repo][CBL_C].

### 3. Fix The Skanky Hardcoded Paths

Now edit the file `CouchbaseLite/build.rs` and edit the hardcoded paths on lines 32-37.
This tells the crate where to find Couchbase Lite's headers and library, and the Clang libraries.

### 4. Build!

    $ cargo build

### 5. Test

**The unit tests must be run single-threaded.** This is because each test case checks for leaks by
counting the number of extant Couchbase Lite objects before and after it runs, and failing if the
number increases. That works only if a single test runs at a time.

    $ LEAK_CHECK=y cargo test -- --test-threads 1

### 6. Sanitizer

    $ LSAN_OPTIONS=suppressions=san.supp RUSTFLAGS="-Zsanitizer=address" cargo +nightly test 

**To diag flaky test** 

    $ LSAN_OPTIONS=suppressions=san.supp RUSTFLAGS="-Zsanitizer=address" cargo +nightly test --verbose --features=flaky-test flaky

## Learning

I've copied the doc-comments from the C API into the Rust files. But Couchbase Lite is fairly
complex, so if you're not already familiar with it, you'll want to start by reading through
the [official documentation][CBLDOCS].

The Rust API is mostly method-for-method compatible with the languages documented there, except
down at the document property level (dictionaries, arrays, etc.) where I haven't yet written
compatible bindings. For those APIs you can check out the document "[Using Fleece][FLEECE]".

(FYI, if you want to see what bindgen's Rust translation of the C API looks like, it's in the file `bindings.rs` in `build/couchbase-lite-*/out`, where "`*`" will be some hex string. This is super unlikely to be useful unless you want to work on improving the high-level bindings themselves.)


[RUST]: https://www.rust-lang.org
[CBL]: https://www.couchbase.com/products/lite
[CBL_C]: https://github.com/couchbase/couchbase-lite-C
[CBLDOCS]: https://docs.couchbase.com/couchbase-lite/current/introduction.html
[FLEECE]: https://github.com/couchbaselabs/fleece/wiki/Using-Fleece
[BINDGEN]: https://rust-lang.github.io/rust-bindgen/
[BINDGEN_INSTALL]: https://rust-lang.github.io/rust-bindgen/requirements.html
