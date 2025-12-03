import React, { useEffect } from "react";
import { createPortal } from "react-dom";
import { Button } from "./ActionButton";

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: React.ReactNode;
  maxWidth?: string;
  headerActions?: React.ReactNode;
}

const Modal: React.FC<ModalProps> = ({
  isOpen,
  onClose,
  title,
  children,
  maxWidth = "600px",
  headerActions,
}) => {
  // Handle ESC key to close modal
  useEffect(() => {
    const handleEscapeKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        onClose();
      }
    };

    if (isOpen) {
      document.addEventListener("keydown", handleEscapeKey);
      // Prevent body scroll when modal is open
      document.body.style.overflow = "hidden";
    }

    return () => {
      document.removeEventListener("keydown", handleEscapeKey);
      document.body.style.overflow = "unset";
    };
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const modalContent = (
    <div
      className="fixed inset-0 flex items-center justify-center z-50 p-4 md:p-2"
      style={{ backgroundColor: "var(--overlay)" }}
      onClick={(e) => {
        // Only close if clicking the overlay, not the modal content
        if (e.target === e.currentTarget) {
          onClose();
        }
      }}
    >
      <div
        className="w-full max-h-[90vh] overflow-auto animate-in fade-in slide-in-from-bottom-4 duration-200"
        style={{
          maxWidth,
          backgroundColor: "var(--bg-secondary)",
          boxShadow: "var(--shadow-lg)",
          border: "1px solid var(--border-primary)",
        }}
      >
        <div
          className="flex justify-between items-center p-6 md:p-4"
          style={{ borderBottom: "1px solid var(--border-secondary)" }}
        >
          <h2
            className="text-xl font-semibold m-0"
            style={{ color: "var(--text-primary)" }}
          >
            {title}
          </h2>
          <div className="flex items-center gap-2">
            {headerActions}
            <Button
              variant="icon"
              size="sm"
              onClick={onClose}
              title="Close modal"
            >
              ×
            </Button>
          </div>
        </div>
        <div className="p-6 md:p-4" style={{ color: "var(--text-primary)" }}>
          {children}
        </div>
      </div>
    </div>
  );

  return createPortal(modalContent, document.body);
};

export default Modal;
