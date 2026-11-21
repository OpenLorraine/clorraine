"""Standalone isolated import/validator probe for an exact produced zip application.

Only this archive is added to the interpreter's import path. The probe never
claims that importing modules constitutes a working compiler installation.
"""

from __future__ import annotations

import importlib
import json
import os
import runpy
import sys
import sysconfig
from pathlib import Path


MODULE_MEMBERS = {
    "tools": "tools/__init__.py",
    "tools.setup": "tools/setup/__init__.py",
    "tools.distribution": "tools/distribution/__init__.py",
    "tools.setup.native_archive": "tools/setup/native_archive.py",
    "tools.setup.native_state": "tools/setup/native_state.py",
    "tools.distribution.manifest": "tools/distribution/manifest.py",
    "tools.setup.install_native": "tools/setup/install_native.py",
}
"""Documented packages/modules that must load from the same exact helper archive."""
REQUIRED_MEMBERS = frozenset({"__main__.py", *MODULE_MEMBERS.values()})
"""Required source subset; additional explicitly included runtime data is allowed."""
EMPTY_INITIALIZERS = ("tools/__init__.py", "tools/setup/__init__.py",
                      "tools/distribution/__init__.py")
"""Package boundaries whose archive contents are specified to be empty."""
FORBIDDEN_MARKER = "BOOTSTRAP_PROBE_FORBIDDEN"
"""Distinct failure marker so a blocked side effect cannot masquerade as CLI rejection."""


def _audit(event: str, _arguments: tuple) -> None:
    """Keep import/invalid-invocation probes offline and prohibit process execution."""
    if event.startswith("socket.") or event in ("subprocess.Popen", "os.system",
                                                "os.exec", "os.posix_spawn",
                                                "os.fork", "os.forkpty"):
        raise AssertionError(FORBIDDEN_MARKER + ": " + event)


def check_origins(archive: Path, baseline_paths: tuple[str, ...]) -> dict[str, str]:
    """Reject checkout, current-directory, site-package and source-recipe dependencies."""
    prefix = str(archive) + os.sep
    stdlib = {Path(sysconfig.get_path(name)).resolve() for name in ("stdlib", "platstdlib")}
    forbidden = ("tools.distribution.package_core", "tools.setup.install_sdk",
                 "tools.distribution.prepare_ndk")
    origins = {}
    for name, module in tuple(sys.modules.items()):
        if name == "__main__" or module is None:
            continue
        if any(name == item or name.startswith(item + ".") for item in forbidden):
            raise AssertionError("helper imported an out-of-closure recipe: " + name)
        specification = getattr(module, "__spec__", None)
        origin = getattr(specification, "origin", None)
        if origin in (None, "built-in", "frozen"):
            continue
        if origin.startswith(prefix):
            if name != "tools" and not name.startswith("tools."):
                raise AssertionError("helper contains a non-stdlib dependency: " + name)
            origins[name] = origin
            continue
        path = Path(origin).resolve()
        if ("site-packages" in path.parts or "dist-packages" in path.parts or
                not any(path.is_relative_to(root) for root in stdlib)):
            raise AssertionError("helper imported outside stdlib/exact archive: " + name + " " + origin)
    for entry in sys.path:
        if entry not in baseline_paths and entry != str(archive) and not entry.startswith(prefix):
            raise AssertionError("helper added an external import directory: " + entry)
    return origins


def import_closure(archive: Path, baseline_paths: tuple[str, ...]) -> dict:
    """Import known public modules and prove their origins, without inspecting source."""
    modules = {}
    for name, member in MODULE_MEMBERS.items():
        module = importlib.import_module(name)
        origin = getattr(getattr(module, "__spec__", None), "origin", None)
        if origin != str(archive) + "/" + member:
            raise AssertionError("known helper module did not load from exact zip: " + name)
        modules[name] = module
    if not callable(getattr(modules["tools.setup.install_native"], "main", None)):
        raise AssertionError("missing public install_native.main(argv=None)")
    validator = modules["tools.distribution.manifest"]
    error = getattr(validator, "ManifestError", None)
    if (not callable(getattr(validator, "parse_manifest", None)) or
            not isinstance(error, type) or not issubclass(error, ValueError)):
        raise AssertionError("missing public manifest parser/error boundary in the zip")
    check_origins(archive, baseline_paths)
    return modules


def main() -> None:
    """Run only test-owned probe modes; none is an installer CLI option."""
    mode, selected, *arguments = sys.argv[1:]
    archive = Path(selected).resolve(strict=True)
    baseline_paths = tuple(sys.path)
    sys.dont_write_bytecode = True
    sys.path.insert(0, str(archive))
    sys.addaudithook(_audit)
    if mode == "entry-status":
        # This is the actual zip entry, not a substitute __main__ or a patched
        # installer. Only deliberately invalid public arguments are supplied.
        sys.argv = [str(archive), *arguments]
        try:
            runpy.run_path(str(archive), run_name="__main__")
        except SystemExit:
            check_origins(archive, baseline_paths)
            raise
        check_origins(archive, baseline_paths)
        return
    modules = import_closure(archive, baseline_paths)
    if mode == "api-status":
        try:
            result = modules["tools.setup.install_native"].main(arguments)
        except SystemExit:
            check_origins(archive, baseline_paths)
            raise
        check_origins(archive, baseline_paths)
        if type(result) is not int:
            raise AssertionError("public installer main must return an integer status")
        raise SystemExit(result)
    if mode == "validate":
        data = Path(arguments[0]).read_bytes()
        validator = modules["tools.distribution.manifest"]
        expected = json.loads(data)
        if validator.parse_manifest(data) != expected:
            raise AssertionError("packaged validator changed valid manifest data")
        bad_schema = dict(expected, schema=2)
        invalid = (json.dumps(bad_schema).encode(),
                   data.replace(b'"schema":1', b'"schema":1,"schema":1', 1))
        for value in invalid:
            try:
                validator.parse_manifest(value)
            except validator.ManifestError:
                pass
            else:
                raise AssertionError("packaged validator accepted invalid metadata")
        check_origins(archive, baseline_paths)
    elif mode != "closure":
        raise AssertionError("unknown test-only probe mode")
    print(json.dumps({"origins": {name: module.__spec__.origin for name, module in modules.items()},
                      "isolated": sys.flags.isolated, "no_site": sys.flags.no_site,
                      "pythonpath_present": "PYTHONPATH" in os.environ}, sort_keys=True))


if __name__ == "__main__":
    main()
