Set-Location C:\Users\Danijel.Wynyard\Source\test_sql
Get-ChildItem -Path C:\Users\Danijel.Wynyard\Source\test_sql -Filter "*.json" | ForEach-Object {
    Remove-Item $_.FullName -Force -Confirm:$false
}
Get-ChildItem -Path C:\Users\Danijel.Wynyard\Source\test_sql -Filter "*.png" | ForEach-Object {
    Remove-Item $_.FullName -Force -Confirm:$false
}
Set-Location C:\Users\Danijel.Wynyard\Source\repos\winbox-stats
cargo clean
cargo build
cargo build --release
Set-Location C:\Users\Danijel.Wynyard\Source\test_sql
if (Test-Path .\winbox-stats.exe) { Remove-Item -Path .\winbox-stats.exe }
Copy-Item C:\Users\Danijel.Wynyard\Source\repos\winbox-stats\target\release\winbox-stats.exe `
  -Destination C:\Users\Danijel.Wynyard\Source\test_sql\winbox-stats.exe
& .\winbox-stats.exe graph 