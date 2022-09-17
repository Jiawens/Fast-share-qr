# Fast-share-qr

Share text or file or directory to other devices by scanning a qr code.

## Usage

### Build

`cargo build --release`

### Cli options

```
USAGE:
    fast-share-qr [OPTIONS] --text <TEXT> --file <FILE> --directory <DIRECTORY>

OPTIONS:
    -d, --directory <DIRECTORY>    Directory you want to share WARNING: NOT IMPLEMENTED YET
        --disable-quiet-zone       Disable quiet zone of the qr code?
    -f, --file <FILE>              File you want to share
    -h, --help                     Print help information
    -p, --port <PORT>              Server's port
    -t, --text <TEXT>              Text you want to share
    -V, --version                  Print version information
```

### Note

Directory sharing haven't been implemented.

Only one item can be shared within one instance.
