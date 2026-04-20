import json
import csv
import os
import subprocess
from collections import defaultdict

# 저장된 json 파일(coverage.json, cargo 명령어 입력 시 파일명 지정)을 파싱해 함수별 커버리지 출력 -> csv 파일 (coverage_report.csv)로 저장
# parse_json.py 실행 위치 : project 폴더

# cargo install rustfilt 설치해야 demangle() 제대로 수행됨

# cargo +nightly llvm-cov --branch --html
# cargo +nightly llvm-cov --branch --json --output-path coverage.json
# parse_json.py 실행 후 결과 비교

demangle_cache = {}


def demangle(name):

    if name in demangle_cache:
        return demangle_cache[name]

    try:
        result = subprocess.run(
            ["rustfilt", name],
            capture_output=True,
            text=True
        )

        demangled = result.stdout.strip()

        if demangled == "":
            demangled = name

    except:
        demangled = name

    demangle_cache[name] = demangled
    return demangled


def compute_region_coverage(regions):

    total = 0
    covered = 0

    for r in regions:
        count = r[4]

        total += 1

        if count > 0:
            covered += 1

    return covered, total

def compute_line_coverage(regions):

    line_map = {}

    for r in regions:
        start_line = r[0]
        end_line = r[2]
        count = r[4]

        for line in range(start_line, end_line + 1):
            if line not in line_map:
                line_map[line] = 0

            line_map[line] = max(line_map[line], count)

    total = len(line_map)
    covered = sum(1 for v in line_map.values() if v > 0)

    return covered, total


def compute_branch_coverage(branches):

    total = len(branches)
    covered = 0

    for b in branches:
        true_count = b[4]
        false_count = b[5]

        if true_count > 0 and false_count > 0:
            covered += 1

    return covered, total

def get_function_range(func):
    regions = func.get("regions", [])

    if not regions:
        return None, None

    start = min(r[0] for r in regions)
    end = max(r[2] for r in regions)

    return start, end

def filter_branches_for_function(func, file_branches):

    start, end = get_function_range(func)

    if start is None:
        return []

    filtered = []

    for b in file_branches:
        line = b[0]

        if start <= line <= end:
            filtered.append(b)

    return filtered


def load_coverage(json_path):

    with open(json_path, "r") as f:
        data = json.load(f)

    coverage_data = data["data"][0]

    functions = coverage_data["functions"]
    files = coverage_data["files"]

    file_branch_map = {}

    for f in files:
        file_branch_map[f["filename"]] = f.get("branches", [])

    results = []

    for func in functions:

        name = demangle(func["name"])

        filename = func["filenames"][0].split("\\")[-1]
        file_full_path = func["filenames"][0]
        file_branches = file_branch_map.get(file_full_path, [])

        regions = func.get("regions", [])
        branches = filter_branches_for_function(func, file_branches)

        line_cov, line_total = compute_line_coverage(regions)
        region_cov, region_total = compute_region_coverage(regions)
        branch_cov, branch_total = compute_branch_coverage(branches)

        func_cov = 1 if func["count"] > 0 else 0

        results.append({
            "file": filename,
            "function": name,
            "line_cov": line_cov,
            "line_total": line_total,
            "region_cov": region_cov,
            "region_total": region_total,
            "func_cov": func_cov,
            "branch_cov": branch_cov,
            "branch_total": branch_total,
        })

    return results


def print_table(results):

    print(
        f"{'File':20} {'Function':50} {'Line Coverage':20} {'Region Coverage':20} {'Function Coverage':20} {'Branch Coverage':20} {'MC/DC Coverage':20}"
    )

    print("-" * 170)

    total_lines = total_lines_cov = 0
    total_regions = total_regions_cov = 0
    total_funcs = total_funcs_cov = 0
    total_branches = total_branches_cov = 0

    for r in results:

        line_percent = (
            100 * r["line_cov"] / r["line_total"]
            if r["line_total"] > 0 else 0
        )

        region_percent = (
            100 * r["region_cov"] / r["region_total"]
            if r["region_total"] > 0 else 0
        )

        branch_percent = (
            100 * r["branch_cov"] / r["branch_total"]
            if r["branch_total"] > 0 else 0
        )

        print(
            f"{r['file']:20} "
            f"{r['function'][:45]:50} "
            f"{line_percent:10.2f}% ({r['line_cov']}/{r['line_total']})".ljust(20),
            f"{region_percent:10.2f}% ({r['region_cov']}/{r['region_total']})".ljust(20),
            f"{100*r['func_cov']:10.2f}% ({r['func_cov']}/1)".ljust(20),
            f"{branch_percent:10.2f}% ({r['branch_cov']}/{r['branch_total']})".ljust(20),
        )

        total_lines += r["line_total"]
        total_lines_cov += r["line_cov"]

        total_regions += r["region_total"]
        total_regions_cov += r["region_cov"]

        total_funcs += 1
        total_funcs_cov += r["func_cov"]

        total_branches += r["branch_total"]
        total_branches_cov += r["branch_cov"]

    print("-" * 170)

    print(
        f"{'Total':20}",
        f"{'':45}",
        f"{100*total_lines_cov/total_lines:10.2f}% ({total_lines_cov}/{total_lines})".ljust(20),
        f"{100*total_regions_cov/total_regions:10.2f}% ({total_regions_cov}/{total_regions})".ljust(20),
        f"{100*total_funcs_cov/total_funcs:10.2f}% ({total_funcs_cov}/{total_funcs})".ljust(20),
        f"{100*total_branches_cov/total_branches:10.2f}% ({total_branches_cov}/{total_branches})".ljust(20),
    )


def save_csv(results, csv_path="coverage_report.csv"):
    """results 리스트를 CSV 파일로 저장"""
    headers = [
        "File", "Function", 
        "Line Covered", "Line Total", "Line Coverage (%)",
        "Region Covered", "Region Total", "Region Coverage (%)",
        "Function Covered", "Function Total", "Function Coverage (%)",
        "Branch Covered", "Branch Total", "Branch Coverage (%)",
    ]

    with open(csv_path, "w", newline="", encoding="utf-8") as f:
        writer = csv.writer(f)
        writer.writerow(headers)

        for r in results:
            line_percent = (
                100 * r["line_cov"] / r["line_total"]
                if r["line_total"] > 0 else 0
            )
            region_percent = (
                100 * r["region_cov"] / r["region_total"]
                if r["region_total"] > 0 else 0
            )
            func_percent = 100 * r["func_cov"]
            branch_percent = (
                100 * r["branch_cov"] / r["branch_total"]
                if r["branch_total"] > 0 else 0
            )

            writer.writerow([
                r["file"],
                r["function"],
                r["line_cov"],
                r["line_total"],
                f"{line_percent:.2f}",
                r["region_cov"],
                r["region_total"],
                f"{region_percent:.2f}",
                r["func_cov"],
                1,
                f"{func_percent:.2f}",
                r["branch_cov"],
                r["branch_total"],
                f"{branch_percent:.2f}",
            
            ])

    print(f"CSV report saved to {csv_path}")

if __name__ == "__main__":

    BASE_DIR = os.path.dirname(os.path.abspath(__file__))
    json_path = os.path.join(BASE_DIR, "coverage.json")

    results = load_coverage(json_path)

    print_table(results)

    save_csv(results, "coverage_report.csv")