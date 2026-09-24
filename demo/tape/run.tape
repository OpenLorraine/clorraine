# Chapter 3: build and run the program on the hosted-amiga profile.
#
# One command drives emit, the m68k C compiler, and the runner; the output is
# the Amiga program's own stdout.
Output build/demo/raw/run.mp4
Set Shell "bash"
Set FontSize 16
Set Width 1100
Set Height 620
Set Padding 20
Set Theme "Dracula"
Set TypingSpeed 80ms

Type "make -s run FILE=spec/tests/corpus/string-parser/program.clo PROFILE=hosted-amiga"
Enter
Sleep 14s
