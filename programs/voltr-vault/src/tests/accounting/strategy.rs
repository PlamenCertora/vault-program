mod tests {
    use crate::utils::*;
    // Test P&L Calculation Functions
    #[test]
    fn test_calc_withdraw_pnl() {
        // Profit scenario: ended up with more than expected after withdraw
        let (is_profit, pnl) = calc_withdraw_pnl(1000, 600, 300).unwrap();
        // Expected after withdraw: 1000 - 300 = 700, actual: 600, loss of 100
        assert_eq!(is_profit, false);
        assert_eq!(pnl, 100);

        // Profit scenario: ended up with more than expected
        let (is_profit, pnl) = calc_withdraw_pnl(1000, 800, 300).unwrap();
        // Expected: 700, actual: 800, profit of 100
        assert_eq!(is_profit, true);
        assert_eq!(pnl, 100);

        // Edge case: withdraw more than balance
        let (is_profit, pnl) = calc_withdraw_pnl(500, 800, 1000).unwrap();
        // Expected would be negative, so any positive amount is profit
        assert_eq!(is_profit, true);
        assert_eq!(pnl, 1300); // 800 + (1000 - 500)
    }

    #[test]
    fn test_calc_deposit_pnl() {
        // Profit scenario
        let (is_profit, pnl) = calc_deposit_pnl(1000, 1200, 100).unwrap();
        // Expected: 1000 + 100 = 1100, actual: 1200, profit of 100
        assert_eq!(is_profit, true);
        assert_eq!(pnl, 100);

        // Loss scenario
        let (is_profit, pnl) = calc_deposit_pnl(1000, 1050, 100).unwrap();
        // Expected: 1100, actual: 1050, loss of 50
        assert_eq!(is_profit, false);
        assert_eq!(pnl, 50);

        // Perfect scenario (no slippage)
        let (is_profit, pnl) = calc_deposit_pnl(1000, 1100, 100).unwrap();
        assert_eq!(is_profit, true);
        assert_eq!(pnl, 0);
    }
}
