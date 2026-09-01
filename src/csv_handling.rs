use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::io::Write;

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

pub fn write_csv(clients: &HashMap<u16, Client>, path: &str) -> Result<(), Box<dyn Error>> {
    let output_path = path.replace("_input.csv", "_output.csv");

    let client_ids = clients.keys().copied().collect::<Vec<_>>();

    let mut file = File::create(&output_path)?;

    writeln!(file, "client, available, held, total, locked")?;
    for id in client_ids {
        let client = &clients[&id];
        writeln!(
            file,
            "{}, {}, {}, {}, {}",
            client.id, client.available, client.held, client.total, client.locked
        )?;
    }

    Ok(())
}
