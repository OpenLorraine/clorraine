# Specification tests for the Windows installer, tools/setup/install.ps1.
#
# Run on Windows 11 with Windows PowerShell 5.1, from the repository root:
#   powershell -NoProfile -ExecutionPolicy Bypass -File tools\setup\tests\install.spec.test.ps1
#
# Each case assembles a copy of the installer with fixture pins, builds fixture
# zips around a small compiled stand-in for clorraine.exe, and runs the
# installer in a child Windows PowerShell that dot-sources the copy, replaces
# only the host facts the case names, and calls Invoke-Installer. PATH edits go
# to a scratch registry key under HKCU\Software. Nothing here needs the
# network, administrator rights, a toolchain, a ROM, or FS-UAE.
#
# Exit code: 0 when every case passes, 1 otherwise. -Filter runs the cases
# whose name contains the given text.
param([string]$Filter = '')

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version 2

$Here = Split-Path -Parent $MyInvocation.MyCommand.Path
$Template = Join-Path (Split-Path -Parent $Here) 'install.ps1'
$SuiteRoot = Join-Path ([IO.Path]::GetTempPath()) ('clorraine-install-spec-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
$ScratchKeyBase = 'Software\ClorraineInstallSpec'
New-Item -ItemType Directory -Path $SuiteRoot | Out-Null
$script:Passed = 0
$script:Failed = 0
$script:CaseNumber = 0

# --- Harness -----------------------------------------------------------------

# Fails the current case with a message.
function Assert-That([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

# Runs one named case and records the result; a failure never stops the suite.
function Test-Case([string]$Name, [scriptblock]$Body) {
    if ($Filter -and $Name -notlike "*$Filter*") { return }
    try {
        & $Body
        $script:Passed++
        Write-Output "PASS $Name"
    } catch {
        $script:Failed++
        Write-Output "FAIL $Name"
        Write-Output ("     " + $_.Exception.Message)
    }
}

# A fresh directory for one case.
function New-CaseRoot {
    $script:CaseNumber++
    $dir = Join-Path $SuiteRoot ("case-{0:d2}" -f $script:CaseNumber)
    New-Item -ItemType Directory -Path $dir | Out-Null
    return $dir
}

# --- Fixtures ----------------------------------------------------------------

$script:FakeExes = @{}

# A console program that prints `clorraine <version>` for --version.
function Get-FakeClorraine([string]$Version) {
    if ($script:FakeExes.ContainsKey($Version)) { return $script:FakeExes[$Version] }
    $class = 'FakeClorraine' + ($script:FakeExes.Count + 1)
    $path = Join-Path $SuiteRoot "$class.exe"
    $source = @"
public static class $class {
    public static int Main(string[] args) {
        if (args.Length == 1 && args[0] == "--version") {
            System.Console.WriteLine("clorraine $Version");
            return 0;
        }
        System.Console.WriteLine("fake clorraine");
        return 0;
    }
}
"@
    Add-Type -TypeDefinition $source -OutputAssembly $path -OutputType ConsoleApplication
    $script:FakeExes[$Version] = $path
    return $path
}

# The release zip name for a version.
function Get-ZipName([string]$Version) { "clorraine-$Version-x86_64-windows.zip" }

# Builds a candidate zip: one top folder with bin\*.exe and the notices.
# Extra entries (name -> text) are added verbatim, for hostile layouts.
function New-FixtureZip([string]$Dir, [string]$Version, [hashtable]$Extra = @{}) {
    Add-Type -AssemblyName System.IO.Compression
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $zip = Join-Path $Dir (Get-ZipName $Version)
    $top = "clorraine-$Version-x86_64-windows"
    $exe = [IO.File]::ReadAllBytes((Get-FakeClorraine $Version))
    $stream = [IO.File]::Open($zip, 'CreateNew')
    $archive = New-Object IO.Compression.ZipArchive($stream, [IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($name in 'clc.exe', 'clorraine.exe', 'clorraine-lsp.exe') {
            $entry = $archive.CreateEntry("$top/bin/$name")
            $writer = $entry.Open(); $writer.Write($exe, 0, $exe.Length); $writer.Dispose()
        }
        foreach ($name in 'LICENSE', 'LICENSE-EXCEPTION.md', 'NOTICE', 'THIRD_PARTY_NOTICES.md', 'README.md') {
            $entry = $archive.CreateEntry("$top/$name")
            $bytes = [Text.Encoding]::ASCII.GetBytes("$name fixture`n")
            $writer = $entry.Open(); $writer.Write($bytes, 0, $bytes.Length); $writer.Dispose()
        }
        foreach ($name in $Extra.Keys) {
            $entry = $archive.CreateEntry($name)
            $bytes = [Text.Encoding]::ASCII.GetBytes($Extra[$name])
            $writer = $entry.Open(); $writer.Write($bytes, 0, $bytes.Length); $writer.Dispose()
        }
    } finally {
        $archive.Dispose()
        $stream.Dispose()
    }
    return $zip
}

# Lowercase SHA-256 of a file.
function Get-Sha256([string]$Path) { (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant() }

# Writes a SHA256SUMS file for the zip, optionally with a different digest.
function New-Checksums([string]$Zip, [string]$Digest = '') {
    if (-not $Digest) { $Digest = Get-Sha256 $Zip }
    $path = Join-Path (Split-Path -Parent $Zip) 'SHA256SUMS'
    $line = $Digest + '  ' + (Split-Path -Leaf $Zip)
    [IO.File]::WriteAllText($path, $line + "`n", [Text.Encoding]::ASCII)
    return $path
}

# The installed version folder name for a zip: <version>-<first 12 hex of SHA-256>.
function Get-VersionFolder([string]$Version, [string]$Zip) { "$Version-" + (Get-Sha256 $Zip).Substring(0, 12) }

# Copies the template with fixture pins for one version.
function New-AssembledInstaller([string]$Dir, [string]$Version) {
    $text = [IO.File]::ReadAllText($Template)
    $pins = @{
        '@VERSION@' = $Version
        '@ARCHIVE_NAME@' = (Get-ZipName $Version)
        '@ARCHIVE_URL@' = 'https://example.invalid/' + (Get-ZipName $Version)
        '@ARCHIVE_BYTES@' = '1'
        '@ARCHIVE_SHA256@' = ('0' * 64)
        '@VSIX_NAME@' = "clorraine-$Version.vsix"
        '@VSIX_URL@' = "https://example.invalid/clorraine-$Version.vsix"
        '@VSIX_BYTES@' = '1'
        '@VSIX_SHA256@' = ('0' * 64)
        '@HELLO_AMIGA_SOURCE@' = "module demo.HelloAmiga;`nfunc main(): i32 { return 0; }"
    }
    foreach ($pin in $pins.Keys) { $text = $text.Replace($pin, $pins[$pin]) }
    $path = Join-Path $Dir "install-$Version.ps1"
    [IO.File]::WriteAllText($path, $text, [Text.Encoding]::ASCII)
    return $path
}

# A scratch user-environment key holding a Path value of the given kind.
function New-ScratchPathKey([string]$Value, [Microsoft.Win32.RegistryValueKind]$Kind = 'ExpandString') {
    $name = "$ScratchKeyBase\" + [guid]::NewGuid().ToString('N').Substring(0, 8)
    $key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($name)
    if ($null -ne $Value) { $key.SetValue('Path', $Value, $Kind) }
    $key.Dispose()
    return "HKCU:\$name"
}

# The raw Path value and its kind from a scratch key.
function Get-ScratchPath([string]$KeyPath) {
    $name = $KeyPath.Substring('HKCU:\'.Length)
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($name)
    try {
        $value = $key.GetValue('Path', $null, [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        $kind = if ($null -ne $value) { $key.GetValueKind('Path') } else { $null }
        return [pscustomobject]@{ Value = $value; Kind = $kind }
    } finally { $key.Dispose() }
}

# Quotes one argument for a PowerShell command line.
function ConvertTo-ArgText([string[]]$Arguments) {
    ($Arguments | ForEach-Object {
        if ($_ -match '^-[A-Za-z]+$') { $_ } else { "'" + $_.Replace("'", "''") + "'" }
    }) -join ' '
}

# Runs the installer copy in a child PowerShell and returns its exit code and
# output. Prelude is PowerShell text run after dot-sourcing (stubs); Body
# replaces the Invoke-Installer call when a case checks a function directly.
function Invoke-Case {
    param(
        [string]$Installer,
        [string]$CaseRoot,
        [string[]]$Arguments = @(),
        [string]$Prelude = '',
        [string]$PathKey = '',
        [string]$Body = ''
    )
    $appdata = Join-Path $CaseRoot 'appdata'
    New-Item -ItemType Directory -Force -Path $appdata | Out-Null
    if (-not $PathKey) { $PathKey = New-ScratchPathKey 'C:\unrelated' }
    if (-not $Body) { $Body = 'exit (Invoke-Installer ' + (ConvertTo-ArgText $Arguments) + ')' }
    $runner = Join-Path $CaseRoot ('runner-' + [guid]::NewGuid().ToString('N').Substring(0, 6) + '.ps1')
    # Each concatenation is parenthesized: the comma binds tighter than +.
    $lines = @(
        '$ErrorActionPreference = ''Stop''',
        ('$env:APPDATA = ''' + $appdata.Replace("'", "''") + ''''),
        '$env:CLORRAINE_INSTALL_LIB = ''1''',
        ('. ''' + $Installer.Replace("'", "''") + ''''),
        'function Test-Interactive { $false }',
        ('$script:PathRegistryKey = ''' + $PathKey + ''''),
        $Prelude,
        $Body
    )
    [IO.File]::WriteAllLines($runner, $lines, [Text.Encoding]::ASCII)
    $out = Join-Path $CaseRoot 'stdout.txt'
    $err = Join-Path $CaseRoot 'stderr.txt'
    $process = Start-Process -FilePath powershell.exe -NoNewWindow -Wait -PassThru `
        -ArgumentList @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$runner`"") `
        -RedirectStandardOutput $out -RedirectStandardError $err
    $stdout = [IO.File]::ReadAllText($out)
    $stderr = [IO.File]::ReadAllText($err)
    return [pscustomobject]@{
        Code = $process.ExitCode
        Out = $stdout
        Err = $stderr
        All = $stdout + $stderr
        PathKey = $PathKey
    }
}

# A prefix with one completed install of the given version; returns the parts.
function New-Installed([string]$CaseRoot, [string]$Version = '9.1.0', [string]$PathKey = '') {
    $prefix = Join-Path $CaseRoot 'prefix'
    $zip = New-FixtureZip $CaseRoot $Version
    $sums = New-Checksums $zip
    $installer = New-AssembledInstaller $CaseRoot $Version
    $result = Invoke-Case -Installer $installer -CaseRoot $CaseRoot -PathKey $PathKey `
        -Arguments @('-Archive', $zip, '-Checksums', $sums, '-Prefix', $prefix, '-NoToolchain')
    Assert-That ($result.Code -eq 0) ("fixture install failed ($($result.Code)): " + $result.All)
    return [pscustomobject]@{
        Prefix = $prefix; Zip = $zip; Sums = $sums; Installer = $installer
        Folder = Join-Path $prefix ('versions\' + (Get-VersionFolder $Version $zip))
        PathKey = $result.PathKey; Result = $result
    }
}

# The resolved target of a junction, or $null when Path is not one.
function Get-JunctionTarget([string]$Path) {
    $item = Get-Item -LiteralPath $Path -Force -ErrorAction SilentlyContinue
    if ($null -eq $item -or $item.LinkType -ne 'Junction') { return $null }
    return (@($item.Target)[0]).TrimEnd('\')
}

# True when two Windows paths name the same folder.
function Test-SamePath([string]$A, [string]$B) {
    [string]::Equals($A.TrimEnd('\'), $B.TrimEnd('\'), [StringComparison]::OrdinalIgnoreCase)
}

# --- Command line ------------------------------------------------------------

Test-Case 'should reject conflicting or incomplete options with exit 2 and the usage text' {
    $root = New-CaseRoot
    $installer = New-AssembledInstaller $root '9.1.0'
    $cases = New-Object System.Collections.ArrayList
    [void]$cases.Add([string[]]@('-Purge'))
    [void]$cases.Add([string[]]@('-Toolchain', 'C:\rc24', '-NoToolchain'))
    [void]$cases.Add([string[]]@('-FsUae', 'C:\fs-uae.exe', '-NoEmulator'))
    [void]$cases.Add([string[]]@('-Archive', 'C:\a.zip'))
    foreach ($arguments in $cases) {
        $result = Invoke-Case -Installer $installer -CaseRoot $root -Arguments $arguments
        Assert-That ($result.Code -eq 2) ("'$($arguments -join ' ')' exited $($result.Code): " + $result.All)
        Assert-That ($result.All -match 'Usage:') ("'$($arguments -join ' ')' printed no usage text: " + $result.All)
    }
}

# --- Host --------------------------------------------------------------------

Test-Case 'should refuse a non-x64 host' {
    $root = New-CaseRoot
    $zip = New-FixtureZip $root '9.1.0'
    $sums = New-Checksums $zip
    $installer = New-AssembledInstaller $root '9.1.0'
    $result = Invoke-Case -Installer $installer -CaseRoot $root -Prelude 'function Get-HostArchitecture { ''Arm64'' }' `
        -Arguments @('-Archive', $zip, '-Checksums', $sums, '-Prefix', (Join-Path $root 'prefix'), '-NoToolchain')
    Assert-That ($result.Code -eq 1) "exit $($result.Code)"
    Assert-That ($result.All -match 'this installer supports 64-bit x86 Windows only') $result.All
    Assert-That (-not (Test-Path (Join-Path $root 'prefix\current'))) 'installed on a refused host'
}

Test-Case 'should warn that Windows 10 is untested and continue' {
    $root = New-CaseRoot
    $zip = New-FixtureZip $root '9.1.0'
    $sums = New-Checksums $zip
    $installer = New-AssembledInstaller $root '9.1.0'
    $prefix = Join-Path $root 'prefix'
    $result = Invoke-Case -Installer $installer -CaseRoot $root -Prelude 'function Get-OsBuild { 19045 }' `
        -Arguments @('-Archive', $zip, '-Checksums', $sums, '-Prefix', $prefix, '-NoToolchain')
    Assert-That ($result.All -match 'Windows 10 is untested; continuing') $result.All
    Assert-That ($result.Code -eq 0) "exit $($result.Code): $($result.All)"
    Assert-That ($null -ne (Get-JunctionTarget (Join-Path $prefix 'current'))) 'the run did not continue to the install'
}

# --- Install -----------------------------------------------------------------

Test-Case 'should install a verified local archive into a version folder behind a current junction' {
    $root = New-CaseRoot
    $key = New-ScratchPathKey 'C:\unrelated;%USERPROFILE%\bin'
    $before = Get-ScratchPath $key
    $installed = New-Installed $root '9.1.0' $key
    Assert-That (Test-Path (Join-Path $installed.Folder 'bin\clorraine.exe')) 'no bin\clorraine.exe in the version folder'
    $target = Get-JunctionTarget (Join-Path $installed.Prefix 'current')
    Assert-That ($null -ne $target) 'current is not a junction'
    Assert-That (Test-SamePath $target $installed.Folder) "current targets $target"
    foreach ($name in 'hello.clo', 'hello-amiga.clo') {
        Assert-That (Test-Path (Join-Path $installed.Prefix "examples\$name")) "examples\$name missing"
    }
    $after = Get-ScratchPath $key
    Assert-That ($after.Value -eq $before.Value -and $after.Kind -eq $before.Kind) "the user PATH changed: $($after.Value)"
}

Test-Case 'should stop on a checksum mismatch and install nothing' {
    $root = New-CaseRoot
    $zip = New-FixtureZip $root '9.1.0'
    $sums = New-Checksums $zip ('a' * 64)
    $installer = New-AssembledInstaller $root '9.1.0'
    $prefix = Join-Path $root 'prefix'
    $result = Invoke-Case -Installer $installer -CaseRoot $root `
        -Arguments @('-Archive', $zip, '-Checksums', $sums, '-Prefix', $prefix, '-NoToolchain')
    Assert-That ($result.Code -eq 1) "exit $($result.Code)"
    Assert-That ($result.All -match [regex]::Escape((Get-ZipName '9.1.0'))) "the error does not name the archive: $($result.All)"
    Assert-That (-not (Test-Path (Join-Path $prefix 'versions\9.1.0-*'))) 'a version folder was created'
    Assert-That (-not (Test-Path (Join-Path $prefix 'current'))) 'current was created'
}

Test-Case 'should refuse a zip entry that climbs out of the version folder before extracting' {
    $root = New-CaseRoot
    $zip = New-FixtureZip $root '9.1.0' @{ '../evil.txt' = 'escaped' }
    $sums = New-Checksums $zip
    $installer = New-AssembledInstaller $root '9.1.0'
    $prefix = Join-Path $root 'prefix'
    $result = Invoke-Case -Installer $installer -CaseRoot $root `
        -Arguments @('-Archive', $zip, '-Checksums', $sums, '-Prefix', $prefix, '-NoToolchain')
    Assert-That ($result.Code -eq 1) "exit $($result.Code)"
    # The installer's own check refuses the entry; tar.exe never runs.
    Assert-That ($result.All -match 'archive contains an unsafe entry') ("not refused before extraction: " + $result.All)
    $escaped = @(Get-ChildItem -LiteralPath $root -Recurse -Force -Filter 'evil.txt' -ErrorAction SilentlyContinue)
    Assert-That ($escaped.Count -eq 0) ('evil.txt was written: ' + (($escaped | ForEach-Object { $_.FullName }) -join ', '))
    Assert-That (-not (Test-Path (Join-Path $prefix 'versions\9.1.0-*'))) 'a version folder was created'
    Assert-That (-not (Test-Path (Join-Path $prefix 'current'))) 'current was created'
}

Test-Case 'should keep both versions side by side and switch current to the newer one' {
    $root = New-CaseRoot
    $a = New-Installed $root '9.1.0'
    $hashA = Get-Sha256 (Join-Path $a.Folder 'bin\clorraine.exe')
    $zipB = New-FixtureZip $root '9.2.0'
    $sumsB = Join-Path $root 'SHA256SUMS-B'
    [IO.File]::WriteAllText($sumsB, (Get-Sha256 $zipB) + '  ' + (Split-Path -Leaf $zipB) + "`n", [Text.Encoding]::ASCII)
    $installerB = New-AssembledInstaller $root '9.2.0'
    $result = Invoke-Case -Installer $installerB -CaseRoot $root -PathKey $a.PathKey `
        -Arguments @('-Archive', $zipB, '-Checksums', $sumsB, '-Prefix', $a.Prefix, '-NoToolchain')
    Assert-That ($result.Code -eq 0) "exit $($result.Code): $($result.All)"
    $folderB = Join-Path $a.Prefix ('versions\' + (Get-VersionFolder '9.2.0' $zipB))
    Assert-That (Test-Path $a.Folder) 'version A was removed'
    Assert-That (Test-Path $folderB) 'version B is missing'
    Assert-That (Test-SamePath (Get-JunctionTarget (Join-Path $a.Prefix 'current')) $folderB) 'current does not target B'
    Assert-That ((Get-Sha256 (Join-Path $a.Folder 'bin\clorraine.exe')) -eq $hashA) "version A's files changed"
}

Test-Case 'should finish an interrupted switch on the next run' {
    $root = New-CaseRoot
    $a = New-Installed $root '9.1.0'
    $current = Join-Path $a.Prefix 'current'
    $pending = Join-Path $a.Prefix 'current.new'
    cmd /c rmdir "`"$current`"" | Out-Null
    New-Item -ItemType Junction -Path $pending -Target $a.Folder | Out-Null
    $result = Invoke-Case -Installer $a.Installer -CaseRoot $root -PathKey $a.PathKey `
        -Arguments @('-Archive', $a.Zip, '-Checksums', $a.Sums, '-Prefix', $a.Prefix, '-NoToolchain')
    Assert-That ($result.Code -eq 0) "exit $($result.Code): $($result.All)"
    Assert-That (Test-SamePath (Get-JunctionTarget $current) $a.Folder) 'current does not target the version'
    Assert-That (-not (Test-Path $pending)) 'current.new is still present'
}

Test-Case 'should refuse to run while another installer holds the lock' {
    $root = New-CaseRoot
    $zip = New-FixtureZip $root '9.1.0'
    $sums = New-Checksums $zip
    $installer = New-AssembledInstaller $root '9.1.0'
    $ready = Join-Path $root 'holder-ready.txt'
    $holder = Start-Process -FilePath powershell.exe -PassThru -WindowStyle Hidden -ArgumentList @(
        '-NoProfile', '-Command',
        "`$m = New-Object System.Threading.Mutex(`$true, 'Local\Clorraine-Install'); Set-Content -LiteralPath '$ready' ok; Start-Sleep 60")
    try {
        $deadline = (Get-Date).AddSeconds(20)
        while (-not (Test-Path $ready) -and (Get-Date) -lt $deadline) { Start-Sleep -Milliseconds 200 }
        Assert-That (Test-Path $ready) 'the lock holder did not start'
        $result = Invoke-Case -Installer $installer -CaseRoot $root `
            -Arguments @('-Archive', $zip, '-Checksums', $sums, '-Prefix', (Join-Path $root 'prefix'), '-NoToolchain')
        Assert-That ($result.Code -eq 1) "exit $($result.Code)"
        Assert-That ($result.All -match 'another Clorraine installer is running') $result.All
    } finally {
        Stop-Process -Id $holder.Id -Force -ErrorAction SilentlyContinue
    }
}

Test-Case 'should keep the active version when a run fails after extraction' {
    $root = New-CaseRoot
    $a = New-Installed $root '9.1.0'
    $zipB = New-FixtureZip $root '9.2.0'
    $sumsB = Join-Path $root 'SHA256SUMS-B'
    [IO.File]::WriteAllText($sumsB, (Get-Sha256 $zipB) + '  ' + (Split-Path -Leaf $zipB) + "`n", [Text.Encoding]::ASCII)
    $installerB = New-AssembledInstaller $root '9.2.0'
    $result = Invoke-Case -Installer $installerB -CaseRoot $root -PathKey $a.PathKey `
        -Prelude 'function Test-ExtractedVersion { throw ''stubbed version failure'' }' `
        -Arguments @('-Archive', $zipB, '-Checksums', $sumsB, '-Prefix', $a.Prefix, '-NoToolchain')
    Assert-That ($result.Code -eq 1) "exit $($result.Code)"
    Assert-That (Test-SamePath (Get-JunctionTarget (Join-Path $a.Prefix 'current')) $a.Folder) 'current no longer targets A'
    $left = @(Get-ChildItem -LiteralPath (Join-Path $a.Prefix 'versions') -Force | Where-Object { $_.Name -like '9.2.0-*' -or $_.Name -like '.tmp-*' })
    Assert-That ($left.Count -eq 0) ('left behind: ' + (($left | ForEach-Object { $_.Name }) -join ', '))
}

# --- PATH --------------------------------------------------------------------

Test-Case 'should add the bin folder once to an expandable user PATH and restore it on uninstall' {
    $root = New-CaseRoot
    $original = '%USERPROFILE%\bin;C:\y'
    $key = New-ScratchPathKey $original
    $zip = New-FixtureZip $root '9.1.0'
    $sums = New-Checksums $zip
    $installer = New-AssembledInstaller $root '9.1.0'
    $prefix = Join-Path $root 'prefix'
    foreach ($run in 1, 2) {
        $result = Invoke-Case -Installer $installer -CaseRoot $root -PathKey $key `
            -Arguments @('-Archive', $zip, '-Checksums', $sums, '-Prefix', $prefix, '-NoToolchain', '-ModifyPath')
        Assert-That ($result.Code -eq 0) "run $run exited $($result.Code): $($result.All)"
    }
    $after = Get-ScratchPath $key
    Assert-That ($after.Kind -eq [Microsoft.Win32.RegistryValueKind]::ExpandString) "kind became $($after.Kind)"
    Assert-That ($after.Value.Contains('%USERPROFILE%\bin')) "an existing entry was expanded: $($after.Value)"
    $bin = Join-Path $prefix 'current\bin'
    $found = @($after.Value.Split(';') | Where-Object { $_ -and (Test-SamePath $_ $bin) })
    Assert-That ($found.Count -eq 1) "the bin folder appears $($found.Count) times: $($after.Value)"
    $result = Invoke-Case -Installer $installer -CaseRoot $root -PathKey $key -Arguments @('-Uninstall', '-Prefix', $prefix)
    Assert-That ($result.Code -eq 0) "uninstall exited $($result.Code): $($result.All)"
    $restored = Get-ScratchPath $key
    Assert-That ($restored.Value -eq $original) "PATH after uninstall: $($restored.Value)"
}

Test-Case 'should remove a PATH entry that differs only in case or a trailing backslash' {
    $root = New-CaseRoot
    $a = New-Installed $root '9.1.0'
    $spelled = (Join-Path $a.Prefix 'current\bin').ToUpperInvariant() + '\'
    $key = New-ScratchPathKey "C:\keep;$spelled;C:\other"
    $result = Invoke-Case -Installer $a.Installer -CaseRoot $root -PathKey $key -Arguments @('-Uninstall', '-Prefix', $a.Prefix)
    Assert-That ($result.Code -eq 0) "exit $($result.Code): $($result.All)"
    $after = Get-ScratchPath $key
    Assert-That ($after.Value -eq 'C:\keep;C:\other') "PATH after uninstall: $($after.Value)"
}

# --- Uninstall and purge ------------------------------------------------------

Test-Case 'should remove the current junction and PATH entry on uninstall and keep the versions' {
    $root = New-CaseRoot
    $key = New-ScratchPathKey 'C:\unrelated'
    $zip = New-FixtureZip $root '9.1.0'
    $sums = New-Checksums $zip
    $installer = New-AssembledInstaller $root '9.1.0'
    $prefix = Join-Path $root 'prefix'
    $result = Invoke-Case -Installer $installer -CaseRoot $root -PathKey $key `
        -Arguments @('-Archive', $zip, '-Checksums', $sums, '-Prefix', $prefix, '-NoToolchain', '-ModifyPath')
    Assert-That ($result.Code -eq 0) "install exited $($result.Code): $($result.All)"
    $folder = Join-Path $prefix ('versions\' + (Get-VersionFolder '9.1.0' $zip))
    $result = Invoke-Case -Installer $installer -CaseRoot $root -PathKey $key -Arguments @('-Uninstall', '-Prefix', $prefix)
    Assert-That ($result.Code -eq 0) "uninstall exited $($result.Code): $($result.All)"
    Assert-That (-not (Test-Path (Join-Path $prefix 'current'))) 'current is still present'
    Assert-That ((Get-ScratchPath $key).Value -eq 'C:\unrelated') "PATH: $((Get-ScratchPath $key).Value)"
    Assert-That (Test-Path (Join-Path $folder 'bin\clorraine.exe')) 'the version tree was removed'
}

Test-Case 'should refuse to uninstall a current junction that points outside the versions folder' {
    $root = New-CaseRoot
    $prefix = Join-Path $root 'prefix'
    $outside = Join-Path $root 'outside'
    New-Item -ItemType Directory -Force -Path (Join-Path $prefix 'versions'), $outside | Out-Null
    Set-Content -LiteralPath (Join-Path $outside 'keep.txt') 'keep'
    New-Item -ItemType Junction -Path (Join-Path $prefix 'current') -Target $outside | Out-Null
    $installer = New-AssembledInstaller $root '9.1.0'
    $result = Invoke-Case -Installer $installer -CaseRoot $root -Arguments @('-Uninstall', '-Prefix', $prefix)
    # The refusal must come from the installer, not from a script that failed to load.
    Assert-That ($result.All -notmatch 'is not recognized as the name of') ("the installer did not run: " + $result.All)
    Assert-That ($result.Code -eq 1) "exit $($result.Code)"
    Assert-That ($null -ne (Get-JunctionTarget (Join-Path $prefix 'current'))) 'current was removed'
    Assert-That (Test-Path (Join-Path $outside 'keep.txt')) 'the outside folder lost keep.txt'
}

Test-Case 'should purge versions examples the downloaded FS-UAE and the config without following junctions' {
    $root = New-CaseRoot
    $a = New-Installed $root '9.1.0'
    $outside = Join-Path $root 'outside'
    New-Item -ItemType Directory -Force -Path $outside | Out-Null
    Set-Content -LiteralPath (Join-Path $outside 'keep.txt') 'keep'
    New-Item -ItemType Junction -Path (Join-Path $a.Folder 'link') -Target $outside | Out-Null
    $fsuae = Join-Path $a.Prefix 'fs-uae\3.2.35'
    New-Item -ItemType Directory -Force -Path $fsuae | Out-Null
    Set-Content -LiteralPath (Join-Path $fsuae 'fs-uae.txt') 'fixture'
    $config = Join-Path $root 'appdata\clorraine\config'
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $config) | Out-Null
    Set-Content -LiteralPath $config 'toolchain=C:\rc24'
    $rc24 = Join-Path $root 'rc24'
    New-Item -ItemType Directory -Force -Path (Join-Path $rc24 'bin') | Out-Null
    $prelude = 'function Get-Rc24Registration { [pscustomobject]@{ DisplayVersion = ''16.2-rc24''; InstallLocation = ''' + $rc24 + ''' } }'
    $result = Invoke-Case -Installer $a.Installer -CaseRoot $root -PathKey $a.PathKey -Prelude $prelude `
        -Arguments @('-Uninstall', '-Purge', '-Prefix', $a.Prefix)
    Assert-That ($result.Code -eq 0) "exit $($result.Code): $($result.All)"
    foreach ($gone in 'versions', 'examples', 'fs-uae\3.2.35', 'current') {
        Assert-That (-not (Test-Path (Join-Path $a.Prefix $gone))) "$gone is still present"
    }
    Assert-That (-not (Test-Path $config)) 'the config is still present'
    Assert-That (Test-Path (Join-Path $outside 'keep.txt')) 'purge followed the junction and removed keep.txt'
    Assert-That (Test-Path (Join-Path $rc24 'bin')) 'purge touched the toolchain'
}

Test-Case 'should refuse to purge while a program runs from the install tree' {
    $root = New-CaseRoot
    $a = New-Installed $root '9.1.0'
    $running = Join-Path $a.Folder 'bin\ping.exe'
    Copy-Item -LiteralPath (Join-Path $env:SystemRoot 'System32\PING.EXE') -Destination $running
    $process = Start-Process -FilePath $running -ArgumentList @('-n', '60', '127.0.0.1') -PassThru -WindowStyle Hidden
    try {
        $result = Invoke-Case -Installer $a.Installer -CaseRoot $root -PathKey $a.PathKey `
            -Arguments @('-Uninstall', '-Purge', '-Prefix', $a.Prefix)
        Assert-That ($result.Code -eq 1) "exit $($result.Code)"
        Assert-That ($result.All -match 'close VS Code and running Clorraine builds, then retry') $result.All
        Assert-That (Test-Path (Join-Path $a.Folder 'bin\clorraine.exe')) 'the version tree was removed'
        Assert-That ($null -ne (Get-JunctionTarget (Join-Path $a.Prefix 'current'))) 'current was removed'
    } finally {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
    }
}

# --- Template and presentation ------------------------------------------------

Test-Case 'should refuse network installs from the unassembled template' {
    $root = New-CaseRoot
    $result = Invoke-Case -Installer $Template -CaseRoot $root -Arguments @('-Prefix', (Join-Path $root 'prefix'), '-NoToolchain')
    Assert-That ($result.Code -eq 1) "exit $($result.Code)"
    Assert-That ($result.All -match 'this installer template is not assembled') $result.All
}

Test-Case 'should keep the template and an assembled copy ASCII-only' {
    $root = New-CaseRoot
    foreach ($path in $Template, (New-AssembledInstaller $root '9.1.0')) {
        $bytes = [IO.File]::ReadAllBytes($path)
        $high = @($bytes | Where-Object { $_ -gt 0x7E -or ($_ -lt 0x20 -and $_ -ne 9 -and $_ -ne 10 -and $_ -ne 13) })
        Assert-That ($high.Count -eq 0) "$path has $($high.Count) non-ASCII or control bytes"
    }
}

Test-Case 'should pick plain markers colour and glyphs from the console capability' {
    $root = New-CaseRoot
    $installer = New-AssembledInstaller $root '9.1.0'
    $body = @'
$esc = [string][char]27
$fail = @()
$plain = New-OutputStyle -VirtualTerminal $false -CodePage 65001 -NoColor $false
if ($plain.Check -ne '[ok]' -or $plain.Cross -ne '[!!]') { $fail += 'no VT: markers' }
if ((($plain.Values | ForEach-Object { [string]$_ }) -join '').Contains($esc)) { $fail += 'no VT: escape bytes' }
$colour = New-OutputStyle -VirtualTerminal $true -CodePage 437 -NoColor $false
if ($colour.Check -ne '[ok]') { $fail += 'VT 437: glyph instead of [ok]' }
if (-not $colour.Green.Contains($esc)) { $fail += 'VT 437: no colour' }
$glyph = New-OutputStyle -VirtualTerminal $true -CodePage 65001 -NoColor $true
if ($glyph.Check -ne [string][char]0x2714 -or $glyph.Cross -ne [string][char]0x2718) { $fail += 'VT 65001: no glyphs' }
if ($glyph.Green -ne '' -or $glyph.Reset -ne '') { $fail += 'NO_COLOR: colour kept' }
if ($fail.Count -gt 0) { [Console]::Out.WriteLine(($fail -join '; ')); exit 1 }
exit 0
'@
    $result = Invoke-Case -Installer $installer -CaseRoot $root -Body $body
    Assert-That ($result.Code -eq 0) $result.All
}

# --- Toolchain, emulator, hellos ---------------------------------------------

# Stubs that record what the installer asked for, one line per call, in
# $Log: downloads, elevation, and every command run through the installed CLI.
function Get-RecordingPrelude([string]$Log, [string]$Extra = '') {
    $quoted = $Log.Replace("'", "''")
    return @"
function Invoke-Download([string]`$Url, [string]`$Destination, [long]`$Bytes, [string]`$Sha256, [int]`$MaxSeconds = 600) {
    Add-Content -LiteralPath '$quoted' -Value ("download|" + `$Url + "|" + `$Bytes + "|" + `$Sha256)
    if (`$env:CLORRAINE_TEST_DOWNLOAD_FROM) { Copy-Item -LiteralPath `$env:CLORRAINE_TEST_DOWNLOAD_FROM -Destination `$Destination }
    else { Set-Content -LiteralPath `$Destination -Value 'fixture' }
}
function Invoke-ElevatedSetup([string]`$Source, [string]`$Target, [long]`$Bytes, [string]`$Sha256) {
    Add-Content -LiteralPath '$quoted' -Value ("elevate|" + `$Target + "|" + `$Bytes + "|" + `$Sha256)
    return 0
}
function Invoke-Clorraine([string[]]`$Arguments, [string]`$WorkingDirectory = '') {
    Add-Content -LiteralPath '$quoted' -Value ("clorraine|" + (`$Arguments -join '|'))
    return [pscustomobject]@{ Code = 0; Out = 'Hello, Amiga, from Clorraine!'; Err = '' }
}
function Find-HostCompiler { [pscustomobject]@{ Kind = 'none'; Path = `$null } }
$Extra
"@
}

# The recorded calls of one kind.
function Get-Recorded([string]$Log, [string]$Kind) {
    if (-not (Test-Path -LiteralPath $Log)) { return @() }
    return @(Get-Content -LiteralPath $Log | Where-Object { $_.StartsWith("$Kind|") })
}

# A registration stub for one amiga-gcc version.
function Get-RegistrationStub([string]$Version, [string]$Root) {
    return "function Get-Rc24Registration { [pscustomobject]@{ DisplayVersion = '$Version'; InstallLocation = '$($Root.Replace("'", "''"))\' } }"
}

# The arguments of a toolchain-step run: install the CLI from the fixture,
# skip the emulator, hellos, editor, and starter unless a case adds them.
function Get-FirstRunArguments($Fixture, [string[]]$More = @()) {
    return @('-Archive', $Fixture.Zip, '-Checksums', $Fixture.Sums, '-Prefix', $Fixture.Prefix,
        '-NoEmulator', '-NoHello', '-NoVSCode', '-NoStarter') + $More
}

# A fixture candidate ready to install, without installing it.
function New-Candidate([string]$CaseRoot, [string]$Version = '9.1.0') {
    $zip = New-FixtureZip $CaseRoot $Version
    return [pscustomobject]@{
        Zip = $zip; Sums = (New-Checksums $zip); Installer = (New-AssembledInstaller $CaseRoot $Version)
        Prefix = (Join-Path $CaseRoot 'prefix')
    }
}

$Rc24Url = 'https://github.com/AmigaPorts/m68k-amigaos-gcc/releases/download/v16.2-rc24/m68k-amigaos-gcc-16.2-rc24-x86_64-w64-mingw32ucrt-setup.exe'
$Rc24Sha = '9dd6973539bb2d514fc1b1e6c0a107cac6f30a9c4076cf4014cbe13013c95512'
$Rc24Target = 'C:\Program Files\Clorraine\amiga-gcc-16.2-rc24'
$FsUaeUrl = 'https://github.com/FrodeSolheim/fs-uae/releases/download/v3.2.35/FS-UAE_3.2.35_Windows_x86-64.zip'
$FsUaeSha = 'b402e2802d518a085c878be96c5c0a656fa3dec57dcf0019d226f3285069063e'

Test-Case 'should reuse an existing rc24 registration without downloading or elevating' {
    $root = New-CaseRoot
    $fixture = New-Candidate $root
    $log = Join-Path $root 'calls.log'
    $rc24 = Join-Path $root 'rc24'
    New-Item -ItemType Directory -Force -Path $rc24 | Out-Null
    $prelude = (Get-RecordingPrelude $log) + "`n" + (Get-RegistrationStub '16.2-rc24' $rc24)
    $result = Invoke-Case -Installer $fixture.Installer -CaseRoot $root -Prelude $prelude -Arguments (Get-FirstRunArguments $fixture)
    Assert-That ($result.Code -eq 0) "exit $($result.Code): $($result.All)"
    Assert-That (@(Get-Recorded $log 'download').Count -eq 0) 'downloaded although rc24 is registered'
    Assert-That (@(Get-Recorded $log 'elevate').Count -eq 0) 'asked for elevation although rc24 is registered'
    $setup = @(Get-Recorded $log 'clorraine' | Where-Object { $_ -like 'clorraine|setup|--toolchain|*' })
    Assert-That ($setup.Count -ge 1) 'clorraine setup was not run'
    Assert-That ($setup[0].Split('|')[3] -eq $rc24) "setup used $($setup[0])"
}

Test-Case 'should stop for another amiga-gcc version unless -ReplaceToolchain is given' {
    $root = New-CaseRoot
    $fixture = New-Candidate $root
    $log = Join-Path $root 'calls.log'
    $prelude = (Get-RecordingPrelude $log) + "`n" + (Get-RegistrationStub '16.2-rc23' (Join-Path $root 'old'))
    $result = Invoke-Case -Installer $fixture.Installer -CaseRoot $root -Prelude $prelude -Arguments (Get-FirstRunArguments $fixture)
    Assert-That ($result.Code -eq 1) "exit $($result.Code)"
    Assert-That ($result.All -match '16\.2-rc23' -and $result.All -match '-ReplaceToolchain') $result.All
    Assert-That (@(Get-Recorded $log 'elevate').Count -eq 0) 'elevated without -ReplaceToolchain'
    $result = Invoke-Case -Installer $fixture.Installer -CaseRoot $root -Prelude $prelude `
        -Arguments (Get-FirstRunArguments $fixture @('-ReplaceToolchain'))
    Assert-That ($result.Code -eq 0) "with -ReplaceToolchain: exit $($result.Code): $($result.All)"
    Assert-That (@(Get-Recorded $log 'elevate').Count -eq 1) 'no elevation with -ReplaceToolchain'
}

Test-Case 'should install rc24 only with -InstallToolchain when nothing is registered' {
    $root = New-CaseRoot
    $fixture = New-Candidate $root
    $log = Join-Path $root 'calls.log'
    $prelude = (Get-RecordingPrelude $log) + "`nfunction Get-Rc24Registration { `$null }"
    $result = Invoke-Case -Installer $fixture.Installer -CaseRoot $root -Prelude $prelude -Arguments (Get-FirstRunArguments $fixture)
    Assert-That ($result.Code -eq 0) "exit $($result.Code): $($result.All)"
    Assert-That (@(Get-Recorded $log 'download').Count -eq 0) 'downloaded without consent'
    Assert-That ($result.All -match '-Toolchain') "no -Toolchain next step: $($result.All)"
    $result = Invoke-Case -Installer $fixture.Installer -CaseRoot $root -Prelude $prelude `
        -Arguments (Get-FirstRunArguments $fixture @('-InstallToolchain'))
    Assert-That ($result.Code -eq 0) "with -InstallToolchain: exit $($result.Code): $($result.All)"
    $download = @(Get-Recorded $log 'download')
    Assert-That ($download.Count -eq 1) "downloads: $($download.Count)"
    Assert-That ($download[0] -eq "download|$Rc24Url|48065003|$Rc24Sha") "download: $($download[0])"
    $elevate = @(Get-Recorded $log 'elevate')
    Assert-That ($elevate.Count -eq 1 -and $elevate[0] -eq "elevate|$Rc24Target|48065003|$Rc24Sha") "elevation: $($elevate -join ', ')"
}

Test-Case 'should keep the CLI and print the -Toolchain step when the UAC prompt is declined' {
    $root = New-CaseRoot
    $fixture = New-Candidate $root
    $log = Join-Path $root 'calls.log'
    $declined = 'function Invoke-ElevatedSetup { throw (New-Object System.ComponentModel.Win32Exception 1223) }'
    $prelude = (Get-RecordingPrelude $log $declined) + "`nfunction Get-Rc24Registration { `$null }"
    $result = Invoke-Case -Installer $fixture.Installer -CaseRoot $root -Prelude $prelude `
        -Arguments (Get-FirstRunArguments $fixture @('-InstallToolchain'))
    Assert-That ($result.Code -eq 1) "exit $($result.Code)"
    Assert-That ($result.All -match '-Toolchain') "no -Toolchain next step: $($result.All)"
    Assert-That ($null -ne (Get-JunctionTarget (Join-Path $fixture.Prefix 'current'))) 'the CLI was removed'
}

# A console program that records its raw command line and exits 0.
function Get-FakeSetup([string]$Dir) {
    $path = Join-Path $Dir 'fake-setup.exe'
    $record = (Join-Path $Dir 'setup-args.txt').Replace('\', '\\')
    Add-Type -OutputType ConsoleApplication -OutputAssembly $path -TypeDefinition @"
public static class FakeSetup {
    public static int Main(string[] args) {
        System.IO.File.WriteAllText("$record", System.Environment.CommandLine);
        return 0;
    }
}
"@
    return $path
}

Test-Case 'should verify the setup inside the admin-only folder and run it with the silent options' {
    $root = New-CaseRoot
    $installer = New-AssembledInstaller $root '9.1.0'
    $admin = Join-Path $root 'Program Files\Clorraine'
    $setup = Get-FakeSetup $root
    $bytes = (Get-Item -LiteralPath $setup).Length
    $sha = Get-Sha256 $setup
    $target = Join-Path $admin 'amiga-gcc-16.2-rc24'
    $record = Join-Path $root 'setup-args.txt'
    $call = "exit (Invoke-ElevatedStaging -AdminRoot '$admin' -Source '$setup' -Target '$target' -Bytes $bytes -Sha256 '{0}')"
    $result = Invoke-Case -Installer $installer -CaseRoot $root -Body ($call -f ('0' * 64))
    Assert-That ($result.Code -eq 3) "mismatch exit $($result.Code): $($result.All)"
    Assert-That (-not (Test-Path -LiteralPath $record)) 'the setup ran despite a mismatch'
    $result = Invoke-Case -Installer $installer -CaseRoot $root -Body ($call -f $sha)
    Assert-That ($result.Code -eq 0) "exit $($result.Code): $($result.All)"
    $commandLine = [IO.File]::ReadAllText($record)
    $expected = '/VERYSILENT /SUPPRESSMSGBOXES /NORESTART /NOCLOSEAPPLICATIONS /SP- /DIR="' + $target + '" /LOG="'
    Assert-That ($commandLine.Contains($expected)) "command line: $commandLine"
    Assert-That ($commandLine -match [regex]::Escape($admin + '\.staging-') + '[^\\"]+\\setup\.log"') "log path: $commandLine"
    $staging = @(Get-ChildItem -LiteralPath $admin -Force -Filter '.staging-*')
    Assert-That ($staging.Count -eq 0) 'the staging folder is still present'
}

Test-Case 'should pick FS-UAE from the flag, environment, config, PATH, the prefix, then Program Files' {
    $root = New-CaseRoot
    $installer = New-AssembledInstaller $root '9.1.0'
    $prefix = Join-Path $root 'prefix'
    $places = [ordered]@{
        flag = Join-Path $root 'flag\fs-uae.exe'
        env = Join-Path $root 'env\fs-uae.exe'
        config = Join-Path $root 'config\fs-uae.exe'
        path = Join-Path $root 'onpath\fs-uae.exe'
        prefix = Join-Path $prefix 'fs-uae\3.2.35\FS-UAE\Windows\x86-64\fs-uae.exe'
        programs = Join-Path $root 'programs\FS-UAE\Windows\x86-64\fs-uae.exe'
    }
    foreach ($file in $places.Values) {
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $file) | Out-Null
        Set-Content -LiteralPath $file -Value 'fixture'
    }
    $configFile = Join-Path $root 'appdata\clorraine\config'
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $configFile) | Out-Null
    $order = @($places.Keys)
    for ($i = 0; $i -lt $order.Count; $i++) {
        $flag = if ($i -le 0) { $places.flag } else { '' }
        $lines = @(
            ('$env:CLORRAINE_FS_UAE = ' + $(if ($i -le 1) { "'" + $places.env + "'" } else { '$null' })),
            ('$env:PATH = ' + $(if ($i -le 3) { "'" + (Split-Path -Parent $places.path) + ";' + `$env:PATH" } else { '$env:PATH' })),
            ('$env:ProgramFiles = ''' + (Join-Path $root 'programs') + ''''))
        if ($i -le 2) { Set-Content -LiteralPath $configFile -Value ('fs-uae=' + $places.config) } else { Remove-Item -LiteralPath $configFile -Force -ErrorAction SilentlyContinue }
        if ($i -eq 5) { Remove-Item -LiteralPath $places.prefix -Force }
        $body = ($lines -join "`n") + "`n[Console]::Out.WriteLine((Find-FsUae '$flag' '$prefix')); exit 0"
        $result = Invoke-Case -Installer $installer -CaseRoot $root -Body $body
        $chosen = $result.Out.Trim()
        Assert-That (Test-SamePath $chosen $places[$order[$i]]) "step $($order[$i]): chose '$chosen'"
    }
}

Test-Case 'should download the pinned FS-UAE only with -InstallFsUae and unpack it under the prefix' {
    $root = New-CaseRoot
    $fixture = New-Candidate $root
    $log = Join-Path $root 'calls.log'
    $rc24 = Join-Path $root 'rc24'
    New-Item -ItemType Directory -Force -Path $rc24 | Out-Null
    $zip = Join-Path $root 'fs-uae-fixture.zip'
    Add-Type -AssemblyName System.IO.Compression
    $stream = [IO.File]::Open($zip, 'CreateNew')
    $archive = New-Object IO.Compression.ZipArchive($stream, [IO.Compression.ZipArchiveMode]::Create)
    $entry = $archive.CreateEntry('FS-UAE/Windows/x86-64/fs-uae.exe')
    $writer = $entry.Open(); $writer.WriteByte(77); $writer.Dispose()
    $archive.Dispose(); $stream.Dispose()
    $isolate = "`$env:CLORRAINE_FS_UAE = `$null; `$env:ProgramFiles = '$root\no-programs'; `$env:CLORRAINE_TEST_DOWNLOAD_FROM = '$zip'"
    $prelude = (Get-RecordingPrelude $log $isolate) + "`n" + (Get-RegistrationStub '16.2-rc24' $rc24)
    $base = @('-Archive', $fixture.Zip, '-Checksums', $fixture.Sums, '-Prefix', $fixture.Prefix, '-NoHello', '-NoVSCode', '-NoStarter')
    $result = Invoke-Case -Installer $fixture.Installer -CaseRoot $root -Prelude $prelude -Arguments $base
    Assert-That ($result.Code -eq 0) "exit $($result.Code): $($result.All)"
    Assert-That (@(Get-Recorded $log 'download').Count -eq 0) 'downloaded FS-UAE without consent'
    $result = Invoke-Case -Installer $fixture.Installer -CaseRoot $root -Prelude $prelude -Arguments ($base + '-InstallFsUae')
    Assert-That ($result.Code -eq 0) "with -InstallFsUae: exit $($result.Code): $($result.All)"
    $download = @(Get-Recorded $log 'download')
    Assert-That ($download.Count -eq 1 -and $download[0] -eq "download|$FsUaeUrl|49123213|$FsUaeSha") "download: $($download -join ', ')"
    Assert-That (Test-Path (Join-Path $fixture.Prefix 'fs-uae\3.2.35\FS-UAE\Windows\x86-64\fs-uae.exe')) 'FS-UAE was not unpacked under the prefix'
}

Test-Case 'should choose a 512 KB KS3.1 A1200 ROM and never a 256 KB one' {
    $root = New-CaseRoot
    $installer = New-AssembledInstaller $root '9.1.0'
    $fakeHome = Join-Path $root 'home'
    $roms = Join-Path $fakeHome 'amiga\roms'
    New-Item -ItemType Directory -Force -Path $roms | Out-Null
    $good = Join-Path $roms 'KICK31A1200.ROM'
    [IO.File]::WriteAllBytes($good, (New-Object byte[] 524299))
    [IO.File]::WriteAllBytes((Join-Path $roms 'kick13.rom'), (New-Object byte[] 262144))
    $body = "`$env:CLORRAINE_KICKSTART = `$null; `$env:USERPROFILE = '$fakeHome'; `$env:PUBLIC = '$root\public'; [Console]::Out.WriteLine((Find-Kickstart '')); exit 0"
    $result = Invoke-Case -Installer $installer -CaseRoot $root -Body $body
    Assert-That (Test-SamePath $result.Out.Trim() $good) "chose '$($result.Out.Trim())'"
    Remove-Item -LiteralPath $good -Force
    $result = Invoke-Case -Installer $installer -CaseRoot $root -Body $body
    Assert-That ($result.Out.Trim() -eq '') "chose '$($result.Out.Trim())' without a 512 KB ROM"
}

Test-Case 'should skip the PC hello with a GCC note for Clang only and the Windows hint for none' {
    $root = New-CaseRoot
    $fixture = New-Candidate $root
    $log = Join-Path $root 'calls.log'
    $rc24 = Join-Path $root 'rc24'
    New-Item -ItemType Directory -Force -Path $rc24 | Out-Null
    $arguments = @('-Archive', $fixture.Zip, '-Checksums', $fixture.Sums, '-Prefix', $fixture.Prefix, '-NoEmulator', '-NoVSCode', '-NoStarter')
    $clang = 'function Find-HostCompiler { [pscustomobject]@{ Kind = ''clang''; Path = ''C:\llvm\bin\clang.exe'' } }'
    $prelude = (Get-RecordingPrelude $log $clang) + "`n" + (Get-RegistrationStub '16.2-rc24' $rc24)
    $result = Invoke-Case -Installer $fixture.Installer -CaseRoot $root -Prelude $prelude -Arguments $arguments
    Assert-That ($result.Code -eq 0) "Clang only: exit $($result.Code): $($result.All)"
    Assert-That ($result.All.Contains('Clang found; the PC hello needs GCC (w64devkit or MSYS2), or set CLORRAINE_CC')) $result.All
    Assert-That (@(Get-Recorded $log 'clorraine' | Where-Object { $_ -like 'clorraine|run|*hello.clo' }).Count -eq 0) 'the PC hello ran with Clang'
    $prelude = (Get-RecordingPrelude $log) + "`n" + (Get-RegistrationStub '16.2-rc24' $rc24)
    $result = Invoke-Case -Installer $fixture.Installer -CaseRoot $root -Prelude $prelude -Arguments $arguments
    Assert-That ($result.Code -eq 0) "no compiler: exit $($result.Code): $($result.All)"
    Assert-That ($result.All.Contains('install GCC (for example w64devkit or MSYS2 UCRT64) or set CLORRAINE_CC')) $result.All
}

# --- Summary -----------------------------------------------------------------

Get-ChildItem -LiteralPath "HKCU:\$ScratchKeyBase" -ErrorAction SilentlyContinue | Remove-Item -Recurse -Force
Remove-Item -LiteralPath "HKCU:\$ScratchKeyBase" -Force -ErrorAction SilentlyContinue
# The suite root holds only fixtures and junctions this suite created; rmdir
# removes junctions without following them.
cmd /c rmdir /s /q "`"$SuiteRoot`"" | Out-Null
Write-Output ("{0} passed, {1} failed" -f $script:Passed, $script:Failed)
if ($script:Failed -gt 0) { exit 1 }
exit 0
