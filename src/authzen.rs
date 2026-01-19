use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Subject {
    pub id: String,
    #[serde(flatten)]
    pub attributes: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Resource {
    pub id: String,
    #[serde(rename = "type")]
    pub resource_type: String,
    #[serde(flatten)]
    pub attributes: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Action {
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvaluationRequest {
    pub subject: Subject,
    pub action: Action,
    pub resource: Resource,
    #[serde(default)]
    pub context: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Decision {
    #[serde(rename = "permit")]
    Permit,
    #[serde(rename = "deny")]
    Deny,
    #[serde(rename = "indeterminate")]
    Indeterminate,
    #[serde(rename = "not_applicable")]
    NotApplicable,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EvaluationResponse {
    pub decision: Decision,
    #[serde(default)]
    pub context: Value,
}

/// Client for communicating with an AuthZEN PDP (Policy Decision Point)
pub struct AuthZenClient {
    pdp_url: String,
    client: reqwest::Client,
}

impl AuthZenClient {
    pub fn new(pdp_url: String) -> Self {
        Self {
            pdp_url,
            client: reqwest::Client::new(),
        }
    }

    /// Perform a single point-check evaluation
    pub async fn evaluate(&self, req: EvaluationRequest) -> bool {
        let url = format!(
            "{}/access/v1/evaluation",
            self.pdp_url.trim_end_matches('/')
        );

        let res = self.client.post(url).json(&req).send().await;

        match res {
            Ok(resp) => {
                if !resp.status().is_success() {
                    eprintln!("[authzen] PDP returned error status: {}", resp.status());
                    return false;
                }

                match resp.json::<EvaluationResponse>().await {
                    Ok(body) => matches!(body.decision, Decision::Permit),
                    Err(e) => {
                        eprintln!("[authzen] Failed to parse PDP response: {}", e);
                        false
                    }
                }
            }
            Err(e) => {
                eprintln!("[authzen] Failed to reach PDP: {}", e);
                false // Fail closed
            }
        }
    }

    /// Retrieve an authorization filter for use in search/list operations.
    /// This uses the "Residue" pattern where the PDP returns the logic required to see resources.
    pub async fn get_search_filter(&self, subject_id: &str, resource_type: &str) -> String {
        let req = EvaluationRequest {
            subject: Subject {
                id: subject_id.to_string(),
                attributes: serde_json::json!({}),
            },
            action: Action {
                name: "read".to_string(),
            },
            resource: Resource {
                id: "".to_string(), // Bulk check
                resource_type: resource_type.to_string(),
                attributes: serde_json::json!({}),
            },
            context: serde_json::json!({}),
        };

        let url = format!(
            "{}/access/v1/evaluation",
            self.pdp_url.trim_end_matches('/')
        );

        // In a real AuthZEN residue implementation, the PDP would returned a specialized response.
        // For early Topaz/AuthZEN interop, we might look for a specific context field.
        let res = self.client.post(url).json(&req).send().await;

        if let Ok(resp) = res {
            if let Ok(body) = resp.json::<EvaluationResponse>().await {
                if let Decision::Permit = body.decision {
                    // Look for a Tantivy filter in the response context
                    if let Some(filter) = body.context.get("filter").and_then(|v| v.as_str()) {
                        return filter.to_string();
                    }
                }
            }
        }

        // Fallback to hardcoded safe filter if PDP fails or doesn't provide one
        format!("involved:{}", subject_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mockito::Server;

    #[tokio::test]
    async fn test_evaluate_permit() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("POST", "/access/v1/evaluation")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"decision": "permit"}"#)
            .create_async()
            .await;

        let client = AuthZenClient::new(server.url());
        let req = EvaluationRequest {
            subject: Subject {
                id: "alice".to_string(),
                attributes: serde_json::json!({}),
            },
            action: Action {
                name: "read".to_string(),
            },
            resource: Resource {
                id: "doc1".to_string(),
                resource_type: "Issue".to_string(),
                attributes: serde_json::json!({}),
            },
            context: serde_json::json!({}),
        };

        let result = client.evaluate(req).await;
        assert!(result);
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_evaluate_deny() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/access/v1/evaluation")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"decision": "deny"}"#)
            .create_async()
            .await;

        let client = AuthZenClient::new(server.url());
        let req = EvaluationRequest {
            subject: Subject {
                id: "bob".to_string(),
                attributes: serde_json::json!({}),
            },
            action: Action {
                name: "read".to_string(),
            },
            resource: Resource {
                id: "doc1".to_string(),
                resource_type: "Issue".to_string(),
                attributes: serde_json::json!({}),
            },
            context: serde_json::json!({}),
        };

        let result = client.evaluate(req).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn test_get_search_filter_success() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/access/v1/evaluation")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(
                r#"{"decision": "permit", "context": {"filter": "involved:alice OR status:open"}}"#,
            )
            .create_async()
            .await;

        let client = AuthZenClient::new(server.url());
        let filter = client.get_search_filter("alice", "Issue").await;
        assert_eq!(filter, "involved:alice OR status:open");
    }

    #[tokio::test]
    async fn test_get_search_filter_fallback() {
        let mut server = Server::new_async().await;
        let _mock = server
            .mock("POST", "/access/v1/evaluation")
            .with_status(500)
            .create_async()
            .await;

        let client = AuthZenClient::new(server.url());
        let filter = client.get_search_filter("alice", "Issue").await;
        assert_eq!(filter, "involved:alice");
    }
}
