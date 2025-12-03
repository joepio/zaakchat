use tempfile::TempDir;

#[tokio::test]
async fn test_tantivy_hyphenated_username_search() {
    // Create temporary directories for test
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().to_path_buf();

    // Initialize storage and search
    let storage = zaakchat::storage::Storage::new(&data_dir).await.unwrap();
    let search_index = zaakchat::search::SearchIndex::open(
        data_dir.join("search_index"),
        false, // Don't spawn committer for tests
        std::time::Duration::from_secs(5),
    ).unwrap();

    // Create a test issue with hyphenated user
    let test_user = "test-user@example.com";
    let issue_id = "test-issue-hyphen";

    let issue_json = serde_json::json!({
        "id": issue_id,
        "title": "Hyphen Issue",
        "status": "open",
        "involved": [test_user]
    });

    let payload_str = serde_json::to_string(&issue_json).unwrap();

    // Index the issue
    search_index.add_resource_payload(
        issue_id,
        "issue",
        "",
        &payload_str,
        Some(chrono::Utc::now()),
    ).await.unwrap();

    // Commit to make it searchable
    search_index.commit().await.unwrap();

    // Test the exact query format used in handlers.rs
    // let username = user_id.split('@').next().unwrap_or(user_id);
    // let query = format!("json_payload.involved:{}", username);

    let username = "test-user";
    let query = format!("json_payload.involved:{}", username);

    println!("\nTesting query: {}", query);
    let results = search_index.search(&storage, &query, 10).await.unwrap();
    println!("  Results: {}", results.len());

    if !results.is_empty() {
        println!("  ✓ Query works!");
    } else {
        println!("  ❌ Query failed!");

        // Try quoted query
        let quoted_query = format!("json_payload.involved:\"{}\"", username);
        println!("\nTesting quoted query: {}", quoted_query);
        let results_quoted = search_index.search(&storage, &quoted_query, 10).await.unwrap();
        println!("  Results: {}", results_quoted.len());

        if !results_quoted.is_empty() {
            println!("  ✓ Quoted query works!");
        } else {
            println!("  ❌ Quoted query failed too!");
        }
    }
}
