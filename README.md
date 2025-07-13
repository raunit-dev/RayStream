# RayStream

RayStream is a Rust application that monitors and parses transactions for the Raydium Launchpad smart contract on the Solana blockchain in real time. It connects to a Yellowstone gRPC endpoint, subscribes to transaction updates involving the Raydium Launchpad program, and identifies, parses, and logs specific instruction types and transaction details.

---

## Features

- **Real-time Monitoring:**  
  Connects to a Yellowstone gRPC endpoint and continuously listens for transaction updates involving the Raydium Launchpad program (`LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj`).

- **Instruction Parsing:**  
  Decodes and classifies Raydium Launchpad instructions, detecting types such as:
    - `Initialize`
    - `BuyExactIn`
    - `BuyExactOut`
    - `SellExactIn`
    - `SellExactOut`
    - `ClaimPlatformFee`
    - `ClaimVestedToken`
    - `CollectFee`
    - `CollectMigrateFee`
    - `CreateConfig`
    - `CreatePlatformConfig`
    - `CreateVestingAccount`
    - `MigrateToAmm`
    - `MigrateToCpswap`
    - `UpdateConfig`
    - `UpdatePlatformConfig`
    - and others, with fallback for unknown types.

- **Transaction Details Logging:**  
  For each relevant transaction, RayStream logs:
    - Transaction signature, status (success/failure), slot, and fee.
    - All involved account keys.
    - Each instruction with program and account details.
    - Token balance changes per account, before and after the transaction.
    - Complete transaction logs from the Solana runtime.

- **Targeted Instruction Detection:**  
  The application is able to focus on certain instruction types (e.g., `Initialize`, `MigrateToAmm`). When these are detected, it logs detailed information about the transaction and instruction index.

- **Robust Error Handling:**  
  Handles gRPC stream interruptions, unexpected messages, and transaction parsing errors gracefully, logging issues for debugging.

---

## How It Works

1. **Setup & Connection:**  
   On startup, RayStream sets up the environment and connects to a Yellowstone gRPC endpoint.

2. **Subscription:**  
   It sends a subscription request to listen specifically for transactions involving the Raydium Launchpad program.

3. **Stream Processing:**  
   For every transaction update received:
   - Parses the transaction using a custom parser.
   - Scans all outer and inner instructions for those belonging to the Raydium Launchpad program.
   - Extracts and classifies instruction types.
   - If a target instruction is found, prints detailed logs including transaction data and token balance changes.

4. **Extensible Instruction Handling:**  
   New instruction types can be added by updating the `RaydiumInstructionType` enum and its discriminator mapping.

---

## Usage

### Requirements

- Rust toolchain (`cargo`)
- Access to a Yellowstone gRPC endpoint

### Running

```sh
cargo run --release
```

The application will connect, subscribe, and begin logging Raydium Launchpad transactions in real time.

---

## File Structure

- `src/main.rs` - Application entrypoint, orchestrates setup and streaming.
- `src/client.rs` - Handles gRPC client creation and connection (not shown above, but implied).
- `src/subscription.rs` - Sends the subscription request to Yellowstone.
- `src/processing.rs` - Core logic for processing and parsing transaction updates, handling instruction decoding, and logging.
- `src/constants.rs` - Contains program IDs and configuration constants.

---

## Example Output

```
Starting to monitor account: LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj
Connected to gRPC endpoint
Subscription request sent. Listening for updates...
Found target instruction: Initialize at index 0
Found Raydium Launchpad transaction!
Parsed Transaction:
Transaction: 5X8...
Status: Success
Slot: 25678901
Fee: 5000 lamports

Account Keys:
  [0] 7GH...
  [1] LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj
  ...

Instructions:
  Instruction 0:
    Program: LanMV9sAd7... (index: 1)
    Accounts:
      ...
    Data: 64 bytes
...

Token Balances:
  Account 0 (7GH...): 1000 → 1200 (mint: RAY...)
  Account 2 (XyZ...): removed balance 500 (mint: USDC...)
...

Transaction Logs:
  [0] Program log: Instruction: Initialize
  ...
```

---

## Customization

- **Adding new instruction types:**  
  Extend the `RaydiumInstructionType` enum and its discriminator mapping in `processing.rs`.
- **Changing monitored program:**  
  Update `RAYDIUM_LAUNCHPAD_PROGRAM` in `constants.rs`.
- **Adjusting log level:**  
  Modify `RUST_LOG_LEVEL` in `constants.rs`.

---

## License

This project currently does not specify a license. Please add one if you intend to distribute or use this publicly.

---

## Authors

- [raunit-dev](https://github.com/raunit-dev)

---

**Note:**  
RayStream is designed for Solana developers, Raydium integrators, and blockchain analytics professionals needing deep insight into Raydium Launchpad on-chain activity.