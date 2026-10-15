import React from "react";
import Modal from "./Modal";
import { Button } from "./ActionButton";
import { useAuth } from "../contexts/AuthContext";

import { useTheme } from "../contexts/ThemeContext";

interface UserSettingsDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

const UserSettingsDialog: React.FC<UserSettingsDialogProps> = ({
  isOpen,
  onClose,
}) => {
  const { user, logout } = useAuth();
  const { themeId, setTheme, colorMode, setColorMode, availableThemes } = useTheme();

  const handleLogout = () => {
    logout();
    onClose();
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="User Settings">
      <div className="space-y-4">
        <div>
          <label
            htmlFor="email"
            className="block text-sm font-medium mb-1"
            style={{ color: "var(--text-secondary)" }}
          >
            Email Address
          </label>
          <div className="flex gap-2">
            <input
              id="email"
              type="email"
              value={user || ""}
              readOnly
              className="flex-1 px-3 py-2 rounded border focus:outline-none"
              style={{
                backgroundColor: "var(--bg-secondary)",
                borderColor: "var(--border-primary)",
                color: "var(--text-primary)",
                opacity: 0.8,
                cursor: "not-allowed"
              }}
            />
            <Button
              variant="primary"
              onClick={handleLogout}
              className="bg-red-600 hover:bg-red-700 border-red-600 whitespace-nowrap"
            >
              Sign Out
            </Button>
          </div>
        </div>

        <div>
          <label
            htmlFor="theme"
            className="block text-sm font-medium mb-1"
            style={{ color: "var(--text-secondary)" }}
          >
            Theme (NLDS-light)
          </label>
          <select
            id="theme"
            value={themeId}
            onChange={(e) => setTheme(e.target.value as any)}
            className="w-full px-3 py-2 rounded border focus:outline-none"
            style={{
              backgroundColor: "var(--bg-secondary)",
              borderColor: "var(--border-primary)",
              color: "var(--text-primary)",
            }}
          >
            {availableThemes.map((theme) => (
              <option key={theme.id} value={theme.id}>
                {theme.name}
              </option>
            ))}
          </select>
        </div>

        <div>
          <label
            htmlFor="color-mode"
            className="block text-sm font-medium mb-1"
            style={{ color: "var(--text-secondary)" }}
          >
            Color Mode
          </label>
          <select
            id="color-mode"
            value={colorMode}
            onChange={(e) => setColorMode(e.target.value as any)}
            className="w-full px-3 py-2 rounded border focus:outline-none"
            style={{
              backgroundColor: "var(--bg-secondary)",
              borderColor: "var(--border-primary)",
              color: "var(--text-primary)",
            }}
          >
            <option value="auto">Auto (System)</option>
            <option value="light">Light</option>
            <option value="dark">Dark</option>
          </select>
        </div>
      </div>

        <div className="flex justify-end gap-2 pt-4">
          <Button variant="secondary" onClick={onClose}>
            Close
          </Button>
        </div>

        <div className="text-center pt-4 border-t border-[var(--border-primary)] mt-4">
          <p className="text-xs text-[var(--text-secondary)]">
            v{__APP_VERSION__} • {new Date(__BUILD_TIMESTAMP__).toLocaleString()}
          </p>
        </div>

    </Modal>
  );
};

export default UserSettingsDialog;
