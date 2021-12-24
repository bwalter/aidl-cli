# Simple AIDL command line tool

Command line to to parse AIDL files and extract informations.

## Features

- display diagnostics
- display items
- convert to JSON

For language-specific features, see [rust-aidl-parser](https://github.com/bwalter/rust-aidl-parser).

## Usage

```
> aidl-cli -h
aidl-cli 0.1.0

USAGE:
    aidl-cli [FLAGS] [OPTIONS] <dir>

FLAGS:
    -i, --items               Display items
    -h, --help                Prints help information
    -q, --hide-diagnostics    Do not show diagnostics
        --pretty              Make pretty (but longer) messages
    -j, --to-json             Convert the whole AST to JSON
    -V, --version             Prints version information

OPTIONS:
    -o, --output-path <output-path>    Output file

ARGS:
    <dir>    The directory where the AIDL files are located
```

Display diagnostics only:
```
> aidl-cli /path/to/project
```

List items and files:
```
> aidl-cli -i /path/to/project
```

## Convert to JSON and extract infos

Display all item names (requires [jq][jq]):
```
> aidl-cli -j /path/to/project | jq '.items[] | .[] | .name'
```

Convert into JSON  all items as { type, name }:
```
> aidl-cli -j /path/to/project | jq '[.items[] | to_entries[] | {type: .key, name: .value.name}]'
```

Show the diff between projects (requires [jd][jd]):
```
> aidl-cli -j /path/to/project1 > project1.json
> aidl-cli -j /path/to/project2 > project2.json
> jd project1.json project2.sjon
```

 [jq]: https://github.com/stedolan/jq
 [jd]: https://github.com/josephburnett/jd

