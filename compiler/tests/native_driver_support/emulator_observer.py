#!/usr/bin/python3
"""Observe a fake emulator process; never interpret an Amiga executable."""

import configparser
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import stat
import sys
import time


# Each helper copy receives a controlled literal tag, never executable metadata.
IDENTITY = "fixture"


def digest(path):
    """Hash one small fixture file without letting a broken test fill memory."""
    with path.open("rb") as stream:
        data = stream.read(16 * 1024 * 1024 + 1)
    assert len(data) <= 16 * 1024 * 1024, path
    return hashlib.sha256(data).hexdigest()


def write_record(path, record):
    """Atomically replace one bounded observation, avoiding partial JSON reads."""
    temporary = path.with_suffix(".writing")
    temporary.write_text(json.dumps(record, sort_keys=True) + "\n", encoding="utf-8")
    temporary.replace(path)


def arguments(argv):
    """Decode documented FS-UAE positional-config and --option=value syntax.

    Hyphens and underscores are interchangeable in option names. Filesystem
    values remain literal; quotes or shell punctuation are not evaluated.
    """
    options = {}
    configs = []
    for arg in argv:
        if arg.startswith("--"):
            key, separator, value = arg[2:].partition("=")
            options[key.replace("-", "_")] = value if separator else "1"
        else:
            configs.append(arg)
    assert len(configs) == 1, ("one explicit configuration is required", argv)
    config = Path(configs[0])
    parser = configparser.ConfigParser(interpolation=None, delimiters=("=",))
    parser.read_string(config.read_text(encoding="utf-8"))
    configured = dict(parser["fs-uae"])
    return config, configured, options


def snapshot():
    """Record declarative configuration, argv, private scopes and staged bytes.

    Base-relative scopes describe documented default-directory containment,
    not a claim that a particular FS-UAE version actually uses those defaults.
    The real-emulator acceptance must observe its effective paths separately.
    """
    config, configured, options = arguments(sys.argv[1:])
    settings = {**configured, **options}
    drive = Path(settings["hard_drive_0"])
    base = Path(settings["base_dir"])
    scopes = {
        "config": str(config),
        "drive": str(drive),
        "cwd": str(Path.cwd()),
        "base": str(base),
        "cache": settings.get("cache_dir", str(base)),
        "state": settings.get("save_states_dir", settings.get("state_dir", str(base))),
        "logs": settings.get("logs_dir", str(base)),
    }
    files = {}
    for path in sorted(drive.rglob("*")):
        mode = path.lstat().st_mode
        if stat.S_ISDIR(mode):
            continue
        assert stat.S_ISREG(mode), ("only copied ordinary files may be mounted", path)
        files[str(path.relative_to(drive))] = digest(path)
    startup = (drive / "s/startup-sequence").read_text(encoding="utf-8")
    early = options.get("base_dir")
    if early is None:
        config_home = Path(os.environ.get("XDG_CONFIG_HOME", str(Path.home() / ".config")))
        base_file = config_home / "fs-uae/base-dir"
        if base_file.is_file():
            early = base_file.read_text(encoding="utf-8").strip()
    return {
        "argv": sys.argv,
        "paths": scopes,
        "configured": configured,
        "settings": settings,
        "early_base": early,
        "files": files,
        "startup": startup,
        "rom_sha256": digest(Path(settings["kickstart_file"])),
        "pid": os.getpid(),
        "violations": [],
    }


def violations(record):
    """Detect removal or mutation while the external child is still running."""
    missing = []
    config = Path(record["paths"]["config"])
    if not config.is_file():
        missing.append("configuration removed before emulator stopped")
    drive = Path(record["paths"]["drive"])
    for relative, expected in record["files"].items():
        path = drive / relative
        if not path.is_file() or digest(path) != expected:
            missing.append("mounted file changed before emulator stopped: " + relative)
    return missing


def observe():
    """Snapshot, optionally wait on a bounded barrier, then return a chosen code.

    Only private declared scopes are touched. Invalid outside paths are recorded
    for assertions but never used as destinations by this observer.
    """
    if sys.argv[1:] in (["--version"], ["--help"]):
        return
    log = Path(os.environ["TEST_EMULATOR_LOG"])
    record_path = log / (IDENTITY + ".emulator.json")
    (log / (IDENTITY + ".pid")).write_text(str(os.getpid()))
    try:
        record = snapshot()
        run_root = Path(record["paths"]["drive"]).parent
        drive = Path(record["paths"]["drive"])
        for key in ("base", "cache", "state", "logs"):
            scope = Path(record["paths"][key])
            if scope.is_absolute() and scope.is_relative_to(run_root) and not scope.is_relative_to(drive):
                scope.mkdir(parents=True, exist_ok=True)
                (scope / ("observer-" + IDENTITY)).write_text(IDENTITY + "\n")
        write_record(record_path, record)
        (log / (IDENTITY + ".started")).write_text("ready\n")
        release = os.environ.get("TEST_EMULATOR_RELEASE")
        deadline = time.monotonic() + 10
        while release and not Path(release).exists():
            record["violations"] = sorted(set(record["violations"] + violations(record)))
            if time.monotonic() >= deadline:
                record["violations"].append("observer barrier timed out")
                write_record(record_path, record)
                raise SystemExit(92)
            time.sleep(0.01)
        record["violations"] = sorted(set(record["violations"] + violations(record)))
        write_record(record_path, record)
        (log / (IDENTITY + ".finished")).write_text("stopping\n")
        if os.environ.get("TEST_EMULATOR_SIGNAL") == "1":
            os.kill(os.getpid(), signal.SIGTERM)
        raise SystemExit(int(os.environ.get("TEST_EMULATOR_STATUS", "0")))
    except (AssertionError, OSError, KeyError, ValueError, configparser.Error) as error:
        write_record(record_path, {"error": str(error), "argv": sys.argv})
        raise SystemExit(91) from error


