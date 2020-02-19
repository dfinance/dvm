# MoveVM in Cosmos

__TODO: main short description__


Executables included:

- vm server
- compilation server
- stdlib builder
- verifier
- mocks
    - clients to vm-server
    - data-source server

## Getting started

### Installation

One of following options:
- __TODO: describe docker-install solution__
- `cargo install --all-features --git https://github.com/WingsDao/move-vm-in-cosmos.git --bin server compiler verify`



## Usage

__TODO: describe usage for final users__

### Docker-based solution

__TODO: describe docker solution__


### Handy solution

1. execute the Node
1. execute the VM server _with second param - ip pointing to the Node_




### Executables

#### VM Server

TODO: description for tool

__Format:__ `server <bind to HOST:PORT> <data-source URI>`

__Example:__ _`cargo run --bin `_ `server "[::1]:50051" "http://[::1]:50052"`


#### Compilation server

TODO: description for tool

__Format:__ `compiler <bind to HOST:PORT> <data-source URI>`

__Example:__ _`cargo run --bin `_ `compiler "[::1]:50054" "http://[::1]:50052"`


#### Std-lib builder

TODO: description for tool

__Format:__ `stdlib-builder <path to stdlib directory> <compiler type [move, mvir]>`

__Example:__ _`cargo run --bin `_ `stdlib-builder stdlib/move move`



#### DS Mock Server

TODO: description for tool

__Format:__ `ds-server <bind to HOST:PORT>`

__Example:__ _`cargo run --bin `_ `ds-server "[::1]:50052"`

#### VM Mock Client

TODO: description for tool

__Format:__ `client <vm-server URI>`

__Example:__ _`cargo run --bin `_ `client "[::1]:50051"`


- - -

## Development


### Prerequisites:

- install [Rust][]; The easiest way to get it is to use [Rustup][]
- install [protoc][]

[Rust]: https://www.rust-lang.org
[Rustup]: https://rustup.rs
[protoc]: https://github.com/protocolbuffers/protobuf/releases


...


<!-- ### Pipeline & Dataflow

1. execute the Node
1. execute the VM server _with second param - ip pointing to the Node_
1. send execute-request to VM from Node
1. apply results of execution. -->


...


- - -

## Contributing

Git-clone the project:
```
git clone --recurse-submodules https://github.com/WingsDao/move-vm-in-cosmos.git
cd move-vm-in-cosmos
```



### Build std-lib

Project can be built with custom standard library using executable `stdlib-builder`.

Format: `cargo run --bin stdlib-builder <path to stdlib directory> <compiler type [move, mvir]>`

Example: `cargo run --bin stdlib-builder stdlib/move move`


### Tests

To build and execute all tests just run `cargo test --all`.

#### Handy testing

1. execute the `ds-server` mock
1. execute the `server` _with second param - uri pointing to the `ds-server`_
1. execute the `client` mock _with param - uri pointing to the `server`_


### Submitting patches

Firstly, set up precommit hook to check all locally

```shell script
ln -s `git rev-parse --show-toplevel`/check_project.sh `git rev-parse --absolute-git-dir`/hooks/pre-commit
```

__TODO: describe contribution rules, license, etc...__


- - -

### Known Issues

Libra as dependency can crash with something like:
`"Failed to launch validator swarm: NodeCrash"` or `"Request failed grpc error: RpcFailure(RpcStatus { status: RpcStatusCode(14), details: Some(\"Name resolution failure\") })"`

As recommended on [Github issues page](https://github.com/libra/libra/issues/225),
try to increase amount of opened file descriptors for current user in operation system (probably it opens too much connections/logs):

```
ulimit -n 8192
```
