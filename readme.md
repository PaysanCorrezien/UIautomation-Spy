# WindowsUIA - Windows UI Automation SPY

A simple tool to inspect the UI Automation Tree of any application.

It print the tree of the UI Automation elements in the console, and display the `rect` of the elements in the screen and the element value.

![Calculator](./assets/calculator.png)

## Usage

1. Git clone this repository .

2. cd into it

3. Run `cargor run "APPWINDOWTITLE"` where `APPWINDOWTITLE` is the title of the window of the application you want to inspect.

4. Enjoy!

## Requirements

You obviously need rust tooling.

## Tips

If you want to list current window title of all Apps running on your system, you can use the following command:

```powershell
 Get-Process | Where-Object {$_.MainWindowTitle -ne ""} | Select-Object ProcessName, MainWindowTitle , MainWindowclass
```

You can take the `MainWindowTitle` of the application you want to inspect and pass it to the program.

_It doesn't work for `Task Manager` because of permission by default_
