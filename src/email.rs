use reqwest::Client;
use serde::Serialize;
use std::env;

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
}

pub async fn send_magic_link(email: &str, token: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let api_token = env::var("POSTMARK_API_TOKEN").map_err(|_| "POSTMARK_API_TOKEN not set")?;
    let sender = env::var("POSTMARK_SENDER_EMAIL").map_err(|_| "POSTMARK_SENDER_EMAIL not set")?;
    let base_url = env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());

    let magic_link = format!("{}/verify-login?token={}", base_url, token);

    let html_body = format!(
        r#"
        <html>
          <body>
            <h1>Log in to ZaakChat</h1>
            <p>Click the link below to log in:</p>
            <p><a href="{}">{}</a></p>
            <p>This link will expire in 15 minutes.</p>
          </body>
        </html>
        "#,
        magic_link, magic_link
    );

    let text_body = format!(
        "Log in to ZaakChat\n\nClick the link below to log in:\n{}\n\nThis link will expire in 15 minutes.",
        magic_link
    );

    let email_payload = PostmarkEmail {
        from: sender,
        to: email.to_string(),
        subject: "Log in to ZaakChat".to_string(),
        html_body,
        text_body,
        message_stream: "outbound".to_string(),
    };

    let client = Client::new();
    let res = client
        .post("https://api.postmarkapp.com/email")
        .header("X-Postmark-Server-Token", api_token)
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
