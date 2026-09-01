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
    mut transactions: HashMap<u32, Operation>,
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
                println!("dispute");
            }
            // takes disputed amount -> checks if transaction is disputed and if so, increases available and decreases held and marks transaction as not disputed
            // NOTE: if there is no such transaction or it is not disputed, we should just ignore it
            OperationType::Resolve => {
                println!("resolve");
            }
            // takes disputed amount -> checks if transaction is disputed and if so, decrease held and total and mark transaction as not disputed
            // NOTE: if there is no such transaction or it is not disputed, we should just ignore it
            OperationType::Chargeback => {
                println!("chargeback");
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

    let (operations, transactions) = match read_operations(&path) {
        Ok(result) => result,
        Err(err) => {
            eprintln!("error reading operations: {err}");
            process::exit(1);
        }
    };

    println!("{operations:#?}");

    let clients = process_operations(operations, transactions);

    println!("{clients:#?}");
}
