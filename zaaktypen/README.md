# Zaaktypen Logic

This directory contains source zaaktype files and the first ZaakChat-native setup for turning those files into executable issue logic.

The XLSX/DOCX files are treated as source material. Runtime code should use normalized JSON definitions generated from them, not parse spreadsheets directly while handling an issue.

## Layout

- `00_-_Zaaktypecatalogus_Omgevingswet_V1_9.xlsx`: source catalog.
- `NN_-_..._V1_9.xlsx`: source workbook per zaaktype.
- `Releasenotes_ZTC_-_Ow_V1_9.docx`: source release notes.
- `logic/zaaktype-definition.schema.json`: schema for normalized zaaktype definitions.
- `import_ztc.py`: importer that reads the XLSX files and writes generated JSON definitions.
- `generated/`: generated normalized definitions. This folder is created by the importer.

## Generate Definitions

```sh
python3 zaaktypen/import_ztc.py
```

The importer uses only Python's standard library. It reads XLSX files as zipped XML, extracts the common ZTC sheets, normalizes them, and writes one JSON file per zaaktype plus an index file.

## Model

A generated `ZaakTypeDefinition` contains:

- metadata from the `Zaaktype` sheet
- ordered statuses from `Statussen`
- inferred linear transitions between imported statuses
- checklist task templates from `Checklists`
- roles and status permissions from `Rollen`
- document types from `Informatieobjecten`
- decision types from `Besluiten`
- result types and archive metadata from `Resultaten`
- related zaaktype links from `Gerelateerde zaken`
- source references back to file, sheet, and row

The generated model is deliberately conservative. It imports the static process skeleton and preserves source references. It does not invent branching conditions or hard automation rules that are not present in the source files.
