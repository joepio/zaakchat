import React from "react";
import Modal from "./Modal";
import { Button } from "./ActionButton";
import { useAuth } from "../contexts/AuthContext";

interface UserSettingsDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

const UserSettingsDialog: React.FC<UserSettingsDialogProps> = ({
  isOpen,
  onClose,
}) => {
  const { user, logout } = useAuth();

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
          <input
            id="email"
            type="email"
            value={user || ""}
            readOnly
            className="w-full px-3 py-2 rounded border focus:outline-none"
            style={{
              backgroundColor: "var(--bg-secondary)",
              borderColor: "var(--border-primary)",
              color: "var(--text-primary)",
              opacity: 0.8,
              cursor: "not-allowed"
            }}
          />
        </div>
      </div>

        <div className="flex justify-end gap-2 pt-4">
          <Button variant="secondary" onClick={onClose}>
            Close
          </Button>
          <Button
            variant="primary"
            onClick={handleLogout}
            className="bg-red-600 hover:bg-red-700 border-red-600"
          >
            Sign Out
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
