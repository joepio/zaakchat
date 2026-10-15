#!/usr/bin/env python3
"""Import ZTC XLSX workbooks into ZaakChat zaaktype logic JSON."""

from __future__ import annotations

import argparse
import json
import re
import unicodedata
import zipfile
from datetime import datetime, timedelta, timezone
from pathlib import Path
from typing import Any
from xml.etree import ElementTree as ET


BASE_DIR = Path(__file__).resolve().parent
CATALOG_FILE = "00_-_Zaaktypecatalogus_Omgevingswet_V1_9.xlsx"
DEFAULT_VERSION = "1.9"

NS = {
    "a": "http://schemas.openxmlformats.org/spreadsheetml/2006/main",
    "r": "http://schemas.openxmlformats.org/officeDocument/2006/relationships",
    "rel": "http://schemas.openxmlformats.org/package/2006/relationships",
}


def text_content(element: ET.Element) -> str:
    return "".join(node.text or "" for node in element.findall(".//a:t", NS))


def column_number(column: str) -> int:
    value = 0
    for char in column:
        value = value * 26 + ord(char) - 64
    return value


def cell_position(reference: str | None) -> tuple[int, int]:
    match = re.match(r"([A-Z]+)([0-9]+)", reference or "")
    if not match:
        return 0, 0
    return int(match.group(2)), column_number(match.group(1))


def clean_text(value: str) -> str:
    return re.sub(r"[ \t\r\f\v]+", " ", value.replace("\xa0", " ")).strip()


def clean_header(value: str) -> str:
    return clean_text(value).replace("\n", " ")


def slugify(value: str) -> str:
    normalized = unicodedata.normalize("NFKD", value)
    ascii_text = normalized.encode("ascii", "ignore").decode("ascii")
    return re.sub(r"[^a-zA-Z0-9]+", "_", ascii_text.lower()).strip("_") or "unnamed"


def excel_date(value: str | None) -> str | None:
    if not value or not re.fullmatch(r"\d+(\.0)?", value):
        return None
    serial = int(float(value))
    if serial < 30000 or serial > 60000:
        return None
    return (datetime(1899, 12, 30) + timedelta(days=serial)).date().isoformat()


def int_value(value: str | None) -> int | None:
    if value is None or not re.fullmatch(r"-?\d+(\.0)?", value):
        return None
    return int(float(value))


def bool_value(value: str | None) -> bool | None:
    if value is None:
        return None
    normalized = value.strip().lower()
    if normalized in {"ja", "yes", "true", "1"}:
        return True
    if normalized in {"nee", "no", "false", "0"}:
        return False
    return None


def split_lines(value: str | None) -> list[str]:
    if not value:
        return []
    return [clean_text(line) for line in value.splitlines() if clean_text(line)]


def source_ref(file_name: str, sheet: str, row: dict[int | str, Any]) -> dict[str, Any]:
    return {"file": file_name, "sheet": sheet, "row": row["__row"]}


def read_shared_strings(zip_file: zipfile.ZipFile) -> list[str]:
    try:
        root = ET.fromstring(zip_file.read("xl/sharedStrings.xml"))
    except KeyError:
        return []
    return [text_content(item) for item in root.findall("a:si", NS)]


def cell_value(cell: ET.Element, shared_strings: list[str]) -> str:
    value_type = cell.get("t")
    value_node = cell.find("a:v", NS)
    if value_type == "s" and value_node is not None and value_node.text:
        return shared_strings[int(value_node.text)]
    if value_type == "inlineStr":
        return text_content(cell)
    return (value_node.text if value_node is not None else "") or ""


