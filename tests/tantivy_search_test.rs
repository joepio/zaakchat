use tempfile::TempDir;

#[tokio::test]
async fn test_tantivy_json_search() {
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

    // Create a test issue with involved user
    let test_user = "test@example.com";
    let issue_id = "test-issue-123";

    let issue_json = serde_json::json!({
        "id": issue_id,
        "title": "Test Issue",
        "status": "open",
        "involved": [test_user, "other@example.com"]
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

    // Test different query formats
    let queries = vec![
        // Full email
        format!("json_payload:{}", test_user),
        format!("{}", test_user),
        format!("\"{}\"", test_user),
        format!("json_payload:\"{}\"", test_user),
        // Just the username part
        "json_payload:test".to_string(),
        "test".to_string(),
        // Try searching for the domain
        "json_payload:example.com".to_string(),
        "example.com".to_string(),
        // Try wildcard
        "json_payload:test*".to_string(),
        // Try involved field specifically (if Tantivy supports nested)
        "json_payload.involved:test".to_string(),
    ];

    let mut working_query = None;

    for query in queries {
        println!("\nTesting query: {}", query);
        let results = search_index.search(&storage, &query, 10).await.unwrap();
        println!("  Results: {}", results.len());

        if !results.is_empty() {
            println!("  ✓ Query works!");
            working_query = Some(query.clone());
            for result in &results {
                println!("    - Found: {} (type: {})", result.id, result.doc_type);
                if let Some(resource) = &result.resource {
                    println!("      Resource: {}", serde_json::to_string_pretty(resource).unwrap());
                }
            }
            break; // Found a working query!
        }
    }

    if let Some(query) = working_query {
        println!("\n✅ Working query format: {}", query);
    } else {
        println!("\n❌ No working query found!");
        panic!("Could not find a working Tantivy query for email search");
    }
}
