import { useEffect, useState } from "react";

interface PushNotificationState {
  isSupported: boolean;
  isSubscribed: boolean;
  permission: NotificationPermission;
  subscription: PushSubscription | null;
}

// VAPID public key - generated with web-push CLI
const VAPID_PUBLIC_KEY =
  "BJQSR9qm7FEPlfcSGoF_ooAszWGzQWa-BgXGKnnnx-SpQBgA1Ls-nsp1Ww-iCbxTjzNJdOw6t75ewRYmUCxt48w";

export function usePushNotifications() {
  const [state, setState] = useState<PushNotificationState>({
    isSupported: false,
    isSubscribed: false,
    permission: "default",
    subscription: null,
  });

  useEffect(() => {
    // Check if service workers and push notifications are supported
    const isSupported =
      "serviceWorker" in navigator &&
      "PushManager" in window &&
      "Notification" in window &&
      window.isSecureContext;

    if (!isSupported) {
      console.warn(
        "Push notifications not available:",
        !window.isSecureContext
          ? "Not a secure context (requires HTTPS or localhost)"
          : "Missing browser support"
      );
      return;
    }

    setState((prev) => ({ ...prev, isSupported: true }));

    // Register service worker
    registerServiceWorker();

    // Check current notification permission
    setState((prev) => ({ ...prev, permission: Notification.permission }));
  }, []);

  const registerServiceWorker = async () => {
    try {
      const registration = await navigator.serviceWorker.register("/sw.js");

      // Check if already subscribed
      const subscription = await registration.pushManager.getSubscription();
      if (subscription) {
        setState((prev) => ({
          ...prev,
          isSubscribed: true,
          subscription,
        }));
      }
    } catch (error) {
      console.error("Service Worker registration failed:", error);
    }
  };

  const requestPermission = async (): Promise<boolean> => {
    if (!state.isSupported) {
      console.warn("Push notifications not supported");
      return false;
    }

    try {
      const permission = await Notification.requestPermission();
      setState((prev) => ({ ...prev, permission }));
      return permission === "granted";
    } catch (error) {
      console.error("Error requesting notification permission:", error);
      return false;
    }
  };

  const subscribe = async (): Promise<PushSubscription | null> => {
    if (!state.isSupported) {
      console.warn("Push notifications not supported");
      return null;
    }

    try {
      // First request permission
      const hasPermission = await requestPermission();
      if (!hasPermission) {
        console.warn("Notification permission denied");
        return null;
      }

      // Get service worker registration
      const registration = await navigator.serviceWorker.ready;

      // Subscribe to push notifications
      const subscription = await registration.pushManager.subscribe({
        userVisibleOnly: true,
        applicationServerKey: urlBase64ToUint8Array(VAPID_PUBLIC_KEY) as any,
      });

      console.log("Push subscription created:", subscription);

      // Send subscription to backend
      await sendSubscriptionToBackend(subscription);

      setState((prev) => ({
        ...prev,
        isSubscribed: true,
        subscription,
      }));

      return subscription;
    } catch (error) {
      console.error("Error subscribing to push notifications:", error);
      return null;
    }
  };

  const unsubscribe = async (): Promise<boolean> => {
    if (!state.subscription) {
      return false;
    }

    try {
      await state.subscription.unsubscribe();

      // Notify backend
      await removeSubscriptionFromBackend(state.subscription);

      setState((prev) => ({
        ...prev,
        isSubscribed: false,
        subscription: null,
      }));

      return true;
    } catch (error) {
      console.error("Error unsubscribing from push notifications:", error);
      return false;
    }
  };

  return {
    ...state,
    requestPermission,
    subscribe,
    unsubscribe,
  };
}

// Helper functions

function urlBase64ToUint8Array(base64String: string): Uint8Array {
  const padding = "=".repeat((4 - (base64String.length % 4)) % 4);
  const base64 = (base64String + padding).replace(/-/g, "+").replace(/_/g, "/");

  const rawData = window.atob(base64);
  const outputArray = new Uint8Array(rawData.length);

  for (let i = 0; i < rawData.length; ++i) {
    outputArray[i] = rawData.charCodeAt(i);
  }
  return outputArray;
}

async function sendSubscriptionToBackend(
  subscription: PushSubscription
): Promise<void> {
  // TODO: Implement API endpoint to store subscription
  // This should send the subscription to your backend
  const response = await fetch("/api/push/subscribe", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(subscription),
  });

  if (!response.ok) {
    throw new Error("Failed to send subscription to backend");
  }
}

async function removeSubscriptionFromBackend(
  subscription: PushSubscription
): Promise<void> {
  // TODO: Implement API endpoint to remove subscription
  await fetch("/api/push/unsubscribe", {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(subscription),
  });
}
