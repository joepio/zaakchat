import React from "react";
import PageHeader from "../PageHeader";

import DocumentationLink from "./DocumentationLink";
import TableOfContents from "./TableOfContents";
import JSONCommitSection from "./sections/JSONCommitSection";
import WhyCloudEventsSection from "./sections/WhyCloudEventsSection";
import ThreeLevelsSection from "./sections/ThreeLevelsSection";
import EventProducersSection from "./sections/EventProducersSection";
import EventConsumersSection from "./sections/EventConsumersSection";
import GetStartedFooter from "./sections/GetStartedFooter";
import SearchDocumentationSection from "./sections/SearchDocumentationSection";

const ApiDocumentationView: React.FC = () => {
  return (
    <>
      <PageHeader />

      <div
        className="p-6 lg:p-8 xl:p-12 max-w-4xl lg:max-w-5xl xl:max-w-6xl mx-auto pt-8 lg:pt-12 xl:pt-16"
        style={{ backgroundColor: "var(--bg-primary)" }}
      >
        <div className="mb-8 lg:mb-12 xl:mb-16">
          <DocumentationLink href="/" variant="back">
            ← Terug naar Dashboard
          </DocumentationLink>
          <h1
            className="text-3xl font-bold mb-2"
            style={{ color: "var(--ro-lintblauw)" }}
          >
            API Documentatie - Real-time Events
          </h1>
          <div className="mb-4">
            <DocumentationLink
              href="/asyncapi.yaml"
              target="_blank"
              rel="noopener noreferrer"
              className="text-sm font-medium"
            >
             AsyncAPI Specificatie .yaml
            </DocumentationLink>
          </div>
        </div>

        <div className="space-y-12 lg:space-y-16 xl:space-y-20">
          {/* Table of Contents */}
          <TableOfContents
            items={[
              { id: "search-endpoint", title: "Search Endpoint & Playground" },
              { id: "json-commit", title: "JSONCommit - Één Event Type" },
              { id: "waarom-cloudevents", title: "Waarom CloudEvents?" },
              { id: "drie-niveaus", title: "De 3 Niveaus van Event Structuur" },
              { id: "events-versturen", title: "Events Versturen (Producers)" },
              { id: "events-ontvangen", title: "Events Ontvangen (Consumers)" },
            ]}
          />

          <SearchDocumentationSection />
          <JSONCommitSection />
          <WhyCloudEventsSection />
          <ThreeLevelsSection />
          <EventProducersSection />
          <EventConsumersSection />
          <GetStartedFooter />
        </div>
      </div>
    </>
  );
};

export default ApiDocumentationView;
