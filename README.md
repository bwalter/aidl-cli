# Simple AIDL command line tool

Command line too to parse AIDL files based on [rust-aidl-parser](https://github.com/bwalter/rust-aidl-parser).

## Features

- display diagnostics
- display items

For language-specific features, see [rust-aidl-parser](https://github.com/bwalter/rust-aidl-parser).

## Usage

```
> cargo run -- -h                    
    Finished dev [unoptimized + debuginfo] target(s) in 0.03s
     Running `target/debug/aidl-cli -h`
aidl-cli 0.1.0

USAGE:
    aidl-cli [FLAGS] <dir>

FLAGS:
    -h, --help       Prints help information
        --pretty     Make pretty (but longer) messages
    -V, --version    Prints version information

ARGS:
    <dir>    The directory where the AIDL files are located
```

## TODO

- extract more infos
- convert to JSON
- diff

