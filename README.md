[![Github.com](https://img.shields.io/badge/bwalter--aidl--cli-blue?logo=github)](https://github.com/bwalter/aidl-cli)
[![Crates.io](https://img.shields.io/crates/v/aidl-cli.svg?logo=rust)](https://crates.io/crates/aidl-cli)
[![Github Actions](https://img.shields.io/github/workflow/status/bwalter/aidl-cli/main?labels=CI)](https://github.com/bwalter/aidl-cli)

# Simple AIDL command line tool

Command line to to parse AIDL files and extract informations.

## Features

- display diagnostics
- display items
- convert to JSON or YAML

For language-specific features, see [rust-aidl-parser](https://github.com/bwalter/rust-aidl-parser).

## Usage

```
USAGE:
    aidl-cli [FLAGS] [OPTIONS] <dir>

FLAGS:
    -i, --items               Display items
    -h, --help                Prints help information
    -q, --hide-diagnostics    Do not show diagnostics
        --pretty              Make pretty (but longer) output
    -j, --to-json             Convert the whole AST to JSON
    -y, --to-yaml             Convert the whole AST to YAML
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

## Convert to JSON

### Format

JSON Structure:
```
{
  "root": <path_to_root_dir>,
  "items": {
    <item_name>: {
        "path": <relative_path_to_item.aidl>,
        "itemType": <interface|parcelable|enum>,
        "elements": {
          <element_name>: {
            "elementType": <method|const|field|enumElement>,
            "name": <element_name>,
            ... (element-specific info, e.g. field type, method args, ...) ...
          },
          ...
        }
    },
    ...
  }
}
```

Example:
```
> aidl-cli --to-json ~/path/to/aidl/project --pretty > test.json
```

/path/to/aidl/project/test/pkg/TestInterface.aidl (input):
```
package test.pkg;

import test.pkg.TestParcelable;

interface TestInterface {
  const int VERSION = 12;
  
  /**
   * Say hello
   */
  String hello(boolean loud, in TestParcelable data);
}
```

/path/to/aidl/project/test/pkg/TestParcelable.aidl (input):
```
package test.pkg;

parcelable TestParcelable {
  /**
   * The first field
   */
  Array<String> field1;

  /**
   * The second field
   */
  int field2;
}
```

test.json (output):
```
{
  "root": "/path/to/aidl/project",
  "items": {
    "test.pkg.TestInterface": {
      "path": "test/pkg/TestInterface.aidl",
      "itemType": "interface",
      "name": "TestInterface",
      "elements": {
        "hello": {
          "elementType": "method",
          "oneway": false,
          "name": "hello",
          "returnType": "String",
          "args": [
            {
              "name": "loud",
              "type": "boolean"
            },
            {
              "name": "data",
              "direction": "in",
              "type": "test.pkg.TestParcelable"
            }
          ],
          "doc": "Say hello"
        },
        "VERSION": {
          "elementType": "const",
          "name": "VERSION",
          "type": "int",
          "value": 12,
        }
      }
    },
    "test.pkg.TestParcelable": {
      "path": "test/pkg/TestParcelable.aidl",
      "itemType": "parcelable",
      "name": "TestParcelable",
      "elements": {
        "field1": {
          "elementType": "field",
          "name": "field1",
          "type": "Array<String>",
          "doc": "The first field"
        },
        "field2": {
          "elementType": "field",
          "name": "field2",
          "type": "int",
          "doc": "The second field"
        }
      }
    }
  }
}
```

### Extract infos

Display all item names (requires [jq][jq]):
```
> aidl-cli -j /path/to/project | jq '.items[] | .name'
```

Display all items as `[{ <itemType>: <name>, elements: [<name>] }]`:
```
> aidl-cli -j /path/to/project | jq '.items[] | { (.itemType): .name, elements: [.elements[] | .name] }
```

Filter items by name (using regex) and display them as `<itemType> <name>`:
```
> aidl-cli -j /path/to/project | jq '.items[] | select(.name | test("^I")) | "\(.itemType) \(.name)"'
```

Show the diff between projects (requires [jd][jd]):
```
> aidl-cli -j /path/to/project1 > project1.json
> aidl-cli -j /path/to/project2 > project2.json
> jd project1.json project2.json
```

## Convert to YAML

Example:
```
> aidl-cli --to-yaml ~/path/to/aidl/project
```

 [jq]: https://github.com/stedolan/jq
 [jd]: https://github.com/josephburnett/jd

