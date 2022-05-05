use rust_decimal::Decimal;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashMap;
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
    transactions: HashMap<TransactionId, Amount>,
    #[serde(skip_serializing)]
    disputes: HashMap<TransactionId, Amount>,
}

impl Account {
    fn new(client_id: ClientId) -> Self {
        Account {
            client_id: client_id,
            available: Decimal::new(0, 4),
            held: Decimal::new(0, 4),
            total: Decimal::new(0, 4),
            locked: false,
            transactions: HashMap::new(),
            disputes: HashMap::new(),
        }
    }

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
            // Only decrease the available amount for disputed deposits.
            if disputed_amount.is_sign_positive() {
                self.available -= disputed_amount;
            }
            self.held += disputed_amount;
            self.disputes.insert(id, *disputed_amount);
        }
        // if the transaction to dispute is not found, ignore and assume an error on the parner's side
    }

    fn resolve(&mut self, id: TransactionId) {
        if let Some(disputed_amount) = self.disputes.remove(&id) {
            // Release available funds only for disputed deposits.
            // Disputed withdrawals (negative disputed amount) do not increase the available
            // amount.
            if disputed_amount.is_sign_positive() {
                self.available += disputed_amount;
            }
            self.held -= disputed_amount;
        }
        // if the dispute is not found, ignore and assume an error on the parner's side
    }

    fn chargeback(&mut self, id: TransactionId) {
        if let Some(disputed_amount) = self.disputes.remove(&id) {
            self.held -= disputed_amount;
            self.total -= disputed_amount;
            // If the disputed amount is negative, then a withdrawal was disputed.
            // We should return the disputed amount on chargeback in this case.
            if disputed_amount.is_sign_negative() {
                self.available -= disputed_amount;
            }
            self.locked = true;
        }
        // if the dispute is not found, ignore and assume an error on the parner's side
    }

    fn add(&mut self, id: TransactionId, amount: Amount) {
        let new_available = self.available + amount;
        // Only adjust balance if the account is not locked and the new available amount is not negative. Ignore the transaction otherwise.
        if !self.locked && !new_available.is_sign_negative() {
            self.available = new_available;
            self.total += amount;
            self.transactions.insert(id, amount);
        }
        // TODO: should we log something to stderr when a transaction can not be handled?
    }
}

struct PaymentsEngine {
    accounts: HashMap<ClientId, Account>,
}

impl PaymentsEngine {
    fn new() -> Self {
        PaymentsEngine {
            accounts: HashMap::new(),
        }
    }

    fn process_transaction(&mut self, transaction: Transaction) {
        let account = self
            .accounts
            .entry(transaction.client_id)
            .or_insert_with(|| Account::new(transaction.client_id));

        match transaction.tx_type {
            TransactionType::Deposit => {
                if let Some(amount) = transaction.amount {
                    account.deposit(transaction.id, amount)
                }
                // TODO: else panic with a pointer to this bad data entry? Or stderr and skip?
            }
            TransactionType::Withdrawal => {
                if let Some(amount) = transaction.amount {
                    account.withdraw(transaction.id, amount)
                }
                // TODO: else panic with a pointer to this bad data entry? Or stderr and skip?
            }
            TransactionType::Dispute => account.dispute(transaction.id),
            TransactionType::Resolve => account.resolve(transaction.id),
            TransactionType::Chargeback => account.chargeback(transaction.id),
        }
    }
}

pub fn run(transactions_csv: impl Read, output: &mut impl Write) -> Result<(), Box<dyn Error>> {
    let mut payments_engine = PaymentsEngine::new();
    process_csv(transactions_csv, &mut payments_engine)?;
    write_account_states_to_csv(payments_engine.accounts, output)
}

fn process_csv(
    transactions_csv: impl Read,
    payments_engine: &mut PaymentsEngine,
) -> Result<(), Box<dyn Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .flexible(true)
        .from_reader(transactions_csv);

    let headers = rdr.byte_headers()?.clone();
    let mut raw_record = csv::ByteRecord::new();

    while rdr.read_byte_record(&mut raw_record)? {
        let transaction: Transaction = raw_record.deserialize(Some(&headers))?;
        payments_engine.process_transaction(transaction);
    }

    Ok(())
}

fn write_account_states_to_csv(
    accounts: HashMap<ClientId, Account>,
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
