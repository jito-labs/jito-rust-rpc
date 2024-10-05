# jito-sdk-rust

[![Discord](https://img.shields.io/discord/938287290806042626?label=Discord&logo=discord&style=flat&color=7289DA)](https://discord.gg/jTSmEzaR)
![Rust](https://img.shields.io/badge/Rust-Language-orange?logo=rust)
![Crates.io](https://img.shields.io/crates/v/jito_sdk_rust?label=crates.io&logo=rust)
[![docs.rs](https://img.shields.io/badge/docs.rs-jito_sdk_rust-blue?logo=rust)](https://docs.rs/jito-sdk-rust/0.1.0/jito_sdk_rust/)

The Jito JSON-RPC Rust SDK provides an interface for interacting with Jito's enhanced Solana infrastructure. This SDK supports methods for managing bundles and transactions, offering improved performance and additional features while interacting with the Block Engine.

## Features

### Bundles
- `getInflightBundleStatuses`: Retrieve the status of in-flight bundles.
- `getBundleStatuses`: Fetch the statuses of submitted bundles.
- `getTipAccounts`: Get accounts eligible for tips.
- `sendBundle`: Submit bundles to the Jito Block Engine.

### Transactions
- `sendTransaction`: Submit transactions with enhanced priority and speed.

## Installation

### Prerequisites

This project requires Rust for development. If you haven't installed Rust yet, follow these steps:

1. **Install Rust**:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Follow the on-screen instructions to complete the installation.

3. Verify the installation:
   ```bash
   rustc --version
   ```

4. (Optional but recommended) Keep Rust up to date:
   ```bash
   rustup update
   ```

### Adding jito-sdk-rust to Your Project

Add the following to your `Cargo.toml`:

```toml
[dependencies]
jito-sdk-rust = "0.1.0"  # Replace with the actual version
```

## Usage Examples

### Basic Transaction Example

To run the basic transaction example:

1. Ensure your environment is set up in `jito-rust-rpc/examples/basic_txn.rs`:

   ```rust
   // Load the sender's keypair
   let sender = Keypair::read_from_file(("/path/to/wallet.json").expect("Failed to read wallet file");

   // Set up receiver pubkey
   let receiver = Pubkey::from_str("YOUR_RECEIVER_PUBKEY")?;
   ```

2. Run the example:
   ```bash
   cargo run --example basic_txn
   ```

### Basic Bundle Example

To run the basic bundle example:

1. Set up your environment in `jito-rust-rpc/examples/basic_bundle.rs`:

   ```rust
   // Load the sender's keypair
   let sender = Keypair::read_from_file(("/path/to/wallet.json").expect("Failed to read wallet file");

   // Set up receiver pubkey
   let receiver = Pubkey::from_str("YOUR_RECEIVER_PUBKEY")?;
   ```

2. Run the example:
   ```bash
   cargo run --example basic_bundle
   ```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Support

For support, please join our [Discord community](https://discord.gg/jTSmEzaR).
