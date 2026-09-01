use serde::Deserialize;

#[derive(Debug, Clone)]
pub enum OperationType {
    Deposit { amount: f64 },
    Withdrawal { amount: f64 },
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Clone)]
pub struct Operation {
    pub id: u32,
    pub op_type: OperationType,
    pub client: u16,
    pub is_disputed: bool,
}

#[derive(Debug)]
pub struct Client {
    pub id: u16,
    pub available: f64,
    pub held: f64,
    pub total: f64,
    pub locked: bool,
}

#[derive(Deserialize)]
pub struct CsvRecord {
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
    pub fn get_tx_amount(&self) -> Option<f64> {
        match self.op_type {
            OperationType::Deposit { amount } | OperationType::Withdrawal { amount } => {
                Some(amount)
            }
            _ => None,
        }
    }
}
