# Clorraine Windows installer.
#
# The released script is assembled by tools/setup/package.sh, which replaces
# the @...@ pins below. The checked-in copy is a template: its network paths
# refuse to run until assembled, while -Archive/-Checksums works for a local
# candidate.
#
# Usage (Windows PowerShell 5.1, a one-time process-scope policy bypass):
#   curl.exe -fsSLo clorraine-install.ps1 <release>/install.ps1
#   powershell -ExecutionPolicy Bypass -File .\clorraine-install.ps1
#
# The installer is per-user: the CLI goes to
# %LOCALAPPDATA%\Programs\Clorraine\versions\<version>-<sha12>, a `current`
# junction names the active version, and `current\bin` is the one user PATH
# entry. Only the optional rc24 toolchain step asks for administrator approval.
# Every download is verified by size and SHA-256 before use, and the active
# version is switched only after the extracted tree passes its version check.
#
# This file is ASCII-only so Windows PowerShell 5.1 reads it the same way in
# every code page.
param(
    [string]$Prefix,
    [switch]$ModifyPath,
    [string]$Archive,
    [string]$Checksums,
    [string]$Toolchain,
    [switch]$NoToolchain,
    [switch]$InstallToolchain,
    [switch]$ReplaceToolchain,
    [string]$FsUae,
    [string]$Kickstart,
    [switch]$NoEmulator,
    [switch]$InstallFsUae,
    [switch]$NoHello,
    [string]$Starter,
    [switch]$NoStarter,
    [switch]$VSCode,
    [switch]$NoVSCode,
    [switch]$Uninstall,
    [switch]$Purge,
    [switch]$Help
)

# --- Release pins (replaced by package.sh) -----------------------------------

$script:Version = '@VERSION@'
$script:ArchiveName = '@ARCHIVE_NAME@'
$script:ArchiveUrl = '@ARCHIVE_URL@'
$script:ArchiveBytes = '@ARCHIVE_BYTES@'
$script:ArchiveSha256 = '@ARCHIVE_SHA256@'
$script:VsixName = '@VSIX_NAME@'
$script:VsixUrl = '@VSIX_URL@'
$script:VsixBytes = '@VSIX_BYTES@'
$script:VsixSha256 = '@VSIX_SHA256@'

# The registry key whose `Path` value is the user PATH; tests use a scratch key.
$script:PathRegistryKey = 'HKCU:\Environment'

# The installer's own lock, shared by every run in this logon session.
$script:MutexName = 'Local\Clorraine-Install'

# Files every Windows candidate carries under its top folder.
$script:CandidateFiles = @(
    'bin/clc.exe', 'bin/clorraine.exe', 'bin/clorraine-lsp.exe',
    'LICENSE', 'LICENSE-EXCEPTION.md', 'NOTICE', 'THIRD_PARTY_NOTICES.md', 'README.md'
)

# --- Output ------------------------------------------------------------------

# The colours and markers for one console. Glyphs need a virtual-terminal
# console in the UTF-8 code page; colour needs virtual-terminal support and
# no NO_COLOR. Otherwise plain [ok]/[!!] markers keep the output readable in
# logs and old consoles. Returns a hashtable of strings.
function New-OutputStyle {
    param([bool]$VirtualTerminal, [int]$CodePage, [bool]$NoColor)
    $style = @{
        Check = '[ok]'; Cross = '[!!]'; Arrow = '>'; Play = '>'
        Bold = ''; Dim = ''; Reset = ''; Green = ''; Yellow = ''; Cyan = ''
    }
    if ($VirtualTerminal -and $CodePage -eq 65001) {
        $style.Check = [string][char]0x2714
        $style.Cross = [string][char]0x2718
        $style.Arrow = [string][char]0x25B8
        $style.Play = [string][char]0x25B6
    }
    if ($VirtualTerminal -and -not $NoColor) {
        $esc = [string][char]27
        $style.Bold = "$esc[1m"; $style.Dim = "$esc[2m"; $style.Reset = "$esc[0m"
        $style.Green = "$esc[38;5;114m"; $style.Yellow = "$esc[38;5;221m"; $style.Cyan = "$esc[38;5;81m"
    }
    return $style
}

# Whether the console renders virtual-terminal sequences, and its code page.
function Get-ConsoleCapability {
    $vt = $false
    if (-not [Console]::IsOutputRedirected) {
        try { $vt = [bool]$Host.UI.SupportsVirtualTerminal } catch { $vt = $false }
    }
    return [pscustomobject]@{ VirtualTerminal = $vt; CodePage = [Console]::OutputEncoding.CodePage }
}

$script:Style = New-OutputStyle -VirtualTerminal $false -CodePage 0 -NoColor $true

