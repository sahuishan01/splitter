use serde_json::{json, Value};
use splitter::{create_app, db::init_pool};
use tempfile::tempdir;

struct TestContext {
    base_url: String,
    client: reqwest::Client,
    _temp_dir: tempfile::TempDir,
}

async fn setup_test_app() -> TestContext {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");
    let db_url = format!("sqlite://{}", db_path.display());

    let pool = init_pool(&db_url).await.expect("Failed to init pool");
    let app = create_app(pool);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("Failed to bind ephemeral port");
    let addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    TestContext {
        base_url: format!("http://{}", addr),
        client: reqwest::Client::new(),
        _temp_dir: temp_dir,
    }
}

#[tokio::test]
async fn test_full_expense_splitting_flow() {
    let ctx = setup_test_app().await;

    // 1. Register first user (System Admin by virtue of being first user)
    let res = ctx
        .client
        .post(format!("{}/api/auth/register", ctx.base_url))
        .json(&json!({
            "email": "admin@example.com",
            "password": "password123",
            "display_name": "Admin User"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let admin_body: Value = res.json().await.unwrap();
    let admin_token = admin_body["token"].as_str().unwrap().to_string();
    assert_eq!(admin_body["user"]["is_admin"], true);

    // 2. Register Alice (Standard user)
    let res = ctx
        .client
        .post(format!("{}/api/auth/register", ctx.base_url))
        .json(&json!({
            "email": "alice@example.com",
            "password": "password123",
            "display_name": "Alice"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let alice_body: Value = res.json().await.unwrap();
    let alice_token = alice_body["token"].as_str().unwrap().to_string();
    let alice_id = alice_body["user"]["id"].as_str().unwrap().to_string();
    assert_eq!(alice_body["user"]["is_admin"], false);

    // 3. Register Bob (Standard user)
    let res = ctx
        .client
        .post(format!("{}/api/auth/register", ctx.base_url))
        .json(&json!({
            "email": "bob@example.com",
            "password": "password123",
            "display_name": "Bob"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let bob_body: Value = res.json().await.unwrap();
    let bob_token = bob_body["token"].as_str().unwrap().to_string();
    let bob_id = bob_body["user"]["id"].as_str().unwrap().to_string();

    // 4. Admin RBAC check: Alice cannot access admin routes
    let res = ctx
        .client
        .get(format!("{}/api/admin/users", ctx.base_url))
        .bearer_auth(&alice_token)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);

    // Admin can access admin routes
    let res = ctx
        .client
        .get(format!("{}/api/admin/users", ctx.base_url))
        .bearer_auth(&admin_token)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let users_list: Value = res.json().await.unwrap();
    assert_eq!(users_list.as_array().unwrap().len(), 3);

    // 5. Alice creates a group
    let res = ctx
        .client
        .post(format!("{}/api/groups", ctx.base_url))
        .bearer_auth(&alice_token)
        .json(&json!({
            "name": "Weekend Trip",
            "description": "Cabin rental and food",
            "default_currency": "USD"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let group: Value = res.json().await.unwrap();
    let group_id = group["id"].as_str().unwrap().to_string();

    // 6. Alice adds Bob to the group
    let res = ctx
        .client
        .post(format!("{}/api/groups/{}/members", ctx.base_url, group_id))
        .bearer_auth(&alice_token)
        .json(&json!({
            "email": "bob@example.com",
            "role": "member"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // 7. Alice records $100 (10000 cents) expense split equally with Bob
    let idempotency_key = "idemp-exp-001";
    let expense_payload = json!({
        "description": "Cabin Rental",
        "amount_cents": 10000,
        "paid_by": alice_id,
        "split_type": "EQUAL",
        "participants": [alice_id, bob_id],
        "idempotency_key": idempotency_key
    });

    let res = ctx
        .client
        .post(format!("{}/api/groups/{}/expenses", ctx.base_url, group_id))
        .bearer_auth(&alice_token)
        .json(&expense_payload)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let exp_res1: Value = res.json().await.unwrap();
    let exp_id = exp_res1["id"].as_str().unwrap().to_string();

    // Idempotency test: Re-submitting the exact same expense returns the identical result
    let res = ctx
        .client
        .post(format!("{}/api/groups/{}/expenses", ctx.base_url, group_id))
        .bearer_auth(&alice_token)
        .json(&expense_payload)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let exp_res2: Value = res.json().await.unwrap();
    assert_eq!(exp_res2["id"], exp_id);

    // Verify expense list has only 1 item (no duplicate created!)
    let res = ctx
        .client
        .get(format!("{}/api/groups/{}/expenses", ctx.base_url, group_id))
        .bearer_auth(&alice_token)
        .send()
        .await
        .unwrap();
    let exp_list: Value = res.json().await.unwrap();
    assert_eq!(exp_list.as_array().unwrap().len(), 1);

    // 8. Check Balances & Simplified Debts
    let res = ctx
        .client
        .get(format!("{}/api/groups/{}/balances", ctx.base_url, group_id))
        .bearer_auth(&bob_token)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let balances_data: Value = res.json().await.unwrap();

    let simplified = balances_data["simplified_debts"].as_array().unwrap();
    assert_eq!(simplified.len(), 1);
    assert_eq!(simplified[0]["from_user_id"], bob_id);
    assert_eq!(simplified[0]["to_user_id"], alice_id);
    assert_eq!(simplified[0]["amount_cents"], 5000); // Bob owes Alice 50.00

    // 9. Bob settles up with Alice ($50.00 / 5000 cents)
    let res = ctx
        .client
        .post(format!("{}/api/groups/{}/settlements", ctx.base_url, group_id))
        .bearer_auth(&bob_token)
        .json(&json!({
            "payer_id": bob_id,
            "payee_id": alice_id,
            "amount_cents": 5000,
            "idempotency_key": "idemp-settle-001"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // 10. Check Balances after settlement: All debts settled!
    let res = ctx
        .client
        .get(format!("{}/api/groups/{}/balances", ctx.base_url, group_id))
        .bearer_auth(&alice_token)
        .send()
        .await
        .unwrap();
    let balances_after: Value = res.json().await.unwrap();
    assert_eq!(balances_after["simplified_debts"].as_array().unwrap().len(), 0);

    // 11. Test Batch Sync Offline Queue Endpoint
    let batch_res = ctx
        .client
        .post(format!("{}/api/sync/batch", ctx.base_url))
        .bearer_auth(&alice_token)
        .json(&json!({
            "mutations": [
                {
                    "idempotency_key": "offline-batch-001",
                    "action": "create_expense",
                    "group_id": group_id,
                    "payload": {
                        "description": "Offline Groceries",
                        "amount_cents": 2000,
                        "paid_by": alice_id,
                        "split_type": "EQUAL",
                        "participants": [alice_id, bob_id]
                    }
                }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(batch_res.status(), 200);
    let batch_body: Value = batch_res.json().await.unwrap();
    assert_eq!(batch_body["results"][0]["status"], "applied");

    // Replay same batch -> should report "already_processed"
    let batch_res2 = ctx
        .client
        .post(format!("{}/api/sync/batch", ctx.base_url))
        .bearer_auth(&alice_token)
        .json(&json!({
            "mutations": [
                {
                    "idempotency_key": "offline-batch-001",
                    "action": "create_expense",
                    "group_id": group_id,
                    "payload": {
                        "description": "Offline Groceries",
                        "amount_cents": 2000,
                        "paid_by": alice_id,
                        "split_type": "EQUAL",
                        "participants": [alice_id, bob_id]
                    }
                }
            ]
        }))
        .send()
        .await
        .unwrap();
    let batch_body2: Value = batch_res2.json().await.unwrap();
    assert_eq!(batch_body2["results"][0]["status"], "already_processed");
}
