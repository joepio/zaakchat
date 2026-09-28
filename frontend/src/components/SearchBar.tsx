import React, { useState, useRef, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useSearch } from "../contexts/SearchContext";
import { useSSE } from "../contexts/SSEContext";

const SearchBar: React.FC = () => {
  const [query, setQuery] = useState("");
  const [isOpen, setIsOpen] = useState(false);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const { search } = useSearch();
  const { events, items } = useSSE();
  const navigate = useNavigate();
  const inputRef = useRef<HTMLInputElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);

  const results = query.trim() ? search(query) : [];

  // Reset selected index when results change
  useEffect(() => {
    setSelectedIndex(0);
  }, [results.length]);

  // Close dropdown when clicking outside
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(event.target as Node) &&
        inputRef.current &&
        !inputRef.current.contains(event.target as Node)
      ) {
        setIsOpen(false);
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  // Handle keyboard shortcuts (Cmd/Ctrl + K)
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "k") {
        e.preventDefault();
        inputRef.current?.focus();
        setIsOpen(true);
      }
    };

    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, []);

  const handleResultClick = (itemId: string, type: string) => {
    // Find the subject (zaakId) for this item
    const itemEvent = events.find((event) => {
      const data = event.data as Record<string, unknown> | undefined;
      // Support both new (resource_id) and old (item_id) field names
      return (data?.resource_id === itemId || data?.item_id === itemId);
    });

    const zaakId = itemEvent?.subject;

    if (type === "issue" && zaakId) {
      navigate(`/zaak/${zaakId}`);
    } else if (zaakId) {
      // Navigate to the zaak page with a hash to scroll to the item
      navigate(`/zaak/${zaakId}#${itemId}`);
    }

    setQuery("");
    setIsOpen(false);
  };

  // Handle keyboard navigation
  const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
    if (!isOpen || results.length === 0) return;

    switch (e.key) {
      case "ArrowDown":
        e.preventDefault();
        setSelectedIndex((prev) => Math.min(prev + 1, results.length - 1));
        break;
      case "ArrowUp":
        e.preventDefault();
        setSelectedIndex((prev) => Math.max(prev - 1, 0));
        break;
      case "Enter":
        e.preventDefault();
        if (results[selectedIndex]) {
          handleResultClick(results[selectedIndex].item_id, results[selectedIndex].type);
        }
        break;
      case "Escape":
        e.preventDefault();
        setIsOpen(false);
        inputRef.current?.blur();
        break;
    }
  };

  const getItemIcon = (type: string) => {
    switch (type) {
      case "issue":
        return <i className="fa-solid fa-clipboard" aria-hidden="true"></i>;
      case "comment":
        return <i className="fa-regular fa-comment" aria-hidden="true"></i>;
      case "task":
        return <i className="fa-solid fa-check" aria-hidden="true"></i>;
      case "planning":
        return <i className="fa-regular fa-calendar" aria-hidden="true"></i>;
      case "document":
        return <i className="fa-regular fa-file-lines" aria-hidden="true"></i>;
      default:
        return <i className="fa-solid fa-thumbtack" aria-hidden="true"></i>;
    }
  };

  const getItemTypeLabel = (type: string) => {
    switch (type) {
      case "issue":
        return "Zaak";
      case "comment":
        return "Reactie";
      case "task":
        return "Taak";
      case "planning":
        return "Planning";
      case "document":
        return "Document";
      default:
        return type;
    }
  };

  const highlightMatch = (text: string | undefined, searchQuery: string) => {
    if (!text || !searchQuery) return text || "";

    // Escape special regex characters
    const escapedQuery = searchQuery.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');

    // Try exact match first
    const exactRegex = new RegExp(`(${escapedQuery})`, "gi");
    const hasExactMatch = exactRegex.test(text);

    if (hasExactMatch) {
      const parts = text.split(exactRegex);
      return parts.map((part, index) => {
        const isMatch = exactRegex.test(part);
        exactRegex.lastIndex = 0; // Reset regex state
        return isMatch ? (
          <strong
            key={index}
            style={{
              backgroundColor: "#fef3c7",
              color: "var(--text-primary)",
              fontWeight: "600",
            }}
          >
            {part}
          </strong>
        ) : (
          part
        );
      });
    }

    // Fallback to just showing the text if no match
    return text;
  };

  return (
    <div className="relative flex-1 max-w-md mx-1 sm:mx-2 md:mx-4">
      <div className="relative">
        <input
          ref={inputRef}
          type="text"
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setIsOpen(true);
          }}
          onFocus={() => setIsOpen(true)}
          onKeyDown={handleKeyDown}
          placeholder="Zoeken..."
          className="w-full form-input text-xs sm:text-sm px-3 sm:px-4 py-1.5 sm:py-2 pl-8 sm:pl-10 pr-3 sm:pr-4 focus:outline-none focus:ring-2"
          style={{
            backgroundColor: "var(--bg-secondary)",
            borderColor: "var(--border-primary)",
            color: "var(--text-primary)",
          }}
        />
        <div
          className="absolute left-2 sm:left-3 top-1/2 transform -translate-y-1/2 text-sm sm:text-base"
          style={{ color: "var(--text-secondary)" }}
        >
          <i className="fa-solid fa-magnifying-glass" aria-hidden="true"></i>
        </div>
      </div>

      {/* Results dropdown */}
      {isOpen && query.trim() && (
        <div
          ref={dropdownRef}
          className="absolute top-full left-0 right-0 mt-2 rounded-lg shadow-lg border max-h-[70vh] sm:max-h-96 overflow-y-auto z-50"
          style={{
            backgroundColor: "var(--bg-primary)",
            borderColor: "var(--border-primary)",
          }}
        >
          {results.length > 0 ? (
            <div className="py-2">
              {results.slice(0, 10).map((result, index) => {
                const itemData = items[result.item_id] as Record<string, unknown> | undefined;
                const title = (itemData?.title || itemData?.cta || itemData?.content || result.item_id) as string;
                const description = itemData?.description as string | undefined;
                const isSelected = index === selectedIndex;

                return (
                  <button
                    key={result.id}
                    onClick={() => handleResultClick(result.item_id, result.type)}
                    className="w-full px-3 sm:px-4 py-2 sm:py-3 text-left transition-colors border-b last:border-b-0"
                    style={{
                      borderColor: "var(--border-primary)",
                      backgroundColor: isSelected ? "var(--bg-secondary)" : "transparent",
                    }}
                  >
                    <div className="flex-1 min-w-0">
                      {result.type !== "unknown" && (
                        <div className="flex items-center gap-2 mb-1">
                          <span
                            className="text-xs px-2 py-0.5 rounded-full font-medium inline-flex items-center gap-1"
                            style={{
                              backgroundColor: "var(--bg-secondary)",
                              color: "var(--text-secondary)",
                            }}
                          >
                            <span className="text-sm leading-none">
                              {getItemIcon(result.type)}
                            </span>
                            {getItemTypeLabel(result.type)}
                          </span>
                        </div>
                      )}
                      <div
                        className="font-medium text-xs sm:text-sm mb-1 line-clamp-2"
                        style={{ color: "var(--text-primary)" }}
                      >
                        {highlightMatch(title, query)}
                      </div>
                      {description && (
                        <div
                          className="text-xs line-clamp-1 sm:line-clamp-2"
                          style={{ color: "var(--text-secondary)" }}
                        >
                          {highlightMatch(description, query)}
                        </div>
                      )}
                    </div>
                  </button>
                );
              })}
            </div>
          ) : (
            <div
              className="px-4 py-8 text-center text-sm"
              style={{ color: "var(--text-secondary)" }}
            >
              Geen resultaten gevonden voor "{query}"
            </div>
          )}
        </div>
      )}
    </div>
  );
};

export default SearchBar;
