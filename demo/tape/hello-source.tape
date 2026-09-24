# Opening chapter: show the smallest Clorraine program in Vim.
#
# The pause is deliberately long so the viewer can read the source before the
# chapter ends.
Output build/demo/raw/hello-source.mp4
Set Shell "bash"
Set FontSize 16
Set Width 1100
Set Height 620
Set Padding 20
Set Theme "Dracula"
Set TypingSpeed 80ms

Type "demo/show-source.sh examples/hello/hello.clo"
Enter
Sleep 13s
Type ":q"
Enter
Sleep 1s
