# munin-plugin

A simple library to ease writing of munin plugins in Rust.

More content coming soon, for now read [docs.rs](https://docs.rs/munin-plugin/latest/munin_plugin/).

## Badges
[![Crates.io](https://img.shields.io/crates/v/munin-plugin)](https://crates.io/crates/munin-plugin)
[![Crates.io](https://img.shields.io/crates/d/munin-plugin)](https://crates.io/crates/munin-plugin)
[![docs.rs](https://img.shields.io/docsrs/munin-plugin)](https://docs.rs/munin-plugin)
[![LGPL-3.0-only licensed](https://img.shields.io/crates/l/munin-plugin)](https://github.com/Ganneff/munin-plugin/blob/main/LICENSE.md)
![GitHub code size in bytes](https://img.shields.io/github/languages/code-size/Ganneff/munin-plugin)
[![GitHub issues](https://img.shields.io/github/issues/Ganneff/munin-plugin)](https://github.com/Ganneff/munin-plugin/issues)
[![codecov](https://codecov.io/gh/Ganneff/munin-plugin/branch/main/graph/badge.svg)](https://codecov.io/gh/Ganneff/munin-plugin)

# Semantic Versioning
This library will follow [Semantic Versioning](https://semver.org/),
that is - until release 1.0.0, anything may change, be prepared to
adjust your code.

# Security
While I try not to put too many bugs in, there is no guarantee. I
avoid writing unsafe Rust code myself, so Rust will safe me from some
bugs, but crates I use most probably use unsafe.

If you find bugs (security or not), I'm always happy about reports or
even patches to fix them.

## Cargo Geiger Safety Report
```

Metric output format: x/y
    x = unsafe code used by the build
    y = total unsafe code found in the crate

Symbols: 
    🔒  = No `unsafe` usage found, declares #![forbid(unsafe_code)]
    ❓  = No `unsafe` usage found, missing #![forbid(unsafe_code)]
    ☢️  = `unsafe` usage found

Functions  Expressions  Impls  Traits  Methods  Dependency

0/0        0/0          0/0    0/0     0/0      🔒  munin-plugin 0.1.10
15/18      442/449      3/3    0/0     11/11    ☢️  ├── anyhow 1.0.57
0/0        0/0          0/0    0/0     0/0      🔒  ├── fastrand 1.7.0
0/0        52/157       0/0    0/0     0/0      ☢️  ├── fs2 0.4.3
0/21       12/368       0/2    0/0     2/40     ☢️  │   └── libc 0.2.126
1/1        16/18        1/1    0/0     0/0      ☢️  ├── log 0.4.17
0/0        0/0          0/0    0/0     0/0      ❓  │   └── cfg-if 1.0.0
0/0        15/15        0/0    0/0     0/0      ☢️  ├── spin_sleep 1.1.1
0/0        25/71        0/0    0/0     0/0      ☢️  └── tempfile 3.3.0
0/0        0/0          0/0    0/0     0/0      ❓      ├── cfg-if 1.0.0
0/0        0/0          0/0    0/0     0/0      🔒      ├── fastrand 1.7.0
0/21       12/368       0/2    0/0     2/40     ☢️      ├── libc 0.2.126
0/0        0/79         0/0    0/0     0/0      ❓      └── remove_dir_all 0.5.3

16/40      562/1157     4/6    0/0     13/51  

```