# Selects the style for this console once per run.
function Initialize-Style {
    $console = Get-ConsoleCapability
    $script:Style = New-OutputStyle -VirtualTerminal $console.VirtualTerminal -CodePage $console.CodePage `
        -NoColor ([bool]$env:NO_COLOR)
}

# Removes terminal-control characters from text that came from outside.
function Get-SafeText([string]$Text) {
    if ($null -eq $Text) { return '' }
    return ($Text -replace '[\x00-\x08\x0B-\x1F\x7F]', '')
}

# Writes one plain line to standard output.
function Write-Line([string]$Text = '') { [Console]::Out.WriteLine($Text) }

# Writes a success line.
function Write-Ok([string]$Text) {
    Write-Line ('  ' + $script:Style.Green + $script:Style.Check + $script:Style.Reset + ' ' + $Text)
}

# Writes a warning line.
function Write-Warn([string]$Text) {
    Write-Line ('  ' + $script:Style.Yellow + $script:Style.Cross + $script:Style.Reset + ' ' + $Text)
}

# Writes a numbered step heading.
function Write-Step([int]$Number, [string]$Title) {
    Write-Line ''
    Write-Line ($script:Style.Cyan + $script:Style.Arrow + " Step $Number of 5" + $script:Style.Reset + '  ' +
        $script:Style.Bold + $Title + $script:Style.Reset)
}

# The help text.
function Get-UsageText {
    return @"
Usage: install.ps1 [options]

Install Clorraine $script:Version for this Windows user
(default prefix: %LOCALAPPDATA%\Programs\Clorraine).

Options:
  -Prefix DIR           installation folder (absolute path)
  -ModifyPath           add the CLI to the user PATH without asking
  -Archive FILE         install an exact local archive
  -Checksums FILE       SHA256SUMS for -Archive
  -Toolchain PATH       use an existing rc24 toolchain root
  -NoToolchain          skip toolchain setup and the hello checks
  -InstallToolchain     install the pinned rc24 toolchain (one UAC prompt)
  -ReplaceToolchain     replace another amiga-gcc version already installed
  -FsUae PATH           FS-UAE executable for the Amiga hello
  -Kickstart PATH       licensed KS3.1 A1200 ROM for the Amiga hello
  -NoEmulator           skip FS-UAE detection and the Amiga hello
  -InstallFsUae         download the pinned FS-UAE 3.2.35 when none is found
  -NoHello              do not run the hello programs
  -Starter DIR          create a starter Amiga project in DIR
  -NoStarter            do not offer a starter project
  -VSCode               install the VS Code extension when detected
  -NoVSCode             skip VS Code detection and install
  -Uninstall            remove the active version link and the PATH entry
  -Purge                with -Uninstall, also remove versions and the config
  -Help                 show this help

Run it with a one-time bypass of the execution policy:
  powershell -ExecutionPolicy Bypass -File .\clorraine-install.ps1
"@
}

# --- Failures ----------------------------------------------------------------

# Stops the run with an exit code and a message for standard error.
function Stop-Install([int]$Code, [string]$Message) {
    $exception = New-Object System.Exception $Message
    $exception.Data['ClorraineExitCode'] = $Code
    throw $exception
}

# Stops the run with a usage error (exit 2).
function Stop-Usage([string]$Message) { Stop-Install 2 $Message }

# --- Host facts (replaceable in tests) ----------------------------------------

# The native architecture of the running Windows (X64, Arm64, X86, ...).
function Get-HostArchitecture {
    try {
        return [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
    } catch {
        # Older .NET Framework builds lack RuntimeInformation; a 32-bit
        # PowerShell on 64-bit Windows reports the native CPU in W6432.
        $native = if ($env:PROCESSOR_ARCHITEW6432) { $env:PROCESSOR_ARCHITEW6432 } else { $env:PROCESSOR_ARCHITECTURE }
        switch ($native) { 'AMD64' { return 'X64' } 'ARM64' { return 'Arm64' } default { return $native } }
    }
}

# The Windows build number (22000 and later is Windows 11).
function Get-OsBuild { return [Environment]::OSVersion.Version.Build }

# True when a person can answer prompts: neither input nor output is redirected.
function Test-Interactive {
    return (-not [Console]::IsInputRedirected) -and (-not [Console]::IsOutputRedirected)
}

# Asks a yes/no question; an empty answer takes the default.
function Read-YesNo([string]$Prompt, [bool]$Default) {
    while ($true) {
        [Console]::Out.Write($Prompt)
        $answer = [Console]::In.ReadLine()
        if ($null -eq $answer -or $answer -eq '') { return $Default }
        if ($answer -match '^(y|yes)$') { return $true }
        if ($answer -match '^(n|no)$') { return $false }
        Write-Line 'Please answer y or n.'
    }
}

# --- Processes ---------------------------------------------------------------

# Quotes one argument for a Windows command line (CommandLineToArgvW rules).
function ConvertTo-NativeArgument([string]$Value) {
    if ($Value -ne '' -and $Value -notmatch '[\s"]') { return $Value }
    $builder = New-Object System.Text.StringBuilder
    [void]$builder.Append('"')
    $slashes = 0
    foreach ($ch in $Value.ToCharArray()) {
        if ($ch -eq '\') { $slashes++; continue }
        if ($ch -eq '"') {
            [void]$builder.Append('\', 2 * $slashes + 1)
        } elseif ($slashes -gt 0) {
            [void]$builder.Append('\', $slashes)
        }
        $slashes = 0
        [void]$builder.Append($ch)
    }
    [void]$builder.Append('\', 2 * $slashes)
    [void]$builder.Append('"')
    return $builder.ToString()
}

# Runs a program without a shell and returns its exit code and output. The
# streams are read asynchronously so a chatty child cannot block on a full
# pipe.
function Invoke-Native([string]$File, [string[]]$Arguments = @(), [string]$WorkingDirectory = '') {
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = $File
    $info.Arguments = (($Arguments | ForEach-Object { ConvertTo-NativeArgument $_ }) -join ' ')
    $info.UseShellExecute = $false
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.CreateNoWindow = $true
    if ($WorkingDirectory) { $info.WorkingDirectory = $WorkingDirectory }
    $process = [System.Diagnostics.Process]::Start($info)
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $stderr = $process.StandardError.ReadToEndAsync()
    $process.WaitForExit()
    return [pscustomobject]@{ Code = $process.ExitCode; Out = $stdout.Result; Err = $stderr.Result }
}

# --- Paths and deletion ------------------------------------------------------

# True for an absolute drive path such as C:\Users\me.
function Test-DrivePath([string]$Path) { return $Path -match '^[A-Za-z]:\\' }

# True when two Windows paths name the same location.
function Test-SamePath([string]$A, [string]$B) {
    return [string]::Equals($A.TrimEnd('\'), $B.TrimEnd('\'), [StringComparison]::OrdinalIgnoreCase)
}

# True when Path is a reparse point (a junction or a symbolic link).
function Test-ReparsePoint([string]$Path) {
    $item = Get-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue
    return ($null -ne $item) -and (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0)
}

# The target of a junction, or $null when Path is not one.
function Get-JunctionTarget([string]$Path) {
    $item = Get-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue
    if ($null -eq $item -or $item.LinkType -ne 'Junction') { return $null }
    return (@($item.Target)[0]).TrimEnd('\')
}

# True when Link is a junction to a folder directly inside Versions: the only
# links this installer creates and therefore the only ones it removes.
function Test-OwnedLink([string]$Link, [string]$Versions) {
    $target = Get-JunctionTarget $Link
    if (-not $target) { return $false }
    return Test-SamePath (Split-Path -Parent $target) $Versions
}

# Removes a junction or symbolic link itself, never what it points to.
function Remove-Link([string]$Path) {
    $item = Get-Item -LiteralPath $Path -Force
    if ($item.PSIsContainer) { [IO.Directory]::Delete($Path, $false) } else { [IO.File]::Delete($Path) }
}

# Deletes a tree without following reparse points: a junction or symbolic
# link inside it is removed as a link, so purge can never reach a folder the
# link points to. Read-only files are cleared first.
function Remove-TreeSafe([string]$Path) {
    if (-not (Test-Path -LiteralPath $Path)) {
        if ($null -ne (Get-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue)) { Remove-Link $Path }
        return
    }
    if (Test-ReparsePoint $Path) { Remove-Link $Path; return }
    $item = Get-Item -LiteralPath $Path -Force
    if (-not $item.PSIsContainer) {
        $item.Attributes = [IO.FileAttributes]::Normal
        [IO.File]::Delete($Path)
        return
    }
    foreach ($child in @(Get-ChildItem -LiteralPath $Path -Force)) { Remove-TreeSafe $child.FullName }
    [IO.Directory]::Delete($Path, $false)
}

# A new private folder under the user's temporary folder.
function New-PrivateTemp([string]$Tag) {
    $path = Join-Path ([IO.Path]::GetTempPath()) ("clorraine-$Tag-" + [guid]::NewGuid().ToString('N').Substring(0, 12))
    New-Item -ItemType Directory -Path $path | Out-Null
    return $path
}

# The lowercase SHA-256 of a file.
function Get-Sha256([string]$Path) {
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

# --- Downloads and verification ----------------------------------------------

# True when the release pins were filled in by package.sh.
function Test-Assembled {
    return ($script:ArchiveBytes -match '^[0-9]+$') -and ($script:ArchiveSha256 -match '^[0-9a-fA-F]{64}$') -and
        ($script:Version -notmatch '@')
}

# Downloads Url over HTTPS only (redirects included) into Destination and
# requires the exact size and SHA-256; a mismatch deletes the file and stops.
function Invoke-Download([string]$Url, [string]$Destination, [long]$Bytes, [string]$Sha256, [int]$MaxSeconds = 600) {
    $curl = Join-Path $env:SystemRoot 'System32\curl.exe'
    $name = Split-Path -Leaf $Destination
    $result = Invoke-Native $curl @(
        '--disable', '--fail', '--silent', '--show-error', '--location',
        '--proto', '=https', '--proto-redir', '=https',
        '--connect-timeout', '20', '--max-time', "$MaxSeconds",
        '--max-filesize', ([string]($Bytes + 1)), '--output', $Destination, $Url)
    if ($result.Code -ne 0) {
        Remove-Item -LiteralPath $Destination -Force -ErrorAction SilentlyContinue
        Stop-Install 1 ("download of $name failed; check HTTPS access and retry (" + (Get-SafeText $result.Err).Trim() + ')')
    }
    $actualBytes = (Get-Item -LiteralPath $Destination).Length
    if ($actualBytes -ne $Bytes) {
        Remove-Item -LiteralPath $Destination -Force
        Stop-Install 1 "$name size mismatch; expected $Bytes bytes, got $actualBytes"
    }
    $actual = Get-Sha256 $Destination
    if ($actual -ne $Sha256.ToLowerInvariant()) {
        Remove-Item -LiteralPath $Destination -Force
        Stop-Install 1 "$name SHA-256 mismatch; expected $Sha256, got $actual"
    }
}

# The digest a SHA256SUMS file records for one file name; exactly one line
# must name it.
function Get-RecordedDigest([string]$ChecksumFile, [string]$Name) {
    $found = @(Get-Content -LiteralPath $ChecksumFile | ForEach-Object {
        if ($_ -match '^([0-9a-fA-F]{64}) [ *](.+)$' -and $Matches[2] -eq $Name) { $Matches[1].ToLowerInvariant() }
    })
    if ($found.Count -ne 1) { Stop-Install 1 "checksum file '$ChecksumFile' has no unique entry for '$Name'" }
    return $found[0]
}

# Verifies a local archive against its checksum file.
function Test-LocalArchive([string]$Path, [string]$ChecksumFile) {
    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) { Stop-Install 1 "archive '$Path' is not a readable file" }
    if (-not (Test-Path -LiteralPath $ChecksumFile -PathType Leaf)) { Stop-Install 1 "checksum file '$ChecksumFile' is not a readable file" }
    $name = Split-Path -Leaf $Path
    $expected = Get-RecordedDigest $ChecksumFile $name
    $actual = Get-Sha256 $Path
    if ($actual -ne $expected) { Stop-Install 1 "$name SHA-256 mismatch; expected $expected, got $actual" }
}

# Checks every entry of a candidate zip before anything is extracted: one top
# folder, no absolute, drive, or parent-relative names, and exactly the
# expected file set. Returns the top folder name.
function Test-CandidateZip([string]$Path) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $top = "clorraine-$script:Version-x86_64-windows"
    $seen = @{}
    $zip = [IO.Compression.ZipFile]::OpenRead($Path)
    try {
        foreach ($entry in $zip.Entries) {
            $name = $entry.FullName.Replace('\', '/')
            $parts = $name.Split('/')
            if ($name.StartsWith('/') -or $name.Contains(':') -or ($parts -contains '..') -or ($parts -contains '.')) {
                Stop-Install 1 ('archive contains an unsafe entry: ' + (Get-SafeText $entry.FullName))
            }
            if ($parts[0] -ne $top) {
                Stop-Install 1 ('archive entry outside ' + $top + ': ' + (Get-SafeText $entry.FullName))
            }
            if ($name.EndsWith('/')) { continue }
            $relative = $name.Substring($top.Length + 1)
            if ($script:CandidateFiles -notcontains $relative) {
                Stop-Install 1 ('archive has an unexpected file: ' + (Get-SafeText $entry.FullName))
            }
            $seen[$relative] = $true
        }
    } finally {
        $zip.Dispose()
    }
    foreach ($file in $script:CandidateFiles) {
        if (-not $seen.ContainsKey($file)) { Stop-Install 1 "archive is missing $file" }
    }
    return $top
}

# Requires the extracted CLI to report exactly this installer's version.
function Test-ExtractedVersion([string]$Tree) {
    $clorraine = Join-Path $Tree 'bin\clorraine.exe'
    $result = Invoke-Native $clorraine @('--version')
    $reported = (Get-SafeText $result.Out).Trim()
    if ($result.Code -ne 0 -or $reported -ne "clorraine $script:Version") {
        Stop-Install 1 "the installed CLI failed its version check: '$reported' (expected 'clorraine $script:Version')"
    }
}

# --- Lock and switch ---------------------------------------------------------

# Takes the installer lock without waiting. An abandoned lock (a run that was
# killed) counts as taken.
function Lock-Installer {
    $mutex = New-Object System.Threading.Mutex($false, $script:MutexName)
    $owned = $false
    try {
        $owned = $mutex.WaitOne(0)
    } catch [System.Threading.AbandonedMutexException] {
        $owned = $true
    }
    if (-not $owned) {
        $mutex.Dispose()
        Stop-Install 1 'another Clorraine installer is running; wait for it to finish'
    }
    return $mutex
}

# Finishes a switch an interrupted run left half done: `current` missing with
# an owned `current.new` becomes `current`; a leftover `current.new` beside a
# `current` is removed.
function Repair-Switch([string]$Root) {
    $versions = Join-Path $Root 'versions'
    $current = Join-Path $Root 'current'
    $pending = Join-Path $Root 'current.new'
    if ($null -eq (Get-Item -LiteralPath $pending -Force -ErrorAction SilentlyContinue)) { return }
    if (-not (Test-OwnedLink $pending $versions)) {
        Stop-Install 1 "'$pending' is not a Clorraine link; remove it and retry"
    }
    if ($null -eq (Get-Item -LiteralPath $current -Force -ErrorAction SilentlyContinue)) {
        [IO.Directory]::Move($pending, $current)
    } else {
        Remove-Link $pending
    }
}

# Points `current` at Folder. A junction cannot be replaced atomically, so the
# new link is made as `current.new` first and renamed after the old link is
# removed; Repair-Switch completes the rename if the run stops in between.
function Set-CurrentVersion([string]$Root, [string]$Folder) {
    $versions = Join-Path $Root 'versions'
    $current = Join-Path $Root 'current'
    $pending = Join-Path $Root 'current.new'
    if ($null -ne (Get-Item -LiteralPath $current -Force -ErrorAction SilentlyContinue) -and
        -not (Test-OwnedLink $current $versions)) {
        Stop-Install 1 "'$current' exists and is not a Clorraine link; remove it and retry"
    }
    New-Item -ItemType Junction -Path $pending -Target $Folder | Out-Null
    if ($null -ne (Get-Item -LiteralPath $current -Force -ErrorAction SilentlyContinue)) { Remove-Link $current }
    [IO.Directory]::Move($pending, $current)
}

# --- User PATH ---------------------------------------------------------------

# Opens the user-environment key named by $script:PathRegistryKey.
function Open-PathKey([bool]$Writable) {
    $name = $script:PathRegistryKey -replace '^HKCU:\\', ''
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($name, $Writable)
    if ($null -eq $key -and $Writable) { $key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($name) }
    return $key
}

# The raw user PATH (environment references unexpanded) and its value kind.
function Get-UserPath {
    $key = Open-PathKey $false
    if ($null -eq $key) { return [pscustomobject]@{ Value = ''; Kind = $null } }
    try {
        $value = $key.GetValue('Path', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        if ($null -eq $value) { return [pscustomobject]@{ Value = ''; Kind = $null } }
        return [pscustomobject]@{ Value = [string]$value; Kind = $key.GetValueKind('Path') }
    } finally { $key.Dispose() }
}

# Writes the user PATH, keeping an expandable value expandable.
function Set-UserPath([string]$Value, $Kind) {
    if ($null -eq $Kind -or ($Kind -ne [Microsoft.Win32.RegistryValueKind]::ExpandString -and $Value.Contains('%'))) {
        $Kind = if ($Value.Contains('%')) { [Microsoft.Win32.RegistryValueKind]::ExpandString } else { [Microsoft.Win32.RegistryValueKind]::String }
    }
    $key = Open-PathKey $true
    try { $key.SetValue('Path', $Value, $Kind) } finally { $key.Dispose() }
    if ($script:PathRegistryKey -eq 'HKCU:\Environment') { Send-EnvironmentChange }
}

# True when one PATH entry names Folder (case and a trailing backslash ignored).
function Test-PathEntry([string]$Entry, [string]$Folder) {
    return ($Entry -ne '') -and (Test-SamePath $Entry $Folder)
}

# Adds Folder to the user PATH once.
function Add-UserPathEntry([string]$Folder) {
    $path = Get-UserPath
    $entries = @($path.Value.Split(';'))
    if (@($entries | Where-Object { Test-PathEntry $_ $Folder }).Count -gt 0) { return }
    $value = if ($path.Value.TrimEnd(';') -eq '') { $Folder } else { $path.Value.TrimEnd(';') + ';' + $Folder }
    Set-UserPath $value $path.Kind
}

# Removes exactly the user PATH entries that name Folder; others are kept.
function Remove-UserPathEntry([string]$Folder) {
    $path = Get-UserPath
    if ($path.Value -eq '') { return }
    $entries = @($path.Value.Split(';'))
    $kept = @($entries | Where-Object { -not (Test-PathEntry $_ $Folder) })
    if ($kept.Count -eq $entries.Count) { return }
    Set-UserPath ($kept -join ';') $path.Kind
}

# Tells running programs (Explorer, new terminals) that the environment
# changed, waiting at most five seconds for any one window.
function Send-EnvironmentChange {
    if (-not ('Clorraine.NativeMethods' -as [type])) {
        Add-Type -Namespace Clorraine -Name NativeMethods -MemberDefinition @'
[DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
public static extern IntPtr SendMessageTimeout(IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam,
    uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);
'@
    }
    $result = [UIntPtr]::Zero
    [void][Clorraine.NativeMethods]::SendMessageTimeout([IntPtr]0xffff, 0x001A, [UIntPtr]::Zero, 'Environment', 0x0002, 5000, [ref]$result)
}

# --- Install -----------------------------------------------------------------

# The default installation folder for this user.
function Get-DefaultPrefix { return Join-Path $env:LOCALAPPDATA 'Programs\Clorraine' }

# Validates the prefix and creates it.
function Resolve-Prefix([string]$Path) {
    if (-not (Test-DrivePath $Path)) { Stop-Install 1 "-Prefix must be an absolute path such as C:\Users\me\Clorraine" }
    if ($Path -match '[;"]' -or $Path -match '[\x00-\x1F\x7F]') {
        Stop-Install 1 '-Prefix must not contain a semicolon, a double quote, or control characters'
    }
    $full = [IO.Path]::GetFullPath($Path).TrimEnd('\')
    New-Item -ItemType Directory -Force -Path $full | Out-Null
    return $full
}

# The writable hello sources under the prefix; an existing file is kept, so a
# user's edit survives a reinstall.
function Install-Examples([string]$Root) {
    $examples = Join-Path $Root 'examples'
    New-Item -ItemType Directory -Force -Path $examples | Out-Null
    $hello = Join-Path $examples 'hello.clo'
    $guest = Join-Path $examples 'hello-amiga.clo'
    $hostSource = @'
// The smallest Clorraine program: print one templated line.
module demo.Hello;

import { println } from std.io;

func main(): i32 {
    let who: str = "Amiga";
    println($"Hello, {who}, from Clorraine!");
    return 0;
}
'@
    if (-not (Test-Path -LiteralPath $hello)) { [IO.File]::WriteAllText($hello, $hostSource.Replace("`r`n", "`n") + "`n", [Text.Encoding]::ASCII) }
    if (-not (Test-Path -LiteralPath $guest)) { [IO.File]::WriteAllText($guest, (Get-HelloAmigaSource) + "`n", [Text.Encoding]::ASCII) }
    return [pscustomobject]@{ Folder = $examples; Hello = $hello; Guest = $guest }
}

