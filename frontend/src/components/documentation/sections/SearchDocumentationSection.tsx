import React, { useState } from "react";
import { useAuth } from "../../../contexts/AuthContext";
import CodeBlock from "../CodeBlock";
import SmartSearchInput from "../../SmartSearchInput";

const SearchDocumentationSection: React.FC = () => {
  const { token, user: authUser } = useAuth();
  const [results, setResults] = useState<any>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleSearch = async (transformedQuery: string) => {
    setLoading(true);
    setError(null);
    setResults(null);

    try {
      const params = new URLSearchParams();
      params.append("q", transformedQuery);

      const response = await fetch(`/query?${params.toString()}`, {
        headers: {
          Authorization: `Bearer ${token}`,
        },
      });

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
    <section id="search-endpoint" className="border-t pt-8" style={{ borderColor: "var(--border-primary)" }}>
      <h2
        className="text-2xl font-semibold mb-4"
        style={{ color: "var(--ro-lintblauw)" }}
      >
        Search Endpoint & Playground
      </h2>

      <div className="space-y-6">
        <div>
          <p className="mb-3" style={{ color: "var(--text-primary)" }}>
            De applicatie bevat een krachtige full-text search endpoint op <code>/query</code>.
            Deze wordt aangedreven door <strong>Tantivy</strong>, een high-performance search engine geschreven in Rust.
          </p>
        </div>

        <div>
          <h3
            className="text-xl font-semibold mb-3"
            style={{ color: "var(--ro-lintblauw)" }}
          >
            Hoe het werkt
          </h3>
          <ul className="list-disc pl-6 mb-4 space-y-2" style={{ color: "var(--text-primary)" }}>
            <li>
              <strong>Indexing:</strong> Zowel CloudEvents als Resources (zoals Zaken en Taken) worden geïndexeerd.
            </li>
            <li>
              <strong>JSON Payload:</strong> De volledige JSON structuur wordt opgeslagen in een <code>json_payload</code> veld.
              Hierdoor kan er gezocht worden op specifieke velden binnen de JSON data.
            </li>
            <li>
              <strong>Autorisatie:</strong> Er wordt automatisch een filter toegepast op basis van de ingelogde gebruiker.
              Je ziet alleen items waar jij bij betrokken bent (<code>involved</code> veld).
            </li>
          </ul>

          <div className="my-4">
             <p className="text-gray-400 text-sm mb-2">Automatisch toegepast filter:</p>
             <CodeBlock
               language="text"
               code={`(json_payload.involved:"${authUser}" OR json_payload.data.resource_data.involved:"${authUser}")`}
             />
          </div>
        </div>

        <div>
           <h3
             className="text-xl font-semibold mb-3"
             style={{ color: "var(--ro-lintblauw)" }}
           >
             Playground
           </h3>
           <p className="mb-4" style={{ color: "var(--text-primary)" }}>
             Gebruik de onderstaande tool om live queries uit te voeren. Probeer de geavanceerde filters!
           </p>

           <div className="bg-gray-900 rounded-xl p-6 border border-gray-700">

             {/* New Smart Input Component */}
             <SmartSearchInput onSearch={handleSearch} isLoading={loading} />

             {error && (
               <div className="mt-4 p-4 bg-red-900/30 border border-red-500/50 rounded text-red-200 text-sm">
                 Error: {error}
               </div>
             )}

             {results && (
               <div className="mt-4 space-y-2">
                 <h4 className="text-sm font-semibold text-gray-300">
                   Resultaten ({results.length})
                 </h4>
                 <div className="bg-black/50 p-4 rounded-md overflow-auto max-h-[400px] border border-gray-700">
                   <pre className="font-mono text-xs text-green-400">
                     {JSON.stringify(results, null, 2)}
                   </pre>
                 </div>
               </div>
             )}
           </div>
        </div>
      </div>
    </section>
  );
};

export default SearchDocumentationSection;
