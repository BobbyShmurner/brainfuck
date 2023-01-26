cargo run tictactoe.b tictactoe
if ($LASTEXITCODE -ne 0) {
	exit
}

clang test.o
if ($LASTEXITCODE -ne 0) {
	exit
}

./a.exe
echo "Exited with code $LASTEXITCODE"