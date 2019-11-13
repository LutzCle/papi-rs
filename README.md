papi-rs
========

## Usage

Add the following to your `Cargo.toml`:

```toml
[dependencies]
papi = "0.1.0"
```

Before building, ensure that PAPI is installed on your system.

## What is papi-rs?

The purpose of this crate is to provide Rust-idiomatic, easy-to-use PAPI bindings.
PAPI is a library that provides a consistent interface to hardware performance
counters. Visit the [PAPI website](http://icl.utk.edu/papi) for more information.

Note that this crate does not provide a high-level interface to PAPI.

## Environment Variables

If PAPI is installed at a custom location on your system (e.g., /opt/papi-5.7.0),
then see the documentation in the [papi-sys crate][papi-sys-env] on how to
configure custom search paths.

## Versions

This library targets the current Rust stable release,
and is currently tested with PAPI version 5.7.0.

## Platforms

The following platforms are currently tested:

* `x86_64-unknown-linux-gnu`
* `powerpc64le-unknown-linux-gnu`

[papi-sys-env]: https://github.com/LutzCle/papi-sys#environment-variables

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
