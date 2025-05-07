# Solana Parallel Transfer Tool

A tool for parallel SOL transfers between multiple wallets using Solana JSON RPC API.

## Features

- Parallel processing of multiple transfers
- Support for multiple sender and recipient wallets
- Transaction status tracking
- Performance statistics
- Multiple private key formats support

## Configuration

Create a `config.yaml` file with the following structure:

```yaml
senders:
  - key: "[1,2,3,...]"  # Private key in JSON array format
    address: "9WzDXwBbmkg8ZTbNMqUxvQRAyrZzDsGYdLVL9zYtAWWM"
  - key: "0x1234..."    # Private key in hex format
    address: "HN7cABqLq46Es1jh92dQQisAq662SmxELLLsHHe4YWrH"
  - key: "base58..."    # Private key in base58 format
    address: "7i3waUWg8aXnSe6E4Hi7EqbuAXSm86tw4d21hPJNsoZv"

recipients:
  - "recipient_address_1"
  - "recipient_address_2"

amount_sol: 0.1  # Amount to send from each wallet in SOL
```

### Supported Private Key Formats

1. **JSON Array Format**
   ```yaml
   key: "[1,2,3,...]"  # Array of 64 bytes
   ```

2. **Hex String Format**
   ```yaml
   key: "0x1234..."    # With or without 0x prefix
   ```

3. **Base58 String Format**
   ```yaml
   key: "base58..."    # Standard Solana base58 format
   ```

## Usage

1. Install dependencies:
   ```bash
   cargo build
   ```

2. Configure your `config.yaml` file with sender and recipient addresses

3. Run the program:
   ```bash
   cargo run
   ```

## Output

The program will output:
- Transaction signatures
- Transaction statuses
- Processing time for each transfer
- Total processing time
- Number of successful transfers

Example output:
```
Transfer Summary:
=================
Successful transfers: 4

Successful Transfer Details:
=========================
From: AvW79pxfz5bbFLghgMmNWnV4FJnj2dvHZ4t9P86K5ryT
To: 7npDRsTxvsG5QFmEZr8znKYWPHuai3d5FUu1RrCwv1Wv
Signature: 3fi39Ezodh534FXj5voEiQF8Ltb952NVysj3i8cadMTV1jPxg4zNNwEqAYKnrRyexd8N12w8GbFQVn1jFhBuomU7
Time: 22.719s

From: J4SUJzUpPrDY6HGVWVEz1e6p8KLMAm2fXczcnokBdSwg
To: 7npDRsTxvsG5QFmEZr8znKYWPHuai3d5FUu1RrCwv1Wv
Signature: UKJedDG7vNsoYZmhhJaYUMiwnkBofkL12QgyaXPvzjeqUJbo9JaGPcLwvaCycZPsyjDFX41BjVQhU4PSYv2DoAu
Time: 22.772s

From: AvW79pxfz5bbFLghgMmNWnV4FJnj2dvHZ4t9P86K5ryT
To: 7npDRsTxvsG5QFmEZr8znKYWPHuai3d5FUu1RrCwv1Wv
Signature: 48P3abxfMyWASVZmqdWW3Naq6ihaGFz8Vsmjowk3oBafj7k4SAA2it9fuZMDhcPM7S5jUrWz6GgixCoXty4V9Ufy
Time: 33.660s

From: J4SUJzUpPrDY6HGVWVEz1e6p8KLMAm2fXczcnokBdSwg
To: 7npDRsTxvsG5QFmEZr8znKYWPHuai3d5FUu1RrCwv1Wv
Signature: UKJedDG7vNsoYZmhhJaYUMiwnkBofkL12QgyaXPvzjeqUJbo9JaGPcLwvaCycZPsyjDFX41BjVQhU4PSYv2DoAu
Time: 33.757s


Total processing time: 33.770s
```

## Security Notes

- Never commit your private keys to version control
- Consider using environment variables or secure key storage in production
- Test with small amounts first
- Keep your private keys secure and never share them 