$interval = [TimeSpan]::FromSeconds(30)
$end      = (Get-Date).AddHours(3)
$next     = (Get-Date).Add($interval)  # first run at +30s

while ($next -le $end) {
    $sleepMs = ($next - (Get-Date)).TotalMilliseconds
    if ($sleepMs -gt 0) { Start-Sleep -Milliseconds $sleepMs }

    # run the command at the tick
    & .\winbox-stats.exe

    # schedule the next tick; catch up if the command took > 30s
    $next = $next.Add($interval)
    while ($next -le (Get-Date)) { $next = $next.Add($interval) }
}
