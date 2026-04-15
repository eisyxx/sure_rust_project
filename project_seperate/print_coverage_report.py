import subprocess
import argparse
import glob
import shutil
import subprocess
import json
import csv
import os

"""
<인자: --test / --ignore / --src>

--test
    : 테스트 필터링 (폴더 단위 테스트 가능)
    : --test unit_test 같이 사용
    : 여러 인자 지정 -> --test integration_test case_test 와 같이 띄어쓰기 이용해 연결
--ignore
    : 테스트 결과에서 제외할 스크립트 지정
    : --ignore "src[\\/](unit_test)[\\/]|src[\\/](integration_test|handler|main)\.rs|src[\\/](repository)[\\/]" 와 같이 사용 (기존 cargo 옵션과 형식 동일)
--src
    : 테스트 결과를 보고 싶은 스크립트 지정
    : --src src/service 와 같이 사용 (src 기준 상대경로)
    : 여러 인자 지정 -> --src src/service src/repository 와 같이 띄어쓰기 이용해 연결

<exe 선택>
    : 숫자 입력
    : 여러개 선택 가능 (, 이용)
    : test 필더링으로 선택했던 스크립트를 선택
    : unit_test 선택 시, 가장 위에 있는 exe 파일만 선택

"""

TOOLCHAIN = "nightly-2025-12-01"

# -----------------------
# util
# -----------------------
def run(cmd):
    cmd = [c for c in cmd if c is not None]
    print("\nRunning:", " ".join(cmd))
    subprocess.run(cmd, check=True)

    return shutil.which("llvm-cov")

def ensure_toolchain():
    try:
        subprocess.run(
            ["rustup", "toolchain", "install", TOOLCHAIN],
            check=True
        )
    except:
        pass

    subprocess.run(
        ["rustup", "component", "add", "llvm-tools-preview", "--toolchain", TOOLCHAIN],
        check=True
    )

def get_llvm_cov():

    sysroot = subprocess.check_output(
        ["rustup", "run", TOOLCHAIN, "rustc", "--print", "sysroot"],
        text=True
    ).strip()

    return os.path.join(
        sysroot,
        "lib",
        "rustlib",
        "x86_64-pc-windows-msvc",
        "bin",
        "llvm-cov.exe"
    )


def find_exes():
    return glob.glob("target/llvm-cov-target/debug/deps/*.exe")


def choose_exes(exe_files):
    print("\n===== EXECUTABLE LIST =====")

    for i, f in enumerate(exe_files):
        print(f"{i + 1}) {os.path.basename(f)}")

    while True:
        raw = input("\n번호 선택 (예: 1 3 5 또는 1,3,5): ")

        try:
            # 공백/콤마 둘 다 지원
            parts = raw.replace(",", " ").split()
            indices = [int(p) - 1 for p in parts]

            if all(0 <= idx < len(exe_files) for idx in indices):
                return [exe_files[idx] for idx in indices]
        except:
            pass

        print("❌ 잘못된 입력")

# -----------------------
# 1. clean
# -----------------------
def clean():
    run(["cargo", f"+{TOOLCHAIN}", "llvm-cov", "clean"])


# -----------------------
# 2. run tests (coverage 생성)
# -----------------------
def run_tests(test_name=None, ignore_regex=None):
    cmd = [
        "cargo", f"+{TOOLCHAIN}", "llvm-cov",
        "--branch",
        "--html",
        "--open"
    ]

    if ignore_regex:
        cmd += ["--ignore-filename-regex", ignore_regex]

    if test_name:
        cmd += ["--"] + test_name

    run(cmd)


# -----------------------
# 3. report
# -----------------------
def run_report(exe, src_list):
    profdata = glob.glob("target/llvm-cov-target/*.profdata")[0]

    llvm_cov = get_llvm_cov()
    cmd = [
        llvm_cov,
        "report",
        exe,
        f"-instr-profile={profdata}",
        "-show-functions",
    ]

    cmd += src_list 

    run(cmd)

def run_report_to_file(exe, src_list):
    profdata = glob.glob("target/llvm-cov-target/*.profdata")[0]
    llvm_cov = get_llvm_cov()

    cmd = [
        llvm_cov,
        "report",
        exe,
        f"-instr-profile={profdata}",
        "-show-functions",
    ]

    cmd += src_list

    with open("coverage_report.log", "w", encoding="utf-8") as f:
        subprocess.run(cmd, stdout=f, check=True)

    print("report 파일 저장 완료: coverage_report.log")

# -----------------------
# pipeline
# -----------------------
def run_pipeline(test=None, ignore=None, src="src"):
    
    ensure_toolchain()
    clean()

    # 1) coverage 생성 + html
    run_tests(test, ignore)

    # 2) exe 선택
    exe_files = find_exes()
    if not exe_files:
        print("❌ exe 없음")
        return

    selected_exes = choose_exes(exe_files)

    for exe in selected_exes:
        run_report_to_file(exe, src)


# -----------------------
# CLI
# -----------------------
if __name__ == "__main__":
    parser = argparse.ArgumentParser()

    parser.add_argument("--test", nargs="+")
    parser.add_argument("--ignore")
    parser.add_argument(
    "--src",
    nargs="+",
    default=["src/service", "src/repository"]
)

    args = parser.parse_args()

    run_pipeline(args.test, args.ignore, args.src)