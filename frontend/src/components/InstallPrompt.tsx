import React, { useEffect, useState } from "react";

// Some browsers (Chrome) expose a BeforeInstallPromptEvent not typed in TS by default
interface BeforeInstallPromptEvent extends Event {
  prompt: () => Promise<void>;
  userChoice: Promise<{ outcome: "accepted" | "dismissed"; platform: string }>;
}

function isStandalone(): boolean {
  // iOS Safari uses navigator.standalone
  // Other browsers support display-mode media query
  const mql = window.matchMedia && window.matchMedia("(display-mode: standalone)");
  const iosStandalone = (window.navigator as any).standalone === true;
  return (mql && mql.matches) || iosStandalone;
}

function isIOS(): boolean {
  return /iphone|ipad|ipod/i.test(window.navigator.userAgent);
}

const InstallPrompt: React.FC = () => {
  const [deferredPrompt, setDeferredPrompt] = useState<BeforeInstallPromptEvent | null>(null);
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    if (isStandalone()) {
      setVisible(false);
      return;
    }

    const handleBeforeInstall = (e: Event) => {
      e.preventDefault();
      setDeferredPrompt(e as BeforeInstallPromptEvent);
      setVisible(true);
    };

    const handleInstalled = () => {
      setVisible(false);
      setDeferredPrompt(null);
    };

    window.addEventListener("beforeinstallprompt", handleBeforeInstall);
    window.addEventListener("appinstalled", handleInstalled);

    return () => {
      window.removeEventListener("beforeinstallprompt", handleBeforeInstall);
      window.removeEventListener("appinstalled", handleInstalled);
    };
  }, []);

  if (!visible && !isIOS()) return null;
  if (isStandalone()) return null;

  const onInstallClick = async () => {
    if (deferredPrompt) {
      await deferredPrompt.prompt();
      try {
        await deferredPrompt.userChoice;
      } finally {
        setDeferredPrompt(null);
        setVisible(false);
      }
    } else {
      // No prompt available (likely iOS) → show help card only
      setVisible(false);
    }
  };

  return (
    <div
      className="fixed inset-x-0 bottom-0 z-50 px-3 sm:px-4 pb-safe"
      style={{ pointerEvents: "none" }}
    >
      <div
        className="mx-auto max-w-3xl rounded-t-lg border shadow-lg p-3 sm:p-4 flex items-center justify-between gap-3"
        style={{
          backgroundColor: "var(--bg-secondary)",
          borderColor: "var(--border-primary)",
          color: "var(--text-primary)",
          pointerEvents: "auto",
        }}
      >
        <div className="flex items-start gap-3">
          <span className="text-xl">📲</span>
          <div className="text-sm">
            <div className="font-semibold">Installeer deze app</div>
            {isIOS() ? (
              <div style={{ color: "var(--text-secondary)" }}>
                Open het deelmenu en kies <strong>Voeg toe aan beginscherm</strong>.
              </div>
            ) : (
              <div style={{ color: "var(--text-secondary)" }}>
                Voeg toe aan je beginscherm voor een snellere, fullscreen ervaring.
              </div>
            )}
          </div>
        </div>

        <div className="flex items-center gap-2">
          {!isIOS() && (
            <button
              onClick={onInstallClick}
              className="px-3 py-1.5 text-sm rounded border"
              style={{
                backgroundColor: "var(--button-primary-bg)",
                color: "var(--text-inverse)",
                borderColor: "var(--button-primary-bg)",
              }}
            >
              Installeren
            </button>
          )}
          <button
            onClick={() => setVisible(false)}
            className="px-3 py-1.5 text-sm rounded border"
            style={{
              backgroundColor: "var(--button-secondary-bg)",
              color: "var(--text-primary)",
              borderColor: "var(--border-primary)",
            }}
          >
            Later
          </button>
        </div>
      </div>
    </div>
  );
};

export default InstallPrompt;
