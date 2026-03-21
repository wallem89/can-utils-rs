Output demo/dump_and_send.gif

Set Shell "bash"
Set FontSize 20
Set Width 1600
Set Height 900

Env PS1 ">$ "

Require bash
Require tmux

Type "clear"
Enter
Sleep 500ms

Type "./demo/demo_dump_and_send.sh"
Enter

Wait+Screen /\? What do you want to do\?/
Sleep 700ms

# LEFT PANE: choose "Start pretty CAN dump"
Down
Enter

Wait+Screen /Select CAN interface to dump/
Sleep 500ms

Enter

Wait+Screen /Pretty CAN Dump on/
Sleep 1000ms

# Move to LEFT PANE
Ctrl+B
Type "o"
Sleep 700ms

Wait+Screen /\? What do you want to do\?/
Sleep 500ms

# LEFT PANE: choose "Send CAN frame"
Down
Down
Down
Enter

Wait+Screen /Select CAN interface/
Sleep 500ms

Enter

Wait+Screen /Enter CAN ID/
Sleep 500ms
Type "7FF"
Enter

Wait+Screen /Enter data bytes/
Sleep 500ms
Type "00 11 22 33 44 55 66"
Enter

Wait+Screen /How do you want to send it\?/
Sleep 500ms
Enter

Sleep 2000ms

# Send one more frame manually by rerunning the left pane app
Type "cargo run"
Enter

Wait+Screen /\? What do you want to do\?/
Sleep 500ms
Down
Down
Down
Enter

Wait+Screen /Select CAN interface/
Sleep 500ms
Enter

Wait+Screen /Enter CAN ID/
Sleep 500ms
Type "123"
Enter

Wait+Screen /Enter data bytes/
Sleep 500ms
Type "DE AD BE EF"
Enter

Wait+Screen /How do you want to send it\?/
Sleep 500ms
Down
Down
Enter

Sleep 500ms
Type "500"
Enter
Enter

Sleep 2500ms

# Back to RIGHT PANE so the dump is visible while ending
Ctrl+B
Type "o"
Sleep 1500ms

Ctrl+C
Sleep 800ms

# Back to LEFT PANE to kill CAN sending
Ctrl+B
Type "o"

Ctrl+C

# Detach tmux cleanly
Type "d"