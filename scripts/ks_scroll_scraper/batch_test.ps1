$ids = @(
    "ziqishuo",
    "3xajj6iezbjntdg",
    "3xx2hqecw8ihfsa",
    "xhh52088610000",
    "C68686886C",
    "Q3500463680",
    "3x2wb5yte8gnxyy",
    "3xq5xxgd28www99",
    "QQ_2627174",
    "3xvzusz42djtnha",
    "3xerpcfxtn87yc9",
    "3x9akpwvt5kz64y",
    "3xdabf56qbpyicw"
)

$exe = ".\target\release\ks_scraper.exe"
$outDir = ".\batch_results"
if (Test-Path $outDir) { Remove-Item "$outDir\*" -Force }
New-Item -ItemType Directory -Force -Path $outDir | Out-Null

$totalStart = Get-Date
$resultLines = @()

Write-Host ""
Write-Host "===== BATCH TEST: $($ids.Count) users =====" -ForegroundColor Cyan
Write-Host ""

for ($i = 0; $i -lt $ids.Count; $i++) {
    $uid = $ids[$i]
    $url = "https://www.kuaishou.com/profile/$uid"
    $csvFile = "$outDir\$uid.csv"
    $start = Get-Date
    
    Write-Host "[$($i+1)/$($ids.Count)] $uid" -ForegroundColor Yellow -NoNewline
    
    & $exe --url $url --output $csvFile --max-empty 8 --timeout 600 2>&1 | Out-Null
    
    $elapsed = [math]::Round(((Get-Date) - $start).TotalSeconds)
    
    # Count CSV lines (subtract 1 for header)
    $videoCount = 0
    $status = "FAIL"
    if (Test-Path $csvFile) {
        $videoCount = (Get-Content $csvFile).Count - 1
        if ($videoCount -gt 0) { $status = "OK" }
        elseif ($videoCount -eq 0) { $status = "EMPTY" }
    }
    
    $resultLines += [PSCustomObject]@{
        No = $i + 1
        UserId = $uid
        Videos = $videoCount
        Time = "${elapsed}s"
        Status = $status
    }
    
    if ($status -eq "OK") {
        Write-Host " -> $videoCount videos, ${elapsed}s" -ForegroundColor Green
    } else {
        Write-Host " -> $status, ${elapsed}s" -ForegroundColor Red
    }
    
    # Wait between users (5-10s)
    if ($i -lt $ids.Count - 1) {
        $wait = Get-Random -Minimum 5 -Maximum 10
        Write-Host "  (wait ${wait}s)" -ForegroundColor DarkGray
        Start-Sleep -Seconds $wait
    }
}

$totalElapsed = [math]::Round(((Get-Date) - $totalStart).TotalSeconds / 60, 1)

Write-Host ""
Write-Host "===== RESULTS =====" -ForegroundColor Cyan
$resultLines | Format-Table -AutoSize

$okCount = ($resultLines | Where-Object { $_.Status -eq "OK" }).Count
$failCount = ($resultLines | Where-Object { $_.Status -ne "OK" }).Count
$totalVideos = ($resultLines | Measure-Object -Property Videos -Sum).Sum
Write-Host "OK: $okCount/$($ids.Count) | FAIL: $failCount | Total Videos: $totalVideos | Time: ${totalElapsed}min" -ForegroundColor Cyan
