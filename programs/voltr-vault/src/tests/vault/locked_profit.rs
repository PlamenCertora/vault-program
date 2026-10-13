mod tests {
    use crate::state::vault::*;

    #[test]
    fn test_locked_profit_degradation() {
        let locked_profit_state = LockedProfitState {
            last_updated_locked_profit: 1000,
            last_report: 0,
        };
        let degradation_duration = 100;

        // Test full lock at start
        assert_eq!(
            locked_profit_state
                .calculate_locked_profit(degradation_duration, 0)
                .unwrap(),
            1000
        );

        // Test half degraded
        assert_eq!(
            locked_profit_state
                .calculate_locked_profit(degradation_duration, 50)
                .unwrap(),
            500
        );

        // Test fully degraded
        assert_eq!(
            locked_profit_state
                .calculate_locked_profit(degradation_duration, 100)
                .unwrap(),
            0
        );

        // Test beyond degradation period
        assert_eq!(
            locked_profit_state
                .calculate_locked_profit(degradation_duration, 101)
                .unwrap(),
            0
        );
    }

    #[test]
    fn test_locked_profit_update() {
        let mut locked_profit_state = LockedProfitState {
            last_updated_locked_profit: 1000,
            last_report: 0,
        };
        let degradation_duration = 100;

        locked_profit_state
            .update_last_report(degradation_duration, 50, 500)
            .unwrap();
        assert_eq!(locked_profit_state.last_updated_locked_profit, 1000); // 500 existing + 500 new
        assert_eq!(locked_profit_state.last_report, 50);
    }
}
