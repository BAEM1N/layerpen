#!/usr/bin/env python3
"""Inspect a built Pointory macOS bundle without launching its graphical app.

Only evidence files and a temporary, read-only DMG mount are created. No screen
capture, audio recording, model loading/downloading, or cloud requests run here.
Exit status: 0 = no failed checks (review unavailable checks), 1 = failed checks,
2 = invocation/platform error. Optional checks are explicitly unavailable when
their input is omitted; a passing probe does not imply usable GPU or microphone.
"""

import argparse
import datetime
import hashlib
import json
import mmap
import os
from pathlib import Path
import platform
import plistlib
import re
import shutil
import subprocess
import sys
import tempfile


RESOURCES = (
    "GETTING-STARTED.md", "INSTALL.ko.md", "LICENSE", "LICENSE.tauri", "LICENSE.lucide",
    "THIRD-PARTY-NOTICES.txt", "USER-GUIDE.ko.md", "third-party-sources.zip",
    "README.ja.md", "README.zh-CN.md", "STT-SETUP.md", "stt/worker.py",
    "stt/providers.py", "stt/acceleration.py", "stt/model_manager.py", "stt/runtime_setup.py", "stt/requirements.txt",
    "stt/setup-windows.ps1",
)
VALIDATION_MARKERS = (
    b"pointory-validation-finished", b"pointory-validation-eval",
    b"POINTORY_VALIDATION_REPORT", b"native-validation",
)


def sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


