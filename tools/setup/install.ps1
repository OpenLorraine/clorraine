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
#
# The parameter block is an advanced one so an unknown or misspelled option is
# collected in $Rest and reported as a usage error, instead of being ignored.
[CmdletBinding(PositionalBinding = $false)]
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
    [switch]$Help,
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$Rest
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

# The installer's own options, for the rules that compare them.
$script:InstallOptions = @(
    'Prefix', 'ModifyPath', 'Archive', 'Checksums', 'Toolchain', 'NoToolchain', 'InstallToolchain',
    'ReplaceToolchain', 'FsUae', 'Kickstart', 'NoEmulator', 'InstallFsUae', 'NoHello', 'Starter',
    'NoStarter', 'VSCode', 'NoVSCode', 'Uninstall', 'Purge'
)

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

# Removes terminal-control characters (C0, DEL, and C1) from text that came
# from outside. Tabs and line breaks are kept unless -SingleLine is given,
# for text shown inside one line.
function Get-SafeText([string]$Text, [switch]$SingleLine) {
    if ($null -eq $Text) { return '' }
    $clean = $Text -replace '[\x00-\x08\x0B\x0C\x0E-\x1F\x7F-\x9F]', ''
    if ($SingleLine) { $clean = $clean -replace '[\t\r\n]', ' ' } else { $clean = $clean -replace '\r(?!\n)', '' }
    return $clean
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

# True in a 64-bit PowerShell; a 32-bit one cannot see the paths of 64-bit
# processes, which the purge in-use check needs.
function Test-64BitProcess { return [Environment]::Is64BitProcess }

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
function Invoke-Native([string]$File, [string[]]$Arguments = @(), [string]$WorkingDirectory = '', [hashtable]$Environment = @{}) {
    $info = New-Object System.Diagnostics.ProcessStartInfo
    foreach ($name in $Environment.Keys) { $info.EnvironmentVariables[$name] = [string]$Environment[$name] }
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
    New-PrivateFolder $path
    return $path
}

# Creates a folder with an access list of its own, in one step: full control
# for the current user, SYSTEM, and Administrators, nothing inherited. A shared
# TMP or a prefix such as C:\Clorraine would otherwise let another local user
# replace a verified file before it is used.
function New-PrivateFolder([string]$Path) {
    $security = New-Object System.Security.AccessControl.DirectorySecurity
    $security.SetAccessRuleProtection($true, $false)
    $inherit = [System.Security.AccessControl.InheritanceFlags]'ContainerInherit, ObjectInherit'
    $sids = @(
        [System.Security.Principal.WindowsIdentity]::GetCurrent().User,
        (New-Object System.Security.Principal.SecurityIdentifier 'S-1-5-18'),
        (New-Object System.Security.Principal.SecurityIdentifier 'S-1-5-32-544')
    )
    foreach ($sid in $sids) {
        $rule = New-Object System.Security.AccessControl.FileSystemAccessRule($sid, 'FullControl', $inherit, 'None', 'Allow')
        $security.AddAccessRule($rule)
    }
    [void][IO.Directory]::CreateDirectory($Path, $security)
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
# folder, no absolute, drive, or parent-relative names, only regular files and
# folders, no repeated name, and exactly the expected file set. Names are
# compared ordinally, so case and culture rules cannot make two names match.
# Returns the top folder name.
function Test-CandidateZip([string]$Path) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $top = "clorraine-$script:Version-x86_64-windows"
    $expected = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::Ordinal)
    foreach ($file in $script:CandidateFiles) { [void]$expected.Add($file) }
    $seen = New-Object 'System.Collections.Generic.HashSet[string]' ([StringComparer]::OrdinalIgnoreCase)
    $zip = [IO.Compression.ZipFile]::OpenRead($Path)
    try {
        foreach ($entry in $zip.Entries) {
            $shown = Get-SafeText $entry.FullName
            $name = $entry.FullName.Replace('\', '/')
            $parts = $name.Split('/')
            if ($name.StartsWith('/') -or $name.Contains(':') -or ($parts -contains '..') -or ($parts -contains '.')) {
                Stop-Install 1 "archive contains an unsafe entry: $shown"
            }
            if (-not [string]::Equals($parts[0], $top, [StringComparison]::Ordinal)) {
                Stop-Install 1 "archive entry outside ${top}: $shown"
            }
            # The Unix file type lives in the high half of the external
            # attributes; only regular files and folders (or none) are allowed.
            $type = ($entry.ExternalAttributes -shr 16) -band 0xF000
            if ($type -ne 0 -and $type -ne 0x8000 -and $type -ne 0x4000) {
                Stop-Install 1 "archive contains a link or special file: $shown"
            }
            if (-not $seen.Add($name.TrimEnd('/'))) { Stop-Install 1 "archive repeats an entry: $shown" }
            if ($name.EndsWith('/')) { continue }
            if (-not $expected.Contains($name.Substring($top.Length + 1))) {
                Stop-Install 1 "archive has an unexpected file: $shown"
            }
        }
    } finally {
        $zip.Dispose()
    }
    foreach ($file in $script:CandidateFiles) {
        if (-not $seen.Contains("$top/$file")) { Stop-Install 1 "archive is missing $file" }
    }
    return $top
}

# Checks what tar.exe actually wrote, before anything in it runs: one top
# folder holding exactly the expected files as regular files, with no link
# anywhere. The extractor may read names from a different part of the zip
# than the check above, so the result is checked as well.
function Test-ExtractedTree([string]$Staging, [string]$Top) {
    $children = @(Get-ChildItem -LiteralPath $Staging -Force)
    if ($children.Count -ne 1 -or -not [string]::Equals($children[0].Name, $Top, [StringComparison]::Ordinal) -or
        -not $children[0].PSIsContainer -or
        ($children[0].Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
        Stop-Install 1 'the extracted archive does not hold exactly one top folder'
    }
    $tree = Join-Path $Staging $Top
    $found = New-Object 'System.Collections.Generic.List[string]'
    $pending = New-Object 'System.Collections.Generic.Stack[string]'
    $pending.Push($tree)
    while ($pending.Count -gt 0) {
        $folder = $pending.Pop()
        foreach ($item in @(Get-ChildItem -LiteralPath $folder -Force)) {
            if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0) {
                Stop-Install 1 ('the extracted archive contains a link: ' + (Get-SafeText $item.Name))
            }
            if ($item.PSIsContainer) { $pending.Push($item.FullName); continue }
            $found.Add($item.FullName.Substring($tree.Length + 1).Replace('\', '/'))
        }
    }
    $sorted = $found.ToArray()
    [Array]::Sort($sorted, [StringComparer]::Ordinal)
    $wanted = [string[]]$script:CandidateFiles.Clone()
    [Array]::Sort($wanted, [StringComparer]::Ordinal)
    if (-not [string]::Equals(($sorted -join '|'), ($wanted -join '|'), [StringComparison]::Ordinal)) {
        Stop-Install 1 'the extracted archive does not hold exactly the expected files'
    }
    return $tree
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
    # A percent sign would make the expandable PATH entry name another folder.
    if ($Path -match '[;"%]' -or $Path -match '[\x00-\x1F\x7F]') {
        Stop-Install 1 '-Prefix must not contain a semicolon, a double quote, a percent sign, or control characters'
    }
    $full = [IO.Path]::GetFullPath($Path).TrimEnd('\')
    if (-not (Test-Path -LiteralPath $full)) {
        $parent = Split-Path -Parent $full
        if ($parent -and -not (Test-Path -LiteralPath $parent)) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
        New-PrivateFolder $full
    }
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
# Returns the version folder. The cleanup runs in `finally`, so it also runs
# when the person presses Ctrl+C: unless the run completed, a version folder
# this run created is removed and the previous `current` is restored.
function Install-Archive([string]$Root, [string]$ArchivePath) {
    $versions = Join-Path $Root 'versions'
    New-Item -ItemType Directory -Force -Path $versions | Out-Null
    $top = Test-CandidateZip $ArchivePath
    $folder = Join-Path $versions ("$script:Version-" + (Get-Sha256 $ArchivePath).Substring(0, 12))
    $staging = Join-Path $versions ('.tmp-' + [guid]::NewGuid().ToString('N').Substring(0, 12))
    $current = Join-Path $Root 'current'
    $pending = Join-Path $Root 'current.new'
    $previous = Get-JunctionTarget $current
    $created = $false
    $switching = $false
    $completed = $false
    try {
        New-Item -ItemType Directory -Path $staging | Out-Null
        $tar = Join-Path $env:SystemRoot 'System32\tar.exe'
        $result = Invoke-Native $tar @('-x', '-f', $ArchivePath, '-C', $staging)
        if ($result.Code -ne 0) { Stop-Install 1 ('cannot extract the archive: ' + (Get-SafeText $result.Err).Trim()) }
        $tree = Test-ExtractedTree $staging $top
        Test-ExtractedVersion $tree
        if (Test-Path -LiteralPath $folder) {
            Test-ExtractedVersion $folder
        } else {
            [IO.Directory]::Move($tree, $folder)
            $created = $true
        }
        # A `current` that is not ours is refused before the switch starts, so
        # the cleanup below never touches it.
        if ($null -ne (Get-Item -LiteralPath $current -Force -ErrorAction SilentlyContinue) -and
            -not (Test-OwnedLink $current $versions)) {
            Stop-Install 1 "'$current' exists and is not a Clorraine link; remove it and retry"
        }
        $switching = $true
        Set-CurrentVersion $Root $folder
        $completed = $true
    } finally {
        if (-not $completed) {
            if ($switching) {
                if ($null -ne (Get-Item -LiteralPath $pending -Force -ErrorAction SilentlyContinue)) { Remove-Link $pending }
                $now = Get-JunctionTarget $current
                $restore = -not ($previous -and $now -and (Test-SamePath $now $previous))
                $present = $null -ne (Get-Item -LiteralPath $current -Force -ErrorAction SilentlyContinue)
                # Only a missing `current` or one of our own links is replaced.
                if ($restore -and (-not $present -or (Test-OwnedLink $current $versions))) {
                    if ($null -ne (Get-Item -LiteralPath $current -Force -ErrorAction SilentlyContinue)) { Remove-Link $current }
                    if ($previous) { New-Item -ItemType Junction -Path $current -Target $previous | Out-Null }
                }
            }
            if ($created -and (Test-Path -LiteralPath $folder)) { Remove-TreeSafe $folder }
        }
        if (Test-Path -LiteralPath $staging) { Remove-TreeSafe $staging }
    }
    return $folder
}

# Adds `current\bin` to the user PATH when asked (or when the person agrees),
# or prints how to do it. Returns True when the entry is on the user PATH.
function Update-UserPath([string]$Root, [bool]$Requested, [bool]$Interactive) {
    $bin = Join-Path $Root 'current\bin'
    $add = $Requested
    if (-not $Requested -and $Interactive) {
        $add = Read-YesNo "  Add $bin to your user PATH? [Y/n] " $true
    }
    if ($add) {
        Add-UserPathEntry $bin
        Write-Ok "added $bin to your user PATH (new terminals see it)"
        return $true
    }
    $present = @((Get-UserPath).Value.Split(';') | Where-Object { Test-PathEntry $_ $bin }).Count -gt 0
    if (-not $present) {
        Write-Line "  to run clorraine from any terminal, add $bin to your user PATH or rerun with -ModifyPath"
    }
    return $present
}

# --- Uninstall ---------------------------------------------------------------

# Removes a folder only when it is an ordinary, empty folder.
function Remove-EmptyFolder([string]$Path) {
    if ((Test-Path -LiteralPath $Path -PathType Container) -and -not (Test-ReparsePoint $Path) -and
        @(Get-ChildItem -LiteralPath $Path -Force).Count -eq 0) {
        [IO.Directory]::Delete($Path, $false)
    }
}

# Removes what this installer created under Root and nothing else: version
# folders and staging folders by their exact name pattern, the two hello
# files, and the FS-UAE it downloaded into fs-uae\3.2.35. The folders that
# held them go only when they end up empty, so a prefix that also holds the
# user's own files keeps them. Nothing is reached through a link.
function Remove-InstallerData([string]$Root) {
    $versions = Join-Path $Root 'versions'
    if ((Test-Path -LiteralPath $versions -PathType Container) -and -not (Test-ReparsePoint $versions)) {
        foreach ($child in @(Get-ChildItem -LiteralPath $versions -Force)) {
            if ($child.Name -cmatch '^[0-9A-Za-z.+-]+-[0-9a-f]{12}$' -or $child.Name -cmatch '^\.tmp-[0-9a-f]{12}$') {
                Remove-TreeSafe $child.FullName
            }
        }
        Remove-EmptyFolder $versions
    }
    $examples = Join-Path $Root 'examples'
    if ((Test-Path -LiteralPath $examples -PathType Container) -and -not (Test-ReparsePoint $examples)) {
        foreach ($name in 'hello.clo', 'hello-amiga.clo') {
            $file = Join-Path $examples $name
            if ((Test-Path -LiteralPath $file -PathType Leaf) -and -not (Test-ReparsePoint $file)) { [IO.File]::Delete($file) }
        }
        Remove-EmptyFolder $examples
    }
    $fsuae = Join-Path $Root 'fs-uae'
    $downloaded = Join-Path $fsuae '3.2.35'
    if ((Test-Path -LiteralPath $fsuae -PathType Container) -and -not (Test-ReparsePoint $fsuae)) {
        if ($null -ne (Get-Item -LiteralPath $downloaded -Force -ErrorAction SilentlyContinue)) { Remove-TreeSafe $downloaded }
        Remove-EmptyFolder $fsuae
    }
}

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
        Remove-InstallerData $Root
        $configDir = Join-Path $env:APPDATA 'clorraine'
        $config = Join-Path $configDir 'config'
        # Only a plain file named config is ours; a folder or link is left.
        if ((Test-Path -LiteralPath $config -PathType Leaf) -and -not (Test-ReparsePoint $config)) {
            [IO.File]::Delete($config)
        }
        Remove-EmptyFolder $configDir
        Write-Ok "removed the Clorraine versions, examples, and $config"
    }
    Write-Line '  The rc24 toolchain and any FS-UAE you installed yourself are kept.'
    Write-Line '  Remove rc24 under Settings > Apps > Installed apps (m68k-amigaos-gcc).'
}

# --- The installed CLI -------------------------------------------------------

# The installation folder of this run, set once the prefix is resolved.
$script:InstallRoot = ''

# Runs the installed clorraine.exe through the stable `current` link and
# returns its exit code and output.
function Invoke-Clorraine([string[]]$Arguments, [string]$WorkingDirectory = '') {
    $clorraine = Join-Path $script:InstallRoot 'current\bin\clorraine.exe'
    return Invoke-Native $clorraine $Arguments $WorkingDirectory
}

# Prints what a person can still do by hand after a step was skipped or failed.
function Write-NextSteps([string]$Root, [bool]$NeedToolchain) {
    $cli = Join-Path $Root 'current\bin\clorraine.exe'
    Write-Line 'Next steps:'
    if ($NeedToolchain) {
        Write-Line '  rerun this installer with -Toolchain <rc24 root> (or -InstallToolchain), or run:'
        Write-Line "  $cli setup --toolchain <rc24 root>"
    }
    Write-Line "  $cli run $(Join-Path $Root 'examples\hello.clo')"
    Write-Line "  $cli run -p hosted-amiga --release --emulator --fs-uae <fs-uae.exe> --kickstart <ks31-a1200.rom> $(Join-Path $Root 'examples\hello-amiga.clo')"
}

# --- rc24 toolchain ----------------------------------------------------------

$script:Rc24Url = 'https://github.com/AmigaPorts/m68k-amigaos-gcc/releases/download/v16.2-rc24/m68k-amigaos-gcc-16.2-rc24-x86_64-w64-mingw32ucrt-setup.exe'
$script:Rc24Bytes = 48065003
$script:Rc24Sha256 = '9dd6973539bb2d514fc1b1e6c0a107cac6f30a9c4076cf4014cbe13013c95512'
$script:Rc24Version = '16.2-rc24'

# The registry key the upstream setup writes (one per machine, any version),
# and the hive it is in; tests point both at a scratch key.
$script:Rc24UninstallKey = 'SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{8D184346-D038-45E9-98A1-43BFC542BE34}_is1'
$script:Rc24RegistryHive = 'LocalMachine'

# The per-machine folder the elevated step installs into. Program Files comes
# from the system's known-folder setting, never from the ProgramFiles
# environment variable, which the standard user controls.
function Get-Rc24AdminRoot {
    return Join-Path ([Environment]::GetFolderPath('ProgramFiles')) 'Clorraine'
}

# The installed amiga-gcc registration (DisplayVersion, InstallLocation), or
# $null when none is registered. Read from the 64-bit registry view, where
# the upstream setup registers.
function Get-Rc24Registration {
    $base = [Microsoft.Win32.RegistryKey]::OpenBaseKey($script:Rc24RegistryHive, 'Registry64')
    try {
        $key = $base.OpenSubKey($script:Rc24UninstallKey)
        if ($null -eq $key) { return $null }
        try {
            return [pscustomobject]@{
                DisplayVersion = [string]$key.GetValue('DisplayVersion')
                InstallLocation = [string]$key.GetValue('InstallLocation')
            }
        } finally { $key.Dispose() }
    } finally { $base.Dispose() }
}

# Exit codes of the elevated step that are its own; the upstream setup uses
# 0 to 8, so these never collide with its codes.
$script:ElevatedMismatch = 101
$script:ElevatedStagingTaken = 102
$script:ElevatedUntrustedRoot = 103
$script:ElevatedError = 104

# The elevated half of the rc24 install. It runs as an administrator, so it
# trusts nothing the standard user could have changed. Without -AdminRoot it
# works out Program Files\Clorraine itself from the system's known-folder
# setting, refuses that folder when it is a link or owned by anyone but
# Administrators, SYSTEM, or TrustedInstaller, and creates a new staging
# folder there that only Administrators and SYSTEM can open. It copies the
# setup in, checks the copy's size and SHA-256, and only then runs it silently,
# logging into the staging folder. The log is kept as rc24-setup.log next to
# the toolchain and the staging folder is removed. Returns the setup's exit
# code, or 101 (the copy does not match), 102 (the staging name is taken), 103
# (untrusted folder), or 104 (any other error). An existing admin folder must
# be a real folder owned by Administrators, SYSTEM, or TrustedInstaller, and
# no other account may hold a right to change it. The owner rule does not
# trust the elevated user's own account: under UAC its unelevated twin could
# rewrite the access list of a folder that account owns.
#
# Tests run without elevation: -AdminRoot and -Target move the folders, and
# the trusted accounts then also include the current user unless -TrustedSid
# names them. The checks run either way. This function is self-contained:
# Invoke-ElevatedSetup sends its text to the elevated PowerShell.
function Invoke-ElevatedStaging([string]$Source, [long]$Bytes, [string]$Sha256, [string]$AdminRoot = '',
        [string]$Target = '', [string[]]$TrustedSid = $null) {
    $ErrorActionPreference = 'Stop'
    $trusted = @('S-1-5-32-544', 'S-1-5-18', 'S-1-5-80-956008885-3418522649-1831038044-1853292631-2271478464')
    if ($AdminRoot) {
        if ($null -eq $TrustedSid) { $TrustedSid = @([Security.Principal.WindowsIdentity]::GetCurrent().User.Value) }
    } else {
        $AdminRoot = Join-Path ([Environment]::GetFolderPath('ProgramFiles')) 'Clorraine'
        $TrustedSid = @()
    }
    $trusted += $TrustedSid
    if (-not $Target) { $Target = Join-Path $AdminRoot 'amiga-gcc-16.2-rc24' }
    try {
        $existing = Get-Item -LiteralPath $AdminRoot -Force -ErrorAction SilentlyContinue
        if ($null -ne $existing) {
            if (($existing.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0 -or -not $existing.PSIsContainer) { return 103 }
            $sidType = [System.Security.Principal.SecurityIdentifier]
            $acl = Get-Acl -LiteralPath $AdminRoot
            if ($trusted -notcontains $acl.GetOwner($sidType).Value) { return 103 }
            # Write data, append, write attributes and extended attributes,
            # delete children, delete, change the access list or the owner,
            # and the generic write and all rights.
            $change = 0x2 -bor 0x4 -bor 0x10 -bor 0x40 -bor 0x100 -bor 0x10000 -bor 0x40000 -bor 0x80000 -bor 0x10000000 -bor 0x40000000
            $mayChange = $trusted + @('S-1-3-0')
            foreach ($rule in $acl.GetAccessRules($true, $true, $sidType)) {
                if ($rule.AccessControlType -ne 'Allow') { continue }
                if (([int64]$rule.FileSystemRights -band $change) -eq 0) { continue }
                if ($mayChange -notcontains $rule.IdentityReference.Value) { return 103 }
            }
        } else {
            New-Item -ItemType Directory -Path $AdminRoot | Out-Null
        }
        $staging = Join-Path $AdminRoot ('.staging-' + [guid]::NewGuid().ToString('N').Substring(0, 12))
        if (Test-Path -LiteralPath $staging) { return 102 }
        $security = New-Object System.Security.AccessControl.DirectorySecurity
        $security.SetAccessRuleProtection($true, $false)
        $inherit = [System.Security.AccessControl.InheritanceFlags]'ContainerInherit, ObjectInherit'
        foreach ($sid in @('S-1-5-32-544', 'S-1-5-18') + $TrustedSid) {
            $principal = New-Object System.Security.Principal.SecurityIdentifier $sid
            $security.AddAccessRule((New-Object System.Security.AccessControl.FileSystemAccessRule($principal, 'FullControl', $inherit, 'None', 'Allow')))
        }
        [void][IO.Directory]::CreateDirectory($staging, $security)
    } catch {
        return 104
    }
    try {
        $setup = Join-Path $staging 'setup.exe'
        Copy-Item -LiteralPath $Source -Destination $setup
        $hash = (Get-FileHash -Algorithm SHA256 -LiteralPath $setup).Hash.ToLowerInvariant()
        if ((Get-Item -LiteralPath $setup).Length -ne $Bytes -or $hash -ne $Sha256.ToLowerInvariant()) { return 101 }
        $log = Join-Path $staging 'setup.log'
        $arguments = '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /NOCLOSEAPPLICATIONS /SP- /DIR="' + $Target + '" /LOG="' + $log + '"'
        $process = Start-Process -FilePath $setup -ArgumentList $arguments -Wait -PassThru
        # Once the setup has run, its own exit code is the answer; a log that
        # cannot be kept does not turn it into a failure before the run.
        try {
            if (Test-Path -LiteralPath $log) { Copy-Item -LiteralPath $log -Destination (Join-Path $AdminRoot 'rc24-setup.log') -Force }
        } catch { }
        return $process.ExitCode
    } catch {
        return 104
    } finally {
        try { [IO.Directory]::Delete($staging, $true) } catch { }
    }
}

# Quotes text as a PowerShell single-quoted literal, including the typographic
# single quotes PowerShell also accepts as delimiters.
function ConvertTo-SingleQuoted([string]$Text) {
    return "'" + [System.Management.Automation.Language.CodeGeneration]::EscapeSingleQuotedStringContent($Text) + "'"
}

# The PowerShell text the elevated process runs: the staging function and one
# call with the source path, size, and SHA-256 as literals. The target folder
# is not passed: the elevated side works it out. -AdminRoot is for tests that
# run the text without elevation.
function Get-ElevatedCommand([string]$Source, [long]$Bytes, [string]$Sha256, [string]$AdminRoot = '',
        [string[]]$TrustedSid = $null) {
    $call = 'exit (Invoke-ElevatedStaging -Source ' + (ConvertTo-SingleQuoted $Source) + " -Bytes $Bytes -Sha256 " +
        (ConvertTo-SingleQuoted $Sha256)
    if ($AdminRoot) { $call += ' -AdminRoot ' + (ConvertTo-SingleQuoted $AdminRoot) }
    if ($null -ne $TrustedSid) { $call += ' -TrustedSid @(' + (($TrustedSid | ForEach-Object { ConvertTo-SingleQuoted $_ }) -join ', ') + ')' }
    return 'function Invoke-ElevatedStaging {' + ${function:Invoke-ElevatedStaging}.ToString() + "}`n" + $call + ')'
}

# Runs Invoke-ElevatedStaging in an elevated PowerShell behind one UAC prompt
# and returns its exit code. Only the function's own text and three values
# cross the boundary; Target is where the elevated side installs and is shown
# in messages. The process is started directly, so a declined prompt surfaces
# as the Win32 error 1223 inside the method's exception.
function Invoke-ElevatedSetup([string]$Source, [string]$Target, [long]$Bytes, [string]$Sha256) {
    $text = Get-ElevatedCommand $Source $Bytes $Sha256
    $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($text))
    $info = New-Object System.Diagnostics.ProcessStartInfo
    $info.FileName = Join-Path ([Environment]::SystemDirectory) 'WindowsPowerShell\v1.0\powershell.exe'
    $info.Arguments = "-NoProfile -ExecutionPolicy Bypass -EncodedCommand $encoded"
    $info.UseShellExecute = $true
    $info.Verb = 'runas'
    $process = [System.Diagnostics.Process]::Start($info)
    $process.WaitForExit()
    return $process.ExitCode
}

# True when an error (or one it wraps) is a declined UAC prompt.
function Test-DeclinedElevation($ErrorRecord) {
    $exception = $ErrorRecord.Exception
    while ($null -ne $exception) {
        if ($exception -is [System.ComponentModel.Win32Exception] -and $exception.NativeErrorCode -eq 1223) { return $true }
        $exception = $exception.InnerException
    }
    return $false
}

# Downloads, verifies, and installs the pinned rc24 behind one UAC prompt.
# Returns the toolchain root.
function Install-Rc24([string]$Root) {
    $work = New-PrivateTemp 'rc24'
    try {
        $setup = Join-Path $work 'amiga-gcc-16.2-rc24-setup.exe'
        Write-Line '  Downloading the rc24 toolchain setup from AmigaPorts (48 MB)...'
        Invoke-Download $script:Rc24Url $setup $script:Rc24Bytes $script:Rc24Sha256 1200
        $target = Join-Path (Get-Rc24AdminRoot) 'amiga-gcc-16.2-rc24'
        $log = Join-Path (Get-Rc24AdminRoot) 'rc24-setup.log'
        $logBefore = if (Test-Path -LiteralPath $log) { (Get-Item -LiteralPath $log).LastWriteTimeUtc } else { $null }
        Write-Line '  Windows asks for administrator approval to install it for all users.'
        try {
            $code = Invoke-ElevatedSetup $setup $target $script:Rc24Bytes $script:Rc24Sha256
        } catch {
            if (Test-DeclinedElevation $_) {
                Write-Warn 'the administrator approval was declined; the CLI stays installed'
                Write-NextSteps $Root $true
                Stop-Install 1 'the rc24 toolchain was not installed'
            }
            throw
        }
        if ($code -ne 0) {
            $reason = switch ($code) {
                $script:ElevatedMismatch { 'the verified setup changed before it could run; nothing was installed' }
                $script:ElevatedStagingTaken { 'the staging folder name was already taken; nothing was installed' }
                $script:ElevatedUntrustedRoot {
                    $admin = Get-Rc24AdminRoot
                    "$admin is a link, is not owned by Administrators, or lets other accounts change it; nothing was installed. " +
                        "If an administrator created it, run in an elevated prompt: icacls `"$admin`" /setowner *S-1-5-32-544"
                }
                $script:ElevatedError { 'the elevated step failed before the setup could run' }
                default {
                    $fresh = (Test-Path -LiteralPath $log) -and ($null -eq $logBefore -or (Get-Item -LiteralPath $log).LastWriteTimeUtc -ne $logBefore)
                    if ($fresh) { "the rc24 setup failed (exit $code); see $log" } else { "the rc24 setup failed (exit $code)" }
                }
            }
            Write-Warn $reason
            Write-NextSteps $Root $true
            Stop-Install 1 'the rc24 toolchain was not installed'
        }
        Write-Ok "installed rc24 in $target"
        return $target
    } finally {
        Remove-TreeSafe $work
    }
}

# Picks the toolchain root: -Toolchain, an existing rc24 registration, or a
# new install with consent. Returns $null when the step is skipped.
function Select-Toolchain([string]$Root, [hashtable]$Options, [bool]$Interactive) {
    if ($Options['Toolchain']) { return $Options['Toolchain'] }
    $registration = Get-Rc24Registration
    if ($null -ne $registration -and $registration.DisplayVersion -eq $script:Rc24Version -and $registration.InstallLocation) {
        $existing = $registration.InstallLocation.TrimEnd('\')
        Write-Ok ("using the installed rc24 toolchain in " + (Get-SafeText $existing -SingleLine))
        return $existing
    }
    if ($null -ne $registration) {
        $other = Get-SafeText $registration.DisplayVersion
        $replace = [bool]$Options['ReplaceToolchain']
        if (-not $replace -and $Interactive) {
            $replace = Read-YesNo "  amiga-gcc $other is installed; replace it with $script:Rc24Version? [y/N] " $false
        } elseif (-not $replace) {
            Stop-Install 1 "amiga-gcc $other is installed, but Clorraine needs $script:Rc24Version; rerun with -ReplaceToolchain to replace it, or pass -Toolchain <rc24 root>"
        }
        if (-not $replace) {
            Write-Warn "kept amiga-gcc $other; the toolchain is not configured"
            Write-NextSteps $Root $true
            return $null
        }
        return Install-Rc24 $Root
    }
    $install = [bool]$Options['InstallToolchain']
    if (-not $install -and $Interactive) {
        $install = Read-YesNo "  Install the rc24 toolchain from AmigaPorts (48 MB, one UAC prompt)? [Y/n] " $true
    }
    if (-not $install) {
        Write-Line '  no toolchain selected; rerun with -InstallToolchain, or with -Toolchain <rc24 root> for an existing one'
        return $null
    }
    return Install-Rc24 $Root
}

# --- FS-UAE and the Kickstart ROM ---------------------------------------------

$script:FsUaeUrl = 'https://github.com/FrodeSolheim/fs-uae/releases/download/v3.2.35/FS-UAE_3.2.35_Windows_x86-64.zip'
$script:FsUaeBytes = 49123213
$script:FsUaeSha256 = 'b402e2802d518a085c878be96c5c0a656fa3dec57dcf0019d226f3285069063e'

# The executable inside an FS-UAE Windows folder.
$script:FsUaeExe = 'FS-UAE\Windows\x86-64\fs-uae.exe'

# One value remembered in the CLI config (`key=value` lines), or ''.
function Get-ConfigValue([string]$Key) {
    $config = Join-Path $env:APPDATA 'clorraine\config'
    if (-not (Test-Path -LiteralPath $config -PathType Leaf)) { return '' }
    foreach ($line in Get-Content -LiteralPath $config -Encoding UTF8) {
        if ($line.StartsWith("$Key=")) { return $line.Substring($Key.Length + 1) }
    }
    return ''
}

# Finds FS-UAE: -FsUae, CLORRAINE_FS_UAE, the CLI config, fs-uae.exe on PATH,
# the copy this installer downloaded, then Program Files. Returns the path or
# $null; a named path that does not exist is reported and not replaced.
function Find-FsUae([string]$Explicit, [string]$Root) {
    foreach ($named in $Explicit, $env:CLORRAINE_FS_UAE) {
        if (-not $named) { continue }
        if (Test-Path -LiteralPath $named -PathType Leaf) { return $named }
        Write-Warn "FS-UAE is not a file: $(Get-SafeText $named)"
        return $null
    }
    $remembered = Get-ConfigValue 'fs-uae'
    if ($remembered -and (Test-Path -LiteralPath $remembered -PathType Leaf)) { return $remembered }
    foreach ($dir in ($env:PATH -split ';')) {
        if (-not $dir -or -not (Test-DrivePath $dir)) { continue }
        $candidate = Join-Path $dir 'fs-uae.exe'
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
    }
    $downloaded = Join-Path $Root ('fs-uae\3.2.35\' + $script:FsUaeExe)
    if (Test-Path -LiteralPath $downloaded -PathType Leaf) { return $downloaded }
    if ($env:ProgramFiles) {
        $programs = Join-Path $env:ProgramFiles $script:FsUaeExe
        if (Test-Path -LiteralPath $programs -PathType Leaf) { return $programs }
    }
    return $null
}

# The user's Documents folder, which OneDrive may have moved away from
# %USERPROFILE%\Documents.
function Get-DocumentsFolder { return [Environment]::GetFolderPath('MyDocuments') }

# Finds a Kickstart 3.1 A1200 ROM: -Kickstart, CLORRAINE_KICKSTART, the CLI
# config, then the usual folders, by the usual names first and then by name
# pattern. A found ROM must have the size of a 512 KB image; a named one is
# left to `clorraine setup` to validate. Returns the path or $null.
function Find-Kickstart([string]$Explicit) {
    foreach ($named in $Explicit, $env:CLORRAINE_KICKSTART, (Get-ConfigValue 'kickstart')) {
        if ($named) { return $named }
    }
    $folders = @(
        (Join-Path $env:USERPROFILE 'amiga\roms'),
        (Join-Path $env:USERPROFILE 'Documents\FS-UAE\Kickstarts'),
        (Join-Path (Get-DocumentsFolder) 'FS-UAE\Kickstarts')
    )
    if ($env:PUBLIC) { $folders += Join-Path $env:PUBLIC 'Documents\Amiga Files\Shared\rom' }
    $sizes = 524288, 524299
    foreach ($folder in $folders) {
        if (-not (Test-Path -LiteralPath $folder -PathType Container)) { continue }
        foreach ($name in 'amiga-os-310-a1200.rom', 'kick31a1200.rom', 'kick31.rom') {
            $file = Get-Item -LiteralPath (Join-Path $folder $name) -ErrorAction SilentlyContinue
            if ($null -ne $file -and -not $file.PSIsContainer -and $sizes -contains $file.Length) { return $file.FullName }
        }
    }
    foreach ($folder in $folders) {
        if (-not (Test-Path -LiteralPath $folder -PathType Container)) { continue }
        foreach ($pattern in '*a1200*.rom', 'kick31*.rom') {
            foreach ($file in @(Get-ChildItem -LiteralPath $folder -Filter $pattern -File -ErrorAction SilentlyContinue)) {
                if ($sizes -contains $file.Length) { return $file.FullName }
            }
        }
    }
    return $null
}

# Downloads the pinned FS-UAE 3.2.35 zip, checks its entries, and unpacks it
# into Root\fs-uae\3.2.35. Returns the executable path.
function Install-FsUae([string]$Root) {
    $work = New-PrivateTemp 'fs-uae'
    $parent = Join-Path $Root 'fs-uae'
    $target = Join-Path $parent '3.2.35'
    $staging = Join-Path $parent ('.tmp-' + [guid]::NewGuid().ToString('N').Substring(0, 12))
    try {
        $zipPath = Join-Path $work 'FS-UAE_3.2.35_Windows_x86-64.zip'
        Write-Line '  Downloading FS-UAE 3.2.35 (49 MB)...'
        Invoke-Download $script:FsUaeUrl $zipPath $script:FsUaeBytes $script:FsUaeSha256 600
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $zip = [IO.Compression.ZipFile]::OpenRead($zipPath)
        try {
            foreach ($entry in $zip.Entries) {
                $name = $entry.FullName.Replace('\', '/')
                $parts = $name.Split('/')
                $type = ($entry.ExternalAttributes -shr 16) -band 0xF000
                if ($name.StartsWith('/') -or $name.Contains(':') -or ($parts -contains '..') -or
                    ($type -ne 0 -and $type -ne 0x8000 -and $type -ne 0x4000)) {
                    Stop-Install 1 ('the FS-UAE archive contains an unsafe entry: ' + (Get-SafeText $entry.FullName))
                }
            }
        } finally { $zip.Dispose() }
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
        if (Test-Path -LiteralPath $target) { Remove-TreeSafe $target }
        New-Item -ItemType Directory -Path $staging | Out-Null
        $tar = Join-Path $env:SystemRoot 'System32\tar.exe'
        $result = Invoke-Native $tar @('-x', '-f', $zipPath, '-C', $staging)
        if ($result.Code -ne 0) { Stop-Install 1 ('cannot extract FS-UAE: ' + (Get-SafeText $result.Err).Trim()) }
        [IO.Directory]::Move($staging, $target)
        $exe = Join-Path $target $script:FsUaeExe
        if (-not (Test-Path -LiteralPath $exe -PathType Leaf)) { Stop-Install 1 "the FS-UAE archive has no $script:FsUaeExe" }
        Write-Ok "installed FS-UAE 3.2.35 in $target"
        return $exe
    } finally {
        if (Test-Path -LiteralPath $staging) { Remove-TreeSafe $staging }
        Remove-TreeSafe $work
    }
}

# Picks FS-UAE and a ROM for the Amiga hello. Returns both (either may be
# $null when that part is skipped).
function Select-Emulator([string]$Root, [hashtable]$Options, [bool]$Interactive) {
    $selection = [pscustomobject]@{ FsUae = $null; Kickstart = $null }
    if ($Options['NoEmulator']) { return $selection }
    $fsuae = Find-FsUae ([string]$Options['FsUae']) $Root
    if (-not $fsuae -and -not $Options['FsUae'] -and -not $env:CLORRAINE_FS_UAE) {
        $install = [bool]$Options['InstallFsUae']
        if (-not $install -and $Interactive) {
            $install = Read-YesNo '  FS-UAE was not found. Download FS-UAE 3.2.35 (49 MB)? [Y/n] ' $true
        }
        if ($install) {
            $fsuae = Install-FsUae $Root
        } else {
            Write-Warn 'FS-UAE was not found; the Amiga run is skipped (rerun with -InstallFsUae, or pass -FsUae)'
            return $selection
        }
    }
    if (-not $fsuae) { return $selection }
    $selection.FsUae = [IO.Path]::GetFullPath($fsuae)
    Write-Ok ('FS-UAE: ' + (Get-SafeText $selection.FsUae -SingleLine))
    $rom = Find-Kickstart ([string]$Options['Kickstart'])
    if ($Interactive) {
        if ($rom) {
            [Console]::Out.Write('  Kickstart 3.1 A1200 ROM [' + (Get-SafeText $rom -SingleLine) + '] (Enter to use, - to skip): ')
            $answer = [Console]::In.ReadLine()
            if ($answer -eq '-') { $rom = $null } elseif ($answer) { $rom = $answer }
        } else {
            [Console]::Out.Write('  Kickstart 3.1 A1200 ROM path (leave empty to skip the Amiga run): ')
            $rom = [Console]::In.ReadLine()
        }
        # Explorer's "Copy as path" wraps the path in double quotes.
        if ($rom) { $rom = $rom.Trim().Trim('"') }
    } else {
        # As on Linux, a run without prompts uses an emulator only when the
        # person named both the emulator and the ROM (by flag or environment)
        # or asked for the FS-UAE download; nothing found by searching alone.
        $namedFsUae = $Options['FsUae'] -or $env:CLORRAINE_FS_UAE -or $Options['InstallFsUae']
        $namedRom = $Options['Kickstart'] -or $env:CLORRAINE_KICKSTART
        if (-not ($namedFsUae -and $namedRom)) {
            if ($namedFsUae) {
                Write-Line '  pass -Kickstart (or set CLORRAINE_KICKSTART) to run the Amiga hello'
            } else {
                Write-Line '  pass -FsUae and -Kickstart (or set CLORRAINE_FS_UAE and CLORRAINE_KICKSTART) to run the Amiga hello'
            }
            return [pscustomobject]@{ FsUae = $null; Kickstart = $null }
        }
    }
    if (-not $rom) {
        Write-Warn 'no Kickstart ROM selected; the Amiga run is skipped'
        return $selection
    }
    $selection.Kickstart = [IO.Path]::GetFullPath($rom)
    return $selection
}

# Runs `clorraine setup` with the toolchain and any selected emulator paths.
# A rejected emulator path is reported and setup runs again without it, so the
# toolchain is still configured. Returns the selection actually remembered.
function Set-Toolchain([string]$Toolchain, $Emulator) {
    $arguments = @('setup', '--toolchain', $Toolchain)
    if ($Emulator.FsUae) { $arguments += '--fs-uae', $Emulator.FsUae }
    if ($Emulator.Kickstart) { $arguments += '--kickstart', $Emulator.Kickstart }
    $result = Invoke-Clorraine $arguments
    if ($result.Code -ne 0 -and ($Emulator.FsUae -or $Emulator.Kickstart) -and
        $result.Err -match 'clorraine: --(fs-uae|kickstart):') {
        Write-Warn ('the emulator paths were not accepted: ' + (Get-SafeText $result.Err).Trim())
        $Emulator = [pscustomobject]@{ FsUae = $null; Kickstart = $null }
        $result = Invoke-Clorraine @('setup', '--toolchain', $Toolchain)
    }
    if ($result.Code -ne 0) {
        Write-Warn ('clorraine setup failed: ' + (Get-SafeText $result.Err).Trim())
        return $null
    }
    foreach ($line in ((Get-SafeText $result.Out) -split "`r?`n")) { if ($line) { Write-Ok $line } }
    return $Emulator
}

# --- Hellos --------------------------------------------------------------------

# The host C compiler the driver would use: CLORRAINE_CC, else gcc, cc, then
# clang on PATH. Kind is gcc, clang, or none; Clang is recognized by its
# --version text.
function Find-HostCompiler {
    $path = $null
    if ($env:CLORRAINE_CC) {
        $command = Get-Command -Name $env:CLORRAINE_CC -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($null -ne $command) { $path = $command.Source }
    } else {
        foreach ($name in 'gcc', 'cc', 'clang') {
            $command = Get-Command -Name $name -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
            if ($null -ne $command) { $path = $command.Source; break }
        }
    }
    if (-not $path) { return [pscustomobject]@{ Kind = 'none'; Path = $null } }
    $version = Invoke-Native $path @('--version')
    $kind = if (($version.Out + $version.Err) -match 'clang') { 'clang' } else { 'gcc' }
    return [pscustomobject]@{ Kind = $kind; Path = $path }
}

# Runs one hello in a private working folder; True for the exact hello line.
function Invoke-Hello([string[]]$Arguments) {
    $work = New-PrivateTemp 'hello'
    try {
        $result = Invoke-Clorraine $Arguments $work
        $line = (Get-SafeText $result.Out).Trim()
        Write-Line ('    ' + $script:Style.Bold + $line + $script:Style.Reset)
        if ($result.Code -ne 0 -and $result.Err) { Write-Line ('    ' + (Get-SafeText $result.Err).Trim()) }
        return ($result.Code -eq 0 -and $line -eq 'Hello, Amiga, from Clorraine!')
    } finally {
        Remove-TreeSafe $work
    }
}

# Runs the PC hello when a GCC-compatible compiler is present, then the Amiga
# hello in FS-UAE when an emulator and a ROM were selected. Returns the two
# outcomes (passed, failed, or skipped).
function Invoke-Hellos($Examples, $Emulator, [bool]$Interactive) {
    $state = [pscustomobject]@{ Host = 'skipped'; Guest = 'skipped' }
    if ($Interactive -and -not (Read-YesNo '  Run the hello program now? [Y/n] ' $true)) { return $state }
    $compiler = Find-HostCompiler
    switch ($compiler.Kind) {
        'none' { Write-Warn 'no C compiler found; the PC hello is skipped (install GCC (for example w64devkit or MSYS2 UCRT64) or set CLORRAINE_CC)' }
        'clang' { Write-Warn 'Clang found; the PC hello needs GCC (w64devkit or MSYS2), or set CLORRAINE_CC' }
        default {
            if (Invoke-Hello @('run', $Examples.Hello)) {
                $state.Host = 'passed'
                Write-Ok 'the compiler, the toolchain, and your C compiler work'
            } else {
                $state.Host = 'failed'
                Write-Warn 'the PC hello did not print the expected line'
                return $state
            }
        }
    }
    if ($Emulator.FsUae -and $Emulator.Kickstart) {
        if ($Interactive -and -not (Read-YesNo '  Now watch it on an Amiga 1200 in FS-UAE? [Y/n] ' $true)) { return $state }
        Write-Line '  Booting an Amiga 1200 in FS-UAE. The intro ends by itself, or click in its window.'
        $guest = @('run', '-p', 'hosted-amiga', '--release', '--emulator', '--fs-uae', $Emulator.FsUae,
            '--kickstart', $Emulator.Kickstart, $Examples.Guest)
        if (Invoke-Hello $guest) {
            $state.Guest = 'passed'
            Write-Ok 'your Amiga said hello'
        } else {
            $state.Guest = 'failed'
            Write-Warn 'the Amiga run did not complete'
        }
    }
    return $state
}

# --- VS Code and the starter ----------------------------------------------------

# How to run a VS Code-family launcher without cmd.exe. The .cmd launchers
# only set ELECTRON_RUN_AS_NODE=1 and run "<exe>" "<cli.js>" %*; doing the
# same directly means no argument is ever parsed by cmd.exe, so a folder name
# with & or % cannot turn into a command. Returns File, Prefix (arguments
# before the caller's), and Environment, or $null when the launcher does not
# have that shape.
function Get-EditorCommand([string]$Launcher) {
    $bin = Split-Path -Parent $Launcher
    $text = [IO.File]::ReadAllText($Launcher)
    $match = [regex]::Match($text, '"%~dp0\.\.\\([^"%]+\.exe)"\s+"%~dp0\.\.\\([^"%]+\\cli\.js)"')
    if (-not $match.Success) { return $null }
    $exe = Join-Path (Split-Path -Parent $bin) $match.Groups[1].Value
    $cli = Join-Path (Split-Path -Parent $bin) $match.Groups[2].Value
    if (-not (Test-Path -LiteralPath $exe -PathType Leaf) -or -not (Test-Path -LiteralPath $cli -PathType Leaf)) { return $null }
    return [pscustomobject]@{ File = $exe; Prefix = @($cli); Environment = @{ ELECTRON_RUN_AS_NODE = '1'; VSCODE_DEV = '' } }
}

# Runs the editor's command-line interface with Arguments and returns the
# result, or $null when the launcher cannot be run without cmd.exe.
function Invoke-Editor([string]$Launcher, [string[]]$Arguments) {
    $command = Get-EditorCommand $Launcher
    if ($null -eq $command) { return $null }
    return Invoke-Native $command.File ($command.Prefix + $Arguments) '' $command.Environment
}

# A working VS Code-family launcher (code.cmd, codium.cmd, code-insiders.cmd
# on PATH or in the default per-user install), or $null.
function Find-VSCode {
    foreach ($name in 'code.cmd', 'codium.cmd', 'code-insiders.cmd') {
        $command = Get-Command -Name $name -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($null -ne $command) { return $command.Source }
    }
    $default = Join-Path $env:LOCALAPPDATA 'Programs\Microsoft VS Code\bin\code.cmd'
    if (Test-Path -LiteralPath $default -PathType Leaf) { return $default }
    return $null
}

# Installs the pinned extension when VS Code is present and the person agrees
# (or passed -VSCode). Returns installed, already-installed, available,
# skipped, unavailable, or failed.
function Install-VSCodeExtension([hashtable]$Options, [bool]$Interactive) {
    if ($Options['NoVSCode']) { return 'skipped' }
    $editor = Find-VSCode
    if (-not $editor) {
        if ($Options['VSCode']) { Write-Warn "VS Code was not found; install it, then run: code --install-extension $script:VsixName" }
        return 'unavailable'
    }
    $listed = Invoke-Editor $editor @('--list-extensions')
    if ($null -eq $listed) {
        Write-Warn "VS Code was found, but its launcher has an unexpected shape; run: code --install-extension $script:VsixName"
        return 'failed'
    }
    if (@($listed.Out -split "`r?`n" | Where-Object { $_.Trim() -ieq 'openlorraine.clorraine' }).Count -gt 0) {
        Write-Ok 'the Clorraine extension is already installed in VS Code'
        return 'already-installed'
    }
    if (-not $Options['VSCode']) {
        if (-not $Interactive) {
            Write-Line '  VS Code detected; rerun with -VSCode to install the extension'
            return 'available'
        }
        if (-not (Read-YesNo '  Install the Clorraine extension in VS Code? [Y/n] ' $true)) { return 'skipped' }
    }
    if ($script:VsixBytes -notmatch '^[0-9]+$' -or $script:VsixSha256 -notmatch '^[0-9a-fA-F]{64}$') {
        Write-Warn 'the VS Code extension is not pinned in this installer; install it from the release'
        return 'failed'
    }
    $work = New-PrivateTemp 'vsix'
    try {
        $vsix = Join-Path $work $script:VsixName
        try {
            Invoke-Download $script:VsixUrl $vsix ([long]$script:VsixBytes) $script:VsixSha256 120
        } catch {
            Write-Warn ('could not download the VS Code extension: ' + (Get-SafeText $_.Exception.Message))
            return 'failed'
        }
        $result = Invoke-Editor $editor @('--install-extension', $vsix)
        if ($null -ne $result -and $result.Code -eq 0) {
            Write-Ok 'installed the Clorraine extension in VS Code'
            return 'installed'
        }
        Write-Warn "could not install the VS Code extension; run: code --install-extension $script:VsixName"
        return 'failed'
    } finally {
        Remove-TreeSafe $work
    }
}

# The default starter folder: .\amiga-starter, or the first free
# amiga-starter-N.
function Get-DefaultStarter {
    $base = Join-Path (Get-Location).ProviderPath 'amiga-starter'
    $candidate = $base
    $n = 2
    while ((Test-Path -LiteralPath $candidate) -and @(Get-ChildItem -LiteralPath $candidate -Force).Count -gt 0) {
        $candidate = "$base-$n"
        $n++
    }
    return $candidate
}

# Offers a starter project and opens it in VS Code. Returns its folder or $null.
function New-Starter([hashtable]$Options, [bool]$Interactive) {
    $dir = [string]$Options['Starter']
    if (-not $dir) {
        if ($Options['NoStarter'] -or -not $Interactive) { return $null }
        $dir = Get-DefaultStarter
        if (-not (Read-YesNo "  Create a starter Amiga project in ${dir}? [Y/n] " $true)) { return $null }
    }
    $result = Invoke-Clorraine @('new', $dir)
    if ($result.Code -ne 0) {
        Write-Warn ("could not create the starter project in ${dir}: " + (Get-SafeText $result.Err).Trim())
        return $null
    }
    Write-Ok "created $dir with main.clo and a VS Code setup"
    if ($Interactive -and -not $Options['NoVSCode']) {
        $editor = Find-VSCode
        $command = if ($editor) { Get-EditorCommand $editor } else { $null }
        if ($null -ne $command -and (Read-YesNo '  Open it in VS Code now? [Y/n] ' $true)) {
            $info = New-Object System.Diagnostics.ProcessStartInfo
            $info.FileName = $command.File
            $info.Arguments = (($command.Prefix + @($dir, (Join-Path $dir 'main.clo'))) | ForEach-Object { ConvertTo-NativeArgument $_ }) -join ' '
            $info.UseShellExecute = $false
            foreach ($name in $command.Environment.Keys) { $info.EnvironmentVariables[$name] = $command.Environment[$name] }
            # PATH for the editor includes the CLI, so its tasks find clorraine.exe.
            $info.EnvironmentVariables['PATH'] = (Join-Path $script:InstallRoot 'current\bin') + ';' + $env:PATH
            [void][System.Diagnostics.Process]::Start($info)
            Write-Ok "opening VS Code: press $($script:Style.Play) Run in FS-UAE in the editor title bar"
        }
    }
    return $dir
}

# --- Summary -------------------------------------------------------------------

# Prints what was set up and the next commands.
function Write-Summary($Summary) {
    # The bare name works only in terminals that see the user PATH entry.
    # PowerShell runs a quoted path only through the call operator.
    $cli = if ($Summary.OnPath) { 'clorraine' } else { '& "' + (Join-Path $Summary.Root 'current\bin\clorraine.exe') + '"' }
    Write-Line ''
    Write-Line ($script:Style.Green + $script:Style.Bold + $script:Style.Check + ' First run complete.' + $script:Style.Reset)
    Write-Line ''
    Write-Line "  CLI        $(Join-Path $Summary.Root 'current\bin\clorraine.exe')"
    Write-Line ('  Toolchain  ' + $(if ($Summary.Toolchain) { Get-SafeText $Summary.Toolchain -SingleLine } else { 'not configured' }))
    Write-Line ('  Emulator   ' + $(if ($Summary.Kickstart) { 'FS-UAE with ' + (Get-SafeText $Summary.Kickstart -SingleLine) } else { 'not set up' }))
    $hello = switch ("$($Summary.Host)/$($Summary.Guest)") {
        'passed/passed' { 'ran on this PC and on the Amiga' }
        'passed/failed' { 'ran on this PC; the Amiga run failed' }
        'skipped/passed' { 'ran on the Amiga' }
        default { if ($Summary.Host -eq 'passed') { 'ran on this PC' } elseif ($Summary.Host -eq 'failed') { 'failed' } else { 'not run' } }
    }
    Write-Line "  Hello      $hello"
    $editor = switch ($Summary.VSCode) {
        'installed' { 'extension installed' }
        'already-installed' { 'extension already installed' }
        'failed' { 'extension not installed' }
        'available' { 'detected; rerun with -VSCode' }
        'skipped' { 'skipped' }
        default { 'not found' }
    }
    Write-Line "  VS Code    $editor"
    Write-Line "  Examples   $(Join-Path $Summary.Root 'examples')"
    if ($Summary.Starter) { Write-Line ('  Project    ' + (Get-SafeText $Summary.Starter -SingleLine)) }
    Write-Line ''
    Write-Line ($script:Style.Bold + 'What next' + $script:Style.Reset)
    if ($Summary.OnPath) { Write-Line '  Open a new terminal so the PATH change applies, and restart VS Code.' }
    if ($Summary.Starter) {
        Write-Line ("  cd `"" + (Get-SafeText $Summary.Starter -SingleLine) + "`"")
        Write-Line '  code .                      (open main.clo, change MESSAGE, press Run in FS-UAE)'
        Write-Line "  $cli run -p hosted-amiga --emulator main.clo"
    } elseif ($Summary.Toolchain) {
        Write-Line "  $cli new my-game            (a starter Amiga project with VS Code tasks)"
    } else {
        Write-Line "  $cli setup --toolchain <rc24 root>"
    }
    if ($Summary.Toolchain) {
        $guest = Join-Path $Summary.Root 'examples\hello-amiga.clo'
        if ($Summary.Kickstart) {
            Write-Line "  $cli run -p hosted-amiga --release --emulator `"$guest`""
        } else {
            Write-Line "  $cli run -p hosted-amiga --release --emulator --kickstart <ks31-a1200.rom> `"$guest`""
        }
    }
    Write-Line ''
    Write-Line ($script:Style.Dim + '  Docs and examples: https://github.com/OpenLorraine/clorraine' + $script:Style.Reset)
}

# --- Command line ------------------------------------------------------------

# Rejects option combinations that cannot mean one thing.
function Test-Options([hashtable]$Options) {
    $given = { param($name) $Options.ContainsKey($name) -and [bool]$Options[$name] }
    if (& $given 'Rest') { Stop-Usage ("unknown option '" + (Get-SafeText (@($Options['Rest'])[0])) + "'") }
    if ((& $given 'Purge') -and -not (& $given 'Uninstall')) { Stop-Usage '-Purge requires -Uninstall' }
    if (& $given 'Uninstall') {
        foreach ($name in $script:InstallOptions) {
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
    [CmdletBinding(PositionalBinding = $false)]
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
        [switch]$Help,
        [Parameter(ValueFromRemainingArguments = $true)][string[]]$Rest
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
    if (-not (Test-64BitProcess)) {
        Stop-Install 1 'run the 64-bit Windows PowerShell (%SystemRoot%\System32\WindowsPowerShell\v1.0\powershell.exe)'
    }
    if ((Get-OsBuild) -lt 22000) { Write-Warn 'Windows 10 is untested; continuing' }
    foreach ($tool in 'curl.exe', 'tar.exe') {
        if (-not (Test-Path -LiteralPath (Join-Path $env:SystemRoot "System32\$tool"))) {
            Stop-Install 1 "this installer needs $tool from Windows; it was not found in System32"
        }
    }

    $root = Resolve-Prefix $(if (& $opt 'Prefix') { & $opt 'Prefix' } else { Get-DefaultPrefix })
    $script:InstallRoot = $root
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
        $onPath = Update-UserPath $root ([bool](& $opt 'ModifyPath')) $interactive
        $examples = Install-Examples $root
        Write-Ok "examples in $($examples.Folder)"
        $summary = [pscustomobject]@{
            Root = $root; OnPath = $onPath; Toolchain = $null; Kickstart = $null
            Host = 'skipped'; Guest = 'skipped'; VSCode = 'unavailable'; Starter = $null
        }
        if (-not (& $opt 'NoToolchain')) {
            if ($interactive) { Write-Step 2 'Amiga toolchain' }
            $toolchain = Select-Toolchain $root $Options $interactive
            if ($toolchain) {
                if ($interactive) { Write-Step 3 'Amiga emulator' }
                $emulator = Select-Emulator $root $Options $interactive
                $remembered = Set-Toolchain $toolchain $emulator
                if ($null -eq $remembered) {
                    Write-NextSteps $root $true
                    Stop-Install 1 'the toolchain could not be configured'
                }
                $summary.Toolchain = $toolchain
                $summary.Kickstart = $remembered.Kickstart
                if (-not (& $opt 'NoHello')) {
                    if ($interactive) { Write-Step 4 'Say hello' }
                    $state = Invoke-Hellos $examples $remembered $interactive
                    $summary.Host = $state.Host
                    $summary.Guest = $state.Guest
                    if ($state.Host -eq 'failed') {
                        Write-NextSteps $root $false
                        Stop-Install 1 'the PC hello failed'
                    }
                }
            }
        }
        if ($interactive) { Write-Step 5 'Your first Amiga project' }
        $summary.VSCode = Install-VSCodeExtension $Options $interactive
        if ($summary.Toolchain -or (& $opt 'Starter')) { $summary.Starter = New-Starter $Options $interactive }
        Write-Summary $summary
    } finally {
        $mutex.ReleaseMutex()
        $mutex.Dispose()
    }
}

if ($env:CLORRAINE_INSTALL_LIB -ne '1') {
    exit (Invoke-Installer @PSBoundParameters)
}
