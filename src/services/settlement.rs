use std::collections::{HashMap, HashSet};
use crate::models::{MemberBalance, SimplifiedDebt};

#[derive(Debug, Clone)]
pub struct SplitDebtEntry {
    pub payer_id: String,
    pub borrower_id: String,
    pub amount_cents: i64,
}

#[derive(Debug, Clone)]
pub struct SettlementDebtEntry {
    pub payer_id: String,
    pub payee_id: String,
    pub amount_cents: i64,
}

/// Computes direct pairwise net debts between each pair of group members.
/// For each expense split: borrower owes payer their split amount.
/// For each settlement: payer reduces their debt to payee.
/// Returns pairwise net debts sorted descending by amount.
pub fn compute_direct_debts(
    splits: &[SplitDebtEntry],
    settlements: &[SettlementDebtEntry],
    user_names: &HashMap<String, String>,
    currency: &str,
) -> Vec<SimplifiedDebt> {
    let mut directed: HashMap<(String, String), i64> = HashMap::new();
    let mut all_users: HashSet<String> = HashSet::new();

    for split in splits {
        if split.borrower_id != split.payer_id && split.amount_cents > 0 {
            all_users.insert(split.borrower_id.clone());
            all_users.insert(split.payer_id.clone());
            *directed
                .entry((split.borrower_id.clone(), split.payer_id.clone()))
                .or_insert(0) += split.amount_cents;
        }
    }

    for s in settlements {
        if s.payer_id != s.payee_id && s.amount_cents > 0 {
            all_users.insert(s.payer_id.clone());
            all_users.insert(s.payee_id.clone());
            *directed
                .entry((s.payer_id.clone(), s.payee_id.clone()))
                .or_insert(0) -= s.amount_cents;
        }
    }

    let user_list: Vec<String> = all_users.into_iter().collect();
    let mut direct_debts = Vec::new();

    for i in 0..user_list.len() {
        for j in (i + 1)..user_list.len() {
            let u1 = &user_list[i];
            let u2 = &user_list[j];

            let owed_1_to_2 = directed.get(&(u1.clone(), u2.clone())).copied().unwrap_or(0);
            let owed_2_to_1 = directed.get(&(u2.clone(), u1.clone())).copied().unwrap_or(0);
            let net = owed_1_to_2 - owed_2_to_1;

            if net > 0 {
                direct_debts.push(SimplifiedDebt {
                    from_user_id: u1.clone(),
                    from_user_name: user_names.get(u1).cloned().unwrap_or_else(|| "User".into()),
                    to_user_id: u2.clone(),
                    to_user_name: user_names.get(u2).cloned().unwrap_or_else(|| "User".into()),
                    amount_cents: net,
                    currency: currency.to_string(),
                });
            } else if net < 0 {
                direct_debts.push(SimplifiedDebt {
                    from_user_id: u2.clone(),
                    from_user_name: user_names.get(u2).cloned().unwrap_or_else(|| "User".into()),
                    to_user_id: u1.clone(),
                    to_user_name: user_names.get(u1).cloned().unwrap_or_else(|| "User".into()),
                    amount_cents: -net,
                    currency: currency.to_string(),
                });
            }
        }
    }

    // Sort descending by amount for clean presentation
    direct_debts.sort_by(|a, b| b.amount_cents.cmp(&a.amount_cents));

    direct_debts
}

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
                total_paid_cents: 5000,
                total_owed_cents: 0,
                net_balance_cents: 5000, // Alice is owed 50.00
            },
            MemberBalance {
                user_id: "bob".into(),
                display_name: "Bob".into(),
                total_paid_cents: 0,
                total_owed_cents: 5000,
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
                total_paid_cents: 6000,
                total_owed_cents: 3000,
                net_balance_cents: 3000,
            },
            MemberBalance {
                user_id: "bob".into(),
                display_name: "Bob".into(),
                total_paid_cents: 3000,
                total_owed_cents: 3000,
                net_balance_cents: 0,
            },
            MemberBalance {
                user_id: "charlie".into(),
                display_name: "Charlie".into(),
                total_paid_cents: 0,
                total_owed_cents: 3000,
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

    #[test]
    fn test_direct_debts_pairwise() {
        let mut user_names = HashMap::new();
        user_names.insert("alice".into(), "Alice".into());
        user_names.insert("bob".into(), "Bob".into());
        user_names.insert("charlie".into(), "Charlie".into());

        // Alice paid 60 for Bob only (e.g. train ticket)
        let splits = vec![
            SplitDebtEntry {
                payer_id: "alice".into(),
                borrower_id: "bob".into(),
                amount_cents: 6000,
            },
            // Charlie paid 20 for Alice
            SplitDebtEntry {
                payer_id: "charlie".into(),
                borrower_id: "alice".into(),
                amount_cents: 2000,
            },
        ];

        // Bob settles 15 to Alice
        let settlements = vec![
            SettlementDebtEntry {
                payer_id: "bob".into(),
                payee_id: "alice".into(),
                amount_cents: 1500,
            },
        ];

        let direct = compute_direct_debts(&splits, &settlements, &user_names, "INR");
        assert_eq!(direct.len(), 2);

        // Bob owes Alice 45.00 (60.00 - 15.00)
        let bob_alice = direct.iter().find(|d| d.from_user_id == "bob" && d.to_user_id == "alice").unwrap();
        assert_eq!(bob_alice.amount_cents, 4500);

        // Alice owes Charlie 20.00
        let alice_charlie = direct.iter().find(|d| d.from_user_id == "alice" && d.to_user_id == "charlie").unwrap();
        assert_eq!(alice_charlie.amount_cents, 2000);
    }
}
