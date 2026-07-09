import pyautogui
import ollama
import io
import time

def analyze_screen():
    print("peeking at your screen, senpai... :3")
    time.sleep(3)

    screenshot = pyautogui.screenshot()
    screenshot.thumbnail((672, 672))

    img_byte_arr = io.BytesIO()
    screenshot.save(img_byte_arr, format='JPEG')
    img_bytes = img_byte_arr.getvalue()

    try:
        response = ollama.generate(
            model='moondream',
            prompt='you are an expert at reading screens. read the text in this image and tell me exactly what it says and what is happening.',
            images=[img_bytes]
        )
        print("\n--- ayesha's analysis ---")
        print(response['response'])
        print("-------------------------\n")
    except Exception as e:
        print(f"\nsystem error: {e}. make sure ollama is running and the 'moondream' model is pulled!")

if __name__ == "__main__":
    analyze_screen()