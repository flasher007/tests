# Solana Block Monitor and Transfer Tool


Create a `config.yaml` file with the following structure:

```yaml
sender_key: "[1,2,3,...]"  # Private key in JSON array format
recipient: "recipient_address"
amount_sol: 0.1  # Amount to send in SOL
grpc_endpoint: "https://grpc.ny.shyft.to"
grpc_api_key: "your-api-key"
```

### Supported Private Key Formats

1. **JSON Array Format**
   ```yaml
   sender_key: "[1,2,3,...]"  # Array of 64 bytes
   ```

2. **Hex String Format**
   ```yaml
   sender_key: "0x1234..."    # With or without 0x prefix
   ```

3. **Base58 String Format**
   ```yaml
   sender_key: "base58..."    # Standard Solana base58 format
   ```

## Installation

1. Install Rust and Cargo if you haven't already:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. Clone the repository and build the project:
   ```bash
   cargo build --release
   ```

## Usage

1. Create your `config.yaml` file with your settings

2. Run the program:
   ```bash
   cargo run --release
   ```

The program will:
- Connect to Geyser GRPC
- Monitor for new blocks
- Send SOL transactions when new blocks are detected
- Print transaction signatures and status
