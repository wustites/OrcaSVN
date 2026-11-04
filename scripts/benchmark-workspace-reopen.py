"""Read-only Windows reopen simulation using Python and the installed SVN CLI.

Does not execute Rust or modify the working copy/application cache. Reproduces
the metadata fallback, not Tauri/WebView startup. All rounds use fresh state;
the filesystem cache is warm. JSON contains counts/timings, never status paths.
"""
import argparse
import ctypes
from ctypes import wintypes
import hashlib
import json
import os
from pathlib import Path
import shutil
import statistics
import subprocess
import time
import tempfile
import xml.etree.ElementTree as ET


class Overlapped(ctypes.Structure):
    _fields_ = [("internal", ctypes.c_size_t), ("internal_high", ctypes.c_size_t),
                ("offset", wintypes.DWORD), ("offset_high", wintypes.DWORD),
                ("event", wintypes.HANDLE)]


class DirectoryWatch:
    """Same ordinary-permission request/cancellation pattern as status_watch.rs."""
    def __init__(self, root):
        self.kernel = ctypes.WinDLL("kernel32", use_last_error=True)
        self.kernel.CreateFileW.argtypes = [wintypes.LPCWSTR, wintypes.DWORD, wintypes.DWORD,
                                           ctypes.c_void_p, wintypes.DWORD, wintypes.DWORD,
                                           wintypes.HANDLE]
        self.kernel.CreateFileW.restype = wintypes.HANDLE
        self.kernel.ReadDirectoryChangesW.argtypes = [wintypes.HANDLE, ctypes.c_void_p,
            wintypes.DWORD, wintypes.BOOL, wintypes.DWORD, ctypes.c_void_p,
            ctypes.POINTER(Overlapped), ctypes.c_void_p]
        self.kernel.ReadDirectoryChangesW.restype = wintypes.BOOL
        self.kernel.CancelIoEx.argtypes = [wintypes.HANDLE, ctypes.POINTER(Overlapped)]
        self.kernel.GetOverlappedResult.argtypes = [wintypes.HANDLE,
            ctypes.POINTER(Overlapped), ctypes.POINTER(wintypes.DWORD), wintypes.BOOL]
        self.kernel.CloseHandle.argtypes = [wintypes.HANDLE]
        self.handle = self.kernel.CreateFileW(str(root), 1, 7, None, 3, 0x42200000, None)
        if self.handle == ctypes.c_void_p(-1).value:
            raise ctypes.WinError(ctypes.get_last_error())
        self.buffer = (ctypes.c_uint32 * 16384)()
        self.overlapped = Overlapped()
        self.pending = False
        success = self.kernel.ReadDirectoryChangesW(self.handle, self.buffer, 65536,
            True, 1 | 2 | 4 | 8 | 16 | 64 | 256, None,
            ctypes.byref(self.overlapped), None)
        if not success and ctypes.get_last_error() != 997:
            error = ctypes.WinError(ctypes.get_last_error())
            self.kernel.CloseHandle(self.handle)
            raise error
        self.pending = True

    def close(self):
        if self.pending:
            self.kernel.CancelIoEx(self.handle, ctypes.byref(self.overlapped))
            count = wintypes.DWORD()
            self.kernel.GetOverlappedResult(self.handle, ctypes.byref(self.overlapped),
                                            ctypes.byref(count), True)
        self.kernel.CloseHandle(self.handle)
        self.pending = False


def normalized(path):
    return os.path.normcase(os.path.abspath(str(path).removeprefix("\\\\?\\")))


