#!/usr/bin/env python3
"""
Generate seed SQL migration for Indonesian administrative regions.
This script extracts data from wilayah_boundaries SQL files and generates
a clean SQL migration file with proper FK relationships.
"""

import os
import re
import sys
from pathlib import Path

# Base directory for source data
BASE_DIR = Path(__file__).parent.parent / "docs/temp/wilayah_boundaries-main/db"

# Output file
OUTPUT_FILE = Path(__file__).parent.parent / "migrations/20260120171436_seed_regions_data.sql"


def escape_sql_string(s: str) -> str:
    """Escape single quotes in SQL strings."""
    return s.replace("'", "''")


def extract_data(file_pattern: str, code_pattern: str) -> list[tuple]:
    """Extract region data from SQL files matching the pattern."""
    data = []
    regex = re.compile(rf"\('{code_pattern}','([^']+)',([-0-9.null]+),([-0-9.null]+)")

    for sql_file in sorted(BASE_DIR.glob(file_pattern)):
        with open(sql_file, "r", encoding="utf-8") as f:
            content = f.read()
            for match in regex.finditer(content):
                code = match.group(0).split("'")[1]
                name = match.group(1)
                lat = match.group(2)
                lng = match.group(3)

                # Convert 'null' string to SQL NULL
                lat_val = "NULL" if lat == "null" else lat
                lng_val = "NULL" if lng == "null" else lng

                data.append((code, name, lat_val, lng_val))

    return data


def generate_provinces_sql(provinces: list[tuple]) -> str:
    """Generate INSERT statements for provinces."""
    lines = ["-- Seed provinces data"]
    lines.append("INSERT INTO provinces (code, name, lat, lng) VALUES")

    values = []
    for code, name, lat, lng in provinces:
        name_escaped = escape_sql_string(name)
        values.append(f"    ('{code}', '{name_escaped}', {lat}, {lng})")

    lines.append(",\n".join(values) + ";")
    return "\n".join(lines)


def generate_regencies_sql(regencies: list[tuple]) -> str:
    """Generate INSERT statements for regencies with FK lookup."""
    lines = ["\n-- Seed regencies data"]
    lines.append("INSERT INTO regencies (code, name, lat, lng, province_id)")
    lines.append("SELECT v.code, v.name, v.lat, v.lng, p.id")
    lines.append("FROM (VALUES")

    values = []
    for code, name, lat, lng in regencies:
        name_escaped = escape_sql_string(name)
        province_code = code[:2]  # First 2 chars is province code
        lat_cast = "NULL::DOUBLE PRECISION" if lat == "NULL" else f"{lat}::DOUBLE PRECISION"
        lng_cast = "NULL::DOUBLE PRECISION" if lng == "NULL" else f"{lng}::DOUBLE PRECISION"
        values.append(f"    ('{code}', '{name_escaped}', {lat_cast}, {lng_cast}, '{province_code}')")

    lines.append(",\n".join(values))
    lines.append(") AS v(code, name, lat, lng, province_code)")
    lines.append("JOIN provinces p ON p.code = v.province_code;")
    return "\n".join(lines)


def generate_districts_sql(districts: list[tuple]) -> str:
    """Generate INSERT statements for districts with FK lookup."""
    lines = ["\n-- Seed districts data"]
    lines.append("INSERT INTO districts (code, name, lat, lng, regency_id)")
    lines.append("SELECT v.code, v.name, v.lat, v.lng, r.id")
    lines.append("FROM (VALUES")

    values = []
    for code, name, lat, lng in districts:
        name_escaped = escape_sql_string(name)
        regency_code = code[:5]  # First 5 chars (XX.XX) is regency code
        lat_cast = "NULL::DOUBLE PRECISION" if lat == "NULL" else f"{lat}::DOUBLE PRECISION"
        lng_cast = "NULL::DOUBLE PRECISION" if lng == "NULL" else f"{lng}::DOUBLE PRECISION"
        values.append(f"    ('{code}', '{name_escaped}', {lat_cast}, {lng_cast}, '{regency_code}')")

    lines.append(",\n".join(values))
    lines.append(") AS v(code, name, lat, lng, regency_code)")
    lines.append("JOIN regencies r ON r.code = v.regency_code;")
    return "\n".join(lines)


def generate_villages_sql(villages: list[tuple], batch_size: int = 1000) -> str:
    """Generate INSERT statements for villages with FK lookup in batches."""
    lines = ["\n-- Seed villages data (in batches for performance)"]

    for i in range(0, len(villages), batch_size):
        batch = villages[i:i + batch_size]
        batch_num = i // batch_size + 1

        lines.append(f"\n-- Village batch {batch_num}")
        lines.append("INSERT INTO villages (code, name, lat, lng, district_id)")
        lines.append("SELECT v.code, v.name, v.lat, v.lng, d.id")
        lines.append("FROM (VALUES")

        values = []
        for code, name, lat, lng in batch:
            name_escaped = escape_sql_string(name)
            district_code = code[:8]  # First 8 chars (XX.XX.XX) is district code
            lat_cast = "NULL::DOUBLE PRECISION" if lat == "NULL" else f"{lat}::DOUBLE PRECISION"
            lng_cast = "NULL::DOUBLE PRECISION" if lng == "NULL" else f"{lng}::DOUBLE PRECISION"
            values.append(f"    ('{code}', '{name_escaped}', {lat_cast}, {lng_cast}, '{district_code}')")

        lines.append(",\n".join(values))
        lines.append(") AS v(code, name, lat, lng, district_code)")
        lines.append("JOIN districts d ON d.code = v.district_code;")

    return "\n".join(lines)


def main():
    print("Extracting province data...")
    provinces = extract_data("prov/*.sql", r"[0-9]{2}")
    print(f"  Found {len(provinces)} provinces")

    print("Extracting regency data...")
    regencies = extract_data("kab/*.sql", r"[0-9]{2}\.[0-9]{2}")
    print(f"  Found {len(regencies)} regencies")

    print("Extracting district data...")
    districts = extract_data("kec/*.sql", r"[0-9]{2}\.[0-9]{2}\.[0-9]{2}")
    print(f"  Found {len(districts)} districts")

    print("Extracting village data...")
    villages = extract_data("kel/*/*.sql", r"[0-9]{2}\.[0-9]{2}\.[0-9]{2}\.[0-9]{4}")
    print(f"  Found {len(villages)} villages")

    print(f"\nGenerating SQL migration file: {OUTPUT_FILE}")

    with open(OUTPUT_FILE, "w", encoding="utf-8") as f:
        f.write("-- Seed data for Indonesian administrative regions\n")
        f.write("-- Source: https://github.com/cahyadsn/wilayah_boundaries\n")
        f.write("-- Generated by scripts/generate_region_seed.py\n\n")

        f.write(generate_provinces_sql(provinces))
        f.write("\n\n")
        f.write(generate_regencies_sql(regencies))
        f.write("\n\n")
        f.write(generate_districts_sql(districts))
        f.write("\n\n")
        f.write(generate_villages_sql(villages))
        f.write("\n")

    print("Done!")
    print(f"\nStatistics:")
    print(f"  Provinces: {len(provinces)}")
    print(f"  Regencies: {len(regencies)}")
    print(f"  Districts: {len(districts)}")
    print(f"  Villages: {len(villages)}")


if __name__ == "__main__":
    main()
