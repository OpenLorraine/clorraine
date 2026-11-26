# Run the Windows acceptance path from the exact candidate and write evidence.
#
# Run in Windows PowerShell 5.1 in the signed-in desktop session, as the
# standard account under test:
#
#   powershell -NoProfile -ExecutionPolicy Bypass -File tools\setup\acceptance.ps1 `
#       -Candidate DIR -Kickstart ROM -Output FILE [-UacConfirmed] [-EditorConfirmed]
#
# It copies the candidate's zip, install.ps1, VSIX, and SHA256SUMS into a
# private folder beside FILE, checks them, and installs with -Archive and
# -Checksums into an isolated prefix and APPDATA. The first install may ask
# for administrator approval to install rc24; a second install of the same
# candidate must reuse that registration without asking. The script then runs
# the PC hello with GCC, the Amiga hello, the four hosted-amiga corpus
# programs, file-copier on hosted-amiga-13, and the return-1 negative in
# FS-UAE, and writes FILE as JSON for `make acceptance-check`. The ROM is
# recorded by name and SHA-256 only. -UacConfirmed and -EditorConfirmed record
# what the person saw: the UAC prompt and a Build/Run in FS-UAE from VS Code.
#
# This runs the self-built candidate installer with the caller's privileges.
# Do not point it at an untrusted candidate.

[CmdletBinding(PositionalBinding = $false)]
param(
    [Parameter(Mandatory = $true)][string]$Candidate,
    [Parameter(Mandatory = $true)][string]$Kickstart,
    [Parameter(Mandatory = $true)][string]$Output,
    [switch]$UacConfirmed,
    [switch]$EditorConfirmed
)

Set-StrictMode -Version 2.0
$ErrorActionPreference = 'Stop'

$Hello = 'Hello, Amiga, from Clorraine!'
$Repo = Split-Path -Parent (Split-Path -Parent $PSScriptRoot)
$CorpusNames = 'file-copier', 'string-parser', 'buffer-container', 'volatile-mmio'

function Stop-Acceptance([int]$Code, [string]$Message) {
    [Console]::Error.WriteLine("acceptance: $Message")
    exit $Code
}

function Write-Note([string]$Text) { [Console]::Out.WriteLine("acceptance: $Text") }

