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

#[tokio::test]
async fn test_exact_percent_edit_and_activity_logs() {
    let ctx = setup_test_app().await;

    // 1. Register Alice (Group Admin)
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
    let alice_token = res.json::<Value>().await.unwrap()["token"].as_str().unwrap().to_string();

    // 2. Register Bob (Member)
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
    let bob_body: Value = res.json().await.unwrap();
    let bob_token = bob_body["token"].as_str().unwrap().to_string();
    let bob_id = bob_body["user"]["id"].as_str().unwrap().to_string();

    // 3. Register Charlie (Member)
    let res = ctx
        .client
        .post(format!("{}/api/auth/register", ctx.base_url))
        .json(&json!({
            "email": "charlie@example.com",
            "password": "password123",
            "display_name": "Charlie"
        }))
        .send()
        .await
        .unwrap();
    let charlie_body: Value = res.json().await.unwrap();
    let charlie_token = charlie_body["token"].as_str().unwrap().to_string();
    let charlie_id = charlie_body["user"]["id"].as_str().unwrap().to_string();

    // 4. Alice creates group
    let res = ctx
        .client
        .post(format!("{}/api/groups", ctx.base_url))
        .bearer_auth(&alice_token)
        .json(&json!({
            "name": "Project Trip",
            "default_currency": "USD"
        }))
        .send()
        .await
        .unwrap();
    let group: Value = res.json().await.unwrap();
    let group_id = group["id"].as_str().unwrap().to_string();

    // 5. Alice adds Bob and Charlie
    ctx.client
        .post(format!("{}/api/groups/{}/members", ctx.base_url, group_id))
        .bearer_auth(&alice_token)
        .json(&json!({ "user_id": bob_id, "role": "member" }))
        .send()
        .await
        .unwrap();

    ctx.client
        .post(format!("{}/api/groups/{}/members", ctx.base_url, group_id))
        .bearer_auth(&alice_token)
        .json(&json!({ "user_id": charlie_id, "role": "member" }))
        .send()
        .await
        .unwrap();

    // 6. Test EXACT split: Alice creates $100 expense with Bob=$60 (6000 cents) and Charlie=$40 (4000 cents)
    let res = ctx
        .client
        .post(format!("{}/api/groups/{}/expenses", ctx.base_url, group_id))
        .bearer_auth(&alice_token)
        .json(&json!({
            "description": "Team Gear",
            "amount_cents": 10000,
            "split_type": "EXACT",
            "splits": [
                { "user_id": bob_id, "amount_cents": 6000 },
                { "user_id": charlie_id, "amount_cents": 4000 }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let exp1: Value = res.json().await.unwrap();
    assert_eq!(exp1["split_type"], "EXACT");
    assert_eq!(exp1["splits"].as_array().unwrap().len(), 2);

    // 7. Test PERCENT split: Bob creates $80 (8000 cents) expense with Bob 50% and Charlie 50%
    let res = ctx
        .client
        .post(format!("{}/api/groups/{}/expenses", ctx.base_url, group_id))
        .bearer_auth(&bob_token)
        .json(&json!({
            "description": "Lunch Buffet",
            "amount_cents": 8000,
            "split_type": "PERCENT",
            "splits": [
                { "user_id": bob_id, "percentage": 50.0 },
                { "user_id": charlie_id, "percentage": 50.0 }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let exp2: Value = res.json().await.unwrap();
    let exp2_id = exp2["id"].as_str().unwrap().to_string();
    assert_eq!(exp2["split_type"], "PERCENT");

    // 8. Test Edit Permissions:
    // Case A: Charlie (non-creator, non-admin) tries to edit Bob's expense -> 403 Forbidden
    let res = ctx
        .client
        .put(format!("{}/api/groups/{}/expenses/{}", ctx.base_url, group_id, exp2_id))
        .bearer_auth(&charlie_token)
        .json(&json!({
            "description": "Hacked Lunch",
            "amount_cents": 1000,
            "split_type": "EQUAL",
            "participants": [charlie_id]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);

    // Case B: Bob (the person who added it) edits the expense -> 200 OK
    let res = ctx
        .client
        .put(format!("{}/api/groups/{}/expenses/{}", ctx.base_url, group_id, exp2_id))
        .bearer_auth(&bob_token)
        .json(&json!({
            "description": "Lunch Buffet (Updated with tip)",
            "amount_cents": 9000,
            "split_type": "PERCENT",
            "splits": [
                { "user_id": bob_id, "percentage": 50.0 },
                { "user_id": charlie_id, "percentage": 50.0 }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let updated_exp2: Value = res.json().await.unwrap();
    assert_eq!(updated_exp2["description"], "Lunch Buffet (Updated with tip)");
    assert_eq!(updated_exp2["amount_cents"], 9000);

    // Case C: Alice (Group Admin) can also edit Bob's expense -> 200 OK
    let res = ctx
        .client
        .put(format!("{}/api/groups/{}/expenses/{}", ctx.base_url, group_id, exp2_id))
        .bearer_auth(&alice_token)
        .json(&json!({
            "description": "Lunch Buffet (Admin Adjusted)",
            "amount_cents": 9500,
            "split_type": "PERCENT",
            "splits": [
                { "user_id": bob_id, "percentage": 50.0 },
                { "user_id": charlie_id, "percentage": 50.0 }
            ]
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // 9. Test Activity Logs:
    // Charlie (member) attempts to view activity logs -> 403 Forbidden
    let res = ctx
        .client
        .get(format!("{}/api/groups/{}/activities", ctx.base_url, group_id))
        .bearer_auth(&charlie_token)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 403);

    // Alice (Group Admin) views activity logs -> 200 OK with recorded actions
    let res = ctx
        .client
        .get(format!("{}/api/groups/{}/activities", ctx.base_url, group_id))
        .bearer_auth(&alice_token)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let logs: Value = res.json().await.unwrap();
    let logs_arr = logs.as_array().unwrap();
    assert!(logs_arr.len() >= 5); // CREATE_GROUP, ADD_MEMBER x2, CREATE_EXPENSE x2, UPDATE_EXPENSE x2

    let actions: Vec<&str> = logs_arr
        .iter()
        .map(|l| l["action"].as_str().unwrap())
        .collect();
    assert!(actions.contains(&"CREATE_GROUP"));
    assert!(actions.contains(&"ADD_MEMBER"));
    assert!(actions.contains(&"CREATE_EXPENSE"));
    assert!(actions.contains(&"UPDATE_EXPENSE"));
}

#[tokio::test]
async fn test_attachment_upload_and_expense_attachment() {
    let ctx = setup_test_app().await;

    // 1. Register User
    let res = ctx
        .client
        .post(format!("{}/api/auth/register", ctx.base_url))
        .json(&json!({
            "email": "uploader@example.com",
            "password": "password123",
            "display_name": "Uploader"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let user_token = res.json::<Value>().await.unwrap()["token"].as_str().unwrap().to_string();

    // 2. Create Group
    let res = ctx
        .client
        .post(format!("{}/api/groups", ctx.base_url))
        .bearer_auth(&user_token)
        .json(&json!({
            "name": "Receipt Group",
            "default_currency": "USD"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let group_id = res.json::<Value>().await.unwrap()["id"].as_str().unwrap().to_string();

    // 3. Upload File
    let form = reqwest::multipart::Form::new()
        .part("file", reqwest::multipart::Part::bytes(b"Mock receipt image content".to_vec()).file_name("receipt.jpg").mime_str("image/jpeg").unwrap())
        .text("group_id", group_id.clone());

    let res = ctx
        .client
        .post(format!("{}/api/upload", ctx.base_url))
        .bearer_auth(&user_token)
        .multipart(form)
        .send()
        .await
        .unwrap();

    assert_eq!(res.status(), 201);
    let upload_resp: Value = res.json().await.unwrap();
    let attachment_id = upload_resp["id"].as_str().unwrap().to_string();
    let file_url = upload_resp["file_url"].as_str().unwrap().to_string();
    assert_eq!(upload_resp["file_name"], "receipt.jpg");

    // 4. Download Attachment
    let res = ctx
        .client
        .get(format!("{}{}", ctx.base_url, file_url))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let bytes = res.bytes().await.unwrap();
    assert_eq!(bytes.as_ref(), b"Mock receipt image content");

    // 5. Create Expense with Attachment
    let res = ctx
        .client
        .post(format!("{}/api/groups/{}/expenses", ctx.base_url, group_id))
        .bearer_auth(&user_token)
        .json(&json!({
            "description": "Dinner at Bistro",
            "amount_cents": 4500,
            "attachment_id": attachment_id,
            "attachment_url": file_url,
            "attachment_name": "receipt.jpg"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let exp_resp: Value = res.json().await.unwrap();
    assert_eq!(exp_resp["attachment_id"], attachment_id);
    assert_eq!(exp_resp["attachment_url"], file_url);
    assert_eq!(exp_resp["attachment_name"], "receipt.jpg");

    // 6. List expenses and verify attachment is returned
    let res = ctx
        .client
        .get(format!("{}/api/groups/{}/expenses", ctx.base_url, group_id))
        .bearer_auth(&user_token)
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let expenses: Value = res.json().await.unwrap();
    let exp = &expenses[0];
    assert_eq!(exp["attachment_id"], attachment_id);
    assert_eq!(exp["attachment_url"], file_url);
    assert_eq!(exp["attachment_name"], "receipt.jpg");
}

#[tokio::test]
async fn test_profile_update_and_password_resets() {
    let ctx = setup_test_app().await;

    // 1. Register Admin
    let res = ctx
        .client
        .post(format!("{}/api/auth/register", ctx.base_url))
        .json(&json!({
            "email": "sysadmin@example.com",
            "password": "adminpassword123",
            "display_name": "Admin"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let admin_token = res.json::<Value>().await.unwrap()["token"].as_str().unwrap().to_string();

    // 2. Register Standard User
    let res = ctx
        .client
        .post(format!("{}/api/auth/register", ctx.base_url))
        .json(&json!({
            "email": "bob@example.com",
            "password": "initialpassword",
            "display_name": "Bob Original"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let bob_data: Value = res.json().await.unwrap();
    let bob_token = bob_data["token"].as_str().unwrap().to_string();
    let bob_id = bob_data["user"]["id"].as_str().unwrap().to_string();

    // 3. Update Bob's Profile Display Name
    let res = ctx
        .client
        .patch(format!("{}/api/auth/profile", ctx.base_url))
        .bearer_auth(&bob_token)
        .json(&json!({
            "display_name": "Bob Upgraded"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
    let profile_resp: Value = res.json().await.unwrap();
    assert_eq!(profile_resp["user"]["display_name"], "Bob Upgraded");

    // 4. Bob Changes His Password
    // 4a. Wrong current password -> 401 Unauthorized
    let res = ctx
        .client
        .post(format!("{}/api/auth/change-password", ctx.base_url))
        .bearer_auth(&bob_token)
        .json(&json!({
            "current_password": "wrongpassword",
            "new_password": "brandnewpassword123"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 401);

    // 4b. Correct current password -> 200 OK
    let res = ctx
        .client
        .post(format!("{}/api/auth/change-password", ctx.base_url))
        .bearer_auth(&bob_token)
        .json(&json!({
            "current_password": "initialpassword",
            "new_password": "brandnewpassword123"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // 4c. Verify login with new password
    let res = ctx
        .client
        .post(format!("{}/api/auth/login", ctx.base_url))
        .json(&json!({
            "email": "bob@example.com",
            "password": "brandnewpassword123"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // 5. Admin Resets User Password
    let res = ctx
        .client
        .post(format!("{}/api/admin/users/{}/reset-password", ctx.base_url, bob_id))
        .bearer_auth(&admin_token)
        .json(&json!({
            "new_password": "adminresetpass456"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);

    // 5a. Verify user logs in with admin-reset password
    let res = ctx
        .client
        .post(format!("{}/api/auth/login", ctx.base_url))
        .json(&json!({
            "email": "bob@example.com",
            "password": "adminresetpass456"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(res.status(), 200);
}


