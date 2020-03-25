# move-vm-in-cosmos

# Getting started.

Build and run the project:
```
git clone --recurse-submodules https://github.com/dfinance/dvm.git
cd dvm
```

Run:

Format: `client` or `server` `IP:PORT`

DVM Server: `cargo run --release --bin dvm "[::1]:50051" "http://[::1]:50052"`


# precommit hook

```shell script
ln -s `git rev-parse --show-toplevel`/check_project.sh `git rev-parse --absolute-git-dir`/hooks/pre-commit
```

# Build std.
`cargo run --bin stdlib-builder <path to stdlib directory> [-o output file] [-verbose]`

Example:
- `cargo run --bin stdlib-builder lang/stdlib -p`
- `cargo run --bin stdlib-builder lang/stdlib -po ./std-out.json`

# Start compiler

`cargo run --bin compiler "<which host:port to use>" <ds-server uri>"`

Example: `cargo run --bin compiler "[::1]:50052" "http://[::1]:50051"`
