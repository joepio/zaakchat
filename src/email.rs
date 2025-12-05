use async_trait::async_trait;
use reqwest::Client;
use serde::Serialize;
use serde_json::json;
use std::sync::Arc;

#[derive(Serialize)]
struct PostmarkHeader {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Value")]
    value: String,
}

#[derive(Serialize)]
struct PostmarkEmail {
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "To")]
    to: String,
    #[serde(rename = "Subject")]
    subject: String,
    #[serde(rename = "HtmlBody")]
    html_body: String,
    #[serde(rename = "TextBody")]
    text_body: String,
    #[serde(rename = "MessageStream")]
    message_stream: String,
    #[serde(rename = "ReplyTo", skip_serializing_if = "Option::is_none")]
    reply_to: Option<String>,
    #[serde(rename = "Headers", skip_serializing_if = "Vec::is_empty")]
    headers: Vec<PostmarkHeader>,
}

#[async_trait]
pub trait EmailTransport: Send + Sync {
    async fn send_magic_link(&self, email: &str, token: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
    async fn send_notification(
        &self,
        to: &str,
        subject: &str,
        html_body: &str,
        text_body: &str,
        reply_to: Option<&str>,
        thread_id: Option<&str>, // Used for In-Reply-To and References
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

pub struct EmailService {
    transport: Arc<dyn EmailTransport>,
}

impl EmailService {
    pub fn new(transport: Arc<dyn EmailTransport>) -> Self {
        Self { transport }
    }

    pub async fn send_magic_link(&self, email: &str, token: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.transport.send_magic_link(email, token).await
    }

    pub async fn send_notification(
        &self,
        to: &str,
        subject: &str,
        html_body: &str,
        text_body: &str,
        reply_to: Option<&str>,
        thread_id: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.transport
            .send_notification(to, subject, html_body, text_body, reply_to, thread_id)
            .await
    }
}

pub struct PostmarkTransport {
    api_token: String,
    sender: String,
    base_url: String,
    client: Client,
}

impl PostmarkTransport {
    pub fn new(api_token: String, sender: String, base_url: String) -> Self {
        Self {
            api_token,
            sender,
            base_url,
            client: Client::new(),
        }
    }
}

#[async_trait]
impl EmailTransport for PostmarkTransport {
    async fn send_magic_link(&self, email: &str, token: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let magic_link = format!("{}/verify-login?token={}", self.base_url, token);

        let html_body = format!(
            r#"<html>
              <body>
                <h1>Log in to ZaakChat</h1>
                <p>Click the link below to log in:</p>
                <p><a href=\"{}\">{}</a></p>
                <p>This link will expire in 15 minutes.</p>
              </body>
            </html>"#,
            magic_link, magic_link
        );

        let text_body = format!(
            "Log in to ZaakChat\n\nClick the link below to log in:\n{}\n\nThis link will expire in 15 minutes.",
            magic_link
        );

        let email_payload = PostmarkEmail {
            from: self.sender.clone(),
            to: email.to_string(),
            subject: "Log in to ZaakChat".to_string(),
            html_body,
            text_body,
            message_stream: "outbound".to_string(),
            reply_to: None,
            headers: vec![],
        };

        let res = self.client
            .post("https://api.postmarkapp.com/email")
            .header("X-Postmark-Server-Token", &self.api_token)
            .json(&email_payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let error_text = res.text().await?;
            return Err(format!("Postmark API failed: {}", error_text).into());
        }

        println!("[email] Sent magic link to {}", email);
        Ok(())
    }

    async fn send_notification(
        &self,
        to: &str,
        subject: &str,
        html_body: &str,
        text_body: &str,
        reply_to: Option<&str>,
        thread_id: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut headers = Vec::new();
        if let Some(tid) = thread_id {
            // Use the thread_id (issue ID) to generate a stable Message-ID-like string
            // Format: <issue-id@zaakchat.nl>
            let msg_id = format!("<{}@zaakchat.nl>", tid);
            headers.push(PostmarkHeader {
                name: "In-Reply-To".to_string(),
                value: msg_id.clone(),
            });
            headers.push(PostmarkHeader {
                name: "References".to_string(),
                value: msg_id,
            });
        }

        let email_payload = PostmarkEmail {
            from: self.sender.clone(),
            to: to.to_string(),
            subject: subject.to_string(),
            html_body: html_body.to_string(),
            text_body: text_body.to_string(),
            message_stream: "outbound".to_string(),
            reply_to: reply_to.map(|s| s.to_string()),
            headers,
        };

        let res = self.client
            .post("https://api.postmarkapp.com/email")
            .header("X-Postmark-Server-Token", &self.api_token)
            .json(&email_payload)
            .send()
            .await?;

        if !res.status().is_success() {
            let error_text = res.text().await?;
            return Err(format!("Postmark API failed: {}", error_text).into());
        }

        println!("[email] Sent notification to {}", to);
        Ok(())
    }
}

pub struct MockTransport {
    base_url: String,
}

impl MockTransport {
    pub fn new(base_url: String) -> Self {
        Self { base_url }
    }
}

#[async_trait]
impl EmailTransport for MockTransport {
    async fn send_magic_link(&self, email: &str, token: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let magic_link = format!("{}/verify-login?token={}", self.base_url, token);
        let mock_path = std::path::Path::new("test_email.json");
        let mock_data = json!({
            "to": email,
            "token": token,
            "magic_link": magic_link,
        });
        std::fs::write(mock_path, serde_json::to_string_pretty(&mock_data)?)?;
        println!("[email] Mock mode: wrote magic link to {}", mock_path.display());
        println!("[email] Magic Link: {}", magic_link);
        Ok(())
    }

    async fn send_notification(
        &self,
        to: &str,
        subject: &str,
        html_body: &str,
        text_body: &str,
        reply_to: Option<&str>,
        thread_id: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mock_path = std::path::Path::new("test_notification.json");
        let mock_data = json!({
            "to": to,
            "subject": subject,
            "html_body": html_body,
            "text_body": text_body,
            "reply_to": reply_to,
            "thread_id": thread_id,
        });
        std::fs::write(mock_path, serde_json::to_string_pretty(&mock_data)?)?;
        println!("[email] Mock mode: wrote notification to {}", mock_path.display());
        Ok(())
    }
}
