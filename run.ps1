$file = $args[0]
$out = $args[1]

cargo run --release $file $out
if ($LASTEXITCODE -ne 0) {
	exit
}

$process = Start-Process -NoNewWindow -FilePath "clang" -ArgumentList "`"$out.o`" -o `"$out.exe`"" -PassThru -Wait
if ($process.ExitCode -ne 0) {
	Write-Host "Clang exited with code " $process.ExitCode
	exit
}

$process = Start-Process -NoNewWindow -FilePath "./$out.exe" -PassThru -Wait
Write-Host "Exited with code " $process.ExitCode