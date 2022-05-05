# rust-payments-engine

This is really my first Rust application beyond a "Hello World".

I have yet to learn more about the language and about the idiomatic
way of using it.

## Implementation

### Reading/Writing of data

Using `serde` and `csv` crates to handle the reading and parsing of the csv.
The library allows to stream the records one by one so dealing with large data
sets is not an issue memory-wise at least.

I "tested"/tried the performance with data sets of up to 100m deposit/withdrawal entries.
Can't add them to this repo because of the GitHub's size limitation, but those were basically
repetitions of the included `deposits-withdrawals-1m.csv`.

I bet the speed can be further improved by not deserializing the lines to structs by default,
but with my limited Rust experience, I didn't want to make a big unsafe mess out of this.

### Handling numbers

I was tempted to just go with an f64 for handling the numbers given I only want to really deal with
up to four decimal places and only addition and subtraction.
However, picked `rust_decimal` - the runtime performance seemed to be almost the same as with
plain f64 with my data sets.

### The logic

This, I believe, should be rather straighforward - have a map of client_ids to accounts,
route each of the client's transactions to the appropriate account by client_id.
The account itself knows how to process each type of transaction - how it affects the balances.

Given the transactions come in sequence and they only reference one client account each,
it should be possible to parallelize the handling by client_id.

## Testing

The business rules are tested using integration tests. I've grown to prefer simple functional
integration tests that only hit the API for small modules like this one.
This doesn't tie the tests to the implementation and the tests focus on the behavior that matters.

