#!/usr/bin/env python3
"""Build a pinned reference candidate; this does not certify TRIP or reproducibility."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess

ROOT = Path(__file__).resolve().parents[1]
LOCK = ROOT / "scripts/reference-build.json"


def digest(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def output(command, cwd, env):
    return subprocess.check_output(command, cwd=cwd, env=env, text=True).strip()


def verify_candidate(build):
    """Reject stale profiles or changed executables/generated sources before use."""
    build = Path(build).resolve()
    lock = json.loads(LOCK.read_text())
    report = json.loads((build / "webloom-build.json").read_text())
    if report.get("status") != "built_candidate" or report.get("lock") != lock:
        raise ValueError("A successful build of the current locked reference profile is required")
    changes = {item["path"]: item["sha256"] for item in lock["local_changes"]}
    if report.get("local_change_sha256") != changes:
        raise ValueError("Build report does not contain the locked local change chain")
    for name, expected in changes.items():
        if digest(ROOT / name) != expected:
            raise ValueError(f"Local reference change checksum mismatch: {name}")
    required = {
        "binary_sha256": {"tex", "tangle", "tie", "dvitype", "pltotf", "tftopl"},
        "generated_sha256": {"tex-final.ch", "tex.p", "tex.pool", "tex0.c", "texini.c", "texd.h"},
    }
    for field, names in required.items():
        if set(report.get(field, {})) != names:
            raise ValueError(f"Incomplete reference artifact manifest: {field}")
        for name, expected in report[field].items():
            if digest(build / "texk/web2c" / name) != expected:
                raise ValueError(f"Built reference artifact changed: {name}")
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=ROOT / "target/reference-source")
    parser.add_argument("--build", type=Path, default=ROOT / "target/reference-build-v1")
    parser.add_argument("--jobs", type=int, default=2)
    args = parser.parse_args()
    if args.jobs < 1:
        parser.error("--jobs must be positive")
    source, build = args.source.resolve(), args.build.resolve()
    if source == build or source in build.parents or build in source.parents:
        parser.error("source and build must be separate, non-nested directories")
    lock = json.loads(LOCK.read_text())
    local_changes = []
    for change in lock["local_changes"]:
        path = ROOT / change["path"]
        if digest(path) != change["sha256"]:
            raise SystemExit(f"Local reference change checksum mismatch: {path}")
        local_changes.append(path)
    env = dict(os.environ, LC_ALL="C", TZ="UTC", SOURCE_DATE_EPOCH=lock["source_date_epoch"])
    if not source.exists():
        source.mkdir(parents=True)
        subprocess.run(["git", "init", str(source)], check=True, env=env)
        subprocess.run(["git", "remote", "add", "origin", lock["repository"]], cwd=source, check=True, env=env)
        subprocess.run(["git", "fetch", "--depth=1", "--filter=blob:none", "origin", lock["revision"]], cwd=source, check=True, env=env)
        subprocess.run(["git", "sparse-checkout", "set", *lock["source_directories"]], cwd=source, check=True, env=env)
        subprocess.run(["git", "checkout", "--detach", lock["revision"]], cwd=source, check=True, env=env)
    if output(["git", "rev-parse", "HEAD"], source, env) != lock["revision"]:
        raise SystemExit("Reference checkout does not match the locked revision; refusing to replace it.")
    if output(["git", "status", "--porcelain", "--untracked-files=normal"], source, env):
        raise SystemExit("Reference checkout has changes; refusing to build an unpinned candidate.")
    if digest(source / "texk/web2c/tex.web") != lock["tex_web_sha256"]:
        raise SystemExit("Reference tex.web does not match the normative source.")
    if digest(ROOT / "vendor/knuth/tex.web") != lock["tex_web_sha256"]:
        raise SystemExit("Repository tex.web does not match the reference lock.")
    # Git checkout timestamps can make shipped generated files appear stale,
    # spuriously requiring makeinfo/flex/bison. Normalize metadata, not bytes,
    # to the pinned source epoch so only intended build outputs are regenerated.
    tracked = subprocess.check_output(["git", "ls-files", "-z"], cwd=source).split(b"\0")
    epoch = int(lock["source_date_epoch"])
    for name in tracked:
        if name:
            path = source / os.fsdecode(name)
            if path.exists() and not path.is_symlink():
                os.utime(path, (epoch, epoch))
    build.mkdir(parents=True, exist_ok=True)
    identity = {"lock_sha256": digest(LOCK), "source": str(source)}
    identity_file = build / "webloom-build-identity.json"
    if identity_file.exists() and json.loads(identity_file.read_text()) != identity:
        raise SystemExit("Build directory belongs to different inputs; use a fresh --build directory.")
    identity_file.write_text(json.dumps(identity, indent=2) + "\n")
    tex_sources = [source / "texk/web2c/tex.web"]
    tex_sources += [source / "texk/web2c" / name for name in lock["change_chain"]]
    tex_sources += local_changes
    # GNU make propagates command-line assignments to recursive builds. TIE
    # receives the original chain followed by our checked local clock change.
    chain_argument = "tex_ch_srcs=" + " ".join(str(path) for path in tex_sources)
    commands = [[str(source / "configure"), *lock["configure"]], ["make", f"-j{args.jobs}", chain_argument]]
    evidence = {
        "status": "building", "qualified": False, "lock": lock, "commands": commands,
        "environment": {key: env[key] for key in ("LC_ALL", "TZ", "SOURCE_DATE_EPOCH")},
        "tool_versions": {tool: output([tool, "--version"], build, env) for tool in ("gcc", "g++", "make", "ld", "git")},
        "change_chain_sha256": {name: digest(source / "texk/web2c" / name) for name in lock["change_chain"]},
        "local_change_sha256": {str(path.relative_to(ROOT)): digest(path) for path in local_changes},
    }
    report = build / "webloom-build.json"
    report.write_text(json.dumps(evidence, indent=2) + "\n")
    for name, command in zip(("configure", "build"), commands):
        log = build / f"{name}.stdout.log"
        print(f"{name}: {log}", flush=True)
        with log.open("wb") as stream:
            result = subprocess.run(command, cwd=build, env=env, stdout=stream, stderr=subprocess.STDOUT)
        if result.returncode:
            evidence.update(status="failed", failed_stage=name, exit_code=result.returncode)
            report.write_text(json.dumps(evidence, indent=2) + "\n")
            raise SystemExit(f"Reference {name} failed ({result.returncode}); see {log}")
    binaries = [build / "texk/web2c" / name for name in ("tex", "tangle", "tie", "dvitype", "pltotf", "tftopl")]
    evidence["binary_sha256"] = {path.name: digest(path) for path in binaries}
    evidence["generated_sha256"] = {name: digest(build / "texk/web2c" / name) for name in ("tex-final.ch", "tex.p", "tex.pool", "tex0.c", "texini.c", "texd.h")}
    libraries = {}
    for binary in binaries:
        linked = output(["ldd", str(binary)], build, env)
        libraries[binary.name] = {
            "ldd": linked,
            "sha256": {name: digest(Path(name)) for name in re.findall(r"(/[^\s()]+)", linked) if Path(name).is_file()},
        }
    evidence.update(status="built_candidate", linked_libraries=libraries)
    report.write_text(json.dumps(evidence, indent=2) + "\n")
    print(f"Built candidate; qualification and repeat-build comparison remain required: {report}")


if __name__ == "__main__":
    main()
