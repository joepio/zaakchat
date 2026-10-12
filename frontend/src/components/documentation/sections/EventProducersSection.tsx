import React from "react";
import Card from "../../Card";

const EventProducersSection: React.FC = () => {
  return (
    <section id="events-versturen">
      <h2
        className="text-2xl font-semibold mb-6"
        style={{ color: "var(--ro-lintblauw)" }}
      >
        3. 📤 Events Versturen (Producers)
      </h2>

      <div className="space-y-6">
        <div className="pl-6">
          <h3 className="font-semibold mb-3">HTTP POST naar /events</h3>
          <p className="mb-4">
            Verstuur een CloudEvent via een eenvoudige HTTP POST request:
          </p>
          <div
            className="p-4 font-mono text-sm overflow-x-auto"
            style={{
              backgroundColor: "var(--bg-secondary)",
              border: "1px solid var(--border-primary)",
              color: "var(--text-primary)",
            }}
          >
            <pre>{`POST /events
Content-Type: application/json

{
  "specversion": "1.0",
  "id": "evt-${Date.now()}",
  "source": "mijn-applicatie",
  "subject": "zaak-123",
  "type": "json.commit",
  "time": "2025-01-15T10:30:00Z",
  "datacontenttype": "application/json",
  "data": {
    "item_type": "issue",
    "item_id": "issue-456",
    "actor": "alice@gemeente.nl",
    "item_data": {
      "title": "Nieuwe parkeervergunning",
      "description": "Aanvraag voor parkeervergunning",
      "status": "open"
    }
  }
}`}</pre>
          </div>
        </div>

        <div className="grid md:grid-cols-3 gap-4">
          <Card padding="md">
            <h4 className="font-semibold mb-2">✨ Nieuw item maken</h4>
            <div className="text-sm space-y-1">
              <div>
                <code
                  className="px-2 py-1"
                  style={{
                    backgroundColor: "var(--ro-grijs-2)",
                    color: "var(--text-primary)",
                  }}
                >
                  type: "json.commit"
                </code>
              </div>
              <div>
                <code
                  className="px-2 py-1"
                  style={{
                    backgroundColor: "var(--ro-grijs-2)",
                    color: "var(--text-primary)",
                  }}
                >
                  item_data: volledig object
                </code>
              </div>
            </div>
          </Card>
          <Card padding="md">
            <h4 className="font-semibold mb-2">🔄 Item bijwerken</h4>
            <div className="text-sm space-y-1">
              <div>
                <code
                  className="px-2 py-1"
                  style={{
                    backgroundColor: "var(--ro-grijs-2)",
                    color: "var(--text-primary)",
                  }}
                >
                  type: "json.commit"
                </code>
              </div>
              <div>
                <code
                  className="px-2 py-1"
                  style={{
                    backgroundColor: "var(--ro-grijs-2)",
                    color: "var(--text-primary)",
                  }}
                >
                  patch: alleen wijzigingen
                </code>
              </div>
            </div>
          </Card>
          <Card padding="md">
            <h4 className="font-semibold mb-2">🗑️ Item verwijderen</h4>
            <div className="text-sm space-y-1">
              <div>
                <code
                  className="px-2 py-1"
                  style={{
                    backgroundColor: "var(--ro-grijs-2)",
                    color: "var(--text-primary)",
                  }}
                >
                  type: "json.commit"
                </code>
              </div>
              <div>
                <code
                  className="px-2 py-1"
                  style={{
                    backgroundColor: "var(--ro-grijs-2)",
                    color: "var(--text-primary)",
                  }}
                >
                  reason: "waarom"
                </code>
              </div>
            </div>
          </Card>
        </div>

        <Card padding="lg">
          <h3 className="font-semibold mb-3">JavaScript Voorbeeld</h3>
          <div
            className="p-4 font-mono text-sm overflow-x-auto"
            style={{
              backgroundColor: "var(--ro-grijs-1)",
              border: "1px solid var(--border-primary)",
              color: "var(--text-primary)",
            }}
          >
            <pre>{`const sendEvent = async (eventData) => {
  const response = await fetch('/events', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      'Authorization': \`Bearer \${token}\`,
    },
    body: JSON.stringify(eventData)
  });

  if (!response.ok) {
    throw new Error(\`HTTP \${response.status}: \${response.statusText}\`);
  }

  return response.json();
};

// Gebruik:
await sendEvent({
  specversion: "1.0",
  id: \`evt-\${Date.now()}\`,
  source: "frontend-app",
  subject: "zaak-123",
  type: "json.commit",
  time: new Date().toISOString(),
  data: {
    item_type: "issue",
    item_id: "issue-456",
    actor: "user@gemeente.nl",
    patch: {
      status: "in_progress",
      assignee: "bob@gemeente.nl"
    }
  }
});`}</pre>
          </div>
        </Card>
      </div>
    </section>
  );
};

export default EventProducersSection;
