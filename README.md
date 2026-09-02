# Toy Payments Engine

A small command-line payments engine written in Rust. It reads a CSV file of financial operations, applies them in order, and prints the resulting client balances to stdout.

The engine supports five operation types: **deposit**, **withdrawal**, **dispute**, **resolve**, and **chargeback**, and tracks each client's `available`, `held`, and `total` funds, plus whether the account is `locked`.

### Operations

- **Deposit** - adds funds to a client's `available` and `total`.
- **Withdrawal** - removes funds from `available` and `total`.
- **Dispute** - references an existing deposit by transaction ID. Moves the disputed amount from `available` to `held` and marks the deposit as disputed. Only applies to deposits belonging to the same client.
- **Resolve** - reverses a dispute: moves the amount from `held` back to `available` and clears the disputed flag.
- **Chargeback** - finalises a dispute against the client: removes the amount from `held` and `total`, clears the disputed flag, and **locks** the client account.

A typical dispute flow looks like: deposit -> dispute -> resolve (funds returned) **or** chargeback (funds removed, client locked).

## Assumptions

1. If client is locked then following operations on their account are ignored.
2. No balance can go negative so withdrawals, disputes, resolves and chargebacks are ignored if any balance would go below 0.
3. Transactions arrive in chronological order.

## How to run

```
cargo run -- ./test_files/1_test_input.csv > ./test_files/1_test_output.csv
```

### Input

The input is a CSV file with a header row and one operation per line:

```
type, client, tx, amount
deposit, 1, 1, 10.0
deposit, 1, 2, 5.0
withdrawal, 1, 3, 3.5
dispute, 1, 1,
resolve, 1, 1,
dispute, 1, 2,
chargeback, 1, 2,
```

| Column   | Description                                                                                                    |
| -------- | -------------------------------------------------------------------------------------------------------------- |
| `type`   | One of `deposit`, `withdrawal`, `dispute`, `resolve`, or `chargeback`                                          |
| `client` | Unique client ID (integer)                                                                                     |
| `tx`     | Unique transaction ID. For dispute/resolve/chargeback, this references the original deposit's `tx`             |
| `amount` | Required for deposit and withdrawal (up to 4 decimal places). Leave empty for dispute, resolve, and chargeback |

See the files in `/test_files` for more examples.

### Output

The engine writes client balances to **stdout**, one row per client, sorted by client ID:

```
client, available, held, total, locked
1, 10, 0, 10, false
2, 5.5, 2, 7.5, false
```

Amounts are printed as decimals with trailing zeros removed (e.g. `50` instead of `50.0`, `14.62` instead of `14.6200`). Redirect stdout to a file to save the result:

```
cargo run -- ./test_files/1_test_input.csv > ./test_files/1_test_output.csv
```

## How to test

```
cargo test
```

## Known problem

- Just ignoring withdrawals, disputes, resolves and chargebacks to prevent underflow is not ideal. It'd need more design work to decide how to approach this problem. Maybe partial withdrawals should be supported.
