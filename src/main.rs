use std::env;
use std::process;

mod money;
mod types;

mod csv_handling;
use csv_handling::{read_csv, write_clients_stdout};

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: {} <operations.csv>", env::args().next().unwrap());
        process::exit(1);
    });

    let clients = match read_csv(&path) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("error reading operations: {err}");
            process::exit(1);
        }
    };

    if let Err(err) = write_clients_stdout(&clients) {
        eprintln!("error writing output: {err}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use csv_handling::process_operation;
    use std::collections::HashMap;
    use types::{Client, Operation, OperationType};

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

    fn run_operations(
        operations: Vec<Operation>,
    ) -> (HashMap<u16, Client>, HashMap<u32, Operation>) {
        let mut clients = HashMap::new();
        let mut deposits = HashMap::new();

        for operation in operations {
            process_operation(&operation, &mut clients, &mut deposits);
            if matches!(operation.op_type, OperationType::Deposit { .. }) {
                deposits.entry(operation.id).or_insert(operation);
            }
        }

        (clients, deposits)
    }

    #[test]
    fn deposit_increases_available_and_total() {
        let operations = vec![deposit(1, 1, 100_000)];
        let (clients, _deposits) = run_operations(operations);

        assert_eq!(client_state(&clients, 1), (100_000, 0, 100_000, false));
    }

    #[test]
    fn withdrawal_decreases_available_and_total() {
        let operations = vec![deposit(1, 1, 100_000), withdrawal(2, 1, 40_000)];
        let (clients, _deposits) = run_operations(operations);

        assert_eq!(client_state(&clients, 1), (60_000, 0, 60_000, false));
    }

    #[test]
    fn dispute_marks_transaction_and_moves_funds_to_held() {
        let operations = vec![deposit(1, 1, 100_000), dispute(1, 1)];
        let (clients, deposits) = run_operations(operations);

        assert!(deposits.get(&1).unwrap().is_disputed);
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
        let (clients, deposits) = run_operations(operations);

        assert!(!deposits.get(&1).unwrap().is_disputed);
        assert!(deposits.get(&2).unwrap().is_disputed);
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
        let (clients, deposits) = run_operations(operations);

        assert!(!deposits.get(&1).unwrap().is_disputed);
        assert!(deposits.get(&2).unwrap().is_disputed);
        assert_eq!(client_state(&clients, 1), (0, 50_000, 50_000, true));
    }

    #[test]
    fn dispute_ignores_non_existing_transaction() {
        let operations = vec![deposit(1, 1, 100_000), dispute(99, 1)];
        let (clients, _deposits) = run_operations(operations);

        assert_eq!(client_state(&clients, 1), (100_000, 0, 100_000, false));
    }

    #[test]
    fn resolve_ignores_non_existing_transaction() {
        let operations = vec![deposit(1, 1, 100_000), resolve(99, 1)];
        let (clients, deposits) = run_operations(operations);

        assert_eq!(client_state(&clients, 1), (100_000, 0, 100_000, false));
        assert!(!deposits.get(&1).unwrap().is_disputed);
    }

    #[test]
    fn chargeback_ignores_non_existing_transaction() {
        let operations = vec![deposit(1, 1, 100_000), chargeback(99, 1)];
        let (clients, deposits) = run_operations(operations);

        assert_eq!(client_state(&clients, 1), (100_000, 0, 100_000, false));
        assert!(!deposits.get(&1).unwrap().is_disputed);
    }

    #[test]
    fn resolve_ignores_non_disputed_transaction() {
        let operations = vec![deposit(1, 1, 100_000), resolve(1, 1)];
        let (clients, deposits) = run_operations(operations);

        assert!(!deposits.get(&1).unwrap().is_disputed);
        assert_eq!(client_state(&clients, 1), (100_000, 0, 100_000, false));
    }

    #[test]
    fn chargeback_ignores_non_disputed_transaction() {
        let operations = vec![deposit(1, 1, 100_000), chargeback(1, 1)];
        let (clients, deposits) = run_operations(operations);

        assert!(!deposits.get(&1).unwrap().is_disputed);
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
        let (clients, deposits) = run_operations(operations);

        assert!(!deposits.get(&1).unwrap().is_disputed);
        assert!(deposits.get(&2).unwrap().is_disputed);
        assert_eq!(client_state(&clients, 1), (0, 50_000, 50_000, true));
        assert_eq!(client_state(&clients, 2), (50_000, 0, 50_000, false));
    }
}
