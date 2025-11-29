import React, { useState, useEffect } from "react";
import Modal from "./Modal";
import { Button } from "./ActionButton";
import { useActor } from "../contexts/ActorContext";

interface UserSettingsDialogProps {
  isOpen: boolean;
  onClose: () => void;
}

const UserSettingsDialog: React.FC<UserSettingsDialogProps> = ({
  isOpen,
  onClose,
}) => {
  const { actor, setActor } = useActor();
  const [email, setEmail] = useState(actor);

  useEffect(() => {
    if (isOpen) {
      setEmail(actor);
    }
  }, [isOpen, actor]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (email.trim()) {
      setActor(email.trim());
      onClose();
    }
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="User Settings">
      <form onSubmit={handleSubmit} className="space-y-4">
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
            value={email}
            onChange={(e) => setEmail(e.target.value)}
            className="w-full px-3 py-2 rounded border focus:outline-none focus:ring-2"
            style={{
              backgroundColor: "var(--bg-primary)",
              borderColor: "var(--border-primary)",
              color: "var(--text-primary)",
            }}
            placeholder="Enter your email"
            required
          />
          <p
            className="mt-1 text-xs"
            style={{ color: "var(--text-tertiary)" }}
          >
            This email will be used to identify you in the system.
          </p>
        </div>

        <div className="flex justify-end gap-2 pt-4">
          <Button variant="secondary" onClick={onClose} type="button">
            Cancel
          </Button>
          <Button variant="primary" type="submit">
            Save Changes
          </Button>
        </div>
      </form>
    </Modal>
  );
};

export default UserSettingsDialog;
