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
  const { actor } = useActor();

  // Derive user initial from actor email
  const userInitial = actor ? actor.charAt(0).toUpperCase() : 'U';

  return (
    <>
      <header
      className="border-b sticky top-0 z-50"
      style={{
        backgroundColor: "var(--bg-secondary)",
        borderColor: "var(--border-primary)",
      }}
    >
      <div className="max-w-4xl mx-auto px-2 sm:px-4 lg:px-8">
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
          <div className="flex items-center space-x-1 sm:space-x-2 flex-shrink-0">
            <Link
              to="/api-docs"
              className="flex items-center justify-center h-10 px-4 rounded-full transition-colors text-xs sm:text-sm font-medium"
              style={{ color: "var(--text-secondary)" }}
              onMouseEnter={(e) => {
                e.currentTarget.style.backgroundColor = "var(--bg-hover)";
                e.currentTarget.style.color = "var(--text-primary)";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.backgroundColor = "transparent";
                e.currentTarget.style.color = "var(--text-secondary)";
              }}
            >
              API Docs
            </Link>
            <NotificationBell currentZaakId={currentZaakId} />
            <button
              onClick={() => setIsSettingsOpen(true)}
              title="Instellingen"
              className="flex items-center justify-center w-10 h-10 rounded-full transition-colors focus:outline-none cursor-pointer"
              onMouseEnter={(e) => {
                e.currentTarget.style.backgroundColor = "var(--bg-hover)";
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.backgroundColor = "transparent";
              }}
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
