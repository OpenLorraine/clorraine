# Run one cmd.exe command line in the signed-in desktop session of this
# Windows host, print its combined output, and exit with its exit code.
#
# Commands started from an SSH logon cannot run the rc24 compiler (cc1.exe
# fails with STATUS_UNTRUSTED_MOUNT_POINT) and FS-UAE needs a desktop for its
# window, so the command runs as a one-shot interactive scheduled task for the
# current user instead. The user must be signed in at the desktop.
#
# Exit codes: the command's own exit code; 124 when it does not finish within
# -TimeoutSeconds; 2 for a usage error.
#
# Example (from the Linux side, through tools/setup/windows-remote.sh):
#   powershell -NoProfile -ExecutionPolicy Bypass -File windows-desktop-run.ps1 `
#     -CommandBase64 Y2xvcnJhaW5lIC0tdmVyc2lvbg== -TimeoutSeconds 60
param(
    # The cmd.exe command line, UTF-8 encoded and then base64 encoded, so it
    # crosses SSH and PowerShell without any quoting.
    [string]$CommandBase64,
    # The cmd.exe command line as plain text, for local use.
    [string]$Command,
    [int]$TimeoutSeconds = 300,
    [string]$WorkingDirectory = (Join-Path $env:USERPROFILE 'clorraine-remote')
)

$ErrorActionPreference = 'Stop'

if ($CommandBase64) {
    $Command = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($CommandBase64))
}
if (-not $Command) {
    [Console]::Error.WriteLine('usage: windows-desktop-run.ps1 -Command LINE | -CommandBase64 B64 [-TimeoutSeconds N] [-WorkingDirectory DIR]')
    exit 2
}

New-Item -ItemType Directory -Force -Path $WorkingDirectory | Out-Null
$id = [guid]::NewGuid().ToString('N').Substring(0, 12)
$runs = Join-Path $env:USERPROFILE 'clorraine-remote\.runs'
$dir = Join-Path $runs $id
New-Item -ItemType Directory -Force -Path $dir | Out-Null

# The command runs from its own batch file in a child cmd.exe, so an `exit`
# in it ends only the child and the redirection captures every part of a
# `a & b` line. The exit code is written to a temporary name and renamed, so
# the poll below never reads a half-written file.
Set-Content -Encoding ascii -Path (Join-Path $dir 'command.cmd') -Value @('@echo off', $Command)
$script = @(
    '@echo off',
    "cd /d `"$WorkingDirectory`"",
    "cmd /d /c `"`"$dir\command.cmd`"`" > `"$dir\out.txt`" 2>&1",
    "echo %ERRORLEVEL% > `"$dir\exit.tmp`"",
    "move /y `"$dir\exit.tmp`" `"$dir\exit.txt`" > nul"
)
Set-Content -Encoding ascii -Path (Join-Path $dir 'run.cmd') -Value $script

$task = "clorraine-remote-$id"
$null = schtasks /Create /F /TN $task /TR "cmd /c `"$dir\run.cmd`"" /SC ONCE /ST 23:59 /IT /RU $env:USERNAME 2>&1
$null = schtasks /Run /TN $task 2>&1
$deadline = (Get-Date).AddSeconds($TimeoutSeconds)
while (-not (Test-Path (Join-Path $dir 'exit.txt')) -and (Get-Date) -lt $deadline) {
    Start-Sleep -Milliseconds 300
}
$null = schtasks /Delete /F /TN $task 2>&1

$code = 124
if (Test-Path (Join-Path $dir 'out.txt')) { Get-Content (Join-Path $dir 'out.txt') }
if (Test-Path (Join-Path $dir 'exit.txt')) {
    $code = [int]((Get-Content (Join-Path $dir 'exit.txt')).Trim())
    Remove-Item -Recurse -Force $dir
} else {
    # The task may still be running, so its folder is left in place.
    [Console]::Error.WriteLine("timeout after $TimeoutSeconds s; is $env:USERNAME signed in at the desktop? (run folder: $dir)")
}
exit $code
