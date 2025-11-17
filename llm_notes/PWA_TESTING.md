# 🧪 Testing Your PWA with Push Notifications

## ✅ What's Been Implemented

1. ✅ Service Worker (`frontend/public/sw.js`)
2. ✅ PWA Manifest (`frontend/public/manifest.json`)
3. ✅ Push Notification Hook (`usePushNotifications`)
4. ✅ Backend Push API (`/api/push/subscribe`, `/api/push/unsubscribe`)
5. ✅ Auto-send notifications when events occur
6. ✅ Push Notification toggle button in header

## 🚀 How to Test

### Step 1: Run the Server
```bash
cargo run
```

Server will start on `http://localhost:8000`

⚠️ **Important**: Service Workers require **HTTPS** in production, but `localhost` is allowed for development!

### Step 2: Open the App

1. Open Chrome/Edge: `http://localhost:8000`
2. Open DevTools (F12)
3. Go to **Application** tab

### Step 3: Check PWA Installation

In DevTools → Application tab:
- **Manifest**: Should show "MijnZaken"
- **Service Workers**: Should show registered service worker

### Step 4: Enable Push Notifications

1. Click the **"🔔 Notificaties inschakelen"** button in the header
2. Browser will ask for permission → Click **Allow**
3. Button should change to **"🔕 Notificaties uitschakelen"**
4. Check console: Should see "Push subscription created"

### Step 5: Test Notifications

**Option A: Create a Comment**
1. Navigate to any zaak (issue)
2. Add a comment
3. You should receive a push notification!

**Option B: Trigger from Another Tab**
1. Keep the app open in one tab
2. Open another tab to `http://localhost:8000`
3. Navigate to a zaak and add a comment
4. First tab should receive notification

**Option C: Test with App Closed** ⭐
1. Subscribe to notifications
2. Close the browser tab completely
3. In terminal, use curl to send an event:
   ```bash
   curl -X POST http://localhost:8000/events \
     -H "Content-Type: application/json" \
     -d '{
       "specversion": "1.0",
       "id": "test-123",
       "type": "json.commit",
       "source": "test",
       "subject": "zaak-1",
       "data": {
         "schema": "http://localhost:8000/schemas/Comment",
         "resource_id": "comment-123",
         "resource_data": {"id": "comment-123", "content": "Test from API", "author": "system"}
       }
     }'
   ```
4. You should receive notification even with browser closed! 🎉

## 🔍 Debugging

### Check Service Worker Status
DevTools → Application → Service Workers
- Should show "activated and running"
- Should show source as `/sw.js`

### Check Push Subscription
In browser console:
```javascript
navigator.serviceWorker.ready.then(reg => {
  reg.pushManager.getSubscription().then(sub => {
    console.log('Subscription:', sub);
  });
});
```

### Check Backend Subscriptions
Look at server logs when you click the button:
```
✅ New push subscription added. Total subscriptions: 1
```

### Test Notification Manually
In browser console:
```javascript
new Notification('Test', {
  body: 'If you see this, notifications work!',
  icon: '/icon-192.png'
});
```

### Check if Notifications are Sent
When you add a comment, check server logs:
```
📤 Push notification sent successfully
```

## 🐛 Troubleshooting

### "Push notifications not supported"
- ✅ Use Chrome, Edge, or Firefox
- ❌ Safari has limited support
- ✅ Make sure it's `localhost` or HTTPS

### "Notification permission denied"
1. Click the 🔒 lock icon in address bar
2. Go to Site Settings
3. Change Notifications to "Allow"
4. Reload page

### Service Worker not registering
1. DevTools → Application → Service Workers
2. Click "Unregister"
3. Reload page
4. Should re-register automatically

### No notifications received
1. Check browser console for errors
2. Check server logs for "Push notification sent"
3. Make sure you're subscribed (button shows "uitschakelen")
4. Try closing and reopening browser

### Notifications work in tab but not when closed
- This is normal! Background notifications only work when:
  - Permission is granted
  - Service worker is installed
  - App was recently used (browser keeps SW alive)

## 📊 Check What's Working

### In Browser (DevTools → Application):
```
✅ Manifest: present
✅ Service Worker: activated
✅ Push subscription: present
✅ Notifications: enabled
```

### In Server Logs:
```
✅ New push subscription added. Total subscriptions: 1
📤 Push notification sent successfully
```

### Visual Confirmation:
```
✅ Button shows "🔕 Notificaties uitschakelen"
✅ Notifications appear when events occur
✅ Clicking notification opens the zaak
```

## 🎯 Next Steps

Once basic testing works:

1. **Add Icons**: Create `frontend/public/icon-192.png` and `icon-512.png`
2. **Test Offline**: DevTools → Network → Offline checkbox
3. **Install PWA**: Click "Install" button in Chrome address bar
4. **Test on Mobile**: Deploy to HTTPS and test on phone
5. **Production Deploy**: Add VAPID private key to environment variables

## 💡 Pro Tips

- **Test Multiple Devices**: Each device gets its own subscription
- **Monitor Subscriptions**: Server logs show total subscription count
- **Test Failure Cases**: Turn off internet, see notifications queue
- **Check Performance**: DevTools → Lighthouse → PWA audit

## 🚀 Production Checklist

Before deploying:

- [ ] Move VAPID private key to environment variable
- [ ] Add real PWA icons (192x192, 512x512)
- [ ] Test on HTTPS domain
- [ ] Test "Add to Home Screen" on mobile
- [ ] Add notification preferences (per-zaak subscriptions)
- [ ] Add unsubscribe logic when user logs out
- [ ] Monitor failed notifications
- [ ] Add rate limiting

Enjoy your fully functional PWA with push notifications! 🎉

