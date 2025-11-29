import React, { useState, useEffect } from "react";
import { useActor } from "../contexts/ActorContext";
import { Button } from "./ActionButton";
import PageHeader from "./PageHeader";

const SearchPlayground: React.FC = () => {
  const { actor } = useActor();
  const [query, setQuery] = useState("*");
  const [user, setUser] = useState(actor);

  useEffect(() => {
    setUser(actor);
  }, [actor]);
  const [results, setResults] = useState<any>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSearch = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setError(null);
    setResults(null);

    try {
      const params = new URLSearchParams();
      params.append("q", query);
      if (user) {
        params.append("user", user);
      }

      const response = await fetch(`/query?${params.toString()}`);
      if (!response.ok) {
        throw new Error(`Search failed: ${response.status} ${response.statusText}`);
      }

      const data = await response.json();
      setResults(data);
    } catch (err: any) {
      setError(err.message || "An error occurred");
    } finally {
      setLoading(false);
    }
  };

  return (
    <div
      className="min-h-screen"
      style={{
        backgroundColor: "var(--bg-primary)",
        color: "var(--text-primary)",
      }}
    >
      <PageHeader />
      <div className="max-w-4xl mx-auto p-8 pt-12">
        <h1 className="text-3xl font-bold mb-8">Search Playground</h1>

        <form onSubmit={handleSearch} className="space-y-4 mb-8">
          <div className="flex flex-col gap-2">
            <label htmlFor="query" className="font-semibold">
              Query (q)
            </label>
            <input
              id="query"
              type="text"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              className="p-2 rounded border border-gray-600 bg-gray-800 text-white"
              placeholder="e.g. *, hallo, involved:alice@example.com"
            />
          </div>

          <div className="flex flex-col gap-2">
            <label htmlFor="user" className="font-semibold">
              User (optional)
            </label>
            <input
              id="user"
              type="text"
              value={user}
              onChange={(e) => setUser(e.target.value)}
              className="p-2 rounded border border-gray-600 bg-gray-800 text-white"
              placeholder="e.g. joepmeindertsma@gmail.com"
            />
          </div>

          <Button type="submit" variant="primary" disabled={loading}>
            {loading ? "Searching..." : "Search"}
          </Button>
        </form>

        {error && (
          <div className="p-4 mb-8 bg-red-900/50 border border-red-500 rounded text-red-200">
            Error: {error}
          </div>
        )}

        {results && (
          <div className="space-y-4">
            <h2 className="text-xl font-semibold">
              Results ({results.length})
            </h2>
            <div className="bg-gray-900 p-4 rounded overflow-auto max-h-[600px]">
              <pre className="font-mono text-sm text-green-400">
                {JSON.stringify(results, null, 2)}
              </pre>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

export default SearchPlayground;
