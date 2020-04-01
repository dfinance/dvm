# Definance Virtual Machine

<!-- ![](https://github.com/dfinance/dvm/workflows/Tests/badge.svg) -->

![](https://github.com/dfinance/dvm/workflows/Audit/badge.svg)


<!-- TODO: short description -->


<!-- ## Overview -->
<!-- TODO: describe project structure, architecture principles, etc.. -->


## Installation

There is two ways to install and try DVM - [with Docker](#the-docker-way) or [with Rust-toolchain](#installation-with-rust--cargo).

### The Docker-Way

```bash
# pull the latest containers
docker pull registry.wings.toys/dfinance/dvm:master
```

Or the same for compose:

```yaml
version: '3.7'
services:
  dvm-compiler:
    container_name: dvm-compiler
    image: registry.wings.toys/dfinance/dvm:master
    restart: always
    network_mode: host
    command: ./compiler "0.0.0.0:50053" "http://127.0.0.1:50052"
  dvm-server:
    container_name: dvm-server
    image: registry.wings.toys/dfinance/dvm:master
    restart: always
    network_mode: host
    command: ./dvm "0.0.0.0:50051" "http://127.0.0.1:50052"
```

#### Containers Usage

```bash
# execute compiler
docker run -d --rm --network host --name compiler -p 50053:50053 registry.wings.toys/dfinance/dvm:master ./compiler "0.0.0.0:50053" "http://127.0.0.1:50052"

# execute virtual machine
docker run -d --rm --network host --name dvm -p 50051:50051 registry.wings.toys/dfinance/dvm:master ./dvm "0.0.0.0:50051" "http://127.0.0.1:50052"
```

```bash
# stop compiler and vm
docker stop compiler
docker stop dvm
```

Check out [Usage](#Usage) part for more info.


- - - - - - - - - -


### Installation with Rust 🦀 Cargo

<!-- TODO: type here something -->


### Prerequisites

- install [Rust][], the easiest way to get it is to use [Rustup][]
- install [protoc][]

[Rust]: https://www.rust-lang.org
[Rustup]: https://rustup.rs
[protoc]: https://github.com/protocolbuffers/protobuf/releases


### Build and Install

To install using `cargo` run the following command:

```bash
cargo install --git https://github.com/dfinance/dvm.git
```

As result you will get the following executables into your `.cargo/bin` directory:

- `dvm` - virtual machine server
- `compiler` - compilation server
- `stdlib-builder` - standard library builder (useful for genesis creation)

Uninstallation: `cargo uninstall dvm`.


## Usage

> Note: Following instructions are for standalone binary executables. If you want to use `cargo` to build & run, just add `cargo run --bin ` at the start of the mentioned command.


### DVM server

`dvm` is a Move/Mvir virtual machine gRPC server.
API described in [protobuf schemas][].

To launch the DVM server run following command:

```bash
# format: <which host:port to listen> <data-source address>
dvm "[::1]:50051" "http://[::1]:50052"
```


### Compilation server

`compiler` is a Move/Mvir compilation gRPC server.
API described in [protobuf schemas][].

To launch the compilation server run:

```bash
# format: <which host:port to listen> <data-source address>
compiler "[::1]:50053" "http://[::1]:50052"
```

> Compiler supports Move lang as well as Mvir.

[protobuf schemas]: https://github.com/dfinance/dvm-proto/tree/master/protos


### Stdlib Builder

`stdlib-builder` is a standard library builder.

To build standard library run:

```bash
# format:   <source directory> [-o output-file] [--verbose] [-p] [--help]`
# print output to stdout:
stdlib-builder lang/stdlib -p
# or write output to the file:
stdlib-builder lang/stdlib -po ./stdlib.json
```

To build your stdlib run:

```bash
stdlib-builder /path-to-your/std-lib -po ./stdlib.json
```

> Note: currently supports Mvir only.


- - - - - - - - - -


## Development

Just clone [this repo][] and hack some:

```bash
# clone the repository
git clone https://github.com/dfinance/dvm.git

cd dvm

# build and run vm
cargo run --bin dvm -- --help
# build and run compiler
cargo run --bin compiler -- --help
```


<!-- TODO: guide for contributors should be here -->



[this repo]: https://github.com/dfinance/dvm


### Tests

To launch tests run:

```bash
cargo test --all
```

### Contributors

This project has the [following contributors](https://github.com/dfinance/dvm/graphs/contributors).

To help project you always can open [issue](https://github.com/dfinance/dvm/pulls) or fork, do changes in your own fork and open [pull request](https://github.com/dfinance/dvm/pulls).


Useful precommit-hook for check all locally:

```bash
ln -s `git rev-parse --show-toplevel`/check_project.sh `git rev-parse --absolute-git-dir`/hooks/pre-commit
```



# License

Copyright © 2020 Wings Foundation

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the [GNU General Public License](https://github.com/dfinance/dvm/blob/master/LICENSE) along with this program.  If not, see <http://www.gnu.org/licenses/>.
