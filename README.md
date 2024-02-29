# String

String is a peer-to-peer chat application built from the ground up with security and privacy in
mind. It uses a custom protocol implemented in Rust and a client built with React and TypeScript
using Tauri.

## Installation

### Prerequisites

This project requires the following OS dependencies:

- Node v20
- NPM v8
- [Rust v1.75](https://www.rust-lang.org/tools/install)
- [Protoc v25 or later](https://grpc.io/docs/protoc-installation/)

> Please ensure that these are installed before continuing, otherwise things will break and fail to
> compile.

## Installing Dependencies

To install our JavaScript dependencies, run

```bash
npm install
```

in the **root directory** of the project.

## Prepare the Backend

In order to build, the backend requires the `protoc` compiler to generate the necessary Rust code
for the two Prisma clients, found in `crates/cache-prisma` and `crates/lighthouse-prisma`.

To do this, run the following from the project root.

```bash
cargo prisma generate --schema=./crates/cache-prisma/prisma/schema.prisma
cargo prisma generate --schema=./crates/lighthouse-prisma/prisma/schema.prisma
```

## License

This project is licensed under the GNU Affero General Public License v3.0 - see the
[LICENSE](LICENSE) file for details.
