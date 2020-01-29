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
ln -s $(pwd)/check_project.sh ./.git/hooks/pre-commit
```