def read_workbook(path: Path) -> dict[str, list[dict[int | str, Any]]]:
    sheets: dict[str, list[dict[int | str, Any]]] = {}
    with zipfile.ZipFile(path) as zip_file:
        shared_strings = read_shared_strings(zip_file)
        workbook_root = ET.fromstring(zip_file.read("xl/workbook.xml"))
        rels_root = ET.fromstring(zip_file.read("xl/_rels/workbook.xml.rels"))
        rels = {
            rel.get("Id"): rel.get("Target")
            for rel in rels_root.findall("rel:Relationship", NS)
        }

        for sheet in workbook_root.findall(".//a:sheet", NS):
            name = sheet.get("name") or "unnamed"
            rel_id = sheet.get(f"{{{NS['r']}}}id")
            target = rels[rel_id]
            target_path = "xl/" + target.lstrip("/") if not target.startswith("xl/") else target
            sheet_root = ET.fromstring(zip_file.read(target_path))
            rows: list[dict[int | str, Any]] = []

            for row_node in sheet_root.findall(".//a:sheetData/a:row", NS):
                row: dict[int | str, Any] = {"__row": int(row_node.get("r", "0"))}
                for cell in row_node.findall("a:c", NS):
                    _, column = cell_position(cell.get("r"))
                    value = clean_text(cell_value(cell, shared_strings))
                    if value:
                        row[column] = value
                if len(row) > 1:
                    rows.append(row)

            sheets[name] = rows

    return sheets


def find_table(rows: list[dict[int | str, Any]], required_headers: set[str]) -> list[dict[str, Any]]:
    header_row: dict[int | str, Any] | None = None
    for row in rows[:8]:
        values = {
            clean_header(str(value)).lower()
            for key, value in row.items()
            if key != "__row"
        }
        if required_headers & values:
            header_row = row
            break

    if not header_row:
        return []

    headers = {
        column: clean_header(str(value))
        for column, value in header_row.items()
        if column != "__row"
    }
    table: list[dict[str, Any]] = []
    for row in rows:
        if row["__row"] <= header_row["__row"]:
            continue
        item: dict[str, Any] = {"__row": row["__row"]}
        for column, value in row.items():
            if column == "__row" or column not in headers:
                continue
            item[headers[column]] = value
        if len(item) > 1:
            table.append(item)
    return table


def find_value(row: dict[str, Any], *names: str) -> str | None:
    normalized = {clean_header(key).lower(): value for key, value in row.items()}
    for name in names:
        value = normalized.get(clean_header(name).lower())
        if isinstance(value, str) and value:
            return value
    return None


def metadata_from_zaaktype(file_name: str, rows: list[dict[int | str, Any]]) -> dict[str, Any]:
    metadata: dict[str, Any] = {}
    for row in rows:
        key = row.get(1)
        value = row.get(2)
        if not isinstance(key, str) or not isinstance(value, str):
            continue
        if key == "Kenmerk" and value == "Waarde":
            continue
        metadata[key] = {
            "value": excel_date(value) if "DATUM" in key.upper() else value,
            "source_ref": source_ref(file_name, "Zaaktype", row),
        }
    return metadata


def parse_status_reference(
    raw: str, status_by_sequence: dict[int, str], status_by_title: dict[str, str]
) -> str:
    raw = clean_text(raw)
    match = re.match(r"^(\d+)\s*-?\s*(.*)$", raw)
    if match:
        sequence = int(match.group(1))
        if sequence in status_by_sequence:
            return status_by_sequence[sequence]
    title = re.sub(r"^\d+\s*-\s*", "", raw)
    return status_by_title.get(title.lower(), slugify(title))


def parse_target(raw: str | None) -> dict[str, str]:
    if not raw:
        return {"raw": ""}
    match = re.match(r"^(\d+)\s*-\s*(.+)$", raw)
    if match:
        return {
            "id": f"omgevingswet/{int(match.group(1)):02d}",
            "title": clean_text(match.group(2)),
            "raw": raw,
        }
    return {"raw": raw}


