$utf8 = [System.Text.UTF8Encoding]::new($false)
$out = @()

# 1. Lexer (skip tests/main at line 512+)
$lexer = Get-Content "C:\Vitalis-V60\compiler\lexer.sl" -Encoding UTF8
$out += $lexer[0..510]
$out += ""

# 2. Parser (skip TK_ dups at 54-127, skip tests at 921+)
$parser = Get-Content "C:\Vitalis-V60\compiler\parser.sl" -Encoding UTF8
$out += $parser[0..52]
$out += $parser[127..919]
$out += ""

# 3. Typechecker (all)
$tc = Get-Content "C:\Vitalis-V60\compiler\typechecker.sl" -Encoding UTF8
$out += $tc
$out += ""

# 4. IR Gen (str_hash removed, all)
$irgen = Get-Content "C:\Vitalis-V60\compiler\ir_gen.sl" -Encoding UTF8
$out += $irgen
$out += ""

# 5. X86 Emit (all)
$x86 = Get-Content "C:\Vitalis-V60\compiler\x86_emit.sl" -Encoding UTF8
$out += $x86
$out += ""

# 6. RegAlloc (all)
$ra = Get-Content "C:\Vitalis-V60\compiler\regalloc.sl" -Encoding UTF8
$out += $ra
$out += ""

# 7. PE Writer (all)
$pe = Get-Content "C:\Vitalis-V60\compiler\pe_writer.sl" -Encoding UTF8
$out += $pe
$out += ""

# 8. Runtime (all)
$rt = Get-Content "C:\Vitalis-V60\compiler\runtime.sl" -Encoding UTF8
$out += $rt
$out += ""

# 9. Integration (all)
$integ = Get-Content "C:\Vitalis-V60\compiler\integration.sl" -Encoding UTF8
$out += $integ
$out += ""

# 10. Proof script (has its own main)
$proof = Get-Content $args[0] -Encoding UTF8
$out += $proof

Write-Host "Total lines: $($out.Count)"
$outpath = $args[1]
[System.IO.File]::WriteAllLines($outpath, $out, $utf8)
Write-Host "Written: $outpath"