def configuration(previous, svn):
    # Preserve the application's executable key only when it identifies the
    # installed CLI used for this measurement; never execute a path from JSON.
    result = {}
    for key in previous:
        if key.startswith("executable:"):
            name = key[len("executable:"):]
            if name == "svn" or normalized(name) == normalized(svn):
                result[key] = 0
    if not result:
        result["executable:" + svn] = 0
    directories = ["/etc/subversion"]
    for variable in ("HOME", "USERPROFILE", "APPDATA"):
        if os.environ.get(variable):
            directories.append(os.path.join(os.environ[variable],
                               "Subversion" if variable == "APPDATA" else ".subversion"))
    candidates = set(key for key in previous if not key.startswith("executable:"))
    candidates.update(os.path.join(directory, name) for directory in directories
                      for name in ("config", "servers"))
    for path in candidates:
        try:
            if os.path.getsize(path) > 1024 * 1024:
                raise RuntimeError("SVN configuration exceeds cache limit")
            data = Path(path).read_bytes()
        except FileNotFoundError:
            continue
        value = 0xcbf29ce484222325
        for byte in data:
            value = ((value ^ byte) * 0x100000001b3) & 0xffffffffffffffff
        result[path] = value
    return result


def validate(snapshot, root):
    if snapshot["version"] != 6 or normalized(snapshot["root"]) != normalized(root):
        raise RuntimeError("Snapshot version/root mismatch")
    if not 0 <= time.time() - snapshot["saved"] < 86400:
        raise RuntimeError("Snapshot expired")
    inventory = snapshot["inventory"]
    files, directories = inventory["files"], inventory["directories"]
    if len(files) > 500000 or "" not in directories or inventory.get("journal"):
        raise RuntimeError("This simulation requires a bounded metadata-only snapshot")
    previous = None
    index = {}
    for position, (path, stamp) in enumerate(files):
        if previous is not None and previous >= path:
            raise RuntimeError("Unsorted snapshot paths")
        if not path or stamp["k"] not in (0, 1, 2) or ":" in path or "\\" in path:
            raise RuntimeError("Invalid snapshot path")
        if any(part in ("", ".", "..") for part in path.split("/")):
            raise RuntimeError("Unsafe snapshot path")
        if stamp["k"] == 0 and path not in directories:
            raise RuntimeError("Incomplete directory index")
        index[path] = position
        previous = path
    seen = set()
    for parent, directory in directories.items():
        if parent and (parent not in index or files[index[parent]][1]["k"] != 0):
            raise RuntimeError("Invalid parent directory")
        previous = -1
        for position in directory["children"]:
            if not previous < position < len(files) or position in seen:
                raise RuntimeError("Invalid child index")
            path = files[position][0]
            if path.rpartition("/")[0] != parent:
                raise RuntimeError("Invalid child parent")
            seen.add(position)
            previous = position
    if len(seen) != len(files):
        raise RuntimeError("Incomplete child index")


def capture(root, previous_config, svn):
    started = time.perf_counter()
    config = configuration(previous_config, svn)
    files, directories = [], {}
    pending = [("", False)]
    directory_stat_seconds = 0
    while pending:
        relative, administrative = pending.pop()
        directory = os.path.join(root, relative)
        before_stat = time.perf_counter()
        stamp = os.stat(directory, follow_symlinks=False)
        directory_stat_seconds += time.perf_counter() - before_stat
        directories[relative] = {"stamp": {"modified": stamp.st_mtime_ns,
            "changed": 0, "device": 0, "inode": 0}, "children": []}
        with os.scandir(directory) as entries:
            for entry in entries:
                if administrative and entry.name in ("pristine", "tmp"):
                    continue
                path = relative + "/" + entry.name if relative else entry.name
                if entry.is_dir(follow_symlinks=False):
                    files.append([path, {"k": 0, "s": 0, "m": 0, "c": 0}])
                    pending.append((path, administrative or entry.name in (".svn", "_svn")))
                else:
                    metadata = entry.stat(follow_symlinks=False)
                    value = {"k": 2 if entry.is_symlink() else 1, "s": metadata.st_size,
                             "m": metadata.st_mtime_ns, "c": 0}
                    if entry.is_symlink():
                        value["l"] = os.readlink(entry.path)
                    files.append([path, value])
        if len(files) > 500000:
            raise RuntimeError("Inventory exceeds cache limit")
    files.sort(key=lambda entry: entry[0])
    for position, (path, _) in enumerate(files):
        directories[path.rpartition("/")[0]]["children"].append(position)
    return {"files": files, "directories": directories, "configuration": config,
            "administration": {}}, {"inventory_ms": (time.perf_counter() - started) * 1000,
            "directory_stat_ms": directory_stat_seconds * 1000}


