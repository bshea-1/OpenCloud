# OpenCloud Suite Installer for Windows
$ErrorActionPreference = "Stop"

$apps = @(
    "photocraft",
    "vectorcraft",
    "filmcraft",
    "lightcraft",
    "pdfcraft",
    "effectcraft",
    "designcraft",
    "soundcraft",
    "cadcraft",
    "deckcraft",
    "gridcraft",
    "wordcraft"
)

$installDir = "$env:LOCALAPPDATA\Programs\OpenCloud"
if (!(Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

$tempDir = [System.IO.Path]::Combine([System.IO.Path]::GetTempPath(), [System.Guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

Write-Host "==> Starting OpenCloud Suite Installation for Windows..." -ForegroundColor Cyan

foreach ($app in $apps) {
    Write-Host "`n--- Checking latest release: $app ---" -ForegroundColor Yellow
    $apiUrl = "https://api.github.com/repos/storytold/$app/releases/latest"
    
    try {
        $release = Invoke-RestMethod -Uri $apiUrl -Headers @{ "User-Agent" = "OpenCloud-Installer" }
        $asset = $release.assets | Where-Object { $_.name -like "*-windows-x64-portable.zip" } | Select-Object -First 1
        
        if (-not $asset) {
            $asset = $release.assets | Where-Object { $_.name -like "*-windows-x64.msi" } | Select-Object -First 1
        }

        if (-not $asset) {
            Write-Warning "No compatible Windows x64 asset found for $app."
            continue
        }

        $destFile = Join-Path $tempDir $asset.name
        Write-Host "  --> Downloading $($asset.name)..." -ForegroundColor Gray
        Invoke-WebRequest -Uri $asset.browser_download_url -OutFile $destFile

        if ($asset.name.EndsWith(".zip")) {
            $appFolder = Join-Path $installDir $app
            Write-Host "  --> Extracting to $appFolder..." -ForegroundColor Gray
            Expand-Archive -Path $destFile -DestinationPath $appFolder -Force
            Write-Host "  [✓] $app installed successfully." -ForegroundColor Green
        } elseif ($asset.name.EndsWith(".msi")) {
            Write-Host "  --> Installing MSI..." -ForegroundColor Gray
            Start-Process msiexec.exe -ArgumentList "/i `"$destFile`" /qn /norestart" -Wait
            Write-Host "  [✓] $app installed via MSI." -ForegroundColor Green
        }
    } catch {
        Write-Error "Failed to install $app : $_"
    }
}

Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
Write-Host "`n==> Installation Complete! Applications located in $installDir" -ForegroundColor Cyan
