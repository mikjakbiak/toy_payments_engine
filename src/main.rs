use serde::Deserialize;
use std::env;
use std::error::Error;
use std::process;

#[derive(Debug)]
enum TransactionType {
    Deposit { amount: f64 },
    Withdrawal { amount: f64 },
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug)]
struct Transaction {
    id: u32,
    tx_type: TransactionType,
    client: u16,
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
    tx_type: String,
    client: u16,
    tx: u32,
    amount: Option<f64>,
}

impl TryFrom<CsvRecord> for Transaction {
    type Error = String;

    fn try_from(record: CsvRecord) -> Result<Self, Self::Error> {
        let tx_type = match record.tx_type.as_str() {
            "deposit" => TransactionType::Deposit {
                amount: record
                    .amount
                    .ok_or_else(|| format!("deposit tx {} missing amount", record.tx))?,
            },
            "withdrawal" => TransactionType::Withdrawal {
                amount: record
                    .amount
                    .ok_or_else(|| format!("withdrawal tx {} missing amount", record.tx))?,
            },
            "dispute" => TransactionType::Dispute,
            "resolve" => TransactionType::Resolve,
            "chargeback" => TransactionType::Chargeback,
            other => return Err(format!("unknown transaction type: {other}")),
        };

        Ok(Transaction {
            id: record.tx,
            tx_type,
            client: record.client,
        })
    }
}

fn read_transactions(path: &str) -> Result<Vec<Transaction>, Box<dyn Error>> {
    let mut reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(path)?;

    let mut transactions = Vec::new();
    for result in reader.deserialize() {
        let record: CsvRecord = result?;
        transactions.push(Transaction::try_from(record)?);
    }
    Ok(transactions)
}

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Usage: {} <transactions.csv>", env::args().next().unwrap());
        process::exit(1);
    });

    match read_transactions(&path) {
        Ok(transactions) => println!("{transactions:#?}"),
        Err(err) => {
            eprintln!("error reading transactions: {err}");
            process::exit(1);
        }
    }
}
