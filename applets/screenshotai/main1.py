import pyautogui
import ollama
import io
import time

def analyze_screen():
    print("📸 peeking at your screen, senpai... :3")
    
    # 2. wait 3 seconds so you have time to get ready
    time.sleep(3) 

    # 3. take a screenshot
    screenshot = pyautogui.screenshot()
    screenshot.thumbnail((672, 672))
    img_byte_arr = io.BytesIO()
    screenshot.save(img_byte_arr, format='JPEG')
    img_bytes = img_byte_arr.getvalue()

    try:
        response = ollama.generate(
            model='moondream',
            prompt='what is on this screen? write at least one sentence.',
            images=[img_bytes]
        )
        print("\n--- ayesha's analysis ---")
        analysis_text = response['response']
        if not analysis_text:
            print("the ai is being shy! it didn't write anything. :(")
        else:
            print(analysis_text)
        print("-------------------------\n")
    except Exception as e:
        print(f"\nsystem error: {e}. make sure ollama is running and the 'moondream' model is pulled!")

if __name__ == "__main__":
    analyze_screen()