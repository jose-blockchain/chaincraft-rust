# Chaincraft Rust

<!--
[![Crates.io](https://img.shields.io/crates/v/chaincraft-rust.svg)](https://crates.io/crates/chaincraft-rust)
[![Documentation](https://docs.rs/chaincraft-rust/badge.svg)](https://docs.rs/chaincraft-rust)
[![Build Status](https://github.com/chaincraft-org/chaincraft-rust/workflows/CI/badge.svg)](https://github.com/chaincraft-org/chaincraft-rust/actions)
-->
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Overview

A high-performance Rust-based platform for blockchain education and prototyping. Chaincraft Rust provides a clean, well-documented implementation of core blockchain concepts with a focus on performance, security, and educational value.

## Features

- **High Performance**: Built with Rust for maximum performance and memory safety
- **Educational Focus**: Well-documented code with clear explanations of blockchain concepts
- **Modular Design**: Pluggable consensus mechanisms, storage backends, and network protocols
- **Cryptographic Primitives**: Support for multiple signature schemes (Ed25519, ECDSA/secp256k1)
- **Network Protocol**: P2P networking with peer discovery and message propagation
- **Flexible Storage**: Memory and persistent storage options with optional SQLite indexing
- **CLI Interface**: Easy-to-use command-line interface for node management

## Quick Start

### Installation

#### From Crates.io

```bash
cargo install chaincraft-rust
```

#### From Source

```bash
git clone https://github.com/jio-gl/chaincraft-rust.git
cd chaincraft-rust
cargo build --release
```

### Running a Node

Start a Chaincraft node with default settings:

```bash
chaincraft-cli start
```

Or with custom configuration:

```bash
chaincraft-cli start --port 8080 --max-peers 20 --debug
```

### Generate a Keypair

```bash
chaincraft-cli keygen
```

## Usage as a Library

Add Chaincraft Rust to your `Cargo.toml`:

```toml
[dependencies]
chaincraft-rust = "0.1.0"
```

### Basic Example

```rust
use chaincraft_rust::{ChaincraftNode, Result};

#[tokio::main]
async fn main() -> Result<()> {
    let mut node = ChaincraftNode::builder()
        .port(21000)
        .max_peers(10)
        .build()?;

    println!("Starting node {} on port {}", node.id(), node.port());
    
    node.start().await?;
    
    // Your application logic here
    
    node.stop().await?;
    Ok(())
}
```

### Advanced Configuration

```rust
use chaincraft_rust::{ChaincraftNode, Result};
use chaincraft_rust::crypto::KeyType;

#[tokio::main]
async fn main() -> Result<()> {
    let mut node = ChaincraftNode::builder()
        .port(21000)
        .max_peers(50)
        .enable_compression()
        .enable_persistent_storage()
        .key_type(KeyType::Ed25519)
        .build()?;

    node.start().await?;
    
    // Node is now running with persistent storage and compression enabled
    
    Ok(())
}
```

## Architecture

Chaincraft Rust is built with a modular architecture:

- **Core**: Basic blockchain data structures and validation logic
- **Consensus**: Pluggable consensus mechanisms (currently implementing PoS)
- **Network**: P2P networking layer with peer discovery
- **Storage**: Flexible storage backends (memory, persistent, indexed)
- **Crypto**: Cryptographic primitives and utilities
- **CLI**: Command-line interface for node management

## Features

### Default Features

- `compression`: Enable message compression for network efficiency

### Optional Features

- `persistent`: Enable persistent storage using sled
- `indexing`: Enable SQLite-based transaction indexing
- `vdf-crypto`: Enable VDF (Verifiable Delay Function) support

Enable features in your `Cargo.toml`:

```toml
[dependencies]
chaincraft-rust = { version = "0.1.0", features = ["persistent", "indexing"] }
```

## Development

### Prerequisites

- Rust 1.70 or later
- Git

### Building

```bash
git clone https://github.com/chaincraft-org/chaincraft-rust.git
cd chaincraft-rust
cargo build
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with all features enabled
cargo test --all-features

# Run integration tests
cargo test --test integration
```

### Running Benchmarks

```bash
cargo bench
```

### Code Coverage

```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

## API Documentation

Full API documentation is available on [docs.rs](https://docs.rs/chaincraft-rust).

To build documentation locally:

```bash
cargo doc --open --all-features
```

## Examples

Check out the `examples/` directory for more usage examples:

- `basic_node.rs`: Simple node setup and operation
- `keypair_generation.rs`: Cryptographic keypair generation and signing
- `chatroom_example.rs`: Decentralized chatroom protocol (create rooms, post messages)
- `randomness_beacon_example.rs`: Verifiable randomness beacon
- `shared_objects_example.rs`: Multi-node network with shared object propagation
- `proof_of_work_example.rs`: Proof of Work mining and verification

Run examples with:

```bash
cargo run --example basic_node
cargo run --example chatroom_example
cargo run --example shared_objects_example
```

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Workflow

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Add tests for your changes
5. Ensure all tests pass (`cargo test --all-features`)
6. Run clippy (`cargo clippy --all-features`)
7. Format your code (`cargo fmt`)
8. Commit your changes (`git commit -am 'Add amazing feature'`)
9. Push to the branch (`git push origin feature/amazing-feature`)
10. Open a Pull Request

## Performance

Chaincraft Rust is designed for high performance:

- Zero-copy serialization where possible
- Efficient async networking with tokio
- Optimized cryptographic operations
- Configurable resource limits

## Security

Security is a top priority:

- Memory-safe Rust implementation
- Cryptographic operations use well-audited libraries
- Network protocol includes message authentication
- Input validation and sanitization throughout

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [Tokio](https://tokio.rs/) for async runtime
- Cryptography powered by [RustCrypto](https://github.com/RustCrypto)
- P2P networking with [libp2p](https://libp2p.io/)

## Roadmap

- [X] Advanced consensus mechanisms (PBFT)
- [ ] Smart contract support
- [ ] Enhanced monitoring and metrics
- [ ] WebAssembly runtime integration

---

For more information, visit our [documentation](https://docs.rs/chaincraft-rust) or [repository](https://github.com/chaincraft-org/chaincraft-rust). 
