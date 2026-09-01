use std::collections::HashMap;
use std::error::Error;
use std::io::{self, Write};

use crate::money::amount_to_string;
use crate::types::{Client, CsvRecord, Operation, OperationType};

pub fn process_operation(
    operation: &Operation,
    clients: &mut HashMap<u16, Client>,
    deposits: &mut HashMap<u32, Operation>,
    previous_tx_id: &mut u32,
) {
    let entry = clients.entry(operation.client).or_insert(Client {
        id: operation.client,
        available: 0,
        held: 0,
        total: 0,
        locked: false,
    });

    if entry.locked {
        eprintln!(
            "Client {} is locked; No operation will be processed",
            entry.id
        );
        return;
    }

    match operation.op_type {
        // increase available and total
        OperationType::Deposit { amount } => {
            if *previous_tx_id == operation.id {
                return;
            }
            entry.available += amount;
            entry.total += amount;

            *previous_tx_id = operation.id;
        }
        // decrease available and total
        OperationType::Withdrawal { amount } => {
            if *previous_tx_id == operation.id {
                return;
            }
            if entry.available < amount || entry.total < amount {
                // TODO: decide what should happen when a withdrawal would make available or total negative
                return;
            }
            entry.available -= amount;
            entry.total -= amount;

            *previous_tx_id = operation.id;
        }
        // takes disputed amount -> decrease available and increase held and mark transaction as disputed
        // NOTE: if there is no such transaction, we should just ignore it
        OperationType::Dispute => {
            if let Some(tx) = deposits.get_mut(&operation.id) {
                if tx.client != operation.client {
                    return;
                }
                if tx.is_disputed {
                    eprintln!(
                        "Tried to dispute transaction {} but it is already disputed",
                        operation.id
                    );
                    return;
                }
                let Some(amount) = tx.get_tx_amount() else {
                    return;
                };
                if entry.available < amount {
                    // TODO: decide what should happen when a dispute would make available negative
                    return;
                }
                entry.available -= amount;
                entry.held += amount;
                tx.is_disputed = true;
            }
        }
        // takes disputed amount -> checks if transaction is disputed and if so, increases available and decreases held and marks transaction as not disputed
        // NOTE: if there is no such transaction or it is not disputed, we should just ignore it
        OperationType::Resolve => {
            if let Some(tx) = deposits.get_mut(&operation.id) {
                if tx.client != operation.client {
                    return;
                }
                if !tx.is_disputed {
                    eprintln!(
                        "Tried to resolve transaction {} but it is not disputed",
                        operation.id
                    );
                    return;
                }
                let Some(amount) = tx.get_tx_amount() else {
                    return;
                };
                if entry.held < amount {
                    // TODO: decide what should happen when a resolve would make held negative
                    return;
                }
                entry.available += amount;
                entry.held -= amount;
                tx.is_disputed = false;
            }
        }
        // takes disputed amount -> checks if transaction is disputed and if so, decrease held and total and mark transaction as not disputed. Marks client as locked.
        // NOTE: if there is no such transaction or it is not disputed, we should just ignore it
        OperationType::Chargeback => {
            if let Some(tx) = deposits.get_mut(&operation.id) {
                if tx.client != operation.client {
                    return;
                }
                if !tx.is_disputed {
                    eprintln!(
                        "Tried to chargeback transaction {} but it is not disputed",
                        operation.id
                    );
                    return;
                }
                let Some(amount) = tx.get_tx_amount() else {
                    return;
                };
                if entry.held < amount || entry.total < amount {
                    // TODO: decide what should happen when a chargeback would make held or total negative
                    return;
                }
                entry.held -= amount;
                entry.total -= amount;
                entry.locked = true;
                tx.is_disputed = false;
            }
        }
    }
}

pub fn read_csv(path: &str) -> Result<HashMap<u16, Client>, Box<dyn Error>> {
    let mut clients = HashMap::new();

    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)?;

    // deposit transactions for disputes and chargebacks
    let mut deposits = HashMap::new();

    let mut previous_tx_id = 0;

    for result in reader.deserialize() {
        let record: CsvRecord = result?;
        let operation = Operation::try_from(record)?;
        process_operation(&operation, &mut clients, &mut deposits, &mut previous_tx_id);
        if matches!(operation.op_type, OperationType::Deposit { .. }) {
            deposits.entry(operation.id).or_insert(operation);
        }
    }
    Ok(clients)
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
