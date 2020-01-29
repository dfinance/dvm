# move-vm-in-cosmos

# Getting started.

Build and run the project: 
```
git clone --recurse-submodules https://github.com/WingsDao/move-vm-in-cosmos.git
cd move-vm-in-cosmos
```

Run:

Format: `client` or `server` `IP:PORT`

Server: `cargo run --bin server "[::1]:50051"`

Client: `cargo run --bin client "http://[::1]:50051"`

# precommit hook

```shell script
ln -s `git rev-parse --show-toplevel`/check_project.sh `git rev-parse --absolute-git-dir`/hooks/pre-commit
```