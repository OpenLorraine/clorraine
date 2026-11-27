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

# The suite works in a folder only this user, SYSTEM, and Administrators can
# open, as the installer keeps its own folders, whatever %TEMP% grants others.
$suiteAcl = New-Object System.Security.AccessControl.DirectorySecurity
$suiteAcl.SetAccessRuleProtection($true, $false)
foreach ($sid in [Security.Principal.WindowsIdentity]::GetCurrent().User.Value, 'S-1-5-18', 'S-1-5-32-544') {
    $principal = New-Object System.Security.Principal.SecurityIdentifier $sid
    $suiteAcl.AddAccessRule((New-Object System.Security.AccessControl.FileSystemAccessRule($principal, 'FullControl', 'ContainerInherit, ObjectInherit', 'None', 'Allow')))
}
[IO.Directory]::SetAccessControl($SuiteRoot, $suiteAcl)

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

Test-Case 'should reject a link entry and a repeated entry' {
    $dir = New-Dir 'zip-types'
    $link = Join-Path $dir 'link.zip'
    Add-Type -AssemblyName System.IO.Compression
    $stream = [IO.File]::Open($link, 'CreateNew')
    $zip = New-Object IO.Compression.ZipArchive($stream, [IO.Compression.ZipArchiveMode]::Create)
    foreach ($name in Get-ValidNames) {
        $entry = $zip.CreateEntry($name)
        $writer = $entry.Open(); $writer.WriteByte(65); $writer.Dispose()
        # A Unix symbolic link (type 0xA000 in the high half of the attributes).
        if ($name -like '*/bin/clorraine.exe') { $entry.ExternalAttributes = 0xA1FF -shl 16 }
    }
    $zip.Dispose(); $stream.Dispose()
    Assert-That ((Get-StopCode { Test-CandidateZip $link }) -eq 1) 'accepted a link entry'
    $repeated = New-Zip (Join-Path $dir 'repeated.zip') ((Get-ValidNames) + 'clorraine-9.1.0-x86_64-windows/README.md')
    Assert-That ((Get-StopCode { Test-CandidateZip $repeated }) -eq 1) 'accepted a repeated entry'
    $case = New-Zip (Join-Path $dir 'case.zip') ((Get-ValidNames) + 'clorraine-9.1.0-x86_64-windows/readme.md')
    Assert-That ((Get-StopCode { Test-CandidateZip $case }) -eq 1) 'accepted a name differing only in case'
}

