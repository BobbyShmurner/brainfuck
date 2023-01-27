$file = $args[0]
$out = $args[1]

cargo run $file $out
if ($LASTEXITCODE -ne 0) {
	exit
}

Start-Process -NoNewWindow -FilePath "clang" -ArgumentList "$out.o -o $out.exe" -Wait
if ($LASTEXITCODE -ne 0) {
	exit
}

Start-Process -NoNewWindow -FilePath "./$out.exe" -Wait
echo "Exited with code $LASTEXITCODE"