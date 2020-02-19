# move-vm-in-cosmos

# Getting started.

Build and run the project:
```
git clone --recurse-submodules https://github.com/WingsDao/move-vm-in-cosmos.git
cd move-vm-in-cosmos
```

Run:

Format: `client` or `server` `IP:PORT`

DS Mock Server: `cargo run --bin ds-server "[::1]:50052"`

VM Server: `cargo run --bin server "[::1]:50051" "http://[::1]:50052"`

VM Mock Client: `cargo run --bin client "http://[::1]:50051"`

# precommit hook

```shell script
ln -s `git rev-parse --show-toplevel`/check_project.sh `git rev-parse --absolute-git-dir`/hooks/pre-commit
```

# Build std.
`cargo run --bin stdlib-builder <path to stdlib directory> <compiler type [move, mvir]>`

Example.
`cargo run --bin stdlib-builder stdlib/mvir mvir`

# Start compiler

`cargo run --bin compiler "<which host:port to use>" <ds-server uri>"`

Example: `cargo run --bin compiler "[::1]:50052" "http://[::1]:50051"`