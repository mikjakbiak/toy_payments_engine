use std::collections::HashMap;
use std::env;
use std::process;

mod money;
mod types;
use types::{Client, Operation, OperationType};

mod csv_handling;
use csv_handling::{read_csv, write_csv};

fn process_operations(
    operations: Vec<Operation>,
    transactions: &mut HashMap<u32, Operation>,
) -> HashMap<u16, Client> {
    let mut clients = HashMap::new();

    for op in operations {
        let entry = clients.entry(op.client).or_insert(Client {
            id: op.client,
            available: 0,
            held: 0,
            total: 0,
            locked: false,
        });

        if entry.locked {
            println!(
                "Client {} is locked; No operation will be processed",
                entry.id
            );
            continue;
        }

        match op.op_type {
            // increase available and total
            OperationType::Deposit { amount } => {
                entry.available += amount;
                entry.total += amount;
            }
            // decrease available and total
            OperationType::Withdrawal { amount } => {
                if entry.available < amount || entry.total < amount {
                    // TODO: decide what should happen when a withdrawal would make available or total negative
                    continue;
                }
                entry.available -= amount;
                entry.total -= amount;
            }
            // takes disputed amount -> decrease available and increase held and mark transaction as disputed
            // NOTE: if there is no such transaction, we should just ignore it
            OperationType::Dispute => {
                if let Some(tx) = transactions.get_mut(&op.id) {
                    if tx.is_disputed {
                        println!(
                            "Tried to dispute transaction {} but it is already disputed",
                            op.id
                        );
                        continue;
                    }
                    let Some(amount) = tx.get_tx_amount() else {
                        continue;
                    };
                    if entry.available < amount {
                        // TODO: decide what should happen when a dispute would make available negative
                        continue;
                    }
                    entry.available -= amount;
                    entry.held += amount;
                    tx.is_disputed = true;
                }
            }
            // takes disputed amount -> checks if transaction is disputed and if so, increases available and decreases held and marks transaction as not disputed
            // NOTE: if there is no such transaction or it is not disputed, we should just ignore it
            OperationType::Resolve => {
                if let Some(tx) = transactions.get_mut(&op.id) {
                    if !tx.is_disputed {
                        println!(
                            "Tried to resolve transaction {} but it is not disputed",
                            op.id
                        );
                        continue;
                    }
                    let Some(amount) = tx.get_tx_amount() else {
                        continue;
                    };
                    if entry.held < amount {
                        // TODO: decide what should happen when a resolve would make held negative
                        continue;
                    }
                    entry.available += amount;
                    entry.held -= amount;
                    tx.is_disputed = false;
                }
            }
            // takes disputed amount -> checks if transaction is disputed and if so, decrease held and total and mark transaction as not disputed. Marks client as locked.
            // NOTE: if there is no such transaction or it is not disputed, we should just ignore it
            OperationType::Chargeback => {
                if let Some(tx) = transactions.get_mut(&op.id) {
                    if !tx.is_disputed {
                        println!(
                            "Tried to chargeback transaction {} but it is not disputed",
                            op.id
                        );
                        continue;
                    }
                    let Some(amount) = tx.get_tx_amount() else {
                        continue;
                    };
                    if entry.held < amount || entry.total < amount {
                        // TODO: decide what should happen when a chargeback would make held or total negative
                        continue;
                    }
                    entry.held -= amount;
                    entry.total -= amount;
                    entry.locked = true;
                    tx.is_disputed = false;
                }
            }
        }
    }

    clients
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: {} <operations.csv>", env::args().next().unwrap());
        process::exit(1);
    });

    let (operations, mut transactions) = match read_csv(&path) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("error reading operations: {err}");
            process::exit(1);
        }
    };

    println!("{operations:#?}, {}", operations.len());

    let clients = process_operations(operations, &mut transactions);

    println!("{clients:#?}");

    match write_csv(&clients, &path) {
        Ok(()) => println!("CSV written successfully"),
        Err(err) => {
            eprintln!("error writing CSV: {err}");
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deposit(id: u32, client: u16, amount: u64) -> Operation {
        Operation {
            id,
            op_type: OperationType::Deposit { amount },
            client,
            is_disputed: false,
        }
    }

    fn withdrawal(id: u32, client: u16, amount: u64) -> Operation {
        Operation {
            id,
            op_type: OperationType::Withdrawal { amount },
            client,
            is_disputed: false,
        }
    }

    fn dispute(tx_id: u32, client: u16) -> Operation {
        Operation {
            id: tx_id,
            op_type: OperationType::Dispute,
            client,
            is_disputed: false,
        }
    }

    fn resolve(tx_id: u32, client: u16) -> Operation {
        Operation {
            id: tx_id,
            op_type: OperationType::Resolve,
            client,
            is_disputed: false,
        }
    }

    fn chargeback(tx_id: u32, client: u16) -> Operation {
        Operation {
            id: tx_id,
            op_type: OperationType::Chargeback,
            client,
            is_disputed: false,
        }
    }

    fn client_state(clients: &HashMap<u16, Client>, id: u16) -> (u64, u64, u64, bool) {
        let client = clients.get(&id).expect("client should exist");
        (client.available, client.held, client.total, client.locked)
    }

    #[test]
    fn deposit_increases_available_and_total() {
        let operations = vec![deposit(1, 1, 100_000)];
        let mut transactions = HashMap::from([(1, operations[0].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert_eq!(client_state(&clients, 1), (100_000, 0, 100_000, false));
    }

    #[test]
    fn withdrawal_decreases_available_and_total() {
        let operations = vec![deposit(1, 1, 100_000), withdrawal(2, 1, 40_000)];
        let mut transactions =
            HashMap::from([(1, operations[0].clone()), (2, operations[1].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert_eq!(client_state(&clients, 1), (60_000, 0, 60_000, false));
    }

    #[test]
    fn dispute_marks_transaction_and_moves_funds_to_held() {
        let operations = vec![deposit(1, 1, 100_000), dispute(1, 1)];
        let mut transactions = HashMap::from([(1, operations[0].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert!(transactions.get(&1).unwrap().is_disputed);
        assert_eq!(client_state(&clients, 1), (0, 100_000, 100_000, false));
    }

    #[test]
    fn resolve_unmarks_dispute_and_returns_funds_to_available() {
        let operations = vec![
            deposit(1, 1, 100_000),
            deposit(2, 1, 50_000),
            dispute(1, 1),
            dispute(2, 1),
            resolve(1, 1),
        ];
        let mut transactions =
            HashMap::from([(1, operations[0].clone()), (2, operations[1].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert!(!transactions.get(&1).unwrap().is_disputed);
        assert!(transactions.get(&2).unwrap().is_disputed);
        assert_eq!(client_state(&clients, 1), (100_000, 50_000, 150_000, false));
    }

    #[test]
    fn chargeback_unmarks_dispute_and_decreases_held_and_total() {
        let operations = vec![
            deposit(1, 1, 100_000),
            deposit(2, 1, 50_000),
            dispute(1, 1),
            dispute(2, 1),
            chargeback(1, 1),
        ];
        let mut transactions =
            HashMap::from([(1, operations[0].clone()), (2, operations[1].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert!(!transactions.get(&1).unwrap().is_disputed);
        assert!(transactions.get(&2).unwrap().is_disputed);
        assert_eq!(client_state(&clients, 1), (0, 50_000, 50_000, true));
    }

    #[test]
    fn dispute_ignores_non_existing_transaction() {
        let operations = vec![deposit(1, 1, 100_000), dispute(99, 1)];
        let mut transactions = HashMap::from([(1, operations[0].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert_eq!(client_state(&clients, 1), (100_000, 0, 100_000, false));
    }

    #[test]
    fn resolve_ignores_non_existing_transaction() {
        let operations = vec![deposit(1, 1, 100_000), resolve(99, 1)];
        let mut transactions = HashMap::from([(1, operations[0].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert_eq!(client_state(&clients, 1), (100_000, 0, 100_000, false));
        assert!(!transactions.get(&1).unwrap().is_disputed);
    }

    #[test]
    fn chargeback_ignores_non_existing_transaction() {
        let operations = vec![deposit(1, 1, 100_000), chargeback(99, 1)];
        let mut transactions = HashMap::from([(1, operations[0].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert_eq!(client_state(&clients, 1), (100_000, 0, 100_000, false));
        assert!(!transactions.get(&1).unwrap().is_disputed);
    }

    #[test]
    fn resolve_ignores_non_disputed_transaction() {
        let operations = vec![deposit(1, 1, 100_000), resolve(1, 1)];
        let mut transactions = HashMap::from([(1, operations[0].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert!(!transactions.get(&1).unwrap().is_disputed);
        assert_eq!(client_state(&clients, 1), (100_000, 0, 100_000, false));
    }

    #[test]
    fn chargeback_ignores_non_disputed_transaction() {
        let operations = vec![deposit(1, 1, 100_000), chargeback(1, 1)];
        let mut transactions = HashMap::from([(1, operations[0].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert!(!transactions.get(&1).unwrap().is_disputed);
        assert_eq!(client_state(&clients, 1), (100_000, 0, 100_000, false));
    }

    #[test]
    fn locked_client_is_ignored() {
        let operations = vec![
            deposit(1, 1, 100_000),
            deposit(2, 1, 50_000),
            dispute(1, 1),
            dispute(2, 1),
            chargeback(1, 1),
            deposit(3, 1, 1_000_000),
            withdrawal(4, 1, 10_000),
            deposit(5, 2, 100_000),
            dispute(2, 1),
            resolve(2, 1),
            chargeback(2, 1),
            withdrawal(6, 2, 50_000),
        ];
        let mut transactions = HashMap::from([
            (1, operations[0].clone()),
            (2, operations[1].clone()),
            (3, operations[5].clone()),
            (4, operations[6].clone()),
            (5, operations[7].clone()),
            (6, operations[11].clone()),
        ]);

        let clients = process_operations(operations, &mut transactions);

        assert!(!transactions.get(&1).unwrap().is_disputed);
        assert!(transactions.get(&2).unwrap().is_disputed);
        assert_eq!(client_state(&clients, 1), (0, 50_000, 50_000, true));
        assert_eq!(client_state(&clients, 2), (50_000, 0, 50_000, false));
    }
}
