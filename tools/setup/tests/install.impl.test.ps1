# Implementation tests for the Windows installer's internals.
#
# Run on Windows 11 with Windows PowerShell 5.1, from the repository root:
#   powershell -NoProfile -ExecutionPolicy Bypass -File tools\setup\tests\install.impl.test.ps1
#
# The installer is dot-sourced with CLORRAINE_INSTALL_LIB=1, so its functions
# are defined without running the install. Cases call them directly: zip
# validation, checksum lines, native argument quoting, PATH edits on unusual
# values, the reparse-safe walker, and the abandoned-lock path. PATH edits use
# a scratch registry key; nothing needs the network or administrator rights.
#
# Exit code: 0 when every case passes, 1 otherwise.
param([string]$Filter = '')

$ErrorActionPreference = 'Stop'

$Here = Split-Path -Parent $MyInvocation.MyCommand.Path
$Template = Join-Path (Split-Path -Parent $Here) 'install.ps1'
$SuiteRoot = Join-Path ([IO.Path]::GetTempPath()) ('clorraine-install-impl-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
$ScratchKey = 'Software\ClorraineInstallImpl\' + [guid]::NewGuid().ToString('N').Substring(0, 8)
New-Item -ItemType Directory -Path $SuiteRoot | Out-Null

$env:CLORRAINE_INSTALL_LIB = '1'
. $Template
$script:Version = '9.1.0'
$script:PathRegistryKey = "HKCU:\$ScratchKey"

$script:Passed = 0
$script:Failed = 0

# Fails the current case with a message.
function Assert-That([bool]$Condition, [string]$Message) {
    if (-not $Condition) { throw $Message }
}

# Runs one named case and records the result.
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

# The exit code a Stop-Install failure carries, or $null when Block succeeds.
function Get-StopCode([scriptblock]$Block) {
    try { & $Block | Out-Null; return $null } catch { return $_.Exception.Data['ClorraineExitCode'] }
}

# A fresh folder for one case.
function New-Dir([string]$Name) {
    $path = Join-Path $SuiteRoot $Name
    New-Item -ItemType Directory -Force -Path $path | Out-Null
    return $path
}

# Builds a zip from name -> text pairs, in order.
function New-Zip([string]$Path, [string[]]$Names) {
    Add-Type -AssemblyName System.IO.Compression
    $stream = [IO.File]::Open($Path, 'CreateNew')
    $zip = New-Object IO.Compression.ZipArchive($stream, [IO.Compression.ZipArchiveMode]::Create)
    try {
        foreach ($name in $Names) {
            $entry = $zip.CreateEntry($name)
            if (-not $name.EndsWith('/')) {
                $writer = $entry.Open(); $writer.WriteByte(65); $writer.Dispose()
            }
        }
    } finally { $zip.Dispose(); $stream.Dispose() }
    return $Path
}

# The complete entry list of a valid 9.1.0 candidate.
function Get-ValidNames {
    $top = 'clorraine-9.1.0-x86_64-windows'
    return @($script:CandidateFiles | ForEach-Object { "$top/$_" })
}

# --- Zip validation ----------------------------------------------------------

Test-Case 'should accept a complete candidate with directory entries' {
    $dir = New-Dir 'zip-ok'
    $names = @('clorraine-9.1.0-x86_64-windows/', 'clorraine-9.1.0-x86_64-windows/bin/') + (Get-ValidNames)
    $zip = New-Zip (Join-Path $dir 'ok.zip') $names
    Assert-That ((Test-CandidateZip $zip) -eq 'clorraine-9.1.0-x86_64-windows') 'wrong top folder'
}

Test-Case 'should reject drive, rooted, dot, and backslash-parent entries' {
    $dir = New-Dir 'zip-bad'
    $index = 0
    foreach ($bad in 'C:/x.txt', '/x.txt', 'clorraine-9.1.0-x86_64-windows/./x', 'clorraine-9.1.0-x86_64-windows\..\x') {
        $index++
        $zip = New-Zip (Join-Path $dir "bad$index.zip") ((Get-ValidNames) + $bad)
        Assert-That ((Get-StopCode { Test-CandidateZip $zip }) -eq 1) "accepted $bad"
    }
}

Test-Case 'should reject a missing file, an extra file, and a second top folder' {
    $dir = New-Dir 'zip-set'
    $valid = Get-ValidNames
    $missing = New-Zip (Join-Path $dir 'missing.zip') ($valid | Where-Object { $_ -notlike '*/NOTICE' })
    $extra = New-Zip (Join-Path $dir 'extra.zip') ($valid + 'clorraine-9.1.0-x86_64-windows/bin/extra.dll')
    $other = New-Zip (Join-Path $dir 'other.zip') ($valid + 'other/README.md')
    foreach ($zip in $missing, $extra, $other) {
        Assert-That ((Get-StopCode { Test-CandidateZip $zip }) -eq 1) "accepted $(Split-Path -Leaf $zip)"
    }
}

# --- Checksums and quoting ---------------------------------------------------

Test-Case 'should read a unique checksum line in text or binary form and refuse duplicates' {
    $dir = New-Dir 'sums'
    $digest = 'ab' * 32
    $text = Join-Path $dir 'text.sums'
    Set-Content -LiteralPath $text -Value ("$digest  a.zip", ('cd' * 32) + '  b.zip') -Encoding Ascii
    Assert-That ((Get-RecordedDigest $text 'a.zip') -eq $digest) 'text form'
    $binary = Join-Path $dir 'binary.sums'
    Set-Content -LiteralPath $binary -Value ($digest.ToUpperInvariant() + ' *a.zip') -Encoding Ascii
    Assert-That ((Get-RecordedDigest $binary 'a.zip') -eq $digest) 'binary form, uppercase digest'
    $twice = Join-Path $dir 'twice.sums'
    Set-Content -LiteralPath $twice -Value ("$digest  a.zip", "$digest  a.zip") -Encoding Ascii
    Assert-That ((Get-StopCode { Get-RecordedDigest $twice 'a.zip' }) -eq 1) 'accepted a duplicate line'
    Assert-That ((Get-StopCode { Get-RecordedDigest $text 'c.zip' }) -eq 1) 'accepted a missing name'
}

Test-Case 'should quote native arguments so the child sees each one unchanged' {
    $cases = @('plain', 'with space', 'quote"inside', 'trailing\', 'C:\a b\', 'a\\"b', '')
    # A C# program receives its arguments split by the standard Windows rules.
    $probe = Join-Path (New-Dir 'quote') 'echo-args.exe'
    Add-Type -OutputType ConsoleApplication -OutputAssembly $probe -TypeDefinition @'
public static class EchoArgs {
    public static int Main(string[] args) {
        foreach (string arg in args) { System.Console.WriteLine("<" + arg + ">"); }
        return 0;
    }
}
'@
    $result = Invoke-Native $probe $cases
    $seen = @($result.Out -split "`r?`n" | Where-Object { $_ -ne '' })
    for ($i = 0; $i -lt $cases.Count; $i++) {
        Assert-That ($seen[$i] -eq ('<' + $cases[$i] + '>')) "argument $i arrived as $($seen[$i])"
    }
}

# --- PATH --------------------------------------------------------------------

# Sets or clears the scratch Path value.
function Set-ScratchPath($Value, [Microsoft.Win32.RegistryValueKind]$Kind = 'String') {
    $key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($ScratchKey)
    try {
        if ($null -eq $Value) { $key.DeleteValue('Path', $false) } else { $key.SetValue('Path', $Value, $Kind) }
    } finally { $key.Dispose() }
}

Test-Case 'should create the PATH value when it is absent and keep a plain value plain' {
    Set-ScratchPath $null
    Add-UserPathEntry 'C:\P\current\bin'
    $path = Get-UserPath
    Assert-That ($path.Value -eq 'C:\P\current\bin') "value: $($path.Value)"
    Assert-That ($path.Kind -eq [Microsoft.Win32.RegistryValueKind]::String) "kind: $($path.Kind)"
    Set-ScratchPath 'C:\a;' 'String'
    Add-UserPathEntry 'C:\P\current\bin'
    Assert-That ((Get-UserPath).Value -eq 'C:\a;C:\P\current\bin') "trailing separator: $((Get-UserPath).Value)"
}

Test-Case 'should leave the PATH value alone when there is nothing to remove' {
    Set-ScratchPath 'C:\a;;C:\b' 'ExpandString'
    Remove-UserPathEntry 'C:\P\current\bin'
    $path = Get-UserPath
    Assert-That ($path.Value -eq 'C:\a;;C:\b') "value changed: $($path.Value)"
    Assert-That ($path.Kind -eq [Microsoft.Win32.RegistryValueKind]::ExpandString) "kind changed: $($path.Kind)"
}

Test-Case 'should remove every duplicate of the entry' {
    Set-ScratchPath 'C:\P\current\bin;C:\a;c:\p\CURRENT\bin\' 'String'
    Remove-UserPathEntry 'C:\P\current\bin'
    Assert-That ((Get-UserPath).Value -eq 'C:\a') "value: $((Get-UserPath).Value)"
}

# --- Deletion ----------------------------------------------------------------

Test-Case 'should delete a tree with read-only files and nested junctions without following them' {
    $outside = New-Dir 'walk-outside'
    Set-Content -LiteralPath (Join-Path $outside 'keep.txt') 'keep'
    $tree = New-Dir 'walk-tree'
    $deep = Join-Path $tree 'a\b'
    New-Item -ItemType Directory -Force -Path $deep | Out-Null
    $readonly = Join-Path $deep 'ro.txt'
    Set-Content -LiteralPath $readonly 'x'
    (Get-Item -LiteralPath $readonly).IsReadOnly = $true
    New-Item -ItemType Junction -Path (Join-Path $deep 'link') -Target $outside | Out-Null
    New-Item -ItemType Junction -Path (Join-Path $tree 'top-link') -Target $outside | Out-Null
    Remove-TreeSafe $tree
    Assert-That (-not (Test-Path -LiteralPath $tree)) 'the tree is still present'
    Assert-That (Test-Path -LiteralPath (Join-Path $outside 'keep.txt')) 'the walker followed a junction'
}

Test-Case 'should remove a junction whose target is gone' {
    $target = New-Dir 'gone-target'
    $link = Join-Path $SuiteRoot 'gone-link'
    New-Item -ItemType Junction -Path $link -Target $target | Out-Null
    [IO.Directory]::Delete($target, $false)
    Remove-TreeSafe $link
    Assert-That ($null -eq (Get-Item -LiteralPath $link -Force -ErrorAction SilentlyContinue)) 'the broken junction remains'
}

Test-Case 'should treat only a junction into versions as an owned link' {
    $root = New-Dir 'owned'
    $versions = New-Dir 'owned\versions'
    $inside = New-Dir 'owned\versions\9.1.0-abc'
    $nested = New-Dir 'owned\versions\9.1.0-abc\bin'
    $plain = New-Dir 'owned\plain'
    New-Item -ItemType Junction -Path (Join-Path $root 'good') -Target $inside | Out-Null
    New-Item -ItemType Junction -Path (Join-Path $root 'deep') -Target $nested | Out-Null
    Assert-That (Test-OwnedLink (Join-Path $root 'good') $versions) 'a version junction is not owned'
    Assert-That (-not (Test-OwnedLink (Join-Path $root 'deep') $versions)) 'a junction deeper than a version is owned'
    Assert-That (-not (Test-OwnedLink $plain $versions)) 'a plain folder is owned'
}

# --- Lock ----------------------------------------------------------------------

Test-Case 'should take over a lock abandoned by a killed installer' {
    $script:MutexName = 'Local\Clorraine-Install-Impl-' + [guid]::NewGuid().ToString('N').Substring(0, 8)
    $powershell = Join-Path $PSHOME 'powershell.exe'
    # The child takes the mutex and exits without releasing it.
    $child = Invoke-Native $powershell @('-NoProfile', '-Command',
        "`$m = New-Object System.Threading.Mutex(`$true, '$script:MutexName'); exit 0")
    Assert-That ($child.Code -eq 0) 'the child did not run'
    $mutex = Lock-Installer
    try {
        Assert-That ($null -ne $mutex) 'no lock returned'
    } finally {
        $mutex.ReleaseMutex(); $mutex.Dispose()
    }
}

# --- Rollback ----------------------------------------------------------------

Test-Case 'should restore the previous current link when the switch fails' {
    $root = New-Dir 'rollback'
    $versions = New-Dir 'rollback\versions'
    $old = New-Dir 'rollback\versions\9.0.0-old'
    New-Item -ItemType Junction -Path (Join-Path $root 'current') -Target $old | Out-Null
    $zip = New-Zip (Join-Path $root 'c.zip') (Get-ValidNames)
    # A failing switch: the extracted tree passes, then the link step throws.
    function Test-ExtractedVersion([string]$Tree) { }
    function Set-CurrentVersion([string]$Root, [string]$Folder) {
        Remove-Link (Join-Path $Root 'current')
        throw 'stubbed switch failure'
    }
    $code = Get-StopCode { Install-Archive $root $zip }
    Assert-That ($null -ne (Get-JunctionTarget (Join-Path $root 'current'))) 'current is missing after the failure'
    Assert-That (Test-SamePath (Get-JunctionTarget (Join-Path $root 'current')) $old) 'current does not target the old version'
    $left = @(Get-ChildItem -LiteralPath $versions -Force | Where-Object { $_.Name -ne '9.0.0-old' })
    Assert-That ($left.Count -eq 0) ('left behind: ' + (($left | ForEach-Object { $_.Name }) -join ', '))
}

# --- Summary -----------------------------------------------------------------

[Microsoft.Win32.Registry]::CurrentUser.DeleteSubKeyTree('Software\ClorraineInstallImpl', $false)
cmd /c rmdir /s /q "`"$SuiteRoot`"" | Out-Null
Write-Output ("{0} passed, {1} failed" -f $script:Passed, $script:Failed)
if ($script:Failed -gt 0) { exit 1 }
exit 0
