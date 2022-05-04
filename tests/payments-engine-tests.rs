#[cfg(test)]
mod tests {
    use std::str;

    fn process_transactions(input: &str) -> String {
        let mut output = Vec::new();
        payments_engine::run(input.as_bytes(), &mut output).unwrap();
        str::from_utf8(&output).unwrap().to_string()
    }

    #[test]
    fn processes_empty_data_set() {
        let output = process_transactions("type, client, tx, amount");

        // TODO: should we write the header when there is no content?
        assert_eq!(output, "");
    }

    #[test]
    fn processes_sample_simple_data_set() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 1.0
            deposit, 2, 2, 2.0
            deposit, 1, 3, 2.0
            withdrawal, 1, 4, 1.5
            withdrawal, 2, 5, 3.0",
        );

        assert_eq!(output.lines().count(), 3);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("2,2,0.0000,2,false\n"));
        assert!(output.contains("1,1.5,0.0000,1.5,false\n"));
    }

    #[test]
    fn credits_deposits() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 1.0
            deposit, 2, 2, 2.0
            deposit, 1, 3, 2.0",
        );

        assert_eq!(output.lines().count(), 3);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("2,2,0.0000,2,false\n"));
        assert!(output.contains("1,3,0.0000,3,false\n"));
    }

    #[test]
    fn does_not_credit_negative_deposit() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 1.0
            deposit, 1, 2, 2.0
            deposit, 1, 3, -2.0",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,3,0.0000,3,false\n"));
    }

    #[test]
    fn withdraws_all_balance() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 1.0
            deposit, 1, 2, 1.0
            withdrawal, 1, 3, 2.0",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,0,0.0000,0,false\n"));
    }

    #[test]
    fn does_not_withdraw_negative_amount() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 1.0
            deposit, 1, 2, 1.0
            withdrawal, 1, 3, -1",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,2,0.0000,2,false\n"));
    }

    #[test]
    fn withdraws_some() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 10.0
            withdrawal, 1, 2, 2.0
            withdrawal, 1, 2, 1.0",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,7,0.0000,7,false\n"));
    }

    #[test]
    fn supports_four_decimal_places() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 10.0
            withdrawal, 1, 2, 0.0001",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,9.9999,0.0000,9.9999,false\n"));
    }

    #[test]
    fn ignores_withdrawal_when_not_enough_funds_available() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 1.0
            withdrawal, 1, 3, 1.1",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,1,0.0000,1,false\n"));
    }

    #[test]
    fn ignores_withdrawal_for_nonexisting_client() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 1.0
            withdrawal, 2, 2, 1.0",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,1,0.0000,1,false\n"));
    }

    #[test]
    fn ignores_withdrawal_before_deposit() {
        let output = process_transactions(
            "type, client, tx, amount
            withdrawal, 1, 2, 1.0
            deposit, 1, 1, 5.0",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,5,0.0000,5,false\n"));
    }

    #[test]
    fn disputes_the_only_deposit() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 10.0
            dispute, 1, 1,",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,0,10,10,false\n"));
    }

    #[test]
    fn can_read_dispute_without_end_coma() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 10.0
            dispute, 1, 1",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,0,10,10,false\n"));
    }

    #[test]
    fn disputes_the_only_withdrawal() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 10.0
            withdrawal, 1, 2, 5.0
            dispute, 1, 2,",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,10,-5,5,false\n"));
    }

    #[test]
    fn can_not_withdraw_beyond_total_when_withdrawal_disputed() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 10.0
            withdrawal, 1, 2, 5.0
            dispute, 1, 2,
            withdrawal, 1, 2, 6.0",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,10,-5,5,false\n"));
    }

    #[test]
    fn disputes_one_of_deposits() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 10.0
            deposit, 1, 2, 5.0
            deposit, 1, 3, 2.0
            dispute, 1, 2,",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,12,5,17,false\n"));
    }

    #[test]
    fn disputes_deposit_after_withdrawal() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 10.0
            withdrawal, 1, 2, 5.0
            dispute, 1, 1,",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,-5,10,5,false\n"));
    }

    #[test]
    fn ignores_dispute_of_non_existing_transaction() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 10.0
            dispute, 1, 2,",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,10,0.0000,10,false\n"));
    }

    #[test]
    fn resolves_disputed_deposit() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 10.0
            deposit, 1, 2, 5.0
            deposit, 1, 3, 2.0
            dispute, 1, 2,
            deposit, 1, 4, 0.0001
            resolve, 1, 2,",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,17.0001,0,17.0001,false\n"));
    }

    #[test]
    fn resolves_disputed_withdrawal() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 10.0
            withdrawal, 1, 2, 5.0
            dispute, 1, 2,
            resolve, 1, 2,",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,5,0,5,false\n"));
    }

    #[test]
    fn ignores_resolving_undisputed_transaction() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 10.0
            deposit, 1, 2, 5.0
            deposit, 1, 3, 2.0
            deposit, 1, 4, 0.0001
            resolve, 1, 2,",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,17.0001,0.0000,17.0001,false\n"));
    }

    #[test]
    fn ignores_resolving_different_than_disputed_transaction() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 10.0
            deposit, 1, 2, 5.0
            deposit, 1, 3, 2.0
            dispute, 1, 2,
            deposit, 1, 4, 0.0001
            resolve, 1, 3,",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,12.0001,5,17.0001,false\n"));
    }

    #[test]
    fn chargebacks_disputed_deposit() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 10.0
            deposit, 1, 2, 5.0
            deposit, 1, 3, 2.0
            dispute, 1, 2,
            deposit, 1, 4, 0.0001
            chargeback, 1, 2,",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,12.0001,0,12.0001,true\n"));
    }

    #[test]
    fn chargebacks_disputed_withdrawal() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 10.0
            withdrawal, 1, 2, 5.0
            dispute, 1, 2,
            withdrawal, 1, 2, 5.0
            chargeback, 1, 2,",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,5,0,5,true\n"));
    }

    #[test]
    fn ignores_chargeback_of_undisputed_deposit() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 10.0
            deposit, 1, 2, 5.0
            deposit, 1, 3, 2.0
            deposit, 1, 4, 0.0001
            chargeback, 1, 2,",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,17.0001,0.0000,17.0001,false\n"));
    }

    #[test]
    fn ignores_chargeback_of_different_than_disputed_deposit() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 10.0
            deposit, 1, 2, 5.0
            deposit, 1, 3, 2.0
            dispute, 1, 2,
            deposit, 1, 4, 0.0001
            chargeback, 1, 3,",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,12.0001,5,17.0001,false\n"));
    }

    #[test]
    fn can_not_deposit_to_locked_account() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 10.0
            dispute, 1, 1,
            chargeback, 1, 1,
            deposit, 1, 2, 0.0001",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,0,0,0,true\n"));
    }

    #[test]
    fn can_not_withdraw_from_locked_account() {
        let output = process_transactions(
            "type, client, tx, amount
            deposit, 1, 1, 10.0
            deposit, 1, 2, 1.0
            dispute, 1, 1,
            chargeback, 1, 1,
            withdrawal, 1, 3, 0.0001",
        );

        assert_eq!(output.lines().count(), 2);
        assert!(output.contains("client,available,held,total,locked\n"));
        assert!(output.contains("1,1,0,1,true\n"));
    }
}