def check(record, rom, source_root, program=None):
    """Assert the generated handoff only, without simulating FS-UAE's loader."""
    assert "error" not in record, record
    paths = {key: Path(value) for key, value in record["paths"].items()}
    drive = paths["drive"]
    run_root = drive.parent
    assert drive.name == "drive", drive
    assert paths["cwd"] != source_root, "the emulator needs a private working directory"
    for source in source_root.glob("*.clo"):
        for profile in ("hosted-amiga", "hosted-amiga-13"):
            reset = source_root / "build/clorraine" / source.stem / profile
            assert not run_root.is_relative_to(reset), (run_root, reset)
    for key, path in paths.items():
        assert path.is_absolute(), (key, path)
        assert path.is_relative_to(run_root), (key, path, run_root)
        if key != "drive":
            assert not path.is_relative_to(drive), (key, path, drive)
        assert not any(ord(character) < 32 or ord(character) == 127 for character in str(path)), path
    settings = record["settings"]
    assert record["configured"]["end_config"] == "1", record
    assert record["early_base"] == str(paths["base"]), ("initialization needs private path control", record)
    assert settings["amiga_model"] == "A1200", settings
    assert settings["kickstart_file"] == str(rom), settings
    assert record["rom_sha256"] == digest(rom), "the selected ROM bytes must be readable"
    assert settings["hard_drive_0_label"] == "CNEXT", settings
    assert settings["hard_drive_0_priority"] == "0", settings
    for key, value in settings.items():
        extra_drive = re.fullmatch(r"hard_drive_([1-9][0-9]*)", key)
        other_media = re.fullmatch(r"(floppy_drive|cdrom_drive|tape_drive)_[0-9]+", key)
        if extra_drive or other_media or key in ("shared_volume", "uae_filesystem2", "uae_hardfile2"):
            assert not value, ("unexpected mounted media", key, value)
    guests = [name for name in record["files"] if name != "s/startup-sequence"]
    assert len(guests) == 1 and "/" not in guests[0], record["files"]
    assert len(record["files"]) == 2, record["files"]
    if program is not None:
        assert record["files"][guests[0]] == digest(program), "stage must copy the completed executable"
    assert guests[0] in record["startup"], "startup must invoke the fixed guest executable"
    assert str(source_root) not in record["startup"], "host source paths are not guest statements"
    assert not record["violations"], record["violations"]


def check_diagnostics(path):
    """Require valid NDJSON with the existing error-record field names."""
    lines = path.read_text(encoding="utf-8").splitlines()
    assert lines, "an error diagnostic is required"
    required = {
        "code", "severity", "message", "help", "file",
        "line", "column", "endLine", "endColumn",
    }
    for line in lines:
        item = json.loads(line)
        assert isinstance(item, dict) and set(item) == required, item
        assert item["severity"] == "error", item
        assert isinstance(item["message"], str) and item["message"], item


def main():
    """Expose only test-helper checks in addition to the fake process boundary."""
    if len(sys.argv) > 1 and sys.argv[1] == "--fixture-json-error":
        check_diagnostics(Path(sys.argv[2]))
        return
    if len(sys.argv) > 1 and sys.argv[1] in ("--fixture-check", "--fixture-fields"):
        record = json.loads(Path(sys.argv[2]).read_text(encoding="utf-8"))
        if sys.argv[1] == "--fixture-check":
            program = Path(sys.argv[5]) if len(sys.argv) > 5 else None
            check(record, Path(sys.argv[3]), Path(sys.argv[4]), program)
        else:
            assert "error" not in record, record
            fields = dict(record["paths"])
            fields["guest"] = next(name for name in record["files"] if name != "s/startup-sequence")
            fields["executable"] = record["argv"][0]
            fields["files"] = json.dumps(record["files"], sort_keys=True)
            fields["startup"] = record["startup"]
            fields["rom"] = record["settings"]["kickstart_file"]
            for key, value in fields.items():
                sys.stdout.buffer.write(key.encode() + b"\0" + value.encode() + b"\0")
    else:
        observe()


if __name__ == "__main__":
    main()