class Preflight:
    def __init__(self, args):
        self.args = args
        self.checks = []
        self.observations = []
        self.secrets = [str(Path.home())]
        self.secrets += [str(value) for value in (args.app, args.dmg, args.stt_python,
                                                args.output_dir) if value]
        self.executable = None
        self.resources = args.app / "Contents" / "Resources"

    def clean(self, value):
        if isinstance(value, dict):
            return {str(key): self.clean(item) for key, item in value.items()}
        if isinstance(value, (list, tuple)):
            return [self.clean(item) for item in value]
        if isinstance(value, str):
            for private_path in sorted(self.secrets, key=len, reverse=True):
                value = value.replace(private_path, "<local-path>")
            value = re.sub(r"/Users/[^/\s]+", "/Users/<user>", value)
            return value
        return value

    def add(self, name, status, detail):
        self.checks.append({"name": name, "status": status, "detail": self.clean(detail)})

    def note(self, name, detail):
        self.observations.append({"name": name, "detail": self.clean(detail)})

    def run(self, argv, *, timeout=45, stdin=None, binary=False):
        environment = os.environ.copy()
        environment.update({"PYTHONDONTWRITEBYTECODE": "1", "HF_HUB_OFFLINE": "1",
                            "TRANSFORMERS_OFFLINE": "1", "HF_DATASETS_OFFLINE": "1"})
        try:
            completed = subprocess.run(
                [str(arg) for arg in argv], input=stdin, capture_output=True,
                text=not binary, timeout=timeout, env=environment,
            )
            return completed.returncode, completed.stdout, completed.stderr
        except subprocess.TimeoutExpired:
            return None, b"" if binary else "", "Command timed out after %s seconds" % timeout
        except OSError as error:
            return None, b"" if binary else "", str(error)

    def command_check(self, name, argv, timeout=45):
        code, stdout, stderr = self.run(argv, timeout=timeout)
        detail = {"exit_code": code, "stdout": stdout[-12000:], "stderr": stderr[-12000:]}
        self.add(name, "passed" if code == 0 else "failed", detail)
        return code, stdout, stderr

    def inspect_platform(self):
        self.add("platform", "passed", {
            "system": platform.system(), "architecture": platform.machine(),
            "macos_version": platform.mac_ver()[0], "kernel_release": platform.release(),
            "preflight_python_version": platform.python_version(),
        })
        self.command_check("macos_build", ["/usr/bin/sw_vers"])
        code, stdout, _ = self.run(["/usr/sbin/sysctl", "-n", "hw.model"])
        self.note("machine_model", stdout.strip() if code == 0 else "Unavailable")
        for program in ("codesign", "hdiutil", "file", "lipo"):
            self.add("tool_" + program, "passed" if shutil.which(program) else "unavailable",
                     "Available" if shutil.which(program) else "Executable not found on PATH")

    def inspect_app(self):
        app = self.args.app
        if not app.is_dir():
            self.add("app_bundle", "failed", "Supplied .app directory does not exist")
            return
        self.add("app_bundle", "passed", {"name": app.name})
        info_path = app / "Contents" / "Info.plist"
        try:
            info = plistlib.loads(info_path.read_bytes())
        except (OSError, plistlib.InvalidFileException, ValueError) as error:
            self.add("info_plist", "failed", str(error))
            return
        fields = {key: info.get(key) for key in (
            "CFBundleName", "CFBundleDisplayName", "CFBundleIdentifier",
            "CFBundleShortVersionString", "CFBundleVersion", "CFBundleExecutable",
            "LSMinimumSystemVersion", "NSMicrophoneUsageDescription",
        )}
        required = ("CFBundleName", "CFBundleIdentifier", "CFBundleShortVersionString",
                    "CFBundleVersion", "CFBundleExecutable", "LSMinimumSystemVersion",
                    "NSMicrophoneUsageDescription")
        missing = [key for key in required if not isinstance(info.get(key), str) or not info[key].strip()]
        self.add("info_plist", "failed" if missing else "passed",
                 {"values": fields, "missing_or_empty": missing})
        self.add("pointory_bundle_name", "passed" if info.get("CFBundleName") == "Pointory" else "failed",
                 {"CFBundleName": info.get("CFBundleName")})
        minimum = info.get("LSMinimumSystemVersion", "")
        current = platform.mac_ver()[0]
        try:
            padded = lambda value: tuple((list(map(int, value.split("."))) + [0, 0, 0])[:3])
            self.add("host_meets_declared_minimum_os", "passed" if padded(current) >= padded(minimum) else "failed",
                     {"minimum": minimum, "host": current,
                      "scope": "Compatibility with older macOS versions has not been tested."})
        except ValueError:
            self.add("host_meets_declared_minimum_os", "failed", "Invalid macOS version value")
        self.command_check("codesign_verification", ["/usr/bin/codesign", "--verify", "--deep", "--strict", "--verbose=2", app])
        code, stdout, stderr = self.run(["/usr/bin/codesign", "-d", "--verbose=4", app])
        signature = stdout + stderr
        # Keep useful signing metadata, excluding arbitrary executable paths.
        metadata = [line for line in signature.splitlines() if line.startswith((
            "Identifier=", "Format=", "CodeDirectory ", "Signature=", "Authority=",
            "TeamIdentifier=", "Runtime Version=", "Sealed Resources ",
        ))]
        kind = "ad-hoc" if "Signature=adhoc" in signature else (
            "Developer ID" if "Authority=Developer ID Application:" in signature else "other/unknown")
        self.add("codesign_metadata", "passed" if code == 0 else "failed",
                 {"signature_type": kind, "metadata": metadata})
        self.note("distribution_scope", {
            "signature_type": kind,
            "notarization": "Not verified; valid local code signing does not establish notarization or Gatekeeper acceptance.",
        })
        name = info.get("CFBundleExecutable")
        if not isinstance(name, str) or Path(name).name != name:
            self.add("app_executable", "failed", "Missing or invalid CFBundleExecutable filename")
            return
        self.executable = app / "Contents" / "MacOS" / name
        if not self.executable.is_file():
            self.add("app_executable", "failed", "Bundle executable is missing")
            self.executable = None
        else:
            self.add("app_executable", "passed", {"name": name, "sha256": sha256(self.executable)})
            self.command_check("executable_file_type", ["/usr/bin/file", self.executable])
            code, stdout, stderr = self.run(["/usr/bin/lipo", "-archs", self.executable])
            architectures = stdout.strip().split()
            self.add("arm64_binary", "passed" if code == 0 and "arm64" in architectures else "failed",
                     {"architectures": architectures, "stderr": stderr})
            try:
                with self.executable.open("rb") as stream, mmap.mmap(stream.fileno(), 0, access=mmap.ACCESS_READ) as binary:
                    found = [marker.decode("ascii") for marker in VALIDATION_MARKERS if binary.find(marker) >= 0]
                self.add("production_excludes_validation_markers", "failed" if found else "passed",
                         {"found": found, "checked": [marker.decode("ascii") for marker in VALIDATION_MARKERS]})
            except (OSError, ValueError) as error:
                self.add("production_excludes_validation_markers", "failed", str(error))
        entries = [{"resource": name, "present": (self.resources / name).is_file(),
                    "nonempty": (self.resources / name).is_file() and (self.resources / name).stat().st_size > 0}
                   for name in RESOURCES]
        self.add("bundled_resources", "passed" if all(item["nonempty"] for item in entries) else "failed", entries)

    def inspect_dmg(self):
        dmg = self.args.dmg
        if dmg is None:
            self.add("dmg", "unavailable", "No --dmg supplied")
            return
        if not dmg.is_file():
            self.add("dmg", "failed", "Supplied DMG does not exist")
            return
        self.add("dmg", "passed", {"name": dmg.name, "bytes": dmg.stat().st_size, "sha256": sha256(dmg)})
        code, _, _ = self.command_check("dmg_verification", ["/usr/bin/hdiutil", "verify", dmg], timeout=60)
        if code != 0:
            self.add("dmg_app_matches", "unavailable", "Skipped mounting an unverified DMG")
            return
        mount = Path(tempfile.mkdtemp(prefix="pointory-preflight-"))
        self.secrets.append(str(mount))
        attached = False
        try:
            code, stdout, stderr = self.run([
                "/usr/bin/hdiutil", "attach", dmg, "-readonly", "-nobrowse",
                "-noautoopen", "-plist", "-mountpoint", mount,
            ], timeout=60, binary=True)
            if isinstance(stderr, bytes):
                stderr = stderr.decode("utf-8", errors="replace")
            attached = os.path.ismount(mount)
            if code != 0:
                self.add("dmg_readonly_mount", "failed", {"exit_code": code, "stderr": stderr})
                return
            try:
                plist = plistlib.loads(stdout)
                attached = attached or any(item.get("mount-point") == str(mount)
                                           for item in plist.get("system-entities", []))
            except (ValueError, plistlib.InvalidFileException):
                self.add("dmg_readonly_mount", "failed", "hdiutil returned an invalid mount report")
                return
            self.add("dmg_readonly_mount", "passed" if attached else "failed",
                     {"mounted": attached, "requested_options": ["readonly", "nobrowse", "noautoopen"]})
            if not attached:
                return
            apps = list(mount.glob("*.app"))
            self.add("dmg_contains_app", "passed" if len(apps) == 1 and apps[0].name == self.args.app.name else "failed",
                     {"app_names": [app.name for app in apps], "applications_shortcut": (mount / "Applications").is_symlink()})
            packaged = mount / self.args.app.name
            if self.executable and packaged.is_dir():
                other = packaged / "Contents" / "MacOS" / self.executable.name
                other_hash = sha256(other) if other.is_file() else None
                expected_hash = sha256(self.executable)
                self.add("dmg_app_matches", "passed" if other_hash == expected_hash else "failed",
                         {"app_executable_sha256": expected_hash, "dmg_executable_sha256": other_hash})
                self.command_check("dmg_app_codesign_verification",
                                   ["/usr/bin/codesign", "--verify", "--deep", "--strict", packaged])
            else:
                self.add("dmg_app_matches", "unavailable", "Source or packaged executable is unavailable")
        finally:
            if attached or os.path.ismount(mount):
                code, stdout, stderr = self.run(["/usr/bin/hdiutil", "detach", mount], timeout=45)
                self.add("dmg_detached", "passed" if code == 0 and not os.path.ismount(mount) else "failed",
                         {"exit_code": code, "stdout": stdout, "stderr": stderr})
            try:
                mount.rmdir()  # Only the empty directory we created; never recurse.
            except OSError:
                self.note("temporary_mount_directory", "Could not remove temporary mount directory; review detach result.")

    def inspect_stt(self):
        interpreter = self.args.stt_python
        worker = self.resources / "stt" / "worker.py"
        if interpreter is None:
            self.add("stt_runtime", "unavailable", "No --stt-python supplied")
            return
        if not interpreter.is_file() or not worker.is_file():
            self.add("stt_runtime", "failed", "STT interpreter or bundled worker is missing")
            return
        probe = r'''
import importlib, importlib.metadata, json, platform
pairs = [("numpy", "numpy"), ("soundcard", "SoundCard"), ("websockets", "websockets"),
         ("faster_whisper", "faster-whisper"), ("ctranslate2", "ctranslate2")]
result = {"python": platform.python_version(), "architecture": platform.machine(), "dependencies": []}
for module, distribution in pairs:
    entry = {"module": module, "distribution": distribution}
    try:
        entry["version"] = importlib.metadata.version(distribution)
        importlib.import_module(module)
        entry["imported"] = True
    except Exception as error:
        entry.update(imported=False, error=type(error).__name__ + ": " + str(error))
    result["dependencies"].append(entry)
print(json.dumps(result))
'''
        code, stdout, stderr = self.run([interpreter, "-B", "-c", probe], timeout=60)
        try:
            runtime = json.loads(stdout.strip().splitlines()[-1])
            ok = code == 0 and all(item["imported"] for item in runtime["dependencies"])
            self.add("stt_runtime_dependencies", "passed" if ok else "failed", runtime)
        except (IndexError, ValueError, KeyError, TypeError):
            self.add("stt_runtime_dependencies", "failed", {"exit_code": code, "stdout": stdout[-4000:], "stderr": stderr[-4000:]})
        for command in ("hardware", "devices"):
            code, stdout, stderr = self.run([interpreter, "-B", worker], timeout=60,
                                          stdin=json.dumps({"command": command}) + "\n")
            try:
                events = [json.loads(line) for line in stdout.splitlines() if line.strip()]
                event = next(item for item in events if item.get("type") == command)
                devices = event["devices"]
                if not isinstance(devices, list) or code != 0:
                    raise ValueError("Worker probe did not complete successfully")
                # Audio IDs/names can contain private hardware identifiers. Keep counts only.
                if command == "devices":
                    detail = {"source": "bundled worker", "input_device_count": len(devices)}
                    self.add("stt_device_enumeration", "passed", detail)
                    self.add("microphone_available", "passed" if devices else "unavailable",
                             "At least one input is listed; recording and permission remain untested."
                             if devices else "No microphone input is listed; recording cannot be verified.")
                else:
                    ids = [str(item.get("id", "")) for item in devices]
                    supported = [item for item in ids if item == "cpu" or item == "cuda" or item.startswith("openvino:")]
                    self.add("stt_hardware_enumeration", "passed",
                             {"source": "bundled worker", "runtime_device_ids": supported})
                    accelerated = [item for item in supported if item != "cpu"]
                    self.add("stt_accelerator_available", "passed" if accelerated else "unavailable",
                             {"runtime_device_ids": accelerated,
                              "scope": "Runtime enumeration only; no model inference was run. Apple GPU/ANE support is not inferred from physical hardware."})
                if stderr.strip():
                    # Do not persist arbitrary device diagnostics; names may identify hardware.
                    self.note("stt_" + command + "_diagnostics", "Worker wrote diagnostic output to stderr; device identifiers are omitted.")
            except (ValueError, KeyError, TypeError, StopIteration):
                self.add("stt_" + command + "_enumeration", "failed",
                         {"exit_code": code, "reason": "Bundled worker failed or returned invalid JSON. Raw device output omitted for privacy."})
        self.add("microphone_recording", "unavailable", "Intentionally excluded: no recording or permission request is made.")
        self.add("stt_model_inference", "unavailable", "Intentionally excluded: no model loading, downloads, or cloud requests.")

    def write_report(self):
        counts = {status: sum(item["status"] == status for item in self.checks)
                  for status in ("passed", "failed", "unavailable")}
        report = {
            "schema_version": 1, "created_at_utc": datetime.datetime.now(datetime.timezone.utc).isoformat(),
            "scope": "Headless macOS preflight; no GUI launch, screen capture, microphone recording, inference, or cloud requests.",
            "result": "failed" if counts["failed"] else "partial" if counts["unavailable"] else "passed",
            "counts": counts, "checks": self.checks, "observations": self.observations,
        }
        self.args.output_dir.mkdir(parents=True, exist_ok=True)
        target = self.args.output_dir / "macos-preflight.json"
        target.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(json.dumps({"report": target.name, "result": report["result"], "counts": counts}))
        return 1 if counts["failed"] else 0


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--app", required=True, type=Path, help="Production Pointory.app bundle to inspect")
    parser.add_argument("--dmg", type=Path, help="Optional DMG to verify and temporarily mount read-only")
    parser.add_argument("--stt-python", type=Path, help="Optional Python executable with STT dependencies installed")
    parser.add_argument("--output-dir", required=True, type=Path, help="Directory for macos-preflight.json (overwritten on rerun)")
    args = parser.parse_args()
    for name in ("app", "dmg", "stt_python", "output_dir"):
        value = getattr(args, name)
        if value is not None:
            setattr(args, name, value.expanduser().absolute())
    if args.output_dir.resolve().is_relative_to(args.app.resolve()):
        parser.error("--output-dir must be outside the inspected .app bundle")
    checker = Preflight(args)
    if platform.system() != "Darwin":
        checker.add("platform", "unavailable", "This preflight requires macOS; use --help on other platforms.")
        checker.write_report()
        return 2
    for name, method in (("platform", checker.inspect_platform), ("app", checker.inspect_app),
                         ("dmg", checker.inspect_dmg), ("stt", checker.inspect_stt)):
        try:
            method()
        except Exception as error:
            checker.add(name + "_unexpected_error", "failed", type(error).__name__ + ": " + str(error))
    return checker.write_report()


if __name__ == "__main__":
    sys.exit(main())
