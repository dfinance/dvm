# DVM - Dfinance Virtual Machine

![](https://github.com/dfinance/dvm/workflows/Tests/badge.svg)
![](https://github.com/dfinance/dvm/workflows/Audit/badge.svg)


<!-- TODO: short description -->


<!-- ## Overview -->
<!-- TODO: describe project structure, architecture principles, etc.. -->


## Related Repositories

- [Dnode][] - Dfinance Blockchain node.
- [PegZone][] - PegZone smart contracts.
- [OracleApp][] - oracle node, that fetch price feed from exchanges.

[Dnode]: https://github.com/dfinance/dnode
[PegZone]: https://github.com/dfinance/eth-peg-zone
[OracleApp]: https://github.com/dfinance/oracle-app


<!-- ## Documentation -->
<!-- - [Usage](https://docs.dfinance.co/move_vm) read how use DVM with Dnode -->


## Installation

There are two ways to install and try DVM - [with Docker](#the-docker-way) or [with Rust-toolchain](#installation-with-rust--cargo).


### The Docker way

You can use this schema for your docker-compose to run everything at once:

```yaml
version: '3.7'
services:
  dvm-compiler:
    container_name: dvm-compiler
    image: dfinance/dvm
    restart: always
    network_mode: host
    command: ./compiler "0.0.0.0:50053" "http://127.0.0.1:50052"
  dvm-server:
    container_name: dvm-server
    image: dfinance/dvm
    restart: always
    network_mode: host
    command: ./dvm "0.0.0.0:50051" "http://127.0.0.1:50052"
```

Or you can pull container from docker hub and run it by yourself:

```bash
# pull the latest containers
docker pull dfinance/dvm
```

That is how you do it:

```bash
# run compiler
docker run -d --rm --network host --name compiler -p 50053:50053 registry.wings.toys/dfinance/dvm:master ./compiler "0.0.0.0:50053" "http://127.0.0.1:50052"

# run virtual machine
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

To launch the DVM server use this command:

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


### Configuration actual for both

#### Positional arguments:

__DVM__ and __compiler__ requires second positional argument described as `<data-source address>`.
This is URI of a data source server, typically [Dnode][], local or external.
This argument can be ommited because we'll read the `DVM_DATA_SOURCE` [environment variable][environment variables] as fallback.

Positional arguments have higher priority than [environment variables][], so overrides their if specified.

For example:

```bash
# using env var:
DVM_DATA_SOURCE="http://[::1]:50052" dvm "[::1]:50051"
# or using positional arg:
dvm "[::1]:50051" "http://[::1]:50052"
# both is same
```

But env vars used just as fallback, so args are higher prioritised.
```bash
DVM_DATA_SOURCE="http://[::1]:42" dvm "[::1]:50051" "http://[::1]:50052"
# There DVM will listen port 50051
# and connect to data source on 50052 port
# ignoring env variable.
```

[Dnode]: https://github.com/dfinance/dnode


#### Environment variables:

- `DVM_DATA_SOURCE` - Data-source address.
  Used if relevant positional argument isn't specified.
- `DVM_LOG` - Log filters. The same as standard `RUST_LOG` environment variable.
  Possible values in verbosity ordering: `error`, `warn`, `info`, `debug` and `trace`.
  For complex filters see [documentation](https://docs.rs/env_logger/#filtering-results)
- `DVM_LOG_STYLE` - Log colors. The same as standard `RUST_LOG_STYLE`.
  Possible values in verbosity ordering: `auto`, `always`, `never`.
- `DVM_SENTRY_DSN` - Optional key-uri, enables crash logging service integration.
  If value ommited, crash logging service will not be initialized.
  E.g.: `DVM_SENTRY_DSN=https://your-dsn@uri dvm "[::1]:50051"`
- `DVM_SENTRY_ENVIRONMENT` - Sets the environment code to separate events from testnet and production.
  Optional. Works with Sentry integration.
  E.g.: `DVM_SENTRY_ENVIRONMENT="testnet"`


### Optional arguments:

Optional arguments have higher priority than [environment variables][], so overrides their if specified.

- `--log` - same as `DVM_LOG`
- `--log-color` - same as `DVM_LOG_STYLE`
- `--sentry-dsn` - same as `DVM_SENTRY_DSN`
- `--sentry-env` - same as `DVM_SENTRY_ENVIRONMENT`

[environment variables]: #environment-variables

For more info run dvm or compiler with `--help`.


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

List of contributors [is here](https://github.com/dfinance/dvm/graphs/contributors).

To help project you always can [open issue](https://github.com/dfinance/dvm/issues/new) or fork, modify code in your own fork and open [pull request](https://github.com/dfinance/dvm/pulls).


Useful precommit-hook to check changes locally:

```bash
ln -s `git rev-parse --show-toplevel`/check_project.sh `git rev-parse --absolute-git-dir`/hooks/pre-commit
```


## License

Copyright © 2020 Wings Stiftung

This program is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for more details.

You should have received a copy of the [GNU General Public License](https://github.com/dfinance/dvm/blob/master/LICENSE) along with this program.  If not, see <http://www.gnu.org/licenses/>.
