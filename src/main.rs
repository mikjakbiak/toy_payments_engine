use serde::Deserialize;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::process;

#[derive(Debug, Clone)]
enum OperationType {
    Deposit { amount: f64 },
    Withdrawal { amount: f64 },
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Clone)]
struct Operation {
    id: u32,
    op_type: OperationType,
    client: u16,
    is_disputed: bool,
}

#[derive(Debug)]
struct Client {
    id: u16,
    available: f64,
    held: f64,
    total: f64,
    locked: bool,
}

#[derive(Deserialize)]
struct CsvRecord {
    #[serde(rename = "type")]
    op_type: String,
    client: u16,
    tx: u32,
    amount: Option<f64>,
}

impl TryFrom<CsvRecord> for Operation {
    type Error = String;

    fn try_from(record: CsvRecord) -> Result<Self, Self::Error> {
        let op_type = match record.op_type.as_str() {
            "deposit" => OperationType::Deposit {
                amount: record
                    .amount
                    .ok_or_else(|| format!("deposit operation {} missing amount", record.tx))?,
            },
            "withdrawal" => OperationType::Withdrawal {
                amount: record
                    .amount
                    .ok_or_else(|| format!("withdrawal operation {} missing amount", record.tx))?,
            },
            "dispute" => OperationType::Dispute,
            "resolve" => OperationType::Resolve,
            "chargeback" => OperationType::Chargeback,
            other => return Err(format!("unknown operation type: {other}")),
        };

        Ok(Operation {
            id: record.tx,
            op_type,
            client: record.client,
            is_disputed: false,
        })
    }
}

impl Operation {
    fn get_tx_amount(&self) -> Option<f64> {
        match self.op_type {
            OperationType::Deposit { amount } | OperationType::Withdrawal { amount } => {
                Some(amount)
            }
            _ => None,
        }
    }
}

fn read_operations(
    path: &str,
) -> Result<(Vec<Operation>, HashMap<u32, Operation>), Box<dyn Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)?;

    // contains all operations
    let mut operations = Vec::new();
    // map of balance mutating operations == transactions
    let mut transactions = HashMap::new();

    for result in reader.deserialize() {
        let record: CsvRecord = result?;
        let operation = Operation::try_from(record)?;
        match operation.op_type {
            OperationType::Deposit { .. } => {
                transactions
                    .entry(operation.id)
                    .or_insert(operation.clone());
            }
            OperationType::Withdrawal { .. } => {
                transactions
                    .entry(operation.id)
                    .or_insert(operation.clone());
            }
            _ => {}
        }
        operations.push(operation);
    }
    Ok((operations, transactions))
}

