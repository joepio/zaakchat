# True Issue Logic: Zaaktype-Driven Process Model

## Goal

ZaakChat currently treats an issue mostly as a generic conversation thread with a status, tasks, planning moments, documents, and comments. That is useful for a demo, but it does not yet capture the domain-specific logic that makes a government case a real zaak: the legal process, the expected stages, the responsible roles, the required documents, the allowed decisions, the possible outcomes, and the deadlines around them.

The goal of true issue logic is to make an issue an instance of a versioned zaaktype. A zaaktype describes what can and should happen. An issue records what actually happened.

The `zaaktypen/` directory is a good example of the source material we need to ingest. The Omgevingswet ZTC v1.9 workbooks contain structured data for statuses, checklists, roles, information objects, decisions, results, subcases, and related case types. They are not directly executable process definitions, but they are close enough to populate most of a first-class process model.

## Design Principles

1. **Imported definitions are canonical input, not runtime truth**

   Excel, Word, BPMN, or other external files should be imported into normalized, versioned JSON resources. Runtime behavior should use the normalized representation, not parse spreadsheets on the fly.

2. **Zaaktype versions are immutable**

   A running issue should reference a specific zaaktype version, for example `omgevingswet/10@1.9`. Updating the catalog creates a new definition version. Existing issues continue on their original version unless explicitly migrated.

3. **The process model must be inspectable**

   Municipal staff and auditors must be able to see why a step, task, deadline, role assignment, or decision was created. Logic that cannot be inspected or explained should not be the primary representation for legal workflow.

4. **Automation is layered on top of the model**

   The base model describes states, obligations, transitions, documents, roles, and outcomes. Automation modules may classify input, assign people, call external APIs, request missing data, or propose decisions, but their effects must be emitted as auditable events.

5. **Every runtime effect is an event**

   Importing a zaaktype, creating an issue, changing status, creating a task, asking for information, assigning a role, missing a deadline, or producing a decision should be stored as CloudEvents / JSONCommit events.

## Source Data We Can Import

From the ZTC workbooks, each zaaktype has roughly these importable parts:

| Workbook sheet | Runtime meaning |
| --- | --- |
| `Zaaktype` | Case type metadata: id, description, category, legal basis, confidentiality, validity dates |
| `Statussen` | Ordered stages of the process, status text, citizen visibility, expected duration, required documents |
| `Checklists` | Human or automated tasks/questions tied to a status |
| `Rollen` | Roles, actor types, and which statuses they may set |
| `Objecten` | Domain objects the case may concern |
| `Eigenschappen` | Structured properties that can be collected on the case |
| `Informatieobjecten` | Incoming, outgoing, and internal document types |
| `Besluiten` | Formal decision types that may be produced |
| `Resultaten` | Final outcomes, archive nomination, retention period, selectielijst class |
| `Deelzaken` | Child case types that may be opened |
| `Gerelateerde zaken` | Related case types and relation semantics |

This source data gives us a strong static skeleton. It does not fully define:

- Branching conditions.
- Transition guards.
- Assignment rules.
- SLA escalation rules beyond basic durations.
- Data extraction rules.
- External API calls.
- Decision predicates.
- Citizen-facing explanation text for every branch.

Those must be added in a ZaakChat-native logic layer.

## Core Runtime Concepts

### ZaakTypeDefinition

A versioned definition imported from source material and optionally enriched by humans or code.

```json
{
  "id": "omgevingswet/10",
  "version": "1.9",
  "title": "Aanvraag beschikking regulier behandelen",
  "source": {
    "kind": "ztc-xlsx",
    "files": [
      "zaaktypen/10_-_Aanvraag_beschikking_regulier_behandelen_V1_9.xlsx",
      "zaaktypen/00_-_Zaaktypecatalogus_Omgevingswet_V1_9.xlsx"
    ],
    "imported_at": "2026-05-04T00:00:00Z"
  },
  "statuses": [],
  "transitions": [],
  "roles": [],
  "documents": [],
  "tasks": [],
  "decisions": [],
  "results": [],
  "relations": [],
  "automation": []
}
```

