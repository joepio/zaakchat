# PWA with Background Notifications - Implementation Guide

## 🎯 Goal
Transform MijnZaken into a scalable PWA with background push notifications that can handle millions of devices.

## 📋 Current Status
✅ PWA manifest created
✅ Service Worker skeleton created
✅ Push notification hook created
⏳ Backend push notification system (needs implementation)
⏳ VAPID keys (needs generation)

---

## 🏗️ Architecture Overview

### Why Move Away from SSE?

**Current (SSE):**
```
Client 1 ─────┐
Client 2 ─────┤
Client 3 ─────┼──► Backend (holds N open connections)
  ...         │
Client N ─────┘
```
- ❌ Millions of persistent HTTP connections
- ❌ High memory usage on server
- ❌ Can't send notifications when app is closed
- ❌ Doesn't work offline

**Target (Web Push):**
```
Clients ──► Subscribe once ──► Backend stores token

When event occurs:
Backend ──► Push Service (FCM/APNS) ──► Millions of devices
```
- ✅ No persistent connections
- ✅ Works when app is closed
- ✅ Scales to millions
- ✅ Battery efficient

---

## 📦 Phase 1: PWA Basics (DONE)

### Files Created:
1. `frontend/public/manifest.json` - PWA manifest
2. `frontend/public/sw.js` - Service worker
3. `frontend/src/hooks/usePushNotifications.ts` - Push notification hook
4. `frontend/index.html` - Updated with PWA meta tags

### Next Steps:
1. Create PWA icons (192x192 and 512x512)
2. Test service worker registration
3. Test "Add to Home Screen" functionality

---

## 🔐 Phase 2: Generate VAPID Keys

VAPID (Voluntary Application Server Identification) keys are needed for Web Push.

### Generate Keys:

```bash
# Option 1: Using web-push CLI
npm install -g web-push
web-push generate-vapid-keys

# Option 2: Using Rust (add to backend)
cargo add web-push
```

### Store Keys Securely:
```env
# .env file (NEVER commit this!)
VAPID_PUBLIC_KEY=BN...
VAPID_PRIVATE_KEY=...
VAPID_SUBJECT=mailto:your-email@example.com
```

### Update Frontend:
Replace `YOUR_VAPID_PUBLIC_KEY_HERE` in `usePushNotifications.ts` with your public key.

---

## 🛠️ Phase 3: Backend Implementation

### 3.1 Add Dependencies to `Cargo.toml`:

```toml
[dependencies]
# ... existing dependencies
web-push = "0.9"
base64 = "0.22"

# For production-scale notification queue
redis = { version = "0.24", features = ["tokio-comp"], optional = true }
```

### 3.2 Create Push Subscription Storage

**Option A: In-Memory (Development)**
```rust
// In main.rs
struct AppState {
    // ... existing fields
    push_subscriptions: Arc<RwLock<HashMap<String, PushSubscription>>>,
}

#[derive(Clone, Serialize, Deserialize)]
struct PushSubscription {
    endpoint: String,
    keys: SubscriptionKeys,
    user_id: Option<String>, // Optional: link to user
}

#[derive(Clone, Serialize, Deserialize)]
struct SubscriptionKeys {
    p256dh: String,
    auth: String,
}
```

**Option B: Database (Production)**
```sql
CREATE TABLE push_subscriptions (
    id UUID PRIMARY KEY,
    endpoint TEXT NOT NULL UNIQUE,
    p256dh TEXT NOT NULL,
    auth TEXT NOT NULL,
    user_id TEXT,
    zaak_id TEXT,  -- Subscribe to specific zaak
    created_at TIMESTAMP DEFAULT NOW(),
    last_used TIMESTAMP
);

CREATE INDEX idx_subscriptions_zaak ON push_subscriptions(zaak_id);
```

### 3.3 Add API Endpoints

```rust
// POST /api/push/subscribe
async fn subscribe_push(
    State(state): State<AppState>,
    Json(subscription): Json<PushSubscription>,
) -> Result<StatusCode, StatusCode> {
    let mut subs = state.push_subscriptions.write().await;
    subs.insert(subscription.endpoint.clone(), subscription);
    Ok(StatusCode::CREATED)
}

// POST /api/push/unsubscribe
async fn unsubscribe_push(
    State(state): State<AppState>,
    Json(subscription): Json<PushSubscription>,
) -> Result<StatusCode, StatusCode> {
    let mut subs = state.push_subscriptions.write().await;
    subs.remove(&subscription.endpoint);
    Ok(StatusCode::OK)
}

// Add to router:
.route("/api/push/subscribe", post(subscribe_push))
.route("/api/push/unsubscribe", post(unsubscribe_push))
```

### 3.4 Send Push Notifications

```rust
use web_push::*;

async fn send_push_notification(
    subscription: &PushSubscription,
    title: &str,
    body: &str,
    url: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let vapid_private_key = std::env::var("VAPID_PRIVATE_KEY")?;

    let payload = serde_json::json!({
        "title": title,
        "body": body,
        "icon": "/icon-192.png",
        "data": {
            "url": url
        }
    });

    let mut builder = WebPushMessageBuilder::new(&subscription)?;
    builder.set_payload(ContentEncoding::Aes128Gcm, payload.to_string().as_bytes());
    builder.set_vapid_signature(VapidSignatureBuilder::from_base64(
        &vapid_private_key,
        URL_SAFE_NO_PAD,
        &subscription.endpoint,
    )?);

    let client = WebPushClient::new()?;
    client.send(builder.build()?).await?;

    Ok(())
}
```

### 3.5 Trigger Notifications on Events

