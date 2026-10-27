import React from "react";
import CodeBlock from "../CodeBlock";

const JSONCommitSection: React.FC = () => {
  return (
    <section id="json-commit">
      <h2
        className="text-2xl font-semibold mb-4"
        style={{ color: "var(--ro-lintblauw)" }}
      >
        JSONCommit - Één Event Type voor Alles
      </h2>

      <div className="space-y-6">
        <div>
          <h3
            className="text-xl font-semibold mb-3"
            style={{ color: "var(--ro-lintblauw)" }}
          >
            Het Kernidee
          </h3>
          <p className="mb-3" style={{ color: "var(--text-primary)" }}>
            Een <code>json.commit</code> event vertegenwoordigt{" "}
            <strong>
              een wijziging aan een JSON resource, geïdentificeerd door ID
            </strong>
            . Of het nu gaat om:
          </p>
          <ul className="list-disc pl-6 mb-4 space-y-2">
            <li>
              <strong>Het aanmaken</strong> van een nieuwe resource →{" "}
              <code>resource_data</code> bevat de volledige resource
            </li>
            <li>
              <strong>Het updaten</strong> van een bestaande resource →{" "}
              <code>patch</code> bevat de wijzigingen (JSON Merge Patch RFC
              7396)
            </li>
            <li>
              <strong>Het verwijderen</strong> van een resource →{" "}
              <code>deleted: true</code> markeert de resource als
              verwijderd. De hele resource (en al zijn events) worden dan uit
              de store verwijderd.
            </li>
          </ul>
          <p className="mb-3" style={{ color: "var(--text-primary)" }}>
            Het is allemaal hetzelfde event type. De velden bepalen wat er
            gebeurt.
          </p>
        </div>

        <div>
          <h3
            className="text-xl font-semibold mb-3"
            style={{ color: "var(--ro-lintblauw)" }}
          >
            Waarom is dit Beter?
          </h3>
          <div className="space-y-3">
            <div>
              <h4 className="font-semibold mb-2">
                ✅ Eenvoudiger Mentaal Model
              </h4>
              <p style={{ color: "var(--text-secondary)" }}>
                In plaats van na te denken over "welk event type moet ik
                gebruiken?", denk je alleen: "Ik commit een wijziging aan deze
                resource". Net zoals Git commits: één operatie voor alle
                veranderingen.
              </p>
            </div>
            <div>
              <h4 className="font-semibold mb-2">
                ✅ Minder Code in Consumers
              </h4>
              <p style={{ color: "var(--text-secondary)" }}>
                Event consumers hoeven niet meer te switchen op event type. Ze
                kijken gewoon: "Heeft deze commit <code>resource_data</code>? Dan
                is het een create. Heeft het <code>patch</code>? Dan is het een
                update of delete."
              </p>
            </div>
            <div>
              <h4 className="font-semibold mb-2">✅ Resource-Georiënteerd</h4>
              <p style={{ color: "var(--text-secondary)" }}>
                De focus ligt op <strong>de resource</strong> en{" "}
                <strong>de wijziging</strong>, niet op de operatie. Dit maakt
                het model intuïtiever voor ontwikkelaars: "Ik update zaak #5"
                in plaats van "Ik stuur een item.updated event voor zaak #5".
              </p>
            </div>
            <div>
              <h4 className="font-semibold mb-2">
                ✅ Event Sourcing Vriendelijk
              </h4>
              <p style={{ color: "var(--text-secondary)" }}>
                Een reeks <code>json.commit</code> events vormt een complete
                geschiedenis van een resource. Perfect voor event sourcing,
                audit trails, en het opbouwen van materialized views.
              </p>
            </div>
          </div>
        </div>

        <div>
          <h3
            className="text-xl font-semibold mb-3"
            style={{ color: "var(--ro-lintblauw)" }}
          >
            JSONCommit Data Structuur
          </h3>
          <p className="mb-3" style={{ color: "var(--text-primary)" }}>
            Het <code>data</code> veld van een <code>json.commit</code> event
            bevat altijd:
          </p>
          <CodeBlock
            language="json"
            code={`{
  "schema": "http://localhost:8000/schemas/Comment",
  "resource_id": "comment-abc123",
  "actor": "alice@gemeente.nl",
  "timestamp": "2025-09-30T12:34:56Z",

  // Voor CREATE (nieuwe resource):
  "resource_data": {
    "id": "comment-abc123",
    "content": "Dit is een nieuwe reactie",
    "author": "alice@gemeente.nl",
    "parent_id": null,
    "mentions": []
  }

  // OF voor UPDATE (bestaande resource):
  "patch": {
    "content": "Bijgewerkte tekst"
  }

  // OF voor DELETE:
  "deleted": true,
  "deletion_reason": "spam"
}`}
          />
        </div>

        <div>
          <h3
            className="text-xl font-semibold mb-3"
            style={{ color: "var(--ro-lintblauw)" }}
          >
            Meerdere commits in één keer
          </h3>
          <p className="mb-3" style={{ color: "var(--text-primary)" }}>
            Met <code>json.commit.transaction</code> stuur je een lijst
            commits in <code>data.commits</code>. De server legt ze{" "}
            <strong>allemaal of geen enkele</strong> vast: faalt één commit,
            dan wordt niets opgeslagen en krijg je een <code>422</code> met
            een verwijzing naar de foute commit. Op de stream verschijnt elke
            commit als eigen <code>json.commit</code> event, met dezelfde{" "}
            <code>transactionid</code>.
          </p>
          <CodeBlock
            language="json"
            code={`POST /events
{
  "specversion": "1.0",
  "type": "json.commit.transaction",
  "source": "frontend",
  "id": "0f8a2c4e-5b1d-4e9a-9c3f-7d6b2a1e8f40",
  "subject": "zaak-123",
  "data": {
    "commits": [
      { "resourceId": "zaak-123", "schema": "Issue",
        "patch": { "status": "in_behandeling" } },
      { "resourceId": "comment-456", "schema": "Comment",
        "resourceData": { "content": "We gaan ermee aan de slag." } }
    ]
  }
}`}
          />
          <p className="mt-3" style={{ color: "var(--text-secondary)" }}>
            Het event-<code>id</code> is de idempotency-sleutel: hetzelfde{" "}
            <code>id</code> opnieuw sturen geeft het oorspronkelijke antwoord
            terug zonder iets dubbel te doen. De server zet{" "}
            <code>actor</code> op de ingelogde gebruiker. Zowel camelCase (
            <code>resourceId</code>) als snake_case (<code>resource_id</code>)
            wordt geaccepteerd.
          </p>
        </div>

        <div>
          <h3
            className="text-xl font-semibold mb-3"
            style={{ color: "var(--ro-lintblauw)" }}
          >
            Voorbeelden
          </h3>

          <div className="space-y-4">
            <div>
              <h4 className="font-semibold mb-2">
                📝 Voorbeeld 1: Nieuwe Reactie Aanmaken
              </h4>
              <CodeBlock
                language="json"
                code={`{
  "specversion": "1.0",
  "id": "evt-12345",
  "type": "json.commit",
  "source": "frontend-app",
  "subject": "zaak-5",
  "time": "2025-09-30T14:22:00Z",
  "dataschema": "http://localhost:8000/schemas/JSONCommit",
  "data": {
    "schema": "http://localhost:8000/schemas/Comment",
    "resource_id": "comment-789",
    "actor": "bob@gemeente.nl",
    "resource_data": {
      "id": "comment-789",
      "content": "Status is bijgewerkt naar in behandeling",
      "author": "bob@gemeente.nl",
      "parent_id": null,
      "mentions": ["alice@gemeente.nl"]
    }
  }
}`}
              />
            </div>

            <div>
              <h4 className="font-semibold mb-2">
                ✏️ Voorbeeld 2: Zaak Bijwerken
              </h4>
              <CodeBlock
                language="json"
                code={`{
  "specversion": "1.0",
  "id": "evt-67890",
  "type": "json.commit",
  "source": "backend-workflow",
  "subject": "zaak-5",
  "time": "2025-09-30T14:25:00Z",
  "dataschema": "http://localhost:8000/schemas/JSONCommit",
  "data": {
    "schema": "http://localhost:8000/schemas/Issue",
    "resource_id": "5",
    "actor": "system@gemeente.nl",
    "patch": {
      "status": "in_progress",
      "assignee": "carol@gemeente.nl"
    }
  }
}`}
              />
            </div>

            <div>
              <h4 className="font-semibold mb-2">
                🗑️ Voorbeeld 3: Taak Verwijderen
              </h4>
              <p className="mb-2 text-sm" style={{ color: "var(--text-secondary)" }}>
                Wanneer <code>deleted</code> op <code>true</code> staat, wordt de hele resource verwijderd uit de store. Optionele metadata zoals <code>deletion_reason</code> kan toegevoegd worden maar wordt niet opgeslagen.
              </p>
              <CodeBlock
                language="json"
                code={`{
  "specversion": "1.0",
  "id": "evt-11223",
  "type": "json.commit",
  "source": "frontend-app",
  "subject": "zaak-5",
  "time": "2025-09-30T14:30:00Z",
  "dataschema": "http://localhost:8000/schemas/JSONCommit",
  "data": {
    "schema": "http://localhost:8000/schemas/Task",
    "resource_id": "task-456",
    "actor": "alice@gemeente.nl",
    "deleted": true,
    "deletion_reason": "duplicaat"
  }
}`}
              />
            </div>
          </div>
        </div>

        <div>
          <h3
            className="text-xl font-semibold mb-3"
            style={{ color: "var(--ro-lintblauw)" }}
          >
            Event Processing Logica
          </h3>
          <p className="mb-3" style={{ color: "var(--text-primary)" }}>
            Het verwerken van <code>json.commit</code> events is eenvoudig:
          </p>
          <CodeBlock
            language="typescript"
            code={`// Pseudocode voor event processing
function processJSONCommit(event: CloudEvent) {
  const { resource_id, resource_data, patch, deleted } = event.data;

  // DELETE: heeft deleted?
  if (deleted) {
    delete store[resource_id];
  }
  // CREATE: heeft resource_data?
  else if (resource_data) {
    store[resource_id] = resource_data;
  }
  // UPDATE: heeft patch?
  else if (patch) {
    store[resource_id] = applyMergePatch(
      store[resource_id],
      patch
    );
  }
}`}
          />
        </div>

        <div
          className="p-4 rounded-lg"
          style={{ backgroundColor: "var(--bg-secondary)" }}
        >
          <h4
            className="font-semibold mb-2"
            style={{ color: "var(--ro-lintblauw)" }}
          >
            💡 Kernpunt
          </h4>
          <p style={{ color: "var(--text-primary)" }}>
            <code>json.commit</code> vereenvoudigt het event model drastisch:{" "}
            <strong>één event type</strong> dat JSON resources bijwerkt via ID.
            Het mentale model wordt: "Ik commit een wijziging aan resource X"
            in plaats van "Ik moet kiezen tussen create/update/delete event
            types".
          </p>
        </div>
      </div>
    </section>
  );
};

export default JSONCommitSection;
