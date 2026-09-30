use crate::models::{MemberBalance, SimplifiedDebt};

/// Computes simplified debts minimizing the total number of transactions needed
/// using a greedy bipartite matching algorithm.
pub fn simplify_debts(
    balances: &[MemberBalance],
    currency: &str,
) -> Vec<SimplifiedDebt> {
    // Separate into debtors (< 0) and creditors (> 0)
    // Note: In our model, net_balance > 0 means the person is owed money (creditor),
    // and net_balance < 0 means the person owes money (debtor).
    let mut debtors: Vec<(String, String, i64)> = balances
        .iter()
        .filter(|b| b.net_balance_cents < 0)
        .map(|b| (b.user_id.clone(), b.display_name.clone(), -b.net_balance_cents))
        .collect();

    let mut creditors: Vec<(String, String, i64)> = balances
        .iter()
        .filter(|b| b.net_balance_cents > 0)
        .map(|b| (b.user_id.clone(), b.display_name.clone(), b.net_balance_cents))
        .collect();

    let mut simplified = Vec::new();

    // Loop until all debts are settled
    while !debtors.is_empty() && !creditors.is_empty() {
        // Sort descending by amount to settle largest debts first
        debtors.sort_by(|a, b| b.2.cmp(&a.2));
        creditors.sort_by(|a, b| b.2.cmp(&a.2));

        let (d_id, d_name, d_amount) = debtors.remove(0);
        let (c_id, c_name, c_amount) = creditors.remove(0);

        let settle_amount = d_amount.min(c_amount);

        if settle_amount > 0 {
            simplified.push(SimplifiedDebt {
                from_user_id: d_id.clone(),
                from_user_name: d_name.clone(),
                to_user_id: c_id.clone(),
                to_user_name: c_name.clone(),
                amount_cents: settle_amount,
                currency: currency.to_string(),
            });
        }

        let d_rem = d_amount - settle_amount;
        let c_rem = c_amount - settle_amount;

        if d_rem > 0 {
            debtors.push((d_id, d_name, d_rem));
        }
        if c_rem > 0 {
            creditors.push((c_id, c_name, c_rem));
        }
    }

    simplified
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_two_person_debt() {
        let balances = vec![
            MemberBalance {
                user_id: "alice".into(),
                display_name: "Alice".into(),
                net_balance_cents: 5000, // Alice is owed 50.00
            },
            MemberBalance {
                user_id: "bob".into(),
                display_name: "Bob".into(),
                net_balance_cents: -5000, // Bob owes 50.00
            },
        ];

        let result = simplify_debts(&balances, "USD");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].from_user_id, "bob");
        assert_eq!(result[0].to_user_id, "alice");
        assert_eq!(result[0].amount_cents, 5000);
    }

    #[test]
    fn test_three_person_transitive_simplification() {
        // Alice paid 60 for Alice, Bob, Charlie (20 each).
        // Bob paid 30 for Alice, Bob, Charlie (10 each).
        // Net:
        // Alice: paid 60 - owed 30 = +30 (Alice is owed 30)
        // Bob: paid 30 - owed 30 = 0
        // Charlie: paid 0 - owed 30 = -30 (Charlie owes 30)
        let balances = vec![
            MemberBalance {
                user_id: "alice".into(),
                display_name: "Alice".into(),
                net_balance_cents: 3000,
            },
            MemberBalance {
                user_id: "bob".into(),
                display_name: "Bob".into(),
                net_balance_cents: 0,
            },
            MemberBalance {
                user_id: "charlie".into(),
                display_name: "Charlie".into(),
                net_balance_cents: -3000,
            },
        ];

        let result = simplify_debts(&balances, "USD");
        // Charlie directly pays Alice 30; Bob is completely bypassed!
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].from_user_id, "charlie");
        assert_eq!(result[0].to_user_id, "alice");
        assert_eq!(result[0].amount_cents, 3000);
    }
}
