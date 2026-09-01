use std::collections::HashMap;
use std::error::Error;
use std::io::{self, Write};

use crate::money::amount_to_string;
use crate::types::{Client, CsvRecord, Operation, OperationType};

pub fn read_csv(path: &str) -> Result<(Vec<Operation>, HashMap<u32, Operation>), Box<dyn Error>> {
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

pub fn write_clients_stdout(clients: &HashMap<u16, Client>) -> Result<(), Box<dyn Error>> {
    let mut client_ids = clients.keys().copied().collect::<Vec<_>>();
    client_ids.sort();

    let mut stdout = io::stdout().lock();

    writeln!(stdout, "client, available, held, total, locked")?;
    for id in client_ids {
        let client = &clients[&id];
        writeln!(
            stdout,
            "{}, {}, {}, {}, {}",
            client.id,
            amount_to_string(client.available),
            amount_to_string(client.held),
            amount_to_string(client.total),
            client.locked
        )?;
    }

    Ok(())
}