# The Amiga hello: the demo-scene intro shown in FS-UAE.
function Get-HelloAmigaSource {
    $source = @'
@HELLO_AMIGA_SOURCE@
'@
    return $source.Replace("`r`n", "`n")
}

# Installs one verified archive into Root and switches `current` to it.
# Returns the version folder. Any failure before the switch removes what this
# run created; a failure during the switch restores the previous link.
function Install-Archive([string]$Root, [string]$ArchivePath) {
    $versions = Join-Path $Root 'versions'
    New-Item -ItemType Directory -Force -Path $versions | Out-Null
    $top = Test-CandidateZip $ArchivePath
    $folder = Join-Path $versions ("$script:Version-" + (Get-Sha256 $ArchivePath).Substring(0, 12))
    $staging = Join-Path $versions ('.tmp-' + [guid]::NewGuid().ToString('N').Substring(0, 12))
    $created = $false
    $previous = Get-JunctionTarget (Join-Path $Root 'current')
    $switched = $false
    try {
        New-Item -ItemType Directory -Path $staging | Out-Null
        $tar = Join-Path $env:SystemRoot 'System32\tar.exe'
        $result = Invoke-Native $tar @('-x', '-f', $ArchivePath, '-C', $staging)
        if ($result.Code -ne 0) { Stop-Install 1 ('cannot extract the archive: ' + (Get-SafeText $result.Err).Trim()) }
        $tree = Join-Path $staging $top
        Test-ExtractedVersion $tree
        if (Test-Path -LiteralPath $folder) {
            Test-ExtractedVersion $folder
        } else {
            [IO.Directory]::Move($tree, $folder)
            $created = $true
        }
        $switched = $true
        Set-CurrentVersion $Root $folder
    } catch {
        if ($switched) {
            $current = Join-Path $Root 'current'
            if ($null -ne (Get-Item -LiteralPath (Join-Path $Root 'current.new') -Force -ErrorAction SilentlyContinue)) {
                Remove-Link (Join-Path $Root 'current.new')
            }
            if ($previous -and -not (Test-SamePath ([string](Get-JunctionTarget $current)) $previous)) {
                if ($null -ne (Get-Item -LiteralPath $current -Force -ErrorAction SilentlyContinue)) { Remove-Link $current }
                New-Item -ItemType Junction -Path $current -Target $previous | Out-Null
            }
        }
        if ($created -and (Test-Path -LiteralPath $folder)) { Remove-TreeSafe $folder }
        throw
    } finally {
        if (Test-Path -LiteralPath $staging) { Remove-TreeSafe $staging }
    }
    return $folder
}

