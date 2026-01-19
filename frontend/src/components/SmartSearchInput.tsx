import React, { useState, useEffect, useRef } from "react";
import { fetchSchema, fetchSchemaIndex } from "../types/interfaces";
import type { SchemaMetadata } from "../types/interfaces";
import { getSuggestions, transformQuery } from "../utils/searchUtils";
import type { Suggestion } from "../utils/searchUtils";
import { useAuth } from "../contexts/AuthContext";

interface SmartSearchInputProps {
  onSearch: (query: string) => void;
  isLoading?: boolean;
}

const SmartSearchInput: React.FC<SmartSearchInputProps> = ({ onSearch, isLoading }) => {
  const { user } = useAuth();
  const [inputValue, setInputValue] = useState("");
  const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [schemaIndex, setSchemaIndex] = useState<SchemaMetadata | null>(null);
  const [activeSchema, setActiveSchema] = useState<any>(null);

  // Derived state: what schema is currently in the query?
  const currentTypeFilter = inputValue.match(/\bis:([a-zA-Z]+)\b/)?.[1];
  // Map lowercase type back to Schema name (usually Capitalized) if possible, or just use as is
  // For simplicity, we assume Schema names match the type filter but capitalized
  const activeSchemaName = currentTypeFilter
    ? (currentTypeFilter.charAt(0).toUpperCase() + currentTypeFilter.slice(1))
    : null;

  const inputRef = useRef<HTMLInputElement>(null);
  const suggestionsRef = useRef<HTMLDivElement>(null);

  // Fetch Schema Index on mount
  useEffect(() => {
    fetchSchemaIndex().then(setSchemaIndex).catch(console.error);
  }, []);

  // Fetch active schema whenever the detected type filter changes
  useEffect(() => {
    if (activeSchemaName) {
      fetchSchema(activeSchemaName).then(setActiveSchema).catch(console.error);
    } else {
      setActiveSchema(null);
    }
  }, [activeSchemaName]);

  // Update suggestions when input changes
  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const newVal = e.target.value;
    setInputValue(newVal);

    const cursor = e.target.selectionStart || 0;
    const newSuggestions = getSuggestions(newVal, cursor, activeSchema);
    setSuggestions(newSuggestions);
    setShowSuggestions(newSuggestions.length > 0);
  };

  const handleSchemaButtonClick = (schemaName: string) => {
      // Toggle logic: if already present, remove it. If different one present, replace it. If query empty, add it.
      const type = schemaName;
      const regex = /\bis:[a-zA-Z]+\b/;

      let newValue = inputValue;
      if (regex.test(inputValue)) {
          // Replace existing is:XX
          if (inputValue.includes(`is:${type}`)) {
             // If clicking same, remove it
             newValue = inputValue.replace(regex, "").replace(/\s\s+/g, " ").trim();
          } else {
             // Replace with new
             newValue = inputValue.replace(regex, `is:${type}`);
          }
      } else {
          // Append
          newValue = `${inputValue} is:${type}`.trim();
      }

      setInputValue(newValue);
      inputRef.current?.focus();
  };

  const handleSuggestionClick = (suggestion: Suggestion) => {
    if (!inputRef.current) return;

    const cursor = inputRef.current.selectionStart || 0;
    const text = inputValue;

    // Find boundaries of current word
    const lastSpace = text.lastIndexOf(" ", cursor - 1);
    const start = lastSpace + 1;

    const currentWord = text.substring(start, cursor); // heuristic for word being typed

    let replacement = suggestion.text;

    if (suggestion.type === "key") {
      replacement += ":";
    } else if (suggestion.type === "value") {
      // If we are replacing a value, we must check if there is a key prefix (e.g. "is:")
      // If so, we should preserve it.
      if (currentWord.includes(":")) {
          const [key] = currentWord.split(":");
          replacement = `${key}:${suggestion.text}`;
      }
    }

    const before = text.substring(0, start);
    // Preserving text after is tricky if we are in middle, assuming appending for now for simplicity
    // or simple replacement of current word context.
    // We assume we are replacing until the next space or end of string.
    const nextSpace = text.indexOf(" ", cursor);
    const end = nextSpace === -1 ? text.length : nextSpace;
    const after = text.substring(end);

    const newValue = before + replacement + after;
    setInputValue(newValue);
    setShowSuggestions(false);

    // Focus and move cursor to end of replacement
    inputRef.current.focus();
    // We need to wait for state update to set cursor position reliably in React,
    // or use a ref for the next cursor position, but simply focusing is usually enough for end-of-input typing.
    // If editing in middle, cursor placement might jump to end.
    // For now, let's just focus.
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const transformed = transformQuery(inputValue, user || undefined);
    onSearch(transformed);
    setShowSuggestions(false);
  };

  // Click outside to close
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (suggestionsRef.current && !suggestionsRef.current.contains(event.target as Node) &&
          inputRef.current && !inputRef.current.contains(event.target as Node)) {
        setShowSuggestions(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  return (
    <div className="relative font-sans">
      <div className="flex gap-2 mb-2 flex-wrap">
        {schemaIndex?.schemas?.map(s => {
          const isSelected = activeSchemaName === s;
          return (
            <button
              key={s}
              onClick={() => handleSchemaButtonClick(s)}
              type="button"
              className={`px-3 py-1 text-xs rounded-full border transition-colors ${
                isSelected
                  ? "bg-blue-600 border-blue-600 text-white"
                  : "bg-gray-800 border-gray-600 text-gray-300 hover:bg-gray-700"
              }`}
            >
              {s}
            </button>
          );
        })}
      </div>

      <form onSubmit={handleSubmit} className="relative">
        <div className="flex items-center gap-2">
          <div className="relative flex-grow">
            <input
              ref={inputRef}
              type="text"
              value={inputValue}
              onChange={handleInputChange}
              onFocus={() => {
                  // Trigger suggestions on focus if empty or typing
                  const cursor = inputRef.current?.selectionStart || 0;
                  const newSuggestions = getSuggestions(inputValue, cursor, activeSchema);
                  setSuggestions(newSuggestions);
                  setShowSuggestions(newSuggestions.length > 0);
              }}
              className="w-full px-4 py-2 bg-gray-900 border border-gray-700 rounded-md text-white focus:ring-2 focus:ring-blue-500 focus:border-transparent outline-none font-mono text-sm"
              placeholder={activeSchemaName
                ? `Filter ${activeSchemaName} properties (e.g. status:open)...`
                : "Search anything or select a type filter above..."}
            />
            {/* Syntax highlighting overlay could go here but skipping for complexity/performance for now */}
          </div>
          <button
            type="submit"
            disabled={isLoading}
            className="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white rounded-md text-sm font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isLoading ? "Searching..." : "Search"}
          </button>
        </div>

        {/* Suggestions Popover */}
        {showSuggestions && suggestions.length > 0 && (
          <div
            ref={suggestionsRef}
            className="absolute z-10 w-full mt-1 bg-gray-800 border border-gray-700 rounded-md shadow-lg max-h-60 overflow-y-auto"
          >
            <ul>
              {suggestions.map((s, i) => (
                <li key={i}>
                  <button
                    type="button"
                    onClick={() => handleSuggestionClick(s)}
                    className="w-full text-left px-4 py-2 hover:bg-gray-700 flex items-center gap-2 text-sm border-b border-gray-700/50 last:border-0"
                  >
                    <span className={`font-mono font-bold ${s.type === 'key' ? 'text-blue-400' : 'text-green-400'}`}>
                      {s.text}
                    </span>
                    {s.description && (
                      <span className="text-gray-400 text-xs truncate flex-1">
                        - {s.description}
                      </span>
                    )}
                  </button>
                </li>
              ))}
            </ul>
          </div>
        )}
      </form>

      <div className="mt-2 text-xs text-gray-500">
        <p>Pro tip: Try <code>assignee:me</code> or <code>status:open</code>. The query will be automatically transformed.</p>
      </div>
    </div>
  );
};

export default SmartSearchInput;
