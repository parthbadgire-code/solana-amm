# ⚡ Solana Constant Product AMM (CPMM) with Dynamic Fees & Protocol Treasury

A high-performance, production-grade Automated Market Maker (AMM) on Solana built with Anchor. Implements a Constant Product Market Maker ($x \cdot y = k$) natively from first principles without external curve dependencies, featuring dynamic fee distribution, automated treasury protocol fee routing, emergency circuit breakers, and end-to-end unit and integration test coverage.

---

## 📑 Table of Contents

- [Overview](#-overview)
- [Key Features](#-key-features)
- [Architecture & Account Model](#-architecture--account-model)
- [CPMM Mathematical Formulation (Challenge 4)](#-cpmm-mathematical-formulation-challenge-4)
- [Fee Structure & Treasury Mechanics](#-fee-structure--treasury-mechanics)
- [Instruction Reference](#-instruction-reference)
- [Mitigating Downtime in DeFi Applications (Challenge 5)](#-mitigating-downtime-in-defi-applications-challenge-5)
- [Test Suite & Verification](#-test-suite--verification)
- [Directory Structure](#-directory-structure)
- [Getting Started](#-getting-started)

---

## 🌟 Overview

This project implements a decentralized exchange liquidity pool using the Constant Product invariant ($x \cdot y = k$). The smart contract is designed with an emphasis on mathematical precision, defensive state management, and protocol security:

- **Zero-Dependency Math**: Implements square root, invariant checks, fee calculations, and rounding controls directly in Rust with `u128` precision, avoiding rounding errors and eliminating external curve crate vulnerabilities.
- **Treasury Routing**: Protocol fees are routed directly to dedicated treasury token accounts during every swap, while LP fees remain in the pool to compound returns for liquidity providers.
- **Circuit Breakers**: Features emergency pause functionality (`locked`) to freeze trading and deposits during network anomalies, oracle failures, or market stress.
- **Stack & Heap Optimization**: Account structures use boxed wrappers (`Box<Account<'info, ...>>`) to comply strictly with Solana's 4KB stack frame limit.

---

## 🚀 Key Features

| Feature | Description |
|---|---|
| **Constant Product Curve** | Pure native implementation of $x \cdot y = k$ with integer square root via the Babylonian / Newton-Raphson method. |
| **Bi-Directional Swaps** | Swap Token X for Token Y, or Token Y for Token X with exact fee splits and slippage validation. |
| **Dynamic Fee Split** | Configurable total fee and protocol fee (basis points). Protocol cut goes to treasury; remainder compounds in the pool. |
| **Slippage Protection** | Strict bounds enforced on all actions: `min_amount_out` on swaps, `max_x` / `max_y` on deposits, and `min_x` / `min_y` on withdrawals. |
| **Emergency Circuit Breaker** | Admin authority can pause operations instantly (`set_locked(true)`) to protect user funds during market turbulence. |
| **Stack Safety** | Heavy account structs are heap-allocated via Solana's bump allocator to guarantee stack frame safety ($< 4096$ bytes). |

---

## 🏛 Architecture & Account Model

### Program Derived Addresses (PDAs)

1. **Config PDA**: `["config", seed.to_le_bytes()]`  
   Holds pool metadata, fees, authority, treasury address, and circuit breaker state.
2. **LP Mint PDA**: `["lp", config.key()]`  
   The SPL token mint representing liquidity provider shares. The `config` PDA is the sole mint authority.
3. **Vault Accounts**: Associated Token Accounts (ATAs) owned by `config` PDA:
   - `vault_x`: ATA for `mint_x` owned by `config`
   - `vault_y`: ATA for `mint_y` owned by `config`
4. **Treasury Accounts**: ATAs owned by the `treasury` wallet:
   - `treasury_x`: Receives protocol fees from Token X swaps
   - `treasury_y`: Receives protocol fees from Token Y swaps

### System Architecture Diagram

```
                              +--------------------------+
                              |      Admin Authority     |
                              +-------------+------------+
                                            |
                         update_fee / update_treasury / set_locked
                                            |
                                            v
+------------------+         +-------------------------------+
|  Trader / User   | ------> |          Config PDA           | <-------+
+--------+---------+  swap   |  ["config", seed.to_le_bytes] |         |
         |                   +---------------+---------------+         |
         |                                   |                         |
         |                         mints / burns LP shares             |
         |                                   v                         |
         |                   +-------------------------------+         |
         |                   |          LP Mint PDA          |         |
         |                   |      ["lp", config.key]       |         |
         |                   +-------------------------------+         |
         |                                                             |
         | transfers in/out                                            |
         v                                                             v
+------------------+         +---------------+---------------+   +-----+-----+
|  Treasury ATAs   | <------+ |  Pool Vault X |  Pool Vault Y |   |    LP     |
| (Protocol Fees)  | protocol| (Token X ATA) | (Token Y ATA) |   | Provider  |
+------------------+   fee   +---------------+---------------+   +-----------+
```

---

## 📐 CPMM Mathematical Formulation (Challenge 4)

Rather than importing third-party libraries (such as `constant-product-curve` or `spl-token-swap`), all mathematical operations are implemented in `programs/solana-amm/src/curve.rs` with extensive unit testing.

### 1. Integer Square Root (Newton-Raphson Method)

For initial liquidity provisioning, we calculate:

$$L_0 = \lfloor \sqrt{x \cdot y} \rfloor$$

To prevent precision degradation and avoid floating-point math in the SBF environment, we use an iterative Babylonian integer square root algorithm operating on `u128`:

```rust
pub fn sqrt_u128(n: u128) -> u128 {
    if n == 0 { return 0; }
    let mut x0 = n / 2;
    if x0 == 0 { return 1; }
    let mut x1 = (x0 + n / x0) / 2;
    while x1 < x0 {
        x0 = x1;
        x1 = (x0 + n / x0) / 2;
    }
    x0
}
```

### 2. Subsequent Liquidity Provisioning

When liquidity already exists ($L > 0$), adding liquidity requires depositing tokens in proportion to current reserves ($X, Y$):

$$\Delta x = \left\lceil \frac{\Delta L \cdot X}{L} \right\rceil = \left\lfloor \frac{\Delta L \cdot X + L - 1}{L} \right\rfloor$$

$$\Delta y = \left\lceil \frac{\Delta L \cdot Y}{L} \right\rceil = \left\lfloor \frac{\Delta L \cdot Y + L - 1}{L} \right\rfloor$$

**Defensive Rounding:** Ceiling division is used for deposit requirements. This guarantees that new liquidity providers cannot dilute existing liquidity providers by depositing fewer tokens than required.

### 3. Liquidity Withdrawal

When burning $\Delta L$ LP tokens, the withdrawer receives:

$$\Delta x = \left\lfloor \frac{\Delta L \cdot X}{L} \right\rfloor, \quad \Delta y = \left\lfloor \frac{\Delta L \cdot Y}{L} \right\rfloor$$

**Defensive Rounding:** Floor division is applied so the withdrawing user never extracts more than their exact share, maintaining pool solvency at all times.

### 4. Swap Calculation with Fee Splitting

Given an input amount $\Delta x$, total fee $F_{\text{total}}$ in basis points, and protocol fee $F_{\text{protocol}}$ in basis points (where $F_{\text{protocol}} \le F_{\text{total}}$):

1. **Total Fee:**  
   $$\text{Fee}_{\text{total}} = \left\lfloor \frac{\Delta x \cdot F_{\text{total}}}{10\,000} \right\rfloor$$

2. **Protocol Fee (Treasury):**  
   $$\text{Fee}_{\text{protocol}} = \left\lfloor \frac{\Delta x \cdot F_{\text{protocol}}}{10\,000} \right\rfloor$$

3. **LP Fee (Retained in Pool):**  
   $$\text{Fee}_{\text{LP}} = \text{Fee}_{\text{total}} - \text{Fee}_{\text{protocol}}$$

4. **Effective Swap Amount:**  
   $$\Delta x_{\text{eff}} = \Delta x - \text{Fee}_{\text{total}}$$

5. **Output Token Amount:**  
   $$\Delta y = \left\lfloor \frac{Y \cdot \Delta x_{\text{eff}}}{X + \Delta x_{\text{eff}}} \right\rfloor$$

### 5. Invariant Invariance & Monotonic Growth Proof

The constant product formula requires:

$$k_{\text{new}} \ge k_{\text{old}}$$

- Before swap: $k_{\text{old}} = X \cdot Y$
- After swap:
  $$X_{\text{new}} = X + \Delta x - \text{Fee}_{\text{protocol}} = X + \Delta x_{\text{eff}} + \text{Fee}_{\text{LP}}$$
  $$Y_{\text{new}} = Y - \Delta y$$

Substituting $\Delta y$:

$$(X + \Delta x_{\text{eff}}) \cdot (Y - \Delta y) = (X + \Delta x_{\text{eff}}) \cdot \left( Y - \left\lfloor \frac{Y \cdot \Delta x_{\text{eff}}}{X + \Delta x_{\text{eff}}} \right\rfloor \right) \ge X \cdot Y$$

Because $\text{Fee}_{\text{LP}} \ge 0$, we have:

$$X_{\text{new}} \cdot Y_{\text{new}} \ge (X + \Delta x_{\text{eff}}) \cdot Y_{\text{new}} \ge X \cdot Y = k_{\text{old}}$$

Thus, the invariant $k$ strictly increases after every swap, directly benefiting liquidity providers.

---

## 💰 Fee Structure & Treasury Mechanics

Fees are parameterized in basis points ($1 \text{ bps} = 0.01\%$):

| Parameter | Default (Tests) | Max Permitted | Destination |
|---|---|---|---|
| **Total Fee (`fee`)** | 30 bps (0.30%) | 1000 bps (10.00%) | Deducted from trader's input |
| **Protocol Fee (`protocol_fee`)** | 5 bps (0.05%) | $\le \text{fee}$ | Transferred to `treasury_x` or `treasury_y` |
| **LP Share** | 25 bps (0.25%) | Balance | Retained inside pool vault to grow $k$ |

### On-Chain Transfer Security
During swaps, protocol fees are sent directly from the user's token account to the treasury's verified token account:
- Constraints verify `treasury_x.mint == config.mint_x` and `treasury_x.owner == config.treasury`.
- If $\text{Fee}_{\text{protocol}} == 0$, the transfer is skipped to conserve compute units.

---

## 🕹 Instruction Reference

### 1. `initialize`
Initializes a new AMM pool instance.
- **Arguments**: `seed: u64`, `fee: u16`, `protocol_fee: u16`, `authority: Pubkey`
- **Accounts**: `initializer`, `mint_x`, `mint_y`, `mint_lp` (PDA), `vault_x` (PDA ATA), `vault_y` (PDA ATA), `treasury` (AccountInfo), `config` (PDA), token programs.

### 2. `deposit`
Adds liquidity to the pool.
- **Arguments**: `amount_lp: u64`, `max_x: u64`, `max_y: u64`
- **Behavior**:
  - If pool is empty: bootstraps pool with `max_x` and `max_y`, minting $\lfloor \sqrt{x \cdot y} \rfloor$ LP tokens.
  - If pool has liquidity: computes required $dx, dy$ to mint `amount_lp`. Rejects if $dx > \text{max\_x}$ or $dy > \text{max\_y}$.

### 3. `withdraw`
Removes liquidity by burning LP tokens.
- **Arguments**: `amount_lp: u64`, `min_x: u64`, `min_y: u64`
- **Behavior**: Burns `amount_lp` from withdrawer and transfers proportional $dx, dy$ from vaults. Rejects if $dx < \text{min\_x}$ or $dy < \text{min\_y}$.

### 4. `swap`
Swaps one token for another.
- **Arguments**: `amount_in: u64`, `min_amount_out: u64`, `is_x_to_y: bool`
- **Behavior**:
  - Computes fee breakdown and output amount $\Delta y$.
  - Rejects if $\Delta y < \text{min\_amount\_out}$ (slippage check).
  - Routes protocol fee to treasury ATA and remainder to pool vault.
  - Releases output tokens from pool vault to trader.

### 5. `update_fee`
Allows admin authority to update fee configuration.
- **Arguments**: `new_fee: u16`, `new_protocol_fee: u16`

### 6. `update_treasury`
Allows admin authority to update treasury address.
- **Arguments**: `new_treasury: Pubkey`

### 7. `set_locked`
Emergency circuit breaker toggle.
- **Arguments**: `locked: bool`
- **Behavior**: When `true`, prevents all `deposit` and `swap` instructions across the pool.

---

## 🛡 Mitigating Downtime in DeFi Applications (Challenge 5)

High availability and resilience are paramount in decentralized finance. On Solana, downtime is rarely a total halt of the validator network; rather, it manifests as **network congestion**, **transaction packet drops (TPU contention)**, **RPC desynchronization**, **priority fee spikes**, or **stale oracle dependencies**. 

Here is an in-depth operational blueprint for mitigating downtime across every layer of the DeFi stack:

```
+-----------------------------------------------------------------------------------+
|                           DEFI DOWNTIME MITIGATION MATRIX                         |
+-----------------------------------------------------------------------------------+
| 1. INFRASTRUCTURE & INGESTION   | Multi-RPC Failovers, Jito MEV Bundles, TPU Routing |
+---------------------------------+-------------------------------------------------+
| 2. PROGRAM & ON-CHAIN STATE    | Circuit Breakers (Pause), Volume Caps, Timelocks|
+---------------------------------+-------------------------------------------------+
| 3. ORACLE INTEGRITY             | Staleness Bounds, Confidence Bands, TWAP Guards |
+---------------------------------+-------------------------------------------------+
| 4. CLIENT & OFF-CHAIN KEEPERS   | Idempotent Nonces, LastValidBlockHeight Retries |
+-----------------------------------------------------------------------------------+
```

### 1. Transaction Landing & Congestion Resistance

- **Dynamic Priority Fees & Micro-Lamport Bumping:**  
  Solana uses local fee markets. During congestion (e.g. popular NFT mints or volatile liquidations), baseline transactions get dropped by TPU (Transaction Processing Unit) schedulers. Applications must query `getRecentPrioritizationFees` and append `ComputeBudgetProgram.setComputeUnitPrice({ microLamports })` dynamically to place transactions in the upper percentiles of leader queues.
- **Strict Compute Unit (CU) Budgeting:**  
  Over-requesting CUs penalizes transactions in validator scheduling. By profiling instructions and adding `ComputeBudgetProgram.setComputeUnitLimit({ units })` fitted to the exact consumption (e.g., ~45,000 CUs for swaps), transactions pack more efficiently into blocks.
- **Jito MEV Bundling:**  
  For critical transactions (liquidations, pool rebalancing, arbitrage), routing via Jito-Solana block builder bundles provides guaranteed inclusion without public mempool drops or front-running vulnerabilities.
- **Multi-RPC Failover Pools:**  
  Production backends must never rely on a single RPC endpoint. Deploy a client-side proxy (e.g. round-robin across Helius, Triton, and QuickNode) with active health checks on slot lag (`getSlot`), blockhash freshness, and WebSocket disconnection recovery.

### 2. On-Chain Circuit Breakers & Graceful Degradation

- **Emergency Pause (`locked` switch):**  
  As implemented in this AMM's `set_locked` instruction, the protocol authority can immediately freeze trading and deposits upon detecting anomalous market conditions, smart contract vulnerabilities, or extreme volatility.
- **Graceful "Withdraw-Only" Mode:**  
  If incoming trading is paused or oracles fail, protocols should preserve liquidity withdrawal rights ("escape hatch") so liquidity providers can reclaim capital without forced liquidation.
- **Epoch / Slot Volume Velocity Limits:**  
  Implement invariant limits on maximum pool turnover per slot to defend against flash-loan attacks or rapid drained reserves during low-liquidity downtime windows.
- **Multi-Sig Governance & Timelocks:**  
  Admin functions (`update_fee`, `update_treasury`, `set_locked`) must be governed by a multi-signature wallet (such as Squads Protocol) with automated timelocks for non-emergency parameters, preventing single-key compromise.

### 3. Oracle Staleness & Data Degradation

- **Confidence Interval Bounds:**  
  When integrating price feeds (e.g. Pyth or Switchboard), transactions must reject quotes where the confidence interval exceeds a safe percentage of the price ($\text{confidence} / \text{price} > 1\%$).
- **Maximum Slot Age Thresholds:**  
  Reject oracle prices if `current_slot - oracle_publish_slot > MAX_ORACLE_DELAY_SLOTS` (e.g., $> 25$ slots or ~10 seconds).
- **Time-Weighted Average Price (TWAP) Fallbacks:**  
  If real-time oracles become unreachable during an RPC or validator disruption, fallback to an internal TWAP calculated from historical pool reserves to prevent catastrophic mispricings.

### 4. Client-Side Idempotency & Keeper Redundancy

- **Idempotent Transaction Submission:**  
  Network timeouts often result in clients re-sending transactions that may have already succeeded on-chain. Employ deterministic transaction seeds (unique SPL transfer memos or client nonces) to ensure double-execution is impossible.
- **Blockhash Expiration Auditing:**  
  Track `lastValidBlockHeight` returned by `getLatestBlockhash`. Transactions must only be re-signed and rebroadcast once the previous blockhash is provably expired, preventing unexpected out-of-order execution.
- **Distributed Keeper Architecture:**  
  Deploy off-chain keeper bots across multiple cloud providers (AWS, GCP, bare metal) with distributed leader election (e.g. via Redis/Raft consensus) to ensure continuous monitoring and liquidation processing even if one data center experiences network outages.

---

## 🧪 Test Suite & Verification

The project includes **19 tests** (6 pure CPMM unit tests in Rust + 13 on-chain integration tests in TypeScript) covering every instruction, error path, and edge case.

### Test Execution Summary

```
$ anchor test

  Constant Product AMM Tests
    ✔ 1. Initialize AMM pool with fees and treasury (58ms)
    ✔ 2. Initial liquidity deposit (bootstrap CPMM pool) (24ms)
    ✔ 3. Subsequent liquidity deposit maintaining constant product ratio (18ms)
    ✔ 4. Deposit slippage protection: reject when required input exceeds max (12ms)
    ✔ 5. Swap Token X for Token Y with fee deduction and treasury protocol fee (26ms)
    ✔ 6. Swap Token Y for Token X with fee deduction and treasury protocol fee (22ms)
    ✔ 7. Swap slippage protection: reject when output is below min_amount_out (11ms)
    ✔ 8. Partial liquidity withdrawal by LP (19ms)
    ✔ 9. Withdrawal slippage protection: reject when min tokens requested exceeds share (10ms)
    ✔ 10. Admin: Update fee and protocol fee configuration (16ms)
    ✔ 11. Admin: Update treasury address (15ms)
    ✔ 12. Emergency Circuit Breaker: lock and unlock pool to mitigate downtime (29ms)
    ✔ 13. Full liquidity withdrawal (18ms)

  13 passing (208ms)
```

```
$ cargo test

running 6 tests
test curve::tests::test_sqrt ... ok
test curve::tests::test_initial_lp ... ok
test curve::tests::test_deposit_amounts_from_lp ... ok
test curve::tests::test_withdraw_amounts ... ok
test curve::tests::test_swap_output_and_fee_split ... ok
test curve::tests::test_invariant_increases_with_lp_fee ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### Passing Test Suite Screenshot

![All Tests Passing](./screenshots/tests-passing.png)

---

## 📁 Directory Structure

```
solana-amm/
├── Anchor.toml                      # Anchor workspace configuration
├── Cargo.toml                       # Workspace Cargo manifest
├── package.json                     # NPM test dependencies & scripts
├── tsconfig.json                    # TypeScript compiler configuration
├── programs/
│   └── solana-amm/
│       ├── Cargo.toml               # Program dependencies & idl-build features
│       └── src/
│           ├── lib.rs               # Program entrypoints & declare_id
│           ├── constants.rs         # BPS denominator, seed constants, max fees
│           ├── curve.rs             # Pure CPMM curve math, Newton sqrt & unit tests
│           ├── error.rs             # Custom AMM error definitions
│           ├── state.rs             # Config account struct & InitSpace
│           └── instructions/
│               ├── mod.rs           # Instruction exports
│               ├── initialize.rs    # Pool & vault initialization
│               ├── deposit.rs       # Liquidity provisioning & LP minting
│               ├── withdraw.rs      # Liquidity removal & LP burning
│               ├── swap.rs          # Constant product swap & fee routing
│               └── admin.rs         # Fee, treasury & emergency lock management
├── tests/
│   └── solana-amm.ts                # Full E2E TypeScript integration test suite
├── scripts/
│   └── generate-screenshot.js       # Screenshot rendering utility
└── screenshots/
    └── tests-passing.png            # Visual verification of passing tests
```

---

## 🛠 Getting Started

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) `1.80+`
- [Solana CLI](https://docs.solanalabs.com/cli/install) `1.18+` or `Agave 2.0+`
- [Anchor CLI](https://www.anchor-lang.com/docs/installation) `0.30+` or `1.0.0`
- [Node.js](https://nodejs.org/) `20+ LTS` and `npm`

### Installation & Build

1. Clone the repository and navigate to the project directory:
   ```bash
   cd solana-amm
   ```

2. Install Node dependencies:
   ```bash
   npm install
   ```

3. Build the smart contract and generate IDL:
   ```bash
   anchor build
   ```

4. Run native Rust unit tests (verifying pure CPMM math):
   ```bash
   cargo test
   ```

5. Run complete on-chain integration tests:
   ```bash
   anchor test
   ```

---

## 📄 License

This project is licensed under the MIT License.