function Get-Sha256([string]$Path) {
    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

# The SHA-256 of text after removing carriage returns, as the checker hashes
# the corpus goldens.
function Get-NormalizedSha256([string]$Path) {
    $bytes = [IO.File]::ReadAllBytes($Path) | Where-Object { $_ -ne 13 }
    $sha = [Security.Cryptography.SHA256]::Create()
    try { $hash = $sha.ComputeHash([byte[]]@($bytes)) } finally { $sha.Dispose() }
    return (($hash | ForEach-Object { $_.ToString('x2') }) -join '')
}

# Quotes one argument so the child's command-line parser sees it unchanged.
function ConvertTo-Argument([string]$Text) {
    if ($Text -ne '' -and $Text -notmatch '[\s"]') { return $Text }
    $builder = New-Object Text.StringBuilder
    [void]$builder.Append('"')
    $slashes = 0
    foreach ($char in $Text.ToCharArray()) {
        if ($char -eq [char]'\') { $slashes++; continue }
        if ($char -eq [char]'"') { [void]$builder.Append([char]'\', 2 * $slashes + 1) }
        elseif ($slashes -gt 0) { [void]$builder.Append([char]'\', $slashes) }
        $slashes = 0
        [void]$builder.Append($char)
    }
    if ($slashes -gt 0) { [void]$builder.Append([char]'\', 2 * $slashes) }
    [void]$builder.Append('"')
    return $builder.ToString()
}

# Runs a program with no console input, so the installer takes its
# non-interactive path, and returns its exit code and both streams.
function Invoke-Child([string]$File, [string[]]$Arguments, [string]$Directory) {
    $info = New-Object Diagnostics.ProcessStartInfo
    $info.FileName = $File
    $info.Arguments = (@($Arguments) | ForEach-Object { ConvertTo-Argument $_ }) -join ' '
    $info.WorkingDirectory = $Directory
    $info.UseShellExecute = $false
    $info.RedirectStandardInput = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    foreach ($name in 'CLORRAINE_TOOLCHAIN', 'CLORRAINE_FS_UAE', 'CLORRAINE_KICKSTART') {
        if ($info.EnvironmentVariables.ContainsKey($name)) { $info.EnvironmentVariables.Remove($name) }
    }
    $process = [Diagnostics.Process]::Start($info)
    $process.StandardInput.Close()
    $out = $process.StandardOutput.ReadToEndAsync()
    $err = $process.StandardError.ReadToEndAsync()
    $process.WaitForExit()
    return [pscustomobject]@{ Code = $process.ExitCode; Out = $out.Result; Err = $err.Result }
}

# Creates a folder only this user, SYSTEM, and Administrators can open.
function New-PrivateFolder([string]$Path) {
    $security = New-Object Security.AccessControl.DirectorySecurity
    $security.SetAccessRuleProtection($true, $false)
    $inherit = [Security.AccessControl.InheritanceFlags]'ContainerInherit, ObjectInherit'
    foreach ($sid in [Security.Principal.WindowsIdentity]::GetCurrent().User.Value, 'S-1-5-18', 'S-1-5-32-544') {
        $principal = New-Object Security.Principal.SecurityIdentifier $sid
        $security.AddAccessRule((New-Object Security.AccessControl.FileSystemAccessRule($principal, 'FullControl', $inherit, 'None', 'Allow')))
    }
    [void][IO.Directory]::CreateDirectory($Path, $security)
}

function Assert-NoEmulator([string]$Step) {
    if (@(Get-Process -Name 'fs-uae' -ErrorAction SilentlyContinue).Count -ne 0) {
        Stop-Acceptance 1 "$Step left an FS-UAE process"
    }
}

# --- Inputs ------------------------------------------------------------------

$build = [int](Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').CurrentBuildNumber
$Candidate = [IO.Path]::GetFullPath($Candidate)
$Kickstart = [IO.Path]::GetFullPath($Kickstart)
$Output = [IO.Path]::GetFullPath($Output)
if (-not (Test-Path -LiteralPath $Candidate -PathType Container)) { Stop-Acceptance 2 "missing candidate folder $Candidate" }
$zips = @(Get-ChildItem -LiteralPath $Candidate -File -Filter 'clorraine-*-x86_64-windows.zip')
if ($zips.Count -ne 1) { Stop-Acceptance 2 "the candidate must hold exactly one Windows zip: $Candidate" }
$zipName = $zips[0].Name
$version = $zipName.Substring('clorraine-'.Length, $zipName.Length - 'clorraine-'.Length - '-x86_64-windows.zip'.Length)
$vsixName = "clorraine-$version.vsix"
foreach ($name in $zipName, 'install.ps1', $vsixName, 'SHA256SUMS') {
    if (-not (Test-Path -LiteralPath (Join-Path $Candidate $name) -PathType Leaf)) { Stop-Acceptance 2 "missing $name in $Candidate" }
}
if (-not (Test-Path -LiteralPath $Kickstart -PathType Leaf)) { Stop-Acceptance 2 "missing Kickstart ROM $Kickstart" }
foreach ($name in 'examples\hello\hello.clo') {
    if (-not (Test-Path -LiteralPath (Join-Path $Repo $name) -PathType Leaf)) { Stop-Acceptance 2 "missing $name under $Repo" }
}
foreach ($name in $CorpusNames) {
    if (-not (Test-Path -LiteralPath (Join-Path $Repo "spec\tests\corpus\$name\program.clo") -PathType Leaf)) {
        Stop-Acceptance 2 "missing the $name corpus program under $Repo"
    }
}
Assert-NoEmulator 'an earlier run'

# --- Private copies ------------------------------------------------------------

$stamp = (Get-Date).ToUniversalTime().ToString('yyyyMMddTHHmmssZ')
$out = Join-Path (Split-Path -Parent $Output) "acceptance-$stamp"
New-PrivateFolder $out
$work = Join-Path $out 'work'
foreach ($name in 'prefix', 'appdata', 'tmp', 'run') { [void][IO.Directory]::CreateDirectory((Join-Path $work $name)) }
foreach ($name in $zipName, 'install.ps1', $vsixName, 'SHA256SUMS') {
    Copy-Item -LiteralPath (Join-Path $Candidate $name) -Destination (Join-Path $out $name)
}
$zip = Join-Path $out $zipName
$installer = Join-Path $out 'install.ps1'
$vsix = Join-Path $out $vsixName
$sums = Join-Path $out 'SHA256SUMS'
$listed = @{}
foreach ($line in [IO.File]::ReadAllLines($sums)) {
    if ($line -match '^([0-9a-f]{64}) [ *](.+)$') { $listed[$Matches[2]] = $Matches[1] }
}
foreach ($name in $zipName, 'install.ps1', $vsixName) {
    if (-not $listed.ContainsKey($name)) { Stop-Acceptance 1 "SHA256SUMS does not list $name" }
    if ((Get-Sha256 (Join-Path $out $name)) -ne $listed[$name]) { Stop-Acceptance 1 "$name does not match SHA256SUMS" }
}

$prefix = Join-Path $work 'prefix'
$env:APPDATA = Join-Path $work 'appdata'
$env:TEMP = Join-Path $work 'tmp'
$env:TMP = $env:TEMP
$powershell = Join-Path ([Environment]::SystemDirectory) 'WindowsPowerShell\v1.0\powershell.exe'
$installArgs = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $installer, '-Archive', $zip,
    '-Checksums', $sums, '-Prefix', $prefix, '-Kickstart', $Kickstart, '-NoVSCode', '-NoStarter')

# --- Install twice -------------------------------------------------------------

Write-Note 'install the candidate (approve the rc24 UAC prompt if it appears)'
$first = Invoke-Child $powershell ($installArgs + @('-InstallToolchain', '-InstallFsUae')) $work
[IO.File]::WriteAllText((Join-Path $out 'install-1.log'), $first.Out + $first.Err)
if ($first.Code -ne 0) { Stop-Acceptance 1 "the first install exited $($first.Code); see install-1.log" }
$cli = Join-Path $prefix 'current\bin\clorraine.exe'
$reported = (Invoke-Child $cli @('--version') $work).Out.Trim()
if ($reported -ne "clorraine $version") { Stop-Acceptance 1 "installed version '$reported' is not clorraine $version" }

Write-Note 'install the same candidate again; it must reuse rc24'
$second = Invoke-Child $powershell ($installArgs + @('-NoHello')) $work
[IO.File]::WriteAllText((Join-Path $out 'install-2.log'), $second.Out + $second.Err)
if ($second.Code -ne 0) { Stop-Acceptance 1 "the second install exited $($second.Code); see install-2.log" }
$reused = $second.Out.Contains('using the installed rc24 toolchain in') -and -not $second.Out.Contains('installed rc24 in')

# The installer verified the rc24 setup and the FS-UAE zip against its pins
# before using them, so the record carries those pins once the registration
# and the configured emulator show which ones were used. The pins are read
# from the installer's text; none of its code runs here.
$installerText = [IO.File]::ReadAllText($installer)
function Get-Pin([string]$Name) {
    $match = [regex]::Match($installerText, '(?m)^\$script:' + $Name + " = '?([^'\r\n]*)'?\r?$")
    if (-not $match.Success) { Stop-Acceptance 1 "install.ps1 has no $Name pin" }
    return $match.Groups[1].Value
}
$rc24Version = Get-Pin 'Rc24Version'
$base = [Microsoft.Win32.RegistryKey]::OpenBaseKey('LocalMachine', 'Registry64')
try {
    $key = $base.OpenSubKey((Get-Pin 'Rc24UninstallKey'))
    $registered = if ($null -ne $key) { try { [string]$key.GetValue('DisplayVersion') } finally { $key.Dispose() } } else { '' }
} finally { $base.Dispose() }
if ($registered -ne $rc24Version) { Stop-Acceptance 1 "rc24 $rc24Version is not registered after the install" }
$configured = ''
$configFile = Join-Path $env:APPDATA 'clorraine\config'
if (Test-Path -LiteralPath $configFile -PathType Leaf) {
    foreach ($line in [IO.File]::ReadAllLines($configFile, [Text.Encoding]::UTF8)) {
        if ($line.StartsWith('fs-uae=')) { $configured = $line.Substring('fs-uae='.Length) }
    }
}
if (-not $configured) { Stop-Acceptance 1 'the install did not configure FS-UAE' }
$fsUaeExe = [IO.Path]::GetFullPath($configured)
$downloaded = [IO.Path]::GetFullPath((Join-Path $prefix ('fs-uae\3.2.35\' + (Get-Pin 'FsUaeExe'))))
$fsInfo = [Diagnostics.FileVersionInfo]::GetVersionInfo($fsUaeExe)
$fsUae = [ordered]@{ version = "$($fsInfo.FileMajorPart).$($fsInfo.FileMinorPart).$($fsInfo.FileBuildPart)" }
if ($fsUaeExe -ieq $downloaded) {
    $fsUae.source = 'downloaded'; $fsUae.sha256 = Get-Pin 'FsUaeSha256'; $fsUae.bytes = [long](Get-Pin 'FsUaeBytes')
} else {
    $fsUae.source = 'existing'
}

# --- PC hello --------------------------------------------------------------------

Write-Note 'run the PC hello with GCC'
$run = Join-Path $work 'run'
$helloSource = Join-Path $Repo 'examples\hello\hello.clo'
$gcc = if ($env:CLORRAINE_CC) { $env:CLORRAINE_CC } else { (Get-Command gcc.exe -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1).Source }
if (-not $gcc) { Stop-Acceptance 1 'the PC hello needs GCC on PATH or CLORRAINE_CC' }
$compiler = ((Invoke-Child $gcc @('--version') $run).Out -split "`n")[0].Trim()
$hostRun = Invoke-Child $cli @('run', $helloSource) $run
[IO.File]::WriteAllText((Join-Path $out 'host-hello.txt'), $hostRun.Out + $hostRun.Err)
$hostResult = if ($hostRun.Code -eq 0 -and $hostRun.Out.Replace("`r", '').Trim() -ceq $Hello) { 'ok' } else { 'failed' }

# --- FS-UAE matrix ---------------------------------------------------------------

function Invoke-Guest([string]$Name, [string]$CompileProfile, [string]$Source, [string]$Expected) {
    Write-Note "run $Name on $CompileProfile in FS-UAE"
    $result = Invoke-Child $cli @('run', '--profile', $CompileProfile, '--emulator', $Source) $run
    $text = Join-Path $out "$Name.txt"
    [IO.File]::WriteAllText($text, $result.Out)
    [IO.File]::AppendAllText((Join-Path $out 'fs-uae.log'), $result.Err)
    if ($result.Code -ne 0) { Stop-Acceptance 1 "$Name exited $($result.Code): $($result.Err)" }
    $want = [IO.File]::ReadAllText($Expected).Replace("`r", '')
    if ($result.Out.Replace("`r", '') -cne $want) { Stop-Acceptance 1 "$Name output does not match $Expected" }
    Assert-NoEmulator $Name
    return Get-NormalizedSha256 $text
}

$helloExpected = Join-Path $out 'hello.expected'
[IO.File]::WriteAllText($helloExpected, "$Hello`n")
[void](Invoke-Guest 'hello' 'hosted-amiga' $helloSource $helloExpected)
$guestOutput = [IO.File]::ReadAllText((Join-Path $out 'hello.txt')).Replace("`r", '').TrimEnd("`n")
$corpus = New-Object Collections.ArrayList
foreach ($name in $CorpusNames) {
    $dir = Join-Path $Repo "spec\tests\corpus\$name"
    $sha = Invoke-Guest $name 'hosted-amiga' (Join-Path $dir 'program.clo') (Join-Path $dir 'expected-output.txt')
    [void]$corpus.Add([ordered]@{ name = $name; profile = 'hosted-amiga'; status = 'ok'; output_sha256 = $sha })
}
$dir = Join-Path $Repo 'spec\tests\corpus\file-copier'
$sha = Invoke-Guest 'file-copier-a500' 'hosted-amiga-13' (Join-Path $dir 'program.clo') (Join-Path $dir 'expected-output.txt')
[void]$corpus.Add([ordered]@{ name = 'file-copier'; profile = 'hosted-amiga-13'; status = 'ok'; output_sha256 = $sha })

Write-Note 'run the return-1 negative'
$negativeSource = Join-Path $work 'return-one.clo'
[IO.File]::WriteAllText($negativeSource, "module return_one;`nfunc main(): i32 {`n    return 1;`n}`n")
$negative = Invoke-Child $cli @('run', '--profile', 'hosted-amiga', '--emulator', $negativeSource) $run
[IO.File]::AppendAllText((Join-Path $out 'fs-uae.log'), $negative.Err)
if ($negative.Code -eq 0) { Stop-Acceptance 1 'the return-1 negative passed' }
if (-not $negative.Err.Contains('exited with code 1')) { Stop-Acceptance 1 'the negative did not report code 1' }
$retained = [regex]::Matches($negative.Err, "retained at '([^']*)'") | Select-Object -Last 1
if ($null -eq $retained) { Stop-Acceptance 1 'the negative did not retain its run' }
$retainedPath = $retained.Groups[1].Value
if (-not [IO.Path]::GetFullPath($retainedPath).StartsWith($work + '\', [StringComparison]::OrdinalIgnoreCase)) {
    Stop-Acceptance 1 "the retained run is outside the private work tree: $retainedPath"
}
$rc = [IO.File]::ReadAllText((Join-Path $retainedPath 'drive\clorraine-rc.txt')).Trim()
if ($rc -ne '1') { Stop-Acceptance 1 'the negative did not record rc 1' }
if (Test-Path -LiteralPath (Join-Path $retainedPath 'drive\clorraine-status.txt')) { Stop-Acceptance 1 'the negative wrote a success status' }
Assert-NoEmulator 'the negative'

# --- Evidence --------------------------------------------------------------------

$administrators = New-Object Security.Principal.SecurityIdentifier 'S-1-5-32-544'
$standard = -not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole($administrators)
$sha = [Security.Cryptography.SHA256]::Create()
try { $helloHash = (($sha.ComputeHash([Text.Encoding]::UTF8.GetBytes($guestOutput)) | ForEach-Object { $_.ToString('x2') }) -join '') } finally { $sha.Dispose() }
$record = [ordered]@{
    version = $version
    platform = 'windows'
    architecture = 'x86_64'
    os_build = $build
    archive = [ordered]@{ name = $zipName; sha256 = (Get-Sha256 $zip); bytes = (Get-Item -LiteralPath $zip).Length }
    installer = [ordered]@{ name = 'install.ps1'; sha256 = (Get-Sha256 $installer) }
    vsix = [ordered]@{ name = $vsixName; sha256 = (Get-Sha256 $vsix); bytes = (Get-Item -LiteralPath $vsix).Length }
    toolchain = [ordered]@{ tag = "v$rc24Version"; sha256 = (Get-Pin 'Rc24Sha256'); bytes = [long](Get-Pin 'Rc24Bytes') }
    kickstart = [ordered]@{ name = (Split-Path -Leaf $Kickstart); sha256 = (Get-Sha256 $Kickstart) }
    fs_uae = $fsUae
    guest = [ordered]@{ status = 'ok'; rc = 0; output = $guestOutput; output_sha256 = $helloHash }
    negative = [ordered]@{ status = 'failed'; rc = $negative.Code }
    corpus = @($corpus)
    host_hello = [ordered]@{ compiler = $compiler; result = $hostResult }
    existing_registration = $(if ($reused) { 'reused' } else { 'not-reused' })
    standard_account = $(if ($standard) { 'installed' } else { 'administrator' })
    manual = [ordered]@{ uac_prompt_seen = [bool]$UacConfirmed; editor_build_run = [bool]$EditorConfirmed }
    timestamp = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
}
$json = $record | ConvertTo-Json -Depth 6
[IO.File]::WriteAllText($Output, $json + "`n", (New-Object Text.UTF8Encoding $false))
Write-Note "wrote $Output (raw output in $out)"
exit 0