def differences(before, after):
    left, right = dict(before["files"]), dict(after["files"])
    changed = sorted(path for path in left.keys() | right.keys() if left.get(path) != right.get(path))
    admin = sum(any(part in (".svn", "_svn") for part in path.split("/")) for path in changed)
    directories = sum(left.get(path, {}).get("k") == 0 or right.get(path, {}).get("k") == 0
                      for path in changed)
    config_changed = before["configuration"] != after["configuration"]
    full = bool(admin or directories or config_changed or len(changed) > 256)
    return changed, full, {"candidate_paths": len(changed), "administration_changes": admin,
        "directory_changes": directories, "configuration_changed": config_changed}


def status(svn, root, paths=None):
    started = time.perf_counter()
    outputs = []
    groups = [None] if paths is None else [paths[i:i + 64] for i in range(0, len(paths), 64)]
    for group in groups:
        args = [svn, "--non-interactive", "status", "--xml"]
        if group is not None:
            args += ["--depth", "empty", "--"] + [path + "@" for path in group]
        result = subprocess.run(args, cwd=root, capture_output=True)
        if result.returncode:
            # Do not print target paths, SVN output or credentials on failure.
            raise RuntimeError("SVN status failed, exit code " + str(result.returncode))
        outputs.append(result.stdout)
    return (time.perf_counter() - started) * 1000, b"\0".join(outputs)


