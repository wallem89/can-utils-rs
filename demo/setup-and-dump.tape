Output demo/setup-and-dump.gif

Set Shell "zsh"
Set FontSize 22
Set Width 1300
Set Height 750

Env PS1 ">$ "

Require zsh

Sleep 500ms

Type "cargo run"
Enter

Wait+Screen /\? What do you want to do\?/
Sleep 1000ms

Down
Sleep 500ms
Enter

Wait+Screen /Select CAN interface to dump:/
Sleep 1000ms

Enter

Wait+Screen /Pretty CAN Dump on/
Sleep 1000ms

Enter

Sleep 5s
Ctrl+C
Sleep 1000ms