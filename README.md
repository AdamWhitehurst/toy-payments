# Toy CSV Payments Engine

## Overview

A toy project that maintains a ledger of account (like a bank's ledger). Takes a csv file path argument, processes the rows as transactions into the Ledger, then outputs the final state of the accounts. Invalid csv rows are ignored.

## Usage

`cargo run -- transactions.csv > out.csv`

## CSV format example

```csv
type, client, tx, amount
deposit, 1, 1, 1.0
withdrawal, 1, 2, 1.0
deposit, 2, 3, 1.0
withdrawal, 2, 4, 1.0
dispute, 2, 4
resolve, 2, 4
```

## Supported Transaction Types

- `deposit` - Putting funds into an account
- `withdrawal` - Taking funds out of an account
- `dispute` - Challenge a transaction by ID
- `resolve` - Concludes a dispute by giving back held funds
- `chargeback` - Reverses a disputed transaction and locks account
