cargo run ticktactoe.b ticktactoe

if ($LASTEXITCODE -ne 0) {
	exit
}

clang test.o
./a.exe

echo "Exited with code $LASTEXITCODE"