### Issue

An issue references a zaaktype definition and stores current runtime state.

```json
{
  "id": "zaak-2026-000123",
  "title": "Aanvraag omgevingsvergunning Dorpsstraat 12",
  "zaaktype": "omgevingswet/10@1.9",
  "status": "beschikking_vastgesteld",
  "assignee": "alice@gemeente.nl",
  "involved": ["applicant@example.nl"],
  "properties": {
    "projectnummer": "P-12345"
  }
}
```

### ProcessInstance

The process instance is a materialized view derived from events. It answers questions like:

- What is the current status?
- Which checklist items are still open?
- Which required documents are missing?
- Which deadlines are active?
- Which roles may perform the next action?
- Which transitions are currently allowed?
- Which citizen-facing planning moments should be shown?

This view can be rebuilt by replaying events.

## Option 1: JSON Schema + DSL Per Zaaktype

In this approach, every zaaktype is represented as JSON. JSON Schema validates the shape. Process logic is expressed in a small domain-specific language.

Example:

```json
{
  "statuses": [
    {
      "id": "zaak_gestart",
      "title": "Zaak gestart",
      "duration": { "value": 0, "unit": "calendar_days" },
      "inform_citizen": false,
      "required_documents": ["Aanvraag", "Ontvangstbevestiging"]
    },
    {
      "id": "intake_afgerond",
      "title": "Intake afgerond"
    }
  ],
  "transitions": [
    {
      "from": "zaak_gestart",
      "to": "intake_afgerond",
      "when": {
        "all": [
          { "document.exists": "Aanvraag" },
          { "checklist.completed": "registratie" }
        ]
      },
      "effects": [
        { "task.create": "toets_volledigheid" },
        { "deadline.start": "beslistermijn" }
      ]
    }
  ]
}
```

### Strengths

- Fits the current ZaakChat architecture: JSON resources, JSON Schema, JSONCommit, generated forms.
- Easy to version, diff, review, store, query, and sign.
- Easy to import from spreadsheets.
- Easy to render as UI: timelines, checklists, role matrices, document requirements.
- Easier than code for policy staff to inspect.
- Can be converted to diagrams.

### Weaknesses

- A DSL must be designed and maintained.
- Complex procedural logic can become awkward.
- Without discipline, the DSL grows into a bad programming language.
- Some users will still find JSON inaccessible.

### Good Fit

Use this as the canonical model for statuses, tasks, documents, deadlines, roles, results, relations, guards, and simple effects.

## Option 2: BPMN / DMN / Decision Modelling Standards

In this approach, zaak logic is modeled with established standards:

- BPMN for process flow.
- DMN for decisions.
- Possibly CMMN for case management, because government cases are often less linear than pure workflow diagrams suggest.

### Strengths

- Familiar in government process modeling.
- Visual by default.
- Tooling exists for diagrams, validation, and simulation.
- BPMN is a better exchange format when municipalities already maintain process models that way.
- DMN is a good fit for decision tables, eligibility checks, routing, and legal rule matrices.

### Weaknesses

- BPMN XML is not pleasant as an application-native runtime model.
- BPMN engines are heavy relative to ZaakChat's current event-sourced JSON architecture.
- BPMN often models the happy path well but struggles with messy casework unless carefully constrained.
- Mapping BPMN tasks/events/gateways to user-facing issue resources still needs a native model.
- Municipal BPMN diagrams may be incomplete, informal, or inconsistent across teams.

### Good Fit

Use BPMN/DMN as import/export formats and visual authoring formats, not necessarily as the internal runtime representation. DMN is especially useful for decisions that should be inspectable as tables.

## Option 3: Everything Is Code

In this approach, a zaaktype is a module written in JavaScript, MoonBit, WebAssembly, or another sandboxed language. The module receives the issue state and events, then emits effects.

Example shape:

```ts
export function evaluate(ctx: IssueContext): Effect[] {
  if (ctx.hasDocument("Aanvraag") && ctx.checklistDone("registratie")) {
    return [
      ctx.transitionTo("intake_afgerond"),
      ctx.createTask("toets_volledigheid")
    ];
  }

  return [];
}
```

### Strengths

- Most powerful option.
- Handles complex branching, integrations, scoring, external data, AI calls, and custom calculations.
- Easy for developers to test with normal unit tests.
- A WASM boundary can make execution portable and sandboxed.

### Weaknesses

- Harder for non-developers to inspect or change.
- Harder to visualize.
- Harder to prove compliance.
- Requires sandboxing, permissions, deterministic execution rules, resource limits, and dependency governance.
- Without guardrails, business policy disappears into opaque code.

### Good Fit

Use code for plugins and automation adapters, not as the only source of truth. Code should enrich or act on a declared process model, and every action it takes should be explainable in terms of input events and emitted effects.

## Recommended Architecture: Hybrid, Model-First

The best design is not one of the three options in isolation. Use a layered model:

1. **Canonical zaaktype model in JSON**

   Imported and normalized from Excel, BPMN, DMN, text, or manual authoring. This is the durable source of truth for runtime.

2. **JSON Schema for validation and forms**

   Every zaaktype definition, issue property set, document requirement, checklist item, transition, and effect type has a schema.

3. **Small DSL for guards and effects**

   Use a constrained expression language for common business logic:

   - `document.exists`
   - `property.equals`
   - `role.has`
   - `checklist.completed`
   - `deadline.expired`
   - `status.is`
   - `result.is`

   Effects should also be constrained:

   - `status.transition`
   - `task.create`
   - `planning.create_or_update`
   - `document.request`
   - `role.assign`
   - `deadline.start`
   - `deadline.escalate`
   - `subcase.create`
   - `comment.system`

4. **DMN-style decision tables for decisions**

   Decision logic should preferably be tables when possible. Tables are readable, testable, and close to legal reasoning.

5. **Sandboxed code plugins for hard cases**

   Use JS or WASM only where the DSL/table model is insufficient. Code plugins should be pure by default: input is process context, output is proposed effects. Side effects such as email, external API calls, and assignments go through explicit capabilities.

6. **Event sourcing as the execution substrate**

   The engine evaluates process definitions against the event stream and emits JSONCommit events. The current issue state is a materialized view, not the authority.

## Import Pipeline

The importer should be a repeatable build step:

```text
source files
  -> extract tables
  -> normalize names and dates
  -> map rows to canonical JSON
  -> validate with JSON Schema
  -> produce import report
  -> commit ZaakTypeDefinition resources as events
```

For ZTC XLSX files:

1. Read the catalog workbook to discover zaaktypes, information object types, and decision types.
2. Read each zaaktype workbook.
3. Convert Excel serial dates to ISO dates.
4. Convert ordered statuses to stable IDs.
5. Convert checklist rows to task templates grouped by status.
6. Convert roles and `Mag zetten Statustype` into role/status permissions.
7. Convert information objects into document type references.
8. Convert result rows into result definitions with archive metadata.
9. Convert related zaaktypes into relation definitions.
10. Preserve source coordinates for auditability, for example sheet name and row number.

The importer should preserve uncertainty. If a row cannot be mapped cleanly, the result should contain warnings rather than silently losing information.

Example import warning:

```json
{
  "severity": "warning",
  "source": {
    "file": "zaaktypen/10_-_Aanvraag_beschikking_regulier_behandelen_V1_9.xlsx",
    "sheet": "Gerelateerde zaken",
    "row": 12
  },
  "message": "Relation target references a zaaktype name that is not present in the catalog."
}
```

## Execution Model

The process engine runs whenever relevant events arrive.

```text
JSONCommit event arrives
  -> update event store
  -> rebuild or update ProcessInstance view
  -> evaluate zaaktype guards and automation
  -> produce proposed effects
  -> apply allowed automatic effects
  -> queue human approvals where needed
  -> emit resulting JSONCommit events
```

Not every effect should be automatic. The definition should distinguish:

- `automatic`: engine may apply directly.
- `suggested`: show to a human as a proposed action.
- `manual`: only a human can perform it.
- `external`: requires a connector or external system permission.

Example:

```json
{
  "id": "request_missing_documents",
  "trigger": { "status.is": "intake_afgerond" },
  "when": { "not": { "document.exists": "Aanvullende informatie" } },
  "mode": "suggested",
  "effects": [
    {
      "document.request": {
        "type": "Aanvullende informatie",
        "from": "aanvrager",
        "deadline": { "value": 14, "unit": "calendar_days" }
      }
    }
  ]
}
```

## Relationship To Current ZaakChat Resources

The current resources can evolve without throwing away the existing architecture:

- `Issue` gains `zaaktype`, `zaaktype_version`, `current_status_id`, `properties`, and possibly `result`.
- `Planning` can be generated from zaaktype statuses and durations.
- `Task` can be generated from checklist items, transition requirements, missing documents, and deadlines.
- `Document` can be typed with imported information object types.
- `Comment` remains the conversation and audit explanation layer.
- `JSONCommit` remains the mutation/event envelope.

New resource types likely needed:

- `ZaakTypeDefinition`
- `ProcessInstance`
- `ChecklistItem`
- `Deadline`
- `Decision`
- `ProcessEffect`
- `ImportReport`

## Citizen Experience

For citizens, the model should produce a simple linear explanation even when the internal process is complex:

- Current step.
- Previous completed steps.
- Expected next steps.
- Expected duration.
- What the municipality is waiting for.
- What the citizen must do.
- Why a delay happened.
- Which decision or result closed the case.

The citizen-facing view should not expose raw BPMN or internal role matrices. It should use the process model to generate a clear timeline.

## Municipality Experience

For municipalities, the same model powers operational controls:

- Which department or role should handle this status.
- Which documents are missing.
- Which legal deadlines are active.
- Which decisions are possible.
- Which result values are allowed.
- Which related or child cases can be opened.
- Which actions are blocked because requirements are not met.
- Which actions were automated and why.

## Governance And Safety

True issue logic has legal consequences, so definitions need governance:

- Definitions are versioned and immutable after publication.
- Draft definitions can be edited and tested.
- Published definitions require approval.
- Imports produce reports and diffs.
- Runtime actions include source definition references.
- Automation modules run with explicit capabilities.
- Dangerous effects require human approval.
- Tests can replay historical issues against new definitions before migration.

## Open Questions

- Should the canonical model align with existing VNG ZGW APIs or intentionally remain ZaakChat-native?
- Should BPMN be supported as first-class import/export in v1, or later?
- Which expression language should be used for guards: custom JSON DSL, JSONLogic, CEL, or something else?
- Which code runtime is acceptable for plugins: JavaScript, WASM, or both?
- How much of the ZTC result/archive metadata should be exposed in the user interface?
- How should municipality-specific deviations from a national zaaktype catalog be represented?
- Should migrations be automatic, opt-in per issue, or forbidden for running issues?

## Proposed First Milestone

Build the smallest useful version around the imported ZTC files:

1. Implement an importer for `zaaktypen/*.xlsx`.
2. Generate `ZaakTypeDefinition` JSON for each workbook.
3. Validate generated definitions with JSON Schema.
4. Store definitions as JSONCommit resources.
5. Let issue creation select a zaaktype.
6. Generate initial planning from `Statussen`.
7. Generate checklist tasks from `Checklists`.
8. Type documents from `Informatieobjecten`.
9. Enforce role/status permissions as warnings first, hard validation later.
10. Show an import report with unmapped rows and lossy conversions.

This keeps the first implementation close to the source files and the current app architecture. It does not require a BPMN engine or code sandbox yet, but it leaves room for both.
