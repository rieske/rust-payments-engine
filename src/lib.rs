use rust_decimal::Decimal;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;
use std::io::Write;

type ClientId = u16;
type TransactionId = u32;
type Amount = Decimal;

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum TransactionType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Deserialize)]
struct Transaction {
    #[serde(rename = "tx")]
    id: TransactionId,
    #[serde(rename = "type")]
    tx_type: TransactionType,
    #[serde(rename = "client")]
    client_id: ClientId,
    #[serde(rename = "amount")]
    amount: Option<Amount>,
}

#[derive(Serialize)]
struct Account {
    #[serde(rename = "client")]
    client_id: ClientId,
    available: Amount,
    held: Amount,
    total: Amount,
    locked: bool,
    #[serde(skip_serializing)]
    transactions: BTreeMap<TransactionId, Amount>,
    #[serde(skip_serializing)]
    disputes: BTreeMap<TransactionId, Amount>,
}

impl Account {
    fn deposit(&mut self, id: TransactionId, amount: Amount) {
        // TODO: log to stderr if deposit of negative amount is encountered?
        if amount.is_sign_positive() {
            self.add(id, amount);
        }
    }

    fn withdraw(&mut self, id: TransactionId, amount: Amount) {
        // TODO: log to stderr if withdrawal of negative amount is encountered?
        if amount.is_sign_positive() {
            self.add(id, -amount);
        }
    }

    fn dispute(&mut self, id: TransactionId) {
        if let Some(disputed_amount) = self.transactions.get(&id) {
            self.available -= disputed_amount;
            self.held += disputed_amount;
            self.disputes.insert(id, *disputed_amount);
        }
        // TODO: should we log something to stderr if there was nothing to dispute?
    }

    fn resolve(&mut self, id: TransactionId) {
        if let Some(disputed_amount) = self.disputes.remove(&id) {
            self.available += disputed_amount;
            self.held -= disputed_amount;
        }
        // TODO: should we log something to stderr if there was no dispute?
    }

    fn chargeback(&mut self, id: TransactionId) {
        if let Some(disputed_amount) = self.disputes.remove(&id) {
            self.held -= disputed_amount;
            self.total -= disputed_amount;
            self.locked = true;
        }
        // TODO: should we log something to stderr if there was no dispute?
    }

    fn add(&mut self, id: TransactionId, amount: Amount) {
        let new_total = self.total + amount;
        // Only adjust balance if the new total amount is not negative. Ignore the transaction otherwise.
        if !self.locked && !new_total.is_sign_negative() {
            self.available += amount;
            self.total = new_total;
            self.transactions.insert(id, amount);
        }
        // TODO: should we log something to stderr when a transaction can not be handled?
    }
}

struct PaymentsEngine {
    accounts: BTreeMap<ClientId, Account>,
}

impl PaymentsEngine {
    fn process_transaction(&mut self, transaction: Transaction) {
        let account = self
            .accounts
            .entry(transaction.client_id)
            .or_insert_with(|| Account {
                client_id: transaction.client_id,
                available: Decimal::new(0, 4),
                held: Decimal::new(0, 4),
                total: Decimal::new(0, 4),
                locked: false,
                transactions: BTreeMap::new(),
                disputes: BTreeMap::new(),
            });

        match transaction.tx_type {
            TransactionType::Deposit => {
                account.deposit(transaction.id, transaction.amount.unwrap())
            }
            TransactionType::Withdrawal => {
                account.withdraw(transaction.id, transaction.amount.unwrap())
            }
            TransactionType::Dispute => account.dispute(transaction.id),
            TransactionType::Resolve => account.resolve(transaction.id),
            TransactionType::Chargeback => account.chargeback(transaction.id),
        }
    }
}

pub fn process_transactions_csv(
    transactions_csv: impl Read,
    output: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_reader(transactions_csv);

    let headers = rdr.byte_headers()?.clone();
    let mut raw_record = csv::ByteRecord::new();

    let mut payments_engine = PaymentsEngine {
        accounts: BTreeMap::new(),
    };
    while rdr.read_byte_record(&mut raw_record)? {
        let transaction: Transaction = raw_record.deserialize(Some(&headers))?;

        payments_engine.process_transaction(transaction);
    }

    write_account_states_as_csv(payments_engine.accounts, output)
}

fn write_account_states_as_csv(
    accounts: BTreeMap<ClientId, Account>,
    output: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let mut wtr = csv::Writer::from_writer(output);

    for account in accounts.values() {
        if !account.transactions.is_empty() {
            wtr.serialize(account)?;
        }
    }

    wtr.flush()?;

    Ok(())
}
