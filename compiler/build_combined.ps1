$utf8 = [System.Text.UTF8Encoding]::new($false)
$out = @()

$lexer = Get-Content "C:\Vitalis-V60\compiler\lexer.sl" -Encoding UTF8
# lexer: skip test functions and main (lines 512+ are tests, 0-indexed=511+)
$out += $lexer[0..510]
$out += ""
$out += "// ======== PARSER ========"
$out += ""

$parser = Get-Content "C:\Vitalis-V60\compiler\parser.sl" -Encoding UTF8
# parser: take lines 1-53, skip lines 54-127 (TK_* dups), take 128-920, skip tests (921+)
$out += $parser[0..52]
$out += $parser[127..919]
$out += ""
$out += "// ======== TYPECHECKER ========"
$out += ""

$tc = Get-Content "C:\Vitalis-V60\compiler\typechecker.sl" -Encoding UTF8
$out += $tc
$out += ""
$out += "// ======== IR GEN ========"
$out += ""

$irgen = Get-Content "C:\Vitalis-V60\compiler\ir_gen.sl" -Encoding UTF8
$out += $irgen
$out += ""
$out += "// ======== X86 EMIT ========"
$out += ""

$x86 = Get-Content "C:\Vitalis-V60\compiler\x86_emit.sl" -Encoding UTF8
$out += $x86
$out += ""
$out += "// ======== REGALLOC ========"
$out += ""

$ra = Get-Content "C:\Vitalis-V60\compiler\regalloc.sl" -Encoding UTF8
$out += $ra
$out += ""
$out += "// ======== PE WRITER ========"
$out += ""

$pe = Get-Content "C:\Vitalis-V60\compiler\pe_writer.sl" -Encoding UTF8
$out += $pe
$out += ""
$out += "// ======== RUNTIME ========"
$out += ""

$rt = Get-Content "C:\Vitalis-V60\compiler\runtime.sl" -Encoding UTF8
$out += $rt
$out += ""
$out += "// ======== INTEGRATION ========"
$out += ""

$integ = Get-Content "C:\Vitalis-V60\compiler\integration.sl" -Encoding UTF8
$out += $integ

$out += ""
$out += "// ======== MAIN (CLI DRIVER) ========"
$out += ""

$main = Get-Content "C:\Vitalis-V60\compiler\main.sl" -Encoding UTF8
$out += $main

Write-Host "Total lines: $($out.Count)"

# Write test_integration.sl (with test harness)
$testout = $out.Clone()
$testout += ""
$testout += "// ======== TESTS (overrides main) ========"
$testout += ""
$tests = Get-Content "C:\Vitalis-V60\compiler\test_integration_main.sl" -Encoding UTF8
# Skip the main() from main.sl — the test harness has its own main
# Actually we just append tests; the LAST main() in file wins in Vitalis
# Let's not add test main - we only build bootstrap.sl (with main.sl's main)

# Write bootstrap.sl (the full compiler)
[System.IO.File]::WriteAllLines("C:\Vitalis-V60\compiler\bootstrap.sl", $out, $utf8)
Write-Host "Written bootstrap.sl ($($out.Count) lines)"

# Also write test_integration.sl (for testing) - uses the test main
$testlines = @()
# Take everything except main.sl
$mainstart = $out.Count - $main.Count - 3  # -3 for the separator comments
$testlines += $out[0..($mainstart-1)]
$testlines += ""
$testlines += "// ======== TESTS ========"
$testlines += ""
$testlines += $tests
[System.IO.File]::WriteAllLines("C:\Vitalis-V60\compiler\test_integration.sl", $testlines, $utf8)
Write-Host "Written test_integration.sl ($($testlines.Count) lines)"
