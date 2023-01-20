# timespoof-rs
(mostly) proof of concept rust program to detour windows' GetSystemTimeAsFileTime to inject a spoofed time

powered by detour-rs [(Hpmason/fix-nightly1.67.0-changes)](https://github.com/Hpmason/detour-rs/tree/fix-nightly1.67.0-changes)

can communicate via TCP (socket ran by injected dll) with a [simple command](https://github.com/Lincoln-LM/timespoof-rs/blob/master/server.py#L74-L76)

## Basic Usage

1. run server.py, this will halt until it receives a connection
2. run program you would like to spoof time for
3. run timespoof.exe
4. enter an identifiable portion of the executable's name (ex. EmuHawk for BizHawk's EmuHawk.exe)
5. server.py should launch its GUI and print out the ip/port of the dll's tcpsocket
6. adjust settings in GUI and click the button whenever they need to be updated
