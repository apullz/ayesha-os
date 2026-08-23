import os
import sys
import subprocess

def check_dependencies():
    try:
        import PIL
        import pystray
    except ImportError:
        print("Installing required dependencies (Pillow, pystray)...")
        subprocess.check_call([sys.executable, "-m", "pip", "install", "-r", "requirements.txt"])

def setup_startup():
    try:
        startup = os.path.expandvars(r"%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup")
        shortcut_path = os.path.join(startup, "DesktopCat.lnk")
        
        choice = input("Do you want to run Desktop Cat at Windows startup? (y/n): ").strip().lower()
        if choice in ('y', 'yes'):
            script = f'''
            $startup = [Environment]::GetFolderPath('Startup')
            $ws = New-Object -ComObject WScript.Shell
            $sc = $ws.CreateShortcut("$startup\\DesktopCat.lnk")
            $sc.TargetPath = 'pythonw.exe'
            $sc.Arguments = '"{os.path.abspath("desktopcat.py")}"'
            $sc.WorkingDirectory = '{os.path.abspath(".")}'
            $sc.Save()
            '''
            subprocess.run(["powershell", "-Command", script], check=True)
            print("Successfully added to Windows startup!")
        else:
            if os.path.exists(shortcut_path):
                os.remove(shortcut_path)
            print("Skipped Windows startup.")
    except Exception as e:
        print(f"Could not configure startup: {e}")

if __name__ == "__main__":
    check_dependencies()
    setup_startup()
    print("Starting Desktop Cat in background...")
    pythonw = sys.executable.replace("python.exe", "pythonw.exe")
    if not os.path.exists(pythonw):
        pythonw = "pythonw"
    subprocess.Popen([pythonw, "desktopcat.py"], cwd=os.path.abspath("."))
    print("Desktop Cat is now running in the background with a system tray icon!")