def round_trip(raw, root, svn):
    started = time.perf_counter()
    watch = DirectoryWatch(root)
    revision = subprocess.Popen([svn, "--non-interactive", "info", "--show-item", "revision"],
                                cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    setup_ms = (time.perf_counter() - started) * 1000
    try:
        stage = time.perf_counter()
        # Both real and synthetic snapshots are read from disk on every round.
        payload = raw.read_bytes() if isinstance(raw, Path) else raw
        if len(payload) > 64 * 1024 * 1024:
            raise RuntimeError("Snapshot exceeds size limit")
        saved = json.loads(payload)
        validate(saved, root)
        cache_ms = (time.perf_counter() - stage) * 1000
        current, timings = capture(root, saved["inventory"]["configuration"], svn)
        stage = time.perf_counter()
        paths, full, reason = differences(saved["inventory"], current)
        compare_ms = (time.perf_counter() - stage) * 1000
        svn_ms = 0
        if full or paths:
            svn_ms, _ = status(svn, root, None if full else paths)
        revision.communicate(timeout=120)
        if revision.returncode:
            raise RuntimeError("Local revision query failed")
        total_ms = (time.perf_counter() - started) * 1000
        return {"total_ms": total_ms, "setup_ms": setup_ms, "cache_read_parse_validate_ms": cache_ms,
            **timings, "compare_ms": compare_ms, "svn_status_ms": svn_ms,
            "query_mode": "full" if full else ("scoped" if paths else "reuse"), **reason}
    finally:
        if revision.poll() is None:
            revision.kill()
            revision.communicate()
        watch.close()


def summary(rounds):
    return {"min_ms": min(row["total_ms"] for row in rounds),
            "median_ms": statistics.median(row["total_ms"] for row in rounds),
            "max_ms": max(row["total_ms"] for row in rounds)}


def status_entries(xml):
    entries = []
    for entry in ET.fromstring(xml).findall(".//entry"):
        state = entry.find("wc-status").attrib
        code = state["item"]
        if code in ("deleted", "replaced"):
            raise RuntimeError("Synthetic baseline requires expanded deleted/replaced statuses")
        entries.append({"path": entry.attrib["path"].replace("\\", "/"),
            "status": code, "status_code": code, "prop_status": state.get("props", "none"),
            "locked": state.get("locked") == "true", "history": state.get("copied") == "true",
            "switched": state.get("switched") == "true"})
    return entries


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--workspace", required=True)
    parser.add_argument("--snapshot", required=True)
    parser.add_argument("--runs", type=int, default=3)
    parser.add_argument("--scenario", choices=("both", "existing", "fresh"), default="both")
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    if os.name != "nt" or not 1 <= args.runs <= 20:
        parser.error("Windows and 1–20 runs required")
    root, snapshot_path = Path(args.workspace).resolve(), Path(args.snapshot).resolve()
    if not (root / ".svn").is_dir():
        parser.error("Use a working-copy root; ancestor administration is not simulated")
    svn = shutil.which("svn")
    if not svn:
        parser.error("SVN CLI required")
    original = snapshot_path.read_bytes()
    saved = json.loads(original)
    validate(saved, root)
    report = {"method": "Python metadata-fallback simulation; not Rust or desktop timing",
        "filesystem_cache": "warm; repeated sequential measurements", "workspace": str(root),
        "runs_per_scenario": args.runs, "existing_cache_sha256": hashlib.sha256(original).hexdigest(),
        "existing_cache_bytes": len(original), "existing_cache_age_seconds": time.time() - saved["saved"],
        "snapshot_version": saved["version"], "has_usn": bool(saved["inventory"].get("journal")),
        "scenarios": {}}
    scenarios = {"both": ("existing-disk-cache", "fresh-baseline-no-extra-edits"),
                 "existing": ("existing-disk-cache",),
                 "fresh": ("fresh-baseline-no-extra-edits",)}[args.scenario]
    for scenario in scenarios:
        temporary_snapshot = None
        if scenario == "existing-disk-cache":
            raw = snapshot_path
        else:
            # Preparing a new baseline is NOT reopening time. Keep it entirely
            # separate from the application's persistent cache and working copy.
            first, _ = capture(root, saved["inventory"]["configuration"], svn)
            _, status_before = status(svn, root)
            second, _ = capture(root, saved["inventory"]["configuration"], svn)
            changed, full, _ = differences(first, second)
            if full or changed or first != second:
                raise RuntimeError("Workspace changed while preparing baseline; rerun when stable")
            raw = json.dumps({"version": 6, "root": str(root), "saved": int(time.time()),
                "inventory": second, "entries": status_entries(status_before)}, separators=(",", ":")).encode()
            report["fresh_baseline_svn_output_sha256"] = hashlib.sha256(status_before).hexdigest()
            report["fresh_baseline_bytes"] = len(raw)
            with tempfile.NamedTemporaryFile(prefix="orcasvn-reopen-", suffix=".json", delete=False) as file:
                file.write(raw)
                temporary_snapshot = Path(file.name)
            raw = temporary_snapshot
            report["fresh_baseline_storage"] = "unique temporary disk file; deleted after measurements"
            report["inventory_entries"] = len(second["files"])
            report["directory_count"] = len(second["directories"])
        rounds = []
        try:
            for index in range(args.runs):
                result = round_trip(raw, root, svn)
                rounds.append(result)
                print(json.dumps({"scenario": scenario, "run": index + 1, **result}), flush=True)
        finally:
            if temporary_snapshot is not None:
                temporary_snapshot.unlink()
        report["scenarios"][scenario] = {"rounds": rounds, "summary": summary(rounds)}
        print(json.dumps({"scenario": scenario, "summary": summary(rounds)}), flush=True)
        Path(args.output).write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    report["existing_cache_changed_during_test"] = hashlib.sha256(snapshot_path.read_bytes()).hexdigest() != report["existing_cache_sha256"]
    Path(args.output).write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    main()