def import_zaaktype(path: Path, imported_at: str) -> dict[str, Any]:
    file_name = path.name
    sheets = read_workbook(path)
    metadata = metadata_from_zaaktype(file_name, sheets.get("Zaaktype", []))
    identification = str(metadata.get("IDENTIFICATIE", {}).get("value", path.name[:2])).zfill(2)
    title = str(metadata.get("OMSCHRIJVING", {}).get("value", path.stem))

    status_rows = find_table(sheets.get("Statussen", []), {"volgnr.", "omschrijving"})
    statuses: list[dict[str, Any]] = []
    status_by_sequence: dict[int, str] = {}
    status_by_title: dict[str, str] = {}

    for row in status_rows:
        sequence = int_value(find_value(row, "Volgnr."))
        status_title = find_value(row, "Omschrijving")
        if sequence is None or not status_title:
            continue
        status_id = f"{sequence}_{slugify(status_title)}"
        status_by_sequence[sequence] = status_id
        status_by_title[status_title.lower()] = status_id

        duration_value = int_value(find_value(row, "Doorlooptijd - duur"))
        duration_unit = find_value(row, "Doorlooptijd - eenheid (2.2)")
        status: dict[str, Any] = {
            "id": status_id,
            "sequence": sequence,
            "title": status_title,
            "inform_citizen": bool_value(find_value(row, "Informeren")) or False,
            "required_documents": split_lines(find_value(row, "Verplichte informatieobjecten")),
            "required_objects": split_lines(find_value(row, "Verplichte zaakobjecten")),
            "required_properties": split_lines(find_value(row, "Verplichte eigenschappen")),
            "source_ref": source_ref(file_name, "Statussen", row),
        }
        if duration_value is not None:
            status["duration"] = {"value": duration_value, "unit": duration_unit or "unknown"}
        for target_key, source_key in [
            ("status_text", "Statustekst"),
            ("description", "Toelichting"),
        ]:
            value = find_value(row, source_key)
            if value:
                status[target_key] = value
        valid_from = excel_date(find_value(row, "Datum begin geldigheid"))
        if valid_from:
            status["valid_from"] = valid_from
        statuses.append(status)

    statuses.sort(key=lambda status: status["sequence"])
    transitions = [
        {
            "id": f"{previous['id']}__to__{current['id']}",
            "from": previous["id"],
            "to": current["id"],
            "source": "inferred_from_status_order",
            "mode": "manual",
            "when": {"status.is": previous["id"]},
            "effects": [{"status.transition": current["id"]}],
        }
        for previous, current in zip(statuses, statuses[1:])
    ]

    checklist_rows = find_table(sheets.get("Checklists", []), {"statustype", "vraagstelling"})
    task_templates = []
    for row in checklist_rows:
        item_name = find_value(row, "Itemnaam")
        question = find_value(row, "Vraagstelling")
        raw_status = find_value(row, "Statustype") or ""
        if not item_name or not question:
            continue
        status_id = parse_status_reference(raw_status, status_by_sequence, status_by_title)
        task_templates.append(
            {
                "id": f"{status_id}__{slugify(item_name)}",
                "title": item_name,
                "question": question,
                "status": status_id,
                "required": bool_value(find_value(row, "Verplicht")) or False,
                "description": find_value(row, "Toelichting") or "",
                "source_ref": source_ref(file_name, "Checklists", row),
            }
        )

    role_rows = find_table(sheets.get("Rollen", []), {"omschrijving", "mag zetten statustype"})
    roles = []
    for row in role_rows:
        role_title = find_value(row, "Omschrijving")
        if not role_title:
            continue
        role: dict[str, Any] = {
            "id": slugify(role_title),
            "title": role_title,
            "may_set_statuses": [
                parse_status_reference(line, status_by_sequence, status_by_title)
                for line in split_lines(find_value(row, "Mag zetten Statustype"))
            ],
            "source_ref": source_ref(file_name, "Rollen", row),
        }
        for target_key, source_key in [
            ("generic_title", "Omschrijving generiek"),
            ("actor_type", "Soort betrokkene"),
        ]:
            value = find_value(row, source_key)
            if value:
                role[target_key] = value
        valid_from = excel_date(find_value(row, "Datum begin geldigheid"))
        if valid_from:
            role["valid_from"] = valid_from
        roles.append(role)

    documents = []
    for row in find_table(sheets.get("Informatieobjecten", []), {"volgnummer", "informatieobjecttype"}):
        title_value = find_value(row, "Informatieobjecttype")
        if not title_value:
            continue
        documents.append(
            {
                "id": slugify(title_value),
                "sequence": int_value(find_value(row, "Volgnummer")) or 0,
                "title": title_value,
                "direction": find_value(row, "Richting") or "Onbekend",
                "source_ref": source_ref(file_name, "Informatieobjecten", row),
            }
        )

    decisions = []
    for row in find_table(sheets.get("Besluiten", []), {"besluittype"}):
        title_value = find_value(row, "Besluittype", "BESLUITTYPE")
        if not title_value:
            continue
        decision: dict[str, Any] = {
            "id": slugify(title_value),
            "title": title_value,
            "source_ref": source_ref(file_name, "Besluiten", row),
        }
        valid_from = excel_date(find_value(row, "Datum begin geldigheid", "DATUM BEGIN GELDIGHEID"))
        if valid_from:
            decision["valid_from"] = valid_from
        decisions.append(decision)

    results = []
    for row in find_table(sheets.get("Resultaten", []), {"omschrijving (2.1)", "omschrijving"}):
        title_value = find_value(row, "OMSCHRIJVING (2.1)", "OMSCHRIJVING")
        if not title_value:
            continue
        result: dict[str, Any] = {
            "id": slugify(title_value),
            "title": title_value,
            "source_ref": source_ref(file_name, "Resultaten", row),
        }
        mappings = {
            "authority": ("BEVOEGD GEZAG",),
            "generic_title": ("OMSCHRIJVING GENERIEK",),
            "title_2_2": ("OMSCHRIJVING (2.2)",),
            "selectielijst_class": ("SELECTIELIJST-KLASSE", "SELECTIELIJSTKLASSE"),
            "archive_nomination": ("ARCHIEFNOMINATIE",),
            "description": ("TOELICHTING",),
            "leads_to_decision_type": ("LEIDT TOT BESLUITTYPE",),
        }
        for target_key, source_names in mappings.items():
            value = find_value(row, *source_names)
            if value:
                result[target_key] = value
        archive_months = int_value(find_value(row, "ARCHIEF-ACTIETERMIJN (mnd)", "ARCHIEFACTIETERMIJN"))
        if archive_months is not None:
            result["archive_action_term_months"] = archive_months
        results.append(result)

    relations = []
    relation_rows = find_table(sheets.get("Gerelateerde zaken", []), {"gerelateerd zaaktype"})
    for row in relation_rows:
        raw_target = find_value(row, "GERELATEERD ZAAKTYPE", "Gerelateerd zaaktype")
        relation_type = find_value(row, "AARD RELATIE", "Aard relatie")
        if not raw_target or not relation_type:
            continue
        relation: dict[str, Any] = {
            "target": parse_target(raw_target),
            "relation_type": relation_type,
            "source_ref": source_ref(file_name, "Gerelateerde zaken", row),
        }
        description = find_value(row, "TOELICHTING", "Toelichting")
        if description:
            relation["description"] = description
        relations.append(relation)

    warnings = []
    if not statuses:
        warnings.append({"severity": "warning", "message": "No statuses were imported."})
    if not task_templates:
        warnings.append({"severity": "info", "message": "No checklist task templates were imported."})

    return {
        "kind": "ZaakTypeDefinition",
        "id": f"omgevingswet/{identification}",
        "version": DEFAULT_VERSION,
        "title": title,
        "source": {
            "kind": "ztc-xlsx",
            "file": file_name,
            "catalog_file": CATALOG_FILE,
            "imported_at": imported_at,
        },
        "metadata": metadata,
        "statuses": statuses,
        "transitions": transitions,
        "task_templates": task_templates,
        "roles": roles,
        "documents": documents,
        "decisions": decisions,
        "results": results,
        "relations": relations,
        "automation": [],
        "import_warnings": warnings,
    }


def write_json(path: Path, data: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=BASE_DIR)
    parser.add_argument("--out", type=Path, default=BASE_DIR / "generated")
    args = parser.parse_args()

    imported_at = datetime.now(timezone.utc).replace(microsecond=0).isoformat()
    definitions = []
    for path in sorted(args.source.glob("[0-9][0-9]_-_*.xlsx")):
        if path.name == CATALOG_FILE:
            continue
        definition = import_zaaktype(path, imported_at)
        output_name = f"{definition['id'].replace('/', '-')}-{definition['version']}.json"
        write_json(args.out / output_name, definition)
        definitions.append(
            {
                "id": definition["id"],
                "version": definition["version"],
                "title": definition["title"],
                "file": output_name,
                "source_file": definition["source"]["file"],
            }
        )

    write_json(
        args.out / "index.json",
        {
            "kind": "ZaakTypeDefinitionIndex",
            "generated_at": imported_at,
            "schema": "../logic/zaaktype-definition.schema.json",
            "definitions": definitions,
        },
    )
    print(f"Imported {len(definitions)} zaaktype definitions into {args.out}")


if __name__ == "__main__":
    main()
