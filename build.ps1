<#
    Wrapper de compilation pour notepad-rs.

    Pourquoi ce script ?
    --------------------
    La cible Rust `x86_64-pc-windows-gnu` a besoin de `dlltool` et de `gcc`.
    Le GCC 6.3 de `C:\MinGW\bin` (32 bits) est present dans le PATH *machine*,
    donc avant WinLibs (GCC 16.2, UCRT, PATH utilisateur) : rustc l'utilise et
    echoue avec « Invalid bfd target » des qu'il doit fabriquer une import
    library 64 bits (`raw-dylib`, utilise par la crate `windows`).

    Ce script prepend simplement le bin de WinLibs au PATH du processus, sans
    modifier la configuration systeme.

    Usage :
        .\build.ps1              # cargo build
        .\build.ps1 test         # cargo test
        .\build.ps1 check        # cargo check
        .\build.ps1 build -Release
        .\build.ps1 run
#>
param(
    [ValidateSet("check", "test", "build", "run", "clean")]
    [string]$Task = "build",
    [switch]$Release,
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$Rest
)

$ErrorActionPreference = "Stop"

$winlibs = Join-Path $env:LOCALAPPDATA "Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\mingw64\bin"

if (-not (Test-Path (Join-Path $winlibs "gcc.exe"))) {
    throw "WinLibs introuvable : $winlibs`nInstallez-le avec : winget install BrechtSanders.WinLibs.POSIX.UCRT"
}

$env:PATH = "$winlibs;$env:PATH"

# Verifie que c'est bien le GCC recent qui sera utilise.
$gccVersion = (& (Join-Path $winlibs "gcc.exe") --version | Select-Object -First 1)
Write-Host "Linker : $gccVersion" -ForegroundColor DarkGray

$cargoArgs = @($Task)
if ($Task -eq "build" -and $Release) { $cargoArgs += "--release" }
$cargoArgs += $Rest

# Cargo ecrit sa progression sur stderr, meme quand tout reussit.
# Sous Windows PowerShell 5.1, `2>&1 | Out-Host` transforme ces lignes
# en ErrorRecord (NativeCommandError). Laisser les flux natifs separes
# conserve l'affichage normal et les diagnostics reels de Cargo.
# PowerShell 7 : le code de retour, capture ci-dessous, fait autorite.
$PSNativeCommandUseErrorActionPreference = $false
& cargo @cargoArgs
$code = $LASTEXITCODE
exit $code
