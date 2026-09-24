# Chapter 2: show a move-then-use program and the compiler rejecting it.
#
# The error is the point of the chapter: use after move is a compile-time
# error, which a plain C compiler cannot provide.
Output build/demo/raw/ownership.mp4
Set Shell "bash"
Set FontSize 16
Set Width 1100
Set Height 620
Set Padding 20
Set Theme "Dracula"
Set TypingSpeed 80ms

Type "demo/show-source.sh examples/ownership-fail.clo"
Enter
Sleep 8s
Type ":q"
Enter
Sleep 1s
Type "make -s run FILE=examples/ownership-fail.clo"
Enter
Sleep 6s