```rust
// Modify handle_event to send push notifications
async fn handle_event(
    State(state): State<AppState>,
    Json(event): Json<Value>,
) -> Result<StatusCode, StatusCode> {
    // ... existing validation logic

    // Send push notifications to subscribed clients
    if let Some(subject) = event.get("subject").and_then(|s| s.as_str()) {
        let subs = state.push_subscriptions.read().await;

        for subscription in subs.values() {
            // Filter: only send to subscribers interested in this zaak
            if let Some(zaak_id) = &subscription.zaak_id {
                if zaak_id == subject {
                    let title = format!("Update voor zaak {}", subject);
                    let body = "Er is een nieuwe update";
                    let url = format!("/zaak/{}", subject);

                    tokio::spawn(async move {
                        if let Err(e) = send_push_notification(
                            subscription, &title, &body, &url
                        ).await {
                            eprintln!("Failed to send push: {}", e);
                        }
                    });
                }
            }
        }
    }

    // ... existing broadcast logic

    Ok(StatusCode::ACCEPTED)
}
```

---

## 📊 Phase 4: Production Scale

### 4.1 Message Queue Architecture

For millions of devices, use a job queue:

```
Event occurs ──► Add to queue (Redis/RabbitMQ)
                       ↓
              Worker processes (horizontal scaling)
                       ↓
              Batch send notifications
              (1000s per second)
```

### 4.2 Use External Push Service

**Option 1: Firebase Cloud Messaging (FCM)**
- ✅ Free tier: unlimited messages
- ✅ Works with Web Push Protocol
- ✅ Handles millions of devices
- ✅ Automatic retry logic

**Option 2: AWS SNS / Azure Notification Hubs**
- ✅ Enterprise grade
- ✅ Pay per notification
- ✅ Global distribution

### 4.3 Database for Subscriptions

Use PostgreSQL with proper indexing:
```sql
-- Fast lookups by zaak
CREATE INDEX idx_zaak_active ON push_subscriptions(zaak_id)
WHERE active = true;

-- Cleanup inactive subscriptions
DELETE FROM push_subscriptions
WHERE last_used < NOW() - INTERVAL '90 days';
```

### 4.4 Rate Limiting & Batching

```rust
// Batch notifications every 5 seconds
let mut batch = Vec::new();
for subscription in subscriptions {
    batch.push(subscription);

    if batch.len() >= 1000 {
        send_batch_notifications(&batch).await;
        batch.clear();
    }
}
```

---

## 🧪 Phase 5: Testing

### Test Locally:

1. **Install PWA:**
   ```
   Open Chrome DevTools → Application → Manifest
   Click "Add to Home Screen"
   ```

2. **Test Service Worker:**
   ```
   Chrome DevTools → Application → Service Workers
   Check registration status
   ```

3. **Test Push Notifications:**
   ```javascript
   // In browser console
   Notification.requestPermission()
   ```

4. **Test Offline:**
   ```
   Chrome DevTools → Network → Offline
   Verify app still loads
   ```

### Load Testing:

```bash
# Simulate 10,000 subscriptions
ab -n 10000 -c 100 -T application/json \
   -p subscription.json \
   http://localhost:8000/api/push/subscribe
```

---

## 🚀 Deployment Checklist

- [ ] Generate VAPID keys
- [ ] Store keys in environment variables
- [ ] Update VAPID public key in frontend
- [ ] Add PWA icons (192x192, 512x512)
- [ ] Test service worker registration
- [ ] Implement backend subscription storage
- [ ] Implement notification sending logic
- [ ] Add rate limiting
- [ ] Set up monitoring (track failed notifications)
- [ ] Test on HTTPS (required for service workers)
- [ ] Add analytics (subscription rates, notification clicks)

---

## 📱 User Experience

### Enable Notifications Flow:

```
1. User visits app
2. Banner: "Enable notifications for real-time updates?"
3. User clicks "Enable"
4. Browser shows permission dialog
5. If granted: subscribe in background
6. Show success message
```

### Notification Examples:

**New Comment:**
```
Title: "Nieuw bericht voor Zaak #123"
Body: "Alice heeft een reactie geplaatst"
Click: Navigate to /zaak/123#comment-456
```

**Status Change:**
```
Title: "Status gewijzigd: Zaak #123"
Body: "Status veranderd naar 'In behandeling'"
Click: Navigate to /zaak/123
```

---

## 💰 Cost Estimation (Production)

### For 1 Million Active Users:
- **FCM**: Free
- **Database (PostgreSQL)**: ~$50/month (Digital Ocean)
- **Redis (queue)**: ~$15/month
- **Backend hosting**: ~$100/month (4 workers)
- **Total**: ~$165/month

### Scalability:
- 1M devices: ~$165/month
- 10M devices: ~$500/month (horizontal scaling)
- 100M devices: ~$2,000/month + CDN

---

## 🔒 Security Considerations

1. **HTTPS Required**: Service workers only work on HTTPS
2. **VAPID Keys**: Keep private key secret, rotate periodically
3. **Subscription Validation**: Verify subscriptions before storing
4. **Rate Limiting**: Prevent abuse (max subscriptions per IP)
5. **User Privacy**: Allow users to unsubscribe easily
6. **Data Retention**: Delete inactive subscriptions after 90 days

---

## 📚 Resources

- [Web Push Protocol](https://web.dev/push-notifications-overview/)
- [Service Workers API](https://developer.mozilla.org/en-US/docs/Web/API/Service_Worker_API)
- [FCM for Web](https://firebase.google.com/docs/cloud-messaging/js/client)
- [web-push Rust crate](https://docs.rs/web-push/latest/web_push/)

---

## ⚡ Quick Start

To get started with PWA today:

1. Build the frontend: `pnpm build`
2. Serve over HTTPS (required for service workers)
3. Open DevTools → Application → Manifest
4. Click "Add to Home Screen"
5. Test offline mode

For full push notifications, complete Phase 2-4 above.

