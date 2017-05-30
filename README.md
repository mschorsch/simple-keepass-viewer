# Simple KeePass Viewer
Displays KeePass .kdbx files in your browser.

# Installation

Install [Rust](https://www.rust-lang.org/en-US/downloads.html)
```
curl -sSf https://static.rust-lang.org/rustup.sh | sh
```

Clone this repository
```
git clone [REPO]
```

Install `OpenSSL`
```
apt-get install openssl libssl-dev
```

Build the server
```rust
cargo build --release
```

Run (http)
```
./target/release/simple-keepass-viewer --insecure
```
or (https)
```
./target/release/simple-keepass-viewer --ssl_cert certificate.p12 --ssl_cert_pw mypassword
```

# Command line options
```
USAGE:
    simple-keepass-viewer.exe [FLAGS] [OPTIONS] --ssl_cert <PKCS12> --ssl_cert_pw <PKCS12 PASSWORD>

FLAGS:
    -h, --help        Prints help information
        --insecure    http instead of https
    -V, --version     Prints version information

OPTIONS:
    -a, --address <ADDRESS>                Sets a custom address [default: 127.0.0.1:8000]
    -d, --directory <DIRECTORY>            Sets a custom directory [default: ./]
        --loglevel <LOGLEVEL>              Sets a custom log level [default: info]  [values: error, warn, info, debug, trace]
        --ssl_cert <PKCS12>                Sets a pkcs12 certificate
        --ssl_cert_pw <PKCS12 PASSWORD>    Sets a password for the pkcs12 certificate
```

# License
MIT