Test-Case 'should refuse an extracted tree with an extra file, a missing file, or a link' {
    $outside = New-Dir 'tree-outside'
    foreach ($shape in 'extra', 'missing', 'link') {
        $staging = New-Dir "tree-$shape"
        $top = Join-Path $staging 'clorraine-9.1.0-x86_64-windows'
        foreach ($file in $script:CandidateFiles) {
            $path = Join-Path $top $file.Replace('/', '\')
            New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
            Set-Content -LiteralPath $path 'x'
        }
        switch ($shape) {
            'extra' { Set-Content -LiteralPath (Join-Path $top 'bin\version.dll') 'x' }
            'missing' { Remove-Item -LiteralPath (Join-Path $top 'NOTICE') }
            'link' { New-Item -ItemType Junction -Path (Join-Path $top 'bin\more') -Target $outside | Out-Null }
        }
        Assert-That ((Get-StopCode { Test-ExtractedTree $staging 'clorraine-9.1.0-x86_64-windows' }) -eq 1) "accepted the $shape tree"
    }
    $good = New-Dir 'tree-good'
    foreach ($file in $script:CandidateFiles) {
        $path = Join-Path $good ('clorraine-9.1.0-x86_64-windows\' + $file.Replace('/', '\'))
        New-Item -ItemType Directory -Force -Path (Split-Path -Parent $path) | Out-Null
        Set-Content -LiteralPath $path 'x'
    }
    Assert-That ($null -eq (Get-StopCode { Test-ExtractedTree $good 'clorraine-9.1.0-x86_64-windows' })) 'refused a good tree'
}

# --- Command line and prefix -------------------------------------------------

Test-Case 'should report an unknown option as a usage error' {
    Assert-That ((Invoke-Installer -Unistall 6>$null 2>$null) -eq 2) 'Invoke-Installer accepted -Unistall'
    $powershell = Join-Path $PSHOME 'powershell.exe'
    $child = Invoke-Native $powershell @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-Command',
        "`$env:CLORRAINE_INSTALL_LIB = `$null; & '$Template' -Prefix '$SuiteRoot\unknown-prefix' -NoToolchian; exit `$LASTEXITCODE")
    Assert-That ($child.Code -eq 2) "the script exited $($child.Code): $($child.Err)"
    Assert-That ($child.Err -match "unknown option '-NoToolchian'") $child.Err
}

Test-Case 'should refuse a percent sign in the prefix' {
    Assert-That ((Get-StopCode { Resolve-Prefix 'C:\x%TEMP%y' }) -eq 1) 'accepted a percent sign'
}

Test-Case 'should give new private folders an access list of their own' {
    $path = New-PrivateTemp 'impl-acl'
    try {
        $acl = Get-Acl -LiteralPath $path
        Assert-That ($acl.AreAccessRulesProtected) 'the folder still inherits its parent rules'
        $me = [System.Security.Principal.WindowsIdentity]::GetCurrent().User
        $mine = @($acl.Access | Where-Object {
            $_.IdentityReference.Translate([System.Security.Principal.SecurityIdentifier]) -eq $me -and
            $_.FileSystemRights -band [System.Security.AccessControl.FileSystemRights]::FullControl })
        Assert-That ($mine.Count -ge 1) 'the current user has no full control'
        $others = @($acl.Access | Where-Object {
            $sid = $_.IdentityReference.Translate([System.Security.Principal.SecurityIdentifier]).Value
            $sid -ne $me.Value -and $sid -ne 'S-1-5-18' -and $sid -ne 'S-1-5-32-544' })
        Assert-That ($others.Count -eq 0) ('other principals: ' + (($others | ForEach-Object { $_.IdentityReference }) -join ', '))
    } finally { Remove-TreeSafe $path }
}

# --- Purge -------------------------------------------------------------------

Test-Case 'should purge only what the installer created and keep the user files beside it' {
    $root = New-Dir 'purge-mixed'
    $versions = New-Dir 'purge-mixed\versions'
    New-Dir 'purge-mixed\versions\9.1.0-0123456789ab\bin' | Out-Null
    New-Dir 'purge-mixed\versions\.tmp-0123456789ab' | Out-Null
    New-Dir 'purge-mixed\versions\mine' | Out-Null
    Set-Content -LiteralPath (Join-Path $versions 'mine\notes.txt') 'keep'
    $examples = New-Dir 'purge-mixed\examples'
    Set-Content -LiteralPath (Join-Path $examples 'hello.clo') 'x'
    Set-Content -LiteralPath (Join-Path $examples 'hello-amiga.clo') 'x'
    Set-Content -LiteralPath (Join-Path $examples 'keep.txt') 'keep'
    $userFsUae = New-Dir 'purge-mixed-user-fs-uae'
    New-Dir 'purge-mixed-user-fs-uae\3.2.35' | Out-Null
    Set-Content -LiteralPath (Join-Path $userFsUae '3.2.35\fs-uae.exe') 'user'
    New-Item -ItemType Junction -Path (Join-Path $root 'fs-uae') -Target $userFsUae | Out-Null
    Remove-InstallerData $root
    Assert-That (-not (Test-Path (Join-Path $versions '9.1.0-0123456789ab'))) 'the version folder remains'
    Assert-That (-not (Test-Path (Join-Path $versions '.tmp-0123456789ab'))) 'the staging folder remains'
    Assert-That (Test-Path (Join-Path $versions 'mine\notes.txt')) 'a user folder in versions was removed'
    Assert-That (-not (Test-Path (Join-Path $examples 'hello.clo'))) 'hello.clo remains'
    Assert-That (Test-Path (Join-Path $examples 'keep.txt')) 'a user file in examples was removed'
    Assert-That (Test-Path (Join-Path $userFsUae '3.2.35\fs-uae.exe')) 'purge followed the fs-uae junction'
}

Test-Case 'should refuse and keep a current item that is not a Clorraine link' {
    $root = New-Dir 'foreign-current'
    $foreign = Join-Path $root 'current'
    Set-Content -LiteralPath $foreign 'the user file'
    $zip = New-Zip (Join-Path $root 'c.zip') (Get-ValidNames)
    function Test-ExtractedVersion([string]$Tree) { }
    $code = Get-StopCode { Install-Archive $root $zip }
    Assert-That ($code -eq 1) "exit code $code"
    Assert-That ((Get-Content -LiteralPath $foreign) -eq 'the user file') 'the user file named current was removed'
    $left = @(Get-ChildItem -LiteralPath (Join-Path $root 'versions') -Force)
    Assert-That ($left.Count -eq 0) ('left behind: ' + (($left | ForEach-Object { $_.Name }) -join ', '))
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

# --- Toolchain and emulator ----------------------------------------------------

Test-Case 'should send the staging function and its values through the encoded elevated command' {
    # A straight and a typographic single quote in the path: both delimit a
    # PowerShell single-quoted literal and must be escaped.
    $dir = New-Dir ("elevate-o'brien-" + [char]0x2019 + 'x')
    $setup = Join-Path $dir 'fake-setup.exe'
    $record = Join-Path $dir 'args.txt'
    Add-Type -OutputType ConsoleApplication -OutputAssembly $setup -TypeDefinition (@'
public static class FakeSetupImpl {
    public static int Main(string[] args) {
        System.IO.File.WriteAllText(@"RECORD", System.Environment.CommandLine);
        return 7;
    }
}
'@).Replace('RECORD', $record.Replace('"', '""'))
    $admin = Join-Path $dir "Programs\Clorraine"
    $text = Get-ElevatedCommand $setup (Get-Item $setup).Length (Get-Sha256 $setup) $admin
    $errors = $null
    [void][System.Management.Automation.Language.Parser]::ParseInput($text, [ref]$null, [ref]$errors)
    Assert-That ($errors.Count -eq 0) "the elevated command does not parse: $($errors | Select-Object -First 1)"
    $encoded = [Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($text))
    $powershell = Join-Path $PSHOME 'powershell.exe'
    $run = Invoke-Native $powershell @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-EncodedCommand', $encoded)
    Assert-That ($run.Code -eq 7) "the setup's exit code did not come back: $($run.Code) $($run.Err)"
    $commandLine = [IO.File]::ReadAllText($record)
    $target = Join-Path $admin 'amiga-gcc-16.2-rc24'
    Assert-That ($commandLine.Contains('/DIR="' + $target + '"')) "the target changed on the way: $commandLine"
    Assert-That (@(Get-ChildItem -LiteralPath $admin -Force -Filter '.staging-*').Count -eq 0) 'staging left behind'
}

Test-Case 'should run the setup from a protected staging folder only the trusted accounts can open' {
    $dir = New-Dir 'elevate-acl'
    $setup = Join-Path $dir 'acl-setup.exe'
    $record = Join-Path $dir 'acl.txt'
    # The stub records the access list of the folder it runs from.
    Add-Type -OutputType ConsoleApplication -OutputAssembly $setup -TypeDefinition (@'
using System.Security.AccessControl;
using System.Security.Principal;
public static class AclSetupImpl {
    public static int Main(string[] args) {
        string here = System.IO.Path.GetDirectoryName(System.Reflection.Assembly.GetEntryAssembly().Location);
        DirectorySecurity security = System.IO.Directory.GetAccessControl(here);
        System.Text.StringBuilder text = new System.Text.StringBuilder(security.AreAccessRulesProtected.ToString());
        foreach (FileSystemAccessRule rule in security.GetAccessRules(true, true, typeof(SecurityIdentifier))) {
            text.Append("|" + rule.IdentityReference.Value + "=" + rule.AccessControlType);
        }
        System.IO.File.WriteAllText(@"RECORD", text.ToString());
        return 0;
    }
}
'@).Replace('RECORD', $record.Replace('"', '""'))
    $admin = Join-Path $dir 'Programs\Clorraine'
    $code = Invoke-ElevatedStaging -Source $setup -Bytes (Get-Item $setup).Length -Sha256 (Get-Sha256 $setup) -AdminRoot $admin
    Assert-That ($code -eq 0) "staging exit $code"
    $me = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    $parts = [IO.File]::ReadAllText($record).Split('|')
    Assert-That ($parts[0] -eq 'True') 'the staging folder inherits its parent access list'
    $expected = @('S-1-5-18=Allow', 'S-1-5-32-544=Allow', "$me=Allow") | Sort-Object
    $actual = @($parts | Select-Object -Skip 1) | Sort-Object
    Assert-That (($actual -join ',') -ceq ($expected -join ',')) "staging access list: $($actual -join ', ')"
}

Test-Case 'should create a missing admin folder that users can read and run but not change' {
    $dir = New-Dir 'elevate-new-root'
    $admin = Join-Path $dir 'Clorraine'
    $setup = Join-Path $dir 'never.exe'
    [IO.File]::WriteAllText($setup, 'not run')
    # The wrong digest stops before the setup runs; the folder is made first.
    $code = Invoke-ElevatedStaging -Source $setup -Bytes (Get-Item $setup).Length -Sha256 ('0' * 64) -AdminRoot $admin
    Assert-That ($code -eq 101) "staging exit $code"
    $acl = Get-Acl -LiteralPath $admin
    Assert-That $acl.AreAccessRulesProtected 'the admin folder inherits its parent access list'
    $me = [Security.Principal.WindowsIdentity]::GetCurrent().User.Value
    $rules = @($acl.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier]) |
        ForEach-Object { $_.IdentityReference.Value + '=' + $_.FileSystemRights })
    $expected = @('S-1-5-18=FullControl', 'S-1-5-32-544=FullControl', "$me=FullControl", 'S-1-5-32-545=ReadAndExecute, Synchronize') | Sort-Object
    Assert-That ((($rules | Sort-Object) -join ',') -ceq ($expected -join ',')) "admin folder access list: $($rules -join ', ')"
    # A second run accepts the folder it created.
    $again = Invoke-ElevatedStaging -Source $setup -Bytes (Get-Item $setup).Length -Sha256 ('0' * 64) -AdminRoot $admin
    Assert-That ($again -eq 101) "second run exit $again"
}

Test-Case 'should stop before downloading when the admin folder path has a space' {
    $root = New-Dir 'space-root'
    $code = $null; $message = ''
    try {
        & {
            function Get-Rc24AdminRoot { 'C:\Program Files\Clorraine' }
            function Invoke-Download { throw 'downloaded' }
            Install-Rc24 $root
        } | Out-Null
    } catch { $code = $_.Exception.Data['ClorraineExitCode']; $message = $_.Exception.Message }
    Assert-That ($code -eq 1) "exit $code, $message"
    Assert-That ($message -match 'contains a space') "message: $message"
}

Test-Case 'should check a folder another account creates between the lookup and the create' {
    $dir = New-Dir 'elevate-race'
    $admin = Join-Path $dir 'Clorraine'
    $setup = Join-Path $dir 'never.exe'
    [IO.File]::WriteAllText($setup, 'not run')
    $script:RaceLookups = 0
    $code = & {
        # The first lookup of the admin folder reports it missing; right then
        # another account makes it with a rule that lets everyone change it.
        function Get-Item {
            param([string]$LiteralPath, [switch]$Force, $ErrorAction)
            if ($LiteralPath -eq $admin -and $script:RaceLookups -eq 0) {
                $script:RaceLookups++
                [void][IO.Directory]::CreateDirectory($admin)
                $acl = Microsoft.PowerShell.Security\Get-Acl -LiteralPath $admin
                $everyone = New-Object System.Security.Principal.SecurityIdentifier 'S-1-1-0'
                $acl.AddAccessRule((New-Object System.Security.AccessControl.FileSystemAccessRule($everyone, 'Modify', 'ContainerInherit, ObjectInherit', 'None', 'Allow')))
                Microsoft.PowerShell.Security\Set-Acl -LiteralPath $admin -AclObject $acl
                return $null
            }
            Microsoft.PowerShell.Management\Get-Item -LiteralPath $LiteralPath -Force:$Force -ErrorAction SilentlyContinue
        }
        Invoke-ElevatedStaging -Source $setup -Bytes (Get-Item $setup).Length -Sha256 ('0' * 64) -AdminRoot $admin
    }
    Assert-That ($script:RaceLookups -eq 1) 'the race hook did not run'
    Assert-That ($code -eq 103) "a folder made in the gap was used: exit $code"
    Assert-That (@(Get-ChildItem -LiteralPath $admin -Force).Count -eq 0) 'the setup was staged in the foreign folder'
}

Test-Case 'should offer to reinstall a registered rc24 whose folder has a space' {
    $root = New-Dir 'space-registration'
    $result = & {
        function Get-Rc24Registration { [pscustomobject]@{ DisplayVersion = '16.2-rc24'; InstallLocation = 'C:\Program Files\Clorraine\amiga-gcc-16.2-rc24\' } }
        function Install-Rc24([string]$Root) { 'reinstalled' }
        $kept = $null; $stop = $null
        try { $kept = Select-Toolchain $root @{} $false } catch { $stop = $_.Exception.Message }
        $replaced = Select-Toolchain $root @{ ReplaceToolchain = $true } $false
        [pscustomobject]@{ Stop = $stop; Kept = $kept; Replaced = $replaced }
    }
    Assert-That ($null -eq $result.Kept -and $result.Stop -match 'uninstall amiga-gcc') "without a flag: kept=$($result.Kept) stop=$($result.Stop)"
    Assert-That ($result.Replaced -eq 'reinstalled') "with -ReplaceToolchain: $($result.Replaced)"
}

Test-Case 'should skip an encrypted ROM whose rom.key is missing' {
    $docs = New-Dir 'rom-key-docs'
    $kick = Join-Path $docs 'FS-UAE\Kickstarts'
    New-Item -ItemType Directory -Force -Path $kick | Out-Null
    $rom = Join-Path $kick 'amiga-os-310-a1200.rom'
    $bytes = New-Object byte[] 524299
    [Text.Encoding]::ASCII.GetBytes('AMIROMTYPE1').CopyTo($bytes, 0)
    [IO.File]::WriteAllBytes($rom, $bytes)
    $find = {
        $env:CLORRAINE_KICKSTART = $null
        $env:USERPROFILE = Join-Path $docs 'no-profile'
        $env:PUBLIC = Join-Path $docs 'no-public'
        function Get-DocumentsFolder { $docs }
        function Get-ConfigValue([string]$Key) { $null }
        Find-Kickstart ''
    }
    $saved = @($env:CLORRAINE_KICKSTART, $env:USERPROFILE, $env:PUBLIC)
    try {
        $without = & $find
        [IO.File]::WriteAllText((Join-Path $kick 'rom.key'), 'key')
        $with = & $find
    } finally { $env:CLORRAINE_KICKSTART, $env:USERPROFILE, $env:PUBLIC = $saved }
    Assert-That ($null -eq $without) "an encrypted ROM without its key was chosen: $without"
    Assert-That ($with -eq $rom) "the ROM with its key was not chosen: $with"
}

Test-Case 'should refuse an admin folder that is a link, a file, foreign-owned, or changeable by others' {
    $dir = New-Dir 'elevate-untrusted'
    $setup = Join-Path $dir 'never.exe'
    [IO.File]::WriteAllText($setup, 'not run')
    $bytes = (Get-Item $setup).Length
    $sha = Get-Sha256 $setup
    $real = New-Dir 'elevate-untrusted-target'
    $link = Join-Path $dir 'link-root'
    New-Item -ItemType Junction -Path $link -Target $real | Out-Null
    Assert-That ((Invoke-ElevatedStaging -Source $setup -Bytes $bytes -Sha256 $sha -AdminRoot $link) -eq 103) 'a junction root was used'
    $file = Join-Path $dir 'file-root'
    [IO.File]::WriteAllText($file, 'x')
    Assert-That ((Invoke-ElevatedStaging -Source $setup -Bytes $bytes -Sha256 $sha -AdminRoot $file) -eq 103) 'a file root was used'
    # The folder is owned by the current user, who is not trusted here.
    $owned = New-Dir 'elevate-untrusted-owned'
    Assert-That ((Invoke-ElevatedStaging -Source $setup -Bytes $bytes -Sha256 $sha -AdminRoot $owned -TrustedSid @()) -eq 103) 'a foreign-owned root was used'
    $open = New-Dir 'elevate-untrusted-open'
    $acl = Get-Acl -LiteralPath $open
    $everyone = New-Object System.Security.Principal.SecurityIdentifier 'S-1-1-0'
    $acl.AddAccessRule((New-Object System.Security.AccessControl.FileSystemAccessRule($everyone, 'Modify', 'ContainerInherit, ObjectInherit', 'None', 'Allow')))
    Set-Acl -LiteralPath $open -AclObject $acl
    Assert-That ((Invoke-ElevatedStaging -Source $setup -Bytes $bytes -Sha256 $sha -AdminRoot $open) -eq 103) 'a root anyone may change was used'
    # Control: the same kind of folder without the extra rule passes the checks
    # and reaches the copy, which the wrong digest then refuses.
    $private = New-Dir 'elevate-untrusted-control'
    Assert-That ((Invoke-ElevatedStaging -Source $setup -Bytes $bytes -Sha256 ('0' * 64) -AdminRoot $private) -eq 101) 'a private root was refused'
    foreach ($root in $real, $owned, $open) {
        Assert-That (@(Get-ChildItem -LiteralPath $root -Force).Count -eq 0) "a refused root was written: $root"
    }
}

Test-Case 'should drop a lone carriage return but keep line breaks in multi-line text' {
    $text = "abc`rxyz`r`nnext"
    Assert-That ((Get-SafeText $text) -ceq "abcxyz`r`nnext") "multi-line: $(Get-SafeText $text)"
    Assert-That ((Get-SafeText $text -SingleLine) -ceq 'abc xyz  next') "single line: $(Get-SafeText $text -SingleLine)"
}

Test-Case 'should take ProgramData from the system and not from the environment' {
    $saved = $env:ProgramData
    $env:ProgramData = Join-Path $SuiteRoot 'not-program-data'
    try {
        $root = Get-Rc24AdminRoot
    } finally { $env:ProgramData = $saved }
    $expected = Join-Path ([Environment]::GetFolderPath('CommonApplicationData')) 'Clorraine'
    Assert-That (Test-SamePath $root $expected) "the admin folder followed the environment: $root"
    Assert-That (-not $root.Contains(' ')) "the admin folder has a space: $root"
    $text = Get-ElevatedCommand 'C:\x\setup.exe' 1 ('0' * 64)
    Assert-That (-not $text.Contains('-AdminRoot') -and -not $text.Contains('-Target')) 'the elevated call passes a folder'
}

Test-Case 'should recognize a declined UAC prompt in the wrapping a .NET method call produces' {
    $declined = New-Object System.ComponentModel.Win32Exception 1223
    $wrapped = New-Object System.Management.Automation.MethodInvocationException('Exception calling "Start"', $declined)
    $record = New-Object System.Management.Automation.ErrorRecord($wrapped, 'Start', 'NotSpecified', $null)
    Assert-That (Test-DeclinedElevation $record) 'a wrapped 1223 was not recognized'
    $other = New-Object System.Management.Automation.ErrorRecord((New-Object System.InvalidOperationException 'canceled'), 'x', 'NotSpecified', $null)
    Assert-That (-not (Test-DeclinedElevation $other)) 'an unrelated error counted as a decline'
}

Test-Case 'should read the amiga-gcc registration and report none when it is absent' {
    $saved = $script:Rc24RegistryHive, $script:Rc24UninstallKey
    $script:Rc24RegistryHive = 'CurrentUser'
    $script:Rc24UninstallKey = "$ScratchKey\Uninstall\{8D184346-D038-45E9-98A1-43BFC542BE34}_is1"
    try {
        Assert-That ($null -eq (Get-Rc24Registration)) 'found a registration in an empty key'
        $key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($script:Rc24UninstallKey)
        $key.SetValue('DisplayVersion', '16.2-rc24')
        $key.SetValue('InstallLocation', 'C:\Tools\rc24\')
        $key.Dispose()
        $found = Get-Rc24Registration
        Assert-That ($found.DisplayVersion -eq '16.2-rc24') "version: $($found.DisplayVersion)"
        Assert-That ($found.InstallLocation -eq 'C:\Tools\rc24\') "location: $($found.InstallLocation)"
        $chosen = Select-Toolchain 'C:\unused' @{} $false
        Assert-That ($chosen -eq 'C:\Tools\rc24') "the trailing backslash was kept: $chosen"
    } finally {
        $script:Rc24RegistryHive, $script:Rc24UninstallKey = $saved
    }
}

Test-Case 'should prefer a usual ROM name in any folder to a name pattern and skip wrong sizes' {
    $saved = $env:USERPROFILE, $env:PUBLIC, $env:CLORRAINE_KICKSTART, $env:APPDATA
    $fake = New-Dir 'rom-home'
    $env:USERPROFILE = $fake
    $env:PUBLIC = Join-Path $fake 'public'
    $env:CLORRAINE_KICKSTART = $null
    $env:APPDATA = New-Dir 'rom-appdata'
    function Get-DocumentsFolder { Join-Path $fake 'Documents' }
    try {
        $roms = New-Dir 'rom-home\amiga\roms'
        $kickstarts = New-Dir 'rom-home\Documents\FS-UAE\Kickstarts'
        # A matching name of the wrong size is never chosen.
        [IO.File]::WriteAllBytes((Join-Path $roms 'kick31-trimmed.rom'), (New-Object byte[] 1000))
        Assert-That ($null -eq (Find-Kickstart '')) "chose a ROM of the wrong size: $(Find-Kickstart '')"
        [IO.File]::WriteAllBytes((Join-Path $roms 'my-A1200-dump.rom'), (New-Object byte[] 524288))
        $exact = Join-Path $kickstarts 'kick31.rom'
        [IO.File]::WriteAllBytes($exact, (New-Object byte[] 524288))
        Assert-That (Test-SamePath (Find-Kickstart '') $exact) "chose $(Find-Kickstart '')"
        Remove-Item -LiteralPath $exact
        Assert-That (Test-SamePath (Find-Kickstart '') (Join-Path $roms 'my-A1200-dump.rom')) "pattern: $(Find-Kickstart '')"
        New-Item -ItemType Directory -Force -Path (Join-Path $env:APPDATA 'clorraine') | Out-Null
        Set-Content -LiteralPath (Join-Path $env:APPDATA 'clorraine\config') -Value 'kickstart=C:\remembered.rom'
        Assert-That ((Find-Kickstart '') -eq 'C:\remembered.rom') 'the remembered ROM was not preferred'
        Assert-That ((Find-Kickstart 'C:\flag.rom') -eq 'C:\flag.rom') 'the flag was not preferred'
    } finally {
        $env:USERPROFILE, $env:PUBLIC, $env:CLORRAINE_KICKSTART, $env:APPDATA = $saved
    }
}

Test-Case 'should run VS Code through its Electron binary so cmd.exe never parses an argument' {
    $vscode = New-Dir 'vscode-fake'
    $bin = New-Dir 'vscode-fake\bin'
    $out = New-Dir 'vscode-fake\07f806f999\resources\app\out'
    Set-Content -LiteralPath (Join-Path $out 'cli.js') -Value '// cli'
    $record = Join-Path $vscode 'seen.txt'
    Add-Type -OutputType ConsoleApplication -OutputAssembly (Join-Path $vscode 'Code.exe') -TypeDefinition (@'
public static class FakeCode {
    public static int Main(string[] args) {
        System.IO.File.WriteAllLines(@"RECORD", new string[] {
            System.Environment.GetEnvironmentVariable("ELECTRON_RUN_AS_NODE") ?? "", string.Join("|", args) });
        return 0;
    }
}
'@).Replace('RECORD', $record.Replace('"', '""'))
    $launcher = Join-Path $bin 'code.cmd'
    Set-Content -LiteralPath $launcher -Encoding Ascii -Value @(
        '@echo off', 'setlocal', 'set VSCODE_DEV=', 'set ELECTRON_RUN_AS_NODE=1',
        '"%~dp0..\Code.exe" "%~dp0..\07f806f999\resources\app\out\cli.js" %*', 'endlocal')
    $hostile = 'C:\work\R&D %PATH% ^x'
    $result = Invoke-Editor $launcher @('--install-extension', $hostile)
    Assert-That ($null -ne $result -and $result.Code -eq 0) 'the editor was not run directly'
    $seen = Get-Content -LiteralPath $record
    Assert-That ($seen[0] -eq '1') 'ELECTRON_RUN_AS_NODE was not set'
    $expected = (Join-Path $out 'cli.js') + '|--install-extension|' + $hostile
    Assert-That ($seen[1] -eq $expected) "arguments changed: $($seen[1])"
    $odd = Join-Path $bin 'odd.cmd'
    Set-Content -LiteralPath $odd -Value '@echo off & start notepad'
    Assert-That ($null -eq (Invoke-Editor $odd @('x'))) 'ran a launcher of an unexpected shape'
}

Test-Case 'should refuse an FS-UAE archive with an unsafe entry before unpacking it' {
    $root = New-Dir 'fsuae-bad'
    $zip = New-Zip (Join-Path $root 'bad.zip') @('FS-UAE/Windows/x86-64/fs-uae.exe', '../evil.txt')
    function Invoke-Download([string]$Url, [string]$Destination, [long]$Bytes, [string]$Sha256, [int]$MaxSeconds) {
        Copy-Item -LiteralPath $zip -Destination $Destination
    }
    $message = ''
    try { Install-FsUae $root | Out-Null } catch { $message = $_.Exception.Message }
    # The installer's own check refuses it; tar.exe never runs.
    Assert-That ($message -match 'the FS-UAE archive contains an unsafe entry') "not refused by the entry check: $message"
    Assert-That (-not (Test-Path (Join-Path $root 'fs-uae\3.2.35'))) 'unpacked anyway'
    Assert-That (-not (Test-Path (Join-Path (Split-Path -Parent $root) 'evil.txt'))) 'wrote outside the prefix'
}

# --- Summary -----------------------------------------------------------------

[Microsoft.Win32.Registry]::CurrentUser.DeleteSubKeyTree('Software\ClorraineInstallImpl', $false)
cmd /c rmdir /s /q "`"$SuiteRoot`"" | Out-Null
Write-Output ("{0} passed, {1} failed" -f $script:Passed, $script:Failed)
if ($script:Failed -gt 0) { exit 1 }
exit 0