# Adds `current\bin` to the user PATH when asked (or when the person agrees),
# or prints how to do it.
function Update-UserPath([string]$Root, [bool]$Requested, [bool]$Interactive) {
    $bin = Join-Path $Root 'current\bin'
    $add = $Requested
    if (-not $Requested -and $Interactive) {
        $add = Read-YesNo "  Add $bin to your user PATH? [Y/n] " $true
    }
    if ($add) {
        Add-UserPathEntry $bin
        Write-Ok "added $bin to your user PATH (new terminals see it)"
    } else {
        Write-Line "  to run clorraine from any terminal, add $bin to your user PATH or rerun with -ModifyPath"
    }
}

# --- Uninstall ---------------------------------------------------------------

# The program files running from inside Root, as full paths.
function Get-ProcessesUnder([string]$Root) {
    $prefix = $Root.TrimEnd('\') + '\'
    return @(Get-Process -ErrorAction SilentlyContinue | ForEach-Object {
        $path = $null
        try { $path = $_.Path } catch { $path = $null }
        if ($path -and $path.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) { $path }
    })
}

# Removes the active version link and the PATH entry; with Purge also the
# versions, examples, the FS-UAE this installer downloaded, and the config.
# Everything is checked before anything is removed.
function Invoke-Uninstall([string]$Root, [bool]$AlsoPurge) {
    $versions = Join-Path $Root 'versions'
    $links = @('current', 'current.new' | ForEach-Object { Join-Path $Root $_ } |
        Where-Object { $null -ne (Get-Item -LiteralPath $_ -Force -ErrorAction SilentlyContinue) })
    foreach ($link in $links) {
        if (-not (Test-OwnedLink $link $versions)) {
            Stop-Install 1 "'$link' is not a Clorraine link; nothing was removed"
        }
    }
    if ($AlsoPurge -and (Get-ProcessesUnder $Root).Count -gt 0) {
        Stop-Install 1 'Clorraine programs are running from this installation; close VS Code and running Clorraine builds, then retry'
    }
    Remove-UserPathEntry (Join-Path $Root 'current\bin')
    foreach ($link in $links) { Remove-Link $link }
    Write-Ok "removed the Clorraine link and PATH entry from $Root"
    if ($AlsoPurge) {
        foreach ($name in 'versions', 'examples', 'fs-uae\3.2.35') {
            $path = Join-Path $Root $name
            if ($null -ne (Get-Item -LiteralPath $path -Force -ErrorAction SilentlyContinue)) { Remove-TreeSafe $path }
        }
        $fsuae = Join-Path $Root 'fs-uae'
        if ((Test-Path -LiteralPath $fsuae) -and @(Get-ChildItem -LiteralPath $fsuae -Force).Count -eq 0) {
            [IO.Directory]::Delete($fsuae, $false)
        }
        $configDir = Join-Path $env:APPDATA 'clorraine'
        $config = Join-Path $configDir 'config'
        if (Test-Path -LiteralPath $config) { Remove-Item -LiteralPath $config -Force }
        if ((Test-Path -LiteralPath $configDir) -and @(Get-ChildItem -LiteralPath $configDir -Force).Count -eq 0) {
            [IO.Directory]::Delete($configDir, $false)
        }
        Write-Ok "removed the Clorraine versions, examples, and $config"
    }
    Write-Line '  The rc24 toolchain and any FS-UAE you installed yourself are kept.'
    Write-Line '  Remove rc24 under Settings > Apps > Installed apps (m68k-amigaos-gcc).'
}

# --- Command line ------------------------------------------------------------

# Rejects option combinations that cannot mean one thing.
function Test-Options([hashtable]$Options) {
    $given = { param($name) $Options.ContainsKey($name) -and [bool]$Options[$name] }
    if ((& $given 'Purge') -and -not (& $given 'Uninstall')) { Stop-Usage '-Purge requires -Uninstall' }
    if (& $given 'Uninstall') {
        foreach ($name in $Options.Keys) {
            if ($name -notin 'Uninstall', 'Purge', 'Prefix' -and (& $given $name)) {
                Stop-Usage "-Uninstall cannot be combined with -$name"
            }
        }
    }
    if ((& $given 'Toolchain') -and ((& $given 'NoToolchain') -or (& $given 'InstallToolchain'))) {
        Stop-Usage '-Toolchain cannot be combined with -NoToolchain or -InstallToolchain'
    }
    if ((& $given 'NoToolchain') -and ((& $given 'InstallToolchain') -or (& $given 'ReplaceToolchain'))) {
        Stop-Usage '-NoToolchain cannot be combined with -InstallToolchain or -ReplaceToolchain'
    }
    foreach ($name in 'FsUae', 'Kickstart', 'InstallFsUae') {
        if ((& $given $name) -and (& $given 'NoEmulator')) { Stop-Usage "-$name cannot be combined with -NoEmulator" }
        if ((& $given $name) -and (& $given 'NoToolchain')) { Stop-Usage "-$name requires the toolchain; remove -NoToolchain" }
    }
    if ((& $given 'VSCode') -and (& $given 'NoVSCode')) { Stop-Usage '-VSCode and -NoVSCode cannot be combined' }
    if ((& $given 'Starter') -and ((& $given 'NoStarter') -or (& $given 'NoToolchain'))) {
        Stop-Usage '-Starter cannot be combined with -NoStarter or -NoToolchain'
    }
    if ((& $given 'Archive') -ne (& $given 'Checksums')) { Stop-Usage '-Archive and -Checksums must be used together' }
}

# --- Main --------------------------------------------------------------------

# Runs the installer and returns its exit code: 0 on success, 1 for a
# failure, 2 for a usage error. Takes the same parameters as the script.
function Invoke-Installer {
    param(
        [string]$Prefix,
        [switch]$ModifyPath,
        [string]$Archive,
        [string]$Checksums,
        [string]$Toolchain,
        [switch]$NoToolchain,
        [switch]$InstallToolchain,
        [switch]$ReplaceToolchain,
        [string]$FsUae,
        [string]$Kickstart,
        [switch]$NoEmulator,
        [switch]$InstallFsUae,
        [switch]$NoHello,
        [string]$Starter,
        [switch]$NoStarter,
        [switch]$VSCode,
        [switch]$NoVSCode,
        [switch]$Uninstall,
        [switch]$Purge,
        [switch]$Help
    )
    $ErrorActionPreference = 'Stop'
    $options = @{}
    foreach ($name in $PSBoundParameters.Keys) { $options[$name] = $PSBoundParameters[$name] }
    try {
        if ($Help) { Write-Line (Get-UsageText); return 0 }
        Test-Options $options
        Invoke-Main $options | Out-Null
        return 0
    } catch {
        $code = $_.Exception.Data['ClorraineExitCode']
        $message = Get-SafeText $_.Exception.Message
        [Console]::Error.WriteLine("clorraine: $message")
        if ($code -eq 2) {
            [Console]::Error.WriteLine((Get-UsageText))
            return 2
        }
        if ($null -eq $code) { return 1 }
        return [int]$code
    }
}

# The install, uninstall, and first-run flow. Throws (Stop-Install) for every
# non-zero outcome.
function Invoke-Main([hashtable]$Options) {
    $opt = { param($name) if ($Options.ContainsKey($name)) { $Options[$name] } else { $null } }
    Initialize-Style
    $interactive = Test-Interactive

    if ((Get-HostArchitecture) -ne 'X64') { Stop-Install 1 'this installer supports 64-bit x86 Windows only' }
    if ((Get-OsBuild) -lt 22000) { Write-Warn 'Windows 10 is untested; continuing' }
    foreach ($tool in 'curl.exe', 'tar.exe') {
        if (-not (Test-Path -LiteralPath (Join-Path $env:SystemRoot "System32\$tool"))) {
            Stop-Install 1 "this installer needs $tool from Windows; it was not found in System32"
        }
    }

    $root = Resolve-Prefix $(if (& $opt 'Prefix') { & $opt 'Prefix' } else { Get-DefaultPrefix })
    $mutex = Lock-Installer
    try {
        Repair-Switch $root
        if (& $opt 'Uninstall') {
            Invoke-Uninstall $root ([bool](& $opt 'Purge'))
            return
        }
        if ($interactive) { Write-Step 1 'Install the Clorraine CLI' }
        $work = New-PrivateTemp 'install'
        try {
            if (& $opt 'Archive') {
                # Verify, then use the same private copy, so a user path is
                # never reopened after its digest was checked.
                Test-LocalArchive (& $opt 'Archive') (& $opt 'Checksums')
                $archivePath = Join-Path $work (Split-Path -Leaf (& $opt 'Archive'))
                Copy-Item -LiteralPath (& $opt 'Archive') -Destination $archivePath
                Test-LocalArchive $archivePath (& $opt 'Checksums')
            } else {
                if (-not (Test-Assembled)) {
                    Stop-Install 1 'this installer template is not assembled; use a released install.ps1 or pass -Archive and -Checksums'
                }
                $archivePath = Join-Path $work $script:ArchiveName
                Invoke-Download $script:ArchiveUrl $archivePath ([long]$script:ArchiveBytes) $script:ArchiveSha256
            }
            $folder = Install-Archive $root $archivePath
        } finally {
            Remove-TreeSafe $work
        }
        Write-Ok "installed Clorraine $script:Version in $folder"
        Update-UserPath $root ([bool](& $opt 'ModifyPath')) $interactive
        $examples = Install-Examples $root
        Write-Ok "examples in $($examples.Folder)"
    } finally {
        $mutex.ReleaseMutex()
        $mutex.Dispose()
    }
}

if ($env:CLORRAINE_INSTALL_LIB -ne '1') {
    exit (Invoke-Installer @PSBoundParameters)
}
