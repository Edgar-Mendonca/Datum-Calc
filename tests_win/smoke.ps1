# Installs Datum Calc on a clean Windows machine the way a user does, and checks it works:
# the previous release first, then this build over it (as the in-app updater does), the publisher,
# the .dcalc file association, the real window (driven through WebView2's debugging port),
# recovery after a crash, and a clean uninstall.
# pwsh tests_win/smoke.ps1 -Installer path\to\DatumCalc-x.y.z-setup.exe -Version x.y.z
param([Parameter(Mandatory)] [string] $Installer, [Parameter(Mandatory)] [string] $Version)
$ErrorActionPreference = "Stop"
$failed = 0
function Check($name, $ok, $info = "") { if ($ok) { "ok   $name" } else { "FAIL $name  $info"; $script:failed++ } }
function App { Get-ItemProperty "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*" -ErrorAction SilentlyContinue | Where-Object DisplayName -eq "Datum Calc" }
$data = Join-Path $env:APPDATA "com.edgarmendonca.datumcalc"
$Installer = (Resolve-Path $Installer).Path

# 1. The previous release, from GitHub, as a user has it today.
$rel = Invoke-RestMethod "https://api.github.com/repos/Edgar-Mendonca/Datum-Calc/releases/latest" -Headers @{ "User-Agent" = "datum-calc-ci" }
$asset = $rel.assets | Where-Object name -like "*-setup.exe" | Select-Object -First 1
$old = Join-Path $env:RUNNER_TEMP $asset.name
Invoke-WebRequest $asset.browser_download_url -OutFile $old
Start-Process $old -ArgumentList "/S" -Wait
Check "the previous release ($($rel.tag_name)) installs" ($null -ne (App))

# 2. This build over it, silently, as the updater runs it.
Start-Process $Installer -ArgumentList "/S" -Wait
$a = App
Check "this build installs over it" ($a.DisplayVersion -eq $Version) "version $($a.DisplayVersion)"
Check "Windows lists the publisher as Datum" ($a.Publisher -eq "Datum") "publisher $($a.Publisher)"
$dir = $a.InstallLocation.Trim('"')
$exe = Get-ChildItem $dir -Filter *.exe | Where-Object Name -notlike "uninstall*" | Select-Object -First 1
Check "the program is in place" ($null -ne $exe) $dir
$assoc = Get-Item "HKCU:\Software\Classes\.dcalc" -ErrorAction SilentlyContinue
Check ".dcalc files open with Datum Calc" ($null -ne $assoc)

# 3. The real window: a sheet "double-clicked" (passed on the command line), an edit, print preview, a recovery copy.
$sheet = Join-Path $env:RUNNER_TEMP "AC circuit.dcalc"
Copy-Item "crates/core/packs/ac-circuit.dcalc" $sheet
$env:DATUM_CALC_TEST_PORT = "9222"
function Start-App($arg) {
  $p = if ($arg) { Start-Process $exe.FullName -ArgumentList "`"$arg`"" -PassThru } else { Start-Process $exe.FullName -PassThru }
  for ($i = 0; $i -lt 60; $i++) { try { Invoke-RestMethod "http://127.0.0.1:9222/json/version" -TimeoutSec 2 | Out-Null; return $p } catch { Start-Sleep 1 } }
  throw "The window did not open its debugging port within a minute."
}
$p = Start-App $sheet
node tests_win/ui.cjs first $Version
Check "the window works (first start)" ($LASTEXITCODE -eq 0)
$copies = @(Get-ChildItem (Join-Path $data "recovery") -Filter *.dcalc -ErrorAction SilentlyContinue)
Check "a recovery copy of unsaved work is kept" ($copies.Count -ge 1)
Stop-Process -Id $p.Id -Force   # a crash
Start-Sleep 3

# 4. After the crash: the copy is offered, Discard removes it, a pack opens.
$p = Start-App $null
node tests_win/ui.cjs recover $Version
Check "the window works (after a crash)" ($LASTEXITCODE -eq 0)
$copies = @(Get-ChildItem (Join-Path $data "recovery") -Filter *.dcalc -ErrorAction SilentlyContinue)
Check "Discard removes the recovery copy" ($copies.Count -eq 0) ($copies | ForEach-Object Name)
Stop-Process -Id $p.Id -Force
Start-Sleep 3

# 5. Uninstall leaves no program behind.
$un = (App).UninstallString.Trim('"')
Start-Process $un -ArgumentList "/S" -Wait
# The uninstaller hands over to a copy of itself, so wait for the program to go.
for ($i = 0; $i -lt 60 -and ((Test-Path $exe.FullName) -or ($null -ne (App))); $i++) { Start-Sleep 1 }
Check "uninstall removes the program" ((-not (Test-Path $exe.FullName)) -and ($null -eq (App)))

if ($failed) { "$failed failed"; exit 1 } else { "all passed" }
