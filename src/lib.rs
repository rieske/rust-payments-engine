use rust_decimal::Decimal;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::error::Error;
use std::io::Read;
use std::io::Write;
use std::str;

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
    fn create(client_id: ClientId) -> Account {
        Account {
            client_id: client_id,
            available: Decimal::new(0, 4),
            held: Decimal::new(0, 4),
            total: Decimal::new(0, 4),
            locked: false,
            transactions: BTreeMap::new(),
            disputes: BTreeMap::new(),
        }
    }

    fn process(&mut self, transaction: Transaction) {
        // TODO: I bet there is a more Rusty way to approach this. Traits something?
        //  on the other hand, with traits, I'd be creating more objects rather than just one
        //  transaction. How much of an overhead is this? Need to benchmark.
        match transaction.tx_type {
            TransactionType::Deposit => self.deposit(transaction.id, transaction.amount.unwrap()),
            TransactionType::Withdrawal => {
                self.withdraw(transaction.id, transaction.amount.unwrap())
            }
            TransactionType::Dispute => self.dispute(transaction.id),
            TransactionType::Resolve => self.resolve(transaction.id),
            TransactionType::Chargeback => self.chargeback(transaction.id),
        }
    }

    fn deposit(&mut self, id: TransactionId, amount: Amount) {
        // TODO: output to stderr if deposit of negative amount is encountered?
        if amount.is_sign_positive() {
            self.add(id, amount);
        }
    }

    fn withdraw(&mut self, id: TransactionId, amount: Amount) {
        // TODO: output to stderr if withdrawal of negative amount is encountered?
        if amount.is_sign_positive() {
            self.add(id, -amount);
        }
    }

    fn dispute(&mut self, id: TransactionId) {
        if let Some(disputed_amount) = self.transactions.get(&id) {
            self.available -= disputed_amount;
            self.held += disputed_amount;
            // TODO: copy the amounts here? Or keep disputed tx ids in a set? Or store mutable
            // objects with (amount, bool) in transactions?
            self.disputes.insert(id, *disputed_amount);
        }
    }

    fn resolve(&mut self, id: TransactionId) {
        if let Some(disputed_amount) = self.disputes.remove(&id) {
            self.available += disputed_amount;
            self.held -= disputed_amount;
        }
        // TODO: should we output something to stderr if there was no dispute?
    }

    fn chargeback(&mut self, id: TransactionId) {
        if let Some(disputed_amount) = self.disputes.remove(&id) {
            self.held -= disputed_amount;
            self.total -= disputed_amount;
            self.locked = true;
        }
        // TODO: should we output something to stderr if there was no dispute?
    }

    fn add(&mut self, id: TransactionId, amount: Amount) {
        let new_total = self.total + amount;
        // Only adjust balance if the new total amount is not negative. Ignore the transaction otherwise.
        if !self.locked && !new_total.is_sign_negative() {
            self.available += amount;
            self.total = new_total;
            self.transactions.insert(id, amount);
        }
        // TODO: should we output something to stderr when a transaction can not be handled?
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

    let mut accounts = BTreeMap::new();
    for result in rdr.deserialize() {
        let transaction: Transaction = result?;

        accounts
            .entry(transaction.client_id)
            .or_insert_with(|| Account::create(transaction.client_id))
            .process(transaction);
    }

    write(accounts, output)
}

fn write(
    accounts: BTreeMap<ClientId, Account>,
    output: &mut impl Write,
) -> Result<(), Box<dyn Error>> {
    let mut wtr = csv::Writer::from_writer(output);

    for acc in accounts.values() {
        if !acc.transactions.is_empty() {
            wtr.serialize(acc)?;
        }
    }

    wtr.flush()?;

    Ok(())
}
