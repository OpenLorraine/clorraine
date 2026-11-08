"""Shared fixtures for the SDK installer unit tests.

Every test works in a temporary directory. The fake runner replaces the one
external-command function; no test needs Docker, the network, or the real
media folder.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import install_sdk  # noqa: E402


class CommandResult:
    """The observable result of one external command."""

    def __init__(self, returncode: int = 0, stdout: str = "", stderr: str = "") -> None:
        self.returncode = returncode
        self.stdout = stdout
        self.stderr = stderr


class FakeRunner:
    """Maps a command prefix to a result and records every call."""

    def __init__(self) -> None:
        self.calls: list[list[str]] = []
        self.responses: list[tuple[tuple[str, ...], CommandResult]] = []
        self.hooks: list[tuple[tuple[str, ...], object]] = []

    def on(self, prefix: tuple[str, ...], result: CommandResult) -> "FakeRunner":
        self.responses.append((prefix, result))
        return self

    def hook(self, prefix: tuple[str, ...], callback) -> "FakeRunner":
        self.hooks.append((prefix, callback))
        return self

    def __call__(self, argv: list[str], timeout: float | None = None) -> CommandResult:
        self.calls.append(list(argv))
        for prefix, callback in self.hooks:
            if tuple(argv[: len(prefix)]) == prefix:
                result = callback(argv)
                if result is not None:
                    return result
        for prefix, result in reversed(self.responses):
            if tuple(argv[: len(prefix)]) == prefix:
                return result
        return CommandResult(1, "", "unexpected command")


def write_root(root: Path, commit: str = "abc123", image: str = "clorraine-sdk:dev") -> None:
    """Creates the minimal repository files the installer reads."""
    (root / "docker").mkdir(parents=True, exist_ok=True)
    (root / "docker" / "versions.env").write_text(
        f"AMIGA_GCC_COMMIT={commit}\n", encoding="utf-8"
    )
    (root / "docker" / ".env").write_text(
        f"IMAGE={image}\nAMIGA_DATA={root / 'media'}\n", encoding="utf-8"
    )


def write_media(root: Path, roms: int = 1, adfs: int = 1, hdfs: int = 1) -> None:
    """Creates empty media files with the expected suffixes.

    Any previous media tree is replaced so a test can start from an empty
    folder.
    """
    media = root / "media"
    if media.exists():
        import shutil

        shutil.rmtree(media)
    for folder, suffix, count in (
        ("roms", ".rom", roms),
        ("adf", ".adf", adfs),
        ("hdf", ".hdf", hdfs),
    ):
        directory = media / folder
        directory.mkdir(parents=True, exist_ok=True)
        for index in range(count):
            (directory / f"file{index}{suffix}").write_bytes(b"")


def probe_output(
    tools: dict[str, bool] | None = None,
    ndk: bool = True,
    commit: str = "abc123",
) -> str:
    """Builds the image probe's stdout for the given state."""
    tools = tools or {
        "m68k-amigaos-gcc": True,
        "m68k-amigaos-gdb": True,
        "vc": True,
        "vamos": True,
    }
    lines = [f"tool:{name}={'ok' if present else 'missing'}" for name, present in tools.items()]
    lines.append(f"ndk={'ok' if ndk else 'missing'}")
    lines.append(f"provenance={commit}")
    return "\n".join(lines) + "\n"


def ready_runner(root: Path, commit: str = "abc123") -> FakeRunner:
    """A runner describing a fully installed SDK."""
    runner = FakeRunner()
    runner.on(("docker", "version"), CommandResult(0, "27.5.1\n"))
    runner.on(("docker", "compose", "version"), CommandResult(0, "2.30.0\n"))
    runner.on(
        ("docker", "image", "inspect"),
        CommandResult(0, "sha256:abc\n"),
    )
    runner.on(
        ("docker", "run"),
        CommandResult(0, probe_output(commit=commit)),
    )
    return runner