fn process_operations(
    operations: Vec<Operation>,
    transactions: &mut HashMap<u32, Operation>,
) -> HashMap<u16, Client> {
    let mut clients = HashMap::new();

    for op in operations {
        let entry = clients.entry(op.client).or_insert(Client {
            id: op.client,
            available: 0.0,
            held: 0.0,
            total: 0.0,
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

    let (operations, mut transactions) = match read_operations(&path) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("error reading operations: {err}");
            process::exit(1);
        }
    };

    println!("{operations:#?}");

    let clients = process_operations(operations, &mut transactions);

    println!("{clients:#?}");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn deposit(id: u32, client: u16, amount: f64) -> Operation {
        Operation {
            id,
            op_type: OperationType::Deposit { amount },
            client,
            is_disputed: false,
        }
    }

    fn withdrawal(id: u32, client: u16, amount: f64) -> Operation {
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

    fn client_state(clients: &HashMap<u16, Client>, id: u16) -> (f64, f64, f64, bool) {
        let client = clients.get(&id).expect("client should exist");
        (client.available, client.held, client.total, client.locked)
    }

    #[test]
    fn deposit_increases_available_and_total() {
        let operations = vec![deposit(1, 1, 10.0)];
        let mut transactions = HashMap::from([(1, operations[0].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert_eq!(client_state(&clients, 1), (10.0, 0.0, 10.0, false));
    }

    #[test]
    fn withdrawal_decreases_available_and_total() {
        let operations = vec![deposit(1, 1, 10.0), withdrawal(2, 1, 4.0)];
        let mut transactions =
            HashMap::from([(1, operations[0].clone()), (2, operations[1].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert_eq!(client_state(&clients, 1), (6.0, 0.0, 6.0, false));
    }

    #[test]
    fn dispute_marks_transaction_and_moves_funds_to_held() {
        let operations = vec![deposit(1, 1, 10.0), dispute(1, 1)];
        let mut transactions = HashMap::from([(1, operations[0].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert!(transactions.get(&1).unwrap().is_disputed);
        assert_eq!(client_state(&clients, 1), (0.0, 10.0, 10.0, false));
    }

    #[test]
    fn resolve_unmarks_dispute_and_returns_funds_to_available() {
        let operations = vec![
            deposit(1, 1, 10.0),
            deposit(2, 1, 5.0),
            dispute(1, 1),
            dispute(2, 1),
            resolve(1, 1),
        ];
        let mut transactions =
            HashMap::from([(1, operations[0].clone()), (2, operations[1].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert!(!transactions.get(&1).unwrap().is_disputed);
        assert!(transactions.get(&2).unwrap().is_disputed);
        assert_eq!(client_state(&clients, 1), (10.0, 5.0, 15.0, false));
    }

    #[test]
    fn chargeback_unmarks_dispute_and_decreases_held_and_total() {
        let operations = vec![
            deposit(1, 1, 10.0),
            deposit(2, 1, 5.0),
            dispute(1, 1),
            dispute(2, 1),
            chargeback(1, 1),
        ];
        let mut transactions =
            HashMap::from([(1, operations[0].clone()), (2, operations[1].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert!(!transactions.get(&1).unwrap().is_disputed);
        assert!(transactions.get(&2).unwrap().is_disputed);
        assert_eq!(client_state(&clients, 1), (0.0, 5.0, 5.0, true));
    }

    #[test]
    fn dispute_ignores_non_existing_transaction() {
        let operations = vec![deposit(1, 1, 10.0), dispute(99, 1)];
        let mut transactions = HashMap::from([(1, operations[0].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert_eq!(client_state(&clients, 1), (10.0, 0.0, 10.0, false));
    }

    #[test]
    fn resolve_ignores_non_existing_transaction() {
        let operations = vec![deposit(1, 1, 10.0), resolve(99, 1)];
        let mut transactions = HashMap::from([(1, operations[0].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert_eq!(client_state(&clients, 1), (10.0, 0.0, 10.0, false));
        assert!(!transactions.get(&1).unwrap().is_disputed);
    }

    #[test]
    fn chargeback_ignores_non_existing_transaction() {
        let operations = vec![deposit(1, 1, 10.0), chargeback(99, 1)];
        let mut transactions = HashMap::from([(1, operations[0].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert_eq!(client_state(&clients, 1), (10.0, 0.0, 10.0, false));
        assert!(!transactions.get(&1).unwrap().is_disputed);
    }

    #[test]
    fn resolve_ignores_non_disputed_transaction() {
        let operations = vec![deposit(1, 1, 10.0), resolve(1, 1)];
        let mut transactions = HashMap::from([(1, operations[0].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert!(!transactions.get(&1).unwrap().is_disputed);
        assert_eq!(client_state(&clients, 1), (10.0, 0.0, 10.0, false));
    }

    #[test]
    fn chargeback_ignores_non_disputed_transaction() {
        let operations = vec![deposit(1, 1, 10.0), chargeback(1, 1)];
        let mut transactions = HashMap::from([(1, operations[0].clone())]);

        let clients = process_operations(operations, &mut transactions);

        assert!(!transactions.get(&1).unwrap().is_disputed);
        assert_eq!(client_state(&clients, 1), (10.0, 0.0, 10.0, false));
    }

    #[test]
    fn locked_client_is_ignored() {
        let operations = vec![
            deposit(1, 1, 10.0),
            deposit(2, 1, 5.0),
            dispute(1, 1),
            dispute(2, 1),
            chargeback(1, 1),
            deposit(3, 1, 100.0),
            withdrawal(4, 1, 1.0),
            deposit(5, 2, 10.0),
            dispute(2, 1),
            resolve(2, 1),
            chargeback(2, 1),
            withdrawal(6, 2, 5.0),
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
        assert_eq!(client_state(&clients, 1), (0.0, 5.0, 5.0, true));
        assert_eq!(client_state(&clients, 2), (5.0, 0.0, 5.0, false));
    }
}
