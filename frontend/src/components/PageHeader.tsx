import React from "react";
import NotificationBell from "./NotificationBell";
import SearchBar from "./SearchBar";
import { Link } from "react-router-dom";
import { useState } from "react";
import UserSettingsDialog from "./UserSettingsDialog";
import { useActor } from "../contexts/ActorContext";
import UserAvatar from "./UserAvatar";

interface PageHeaderProps {
  currentZaakId?: string;
}

const PageHeader: React.FC<PageHeaderProps> = ({ currentZaakId }) => {
  const [isSettingsOpen, setIsSettingsOpen] = useState(false);
  const { userInitial } = useActor();

  return (
    <>
      <header
      className="border-b sticky top-0 z-50"
      style={{
        backgroundColor: "var(--bg-secondary)",
        borderColor: "var(--border-primary)",
      }}
    >
      <div className="max-w-7xl mx-auto px-2 sm:px-4 lg:px-8">
        <div className="flex items-center justify-between h-14 sm:h-16 gap-2 sm:gap-4">
          {/* Left side - Logo/Title area */}
          <div className="flex items-center space-x-2 sm:space-x-4 flex-shrink-0">
            <div className="flex-shrink-0">
              <h1
                className="text-base sm:text-xl font-bold"
                style={{
                  color: "var(--logo-primary)",
                  transition: "color 0.2s ease",
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.color = "var(--logo-hover)";
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.color = "var(--logo-primary)";
                }}
              >
                <Link
                  to="/"
                  style={{ color: "inherit", textDecoration: "none" }}
                >
                  ZaakChat
                </Link>
              </h1>
            </div>
            <div className="hidden lg:block">
              <div
                className="text-sm"
                style={{ color: "var(--text-tertiary)" }}
              >
                Real-time zaakbehandeling
              </div>
            </div>
          </div>

          {/* Center - Search Bar */}
          <SearchBar />

          {/* Right side - Navigation and notifications */}
          <div className="flex items-center space-x-2 sm:space-x-4 flex-shrink-0">
            <Link
              to="/api-docs"
              className="block text-xs sm:text-sm font-medium"
              style={{ color: "var(--text-secondary)" }}
            >
              API Docs
            </Link>
            <NotificationBell currentZaakId={currentZaakId} />
            <button
              onClick={() => setIsSettingsOpen(true)}
              title="User Settings"
              className="focus:outline-none hover:opacity-80 transition-opacity"
            >
              <UserAvatar name={userInitial} size="sm" />
            </button>
          </div>
        </div>
      </div>
    </header>

    <UserSettingsDialog
      isOpen={isSettingsOpen}
      onClose={() => setIsSettingsOpen(false)}
    />
    </>
  );
};

export default PageHeader;
