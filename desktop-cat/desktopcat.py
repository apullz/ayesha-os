import tkinter as tk
from PIL import Image, ImageTk, ImageDraw
import ctypes
import ctypes.wintypes
import math
import os
import random
import threading

import pystray

user32 = ctypes.windll.user32

SPRITE_SIZE = 32
DISPLAY_SCALE = 1.5
DISPLAY_SIZE = int(SPRITE_SIZE * DISPLAY_SCALE)

screen_w = user32.GetSystemMetrics(0)
screen_h = user32.GetSystemMetrics(1)

script_dir = os.path.dirname(os.path.abspath(__file__))
atlas = Image.open(os.path.join(script_dir, "oneko_sprite.png")).convert("RGBA")

COLLAR_COLOR = (200, 30, 30, 255)
COLLAR_LEN = 8

COLLAR_ANGLES = {
    "idle": 0, "alert": 0, "sleeping": 0, "tired": 0,
    "scratchSelf": 0,
    "scratchWallN": 0, "scratchWallS": 0, "scratchWallE": 75, "scratchWallW": 105,
    "N": 0, "NE": 45, "E": 75, "SE": 135,
    "S": 0, "SW": 45, "W": 105, "NW": 135,
}

COLLAR_OFFSETS = {
    "E": (5, 0), "W": (-5, 0),
    "scratchWallE": (5, 0), "scratchWallW": (-5, 0),
}

COLLAR_TOP_EXT = {
    "E": 4, "W": 4,
    "scratchWallE": 4, "scratchWallW": 4,
}

NO_COLLAR_ANIMS = {"sleeping", "tired", "scratchSelf"}

collar_cache = {}
def get_collar_img(angle, top_ext=0):
    """top_ext: extra pixels extending above center (left side pre-rotation)"""
    key = (angle, top_ext)
    if key not in collar_cache:
        left = COLLAR_LEN // 2 + top_ext
        right = COLLAR_LEN // 2
        total = left + right
        size = total + 2
        img = Image.new("RGBA", (size, size), (0, 0, 0, 0))
        d = ImageDraw.Draw(img)
        cy = size // 2
        cx = size // 2
        d.line([(cx - left, cy), (cx + right, cy)], fill=COLLAR_COLOR, width=1)
        if angle != 0:
            img = img.rotate(-angle, resample=Image.NEAREST, expand=True)
            bbox = img.getbbox()
            if bbox:
                img = img.crop(bbox)
        collar_cache[key] = img
    return collar_cache[key]

def recolor_sprite(img):
    pixels = img.load()
    for y in range(img.height):
        for x in range(img.width):
            r, g, b, a = pixels[x, y]
            if a == 0:
                continue
            if r > 200 and g > 200 and b > 200:
                pixels[x, y] = (30, 30, 30, a)
            elif r > 200 and g < 100 and b < 100:
                pixels[x, y] = (220, 30, 30, a)
    return img

def extract_frame(x, y, has_collar=True):
    crop = atlas.crop((x, y, x + SPRITE_SIZE, y + SPRITE_SIZE))
    crop = recolor_sprite(crop.copy())
    return crop.resize((DISPLAY_SIZE, DISPLAY_SIZE), Image.NEAREST)

animations = {
    "idle":        [extract_frame(96, 96)],
    "alert":       [extract_frame(224, 96)],
    "sleeping":    [extract_frame(64, 0), extract_frame(64, 32)],
    "tired":       [extract_frame(96, 64)],
    "scratchSelf": [extract_frame(160, 0), extract_frame(192, 0), extract_frame(224, 0)],
    "scratchWallN":[extract_frame(0, 0), extract_frame(0, 32)],
    "scratchWallS":[extract_frame(224, 32), extract_frame(192, 64)],
    "scratchWallE":[extract_frame(64, 64), extract_frame(64, 96)],
    "scratchWallW":[extract_frame(128, 0), extract_frame(128, 32)],
    "N":           [extract_frame(32, 64), extract_frame(32, 96)],
    "NE":          [extract_frame(0, 64), extract_frame(0, 96)],
    "E":           [extract_frame(96, 0), extract_frame(96, 32)],
    "SE":          [extract_frame(160, 32), extract_frame(160, 64)],
    "S":           [extract_frame(192, 96), extract_frame(224, 64)],
    "SW":          [extract_frame(160, 96), extract_frame(192, 32)],
    "W":           [extract_frame(128, 64), extract_frame(128, 96)],
    "NW":          [extract_frame(32, 0), extract_frame(32, 32)],
}

def make_heart_image():
    grid = {}
    fill = [
        (5,1),(6,1),(9,1),(10,1),
        (3,2),(4,2),(5,2),(6,2),(7,2),(8,2),(9,2),(10,2),(11,2),(12,2),
        (2,3),(3,3),(4,3),(5,3),(6,3),(7,3),(8,3),(9,3),(10,3),(11,3),(12,3),(13,3),
        (2,4),(3,4),(4,4),(5,4),(6,4),(7,4),(8,4),(9,4),(10,4),(11,4),(12,4),(13,4),
        (2,5),(3,5),(4,5),(5,5),(6,5),(7,5),(8,5),(9,5),(10,5),(11,5),(12,5),(13,5),
        (3,6),(4,6),(5,6),(6,6),(7,6),(8,6),(9,6),(10,6),(11,6),(12,6),
        (4,7),(5,7),(6,7),(7,7),(8,7),(9,7),(10,7),(11,7),
        (5,8),(6,8),(7,8),(8,8),(9,8),(10,8),
        (6,9),(7,9),(8,9),(9,9),
        (7,10),(8,10),
    ]
    for x, y in fill:
        grid[(x, y)] = (165, 42, 43, 255)
    shine = [(5,1),(6,1),(3,2),(4,2),(5,2),(3,3)]
    for x, y in shine:
        grid[(x, y)] = (255, 255, 255, 255)
    dark = [(11,5),(12,5),(12,4),(13,4),(12,3),(13,3),(11,6),(12,6)]
    for x, y in dark:
        if (x, y) in grid:
            grid[(x, y)] = (120, 30, 30, 255)
    heart = Image.new("RGBA", (16, 16), (0, 0, 0, 0))
    px = heart.load()
    for (x, y), color in grid.items():
        px[x, y] = color
    return heart

heart_img = make_heart_image()

root = tk.Tk()
root.withdraw()
root.overrideredirect(True)
root.attributes("-topmost", True)
root.config(bg="magenta")
root.attributes("-transparentcolor", "magenta")
HEART_H = 24
WINDOW_W = DISPLAY_SIZE + 200
WINDOW_H = DISPLAY_SIZE + HEART_H + 200
root.geometry(f"{WINDOW_W}x{WINDOW_H}")

GWL_EXSTYLE = -20
WS_EX_TRANSPARENT = 0x00000020
WS_EX_TOOLWINDOW = 0x00000080

canvas = tk.Canvas(root, width=WINDOW_W, height=WINDOW_H,
                   bg="magenta", highlightthickness=0)
canvas.pack()

root.update_idletasks()
hwnd = user32.GetParent(root.winfo_id())
if not hwnd:
    hwnd = root.winfo_id()
ex = user32.GetWindowLongW(hwnd, GWL_EXSTYLE)
user32.SetWindowLongW(hwnd, GWL_EXSTYLE, ex | WS_EX_TRANSPARENT | WS_EX_TOOLWINDOW)

photo_cache = {}
def pil_to_photo(pil_img):
    key = pil_img.tobytes()
    if key not in photo_cache:
        photo_cache[key] = ImageTk.PhotoImage(pil_img)
    return photo_cache[key]

pt = ctypes.wintypes.POINT()
user32.GetCursorPos(ctypes.byref(pt))
cat_x = float(pt.x)
cat_y = float(pt.y)
cat_speed = 5.0
current_anim = "idle"
frame_idx = 0
anim_timer = 0
ANIM_FRAME_MS = 100

cat_visible = True
idle_time = 0
idle_animation = None
idleAnimationFrame = 0

pet_timer = 0
show_heart = False
heart_timer = 0
HEART_PET_TIME = 1500
HEART_SHOW_TIME = 2500

def get_cursor_pos():
    pt = ctypes.wintypes.POINT()
    user32.GetCursorPos(ctypes.byref(pt))
    return pt.x, pt.y

prev_mx, prev_my = get_cursor_pos()

CAT_ORIGIN_X = WINDOW_W // 2
CAT_ORIGIN_Y = HEART_H + DISPLAY_SIZE // 2 + 10

def reset_idle_animation():
    global idle_animation, idleAnimationFrame
    idle_animation = None
    idleAnimationFrame = 0

def update():
    global cat_x, cat_y, current_anim, frame_idx, anim_timer
    global idle_time, idle_animation, idleAnimationFrame
    global pet_timer, show_heart, heart_timer
    global prev_mx, prev_my

    dt = 16
    anim_timer += dt

    mx, my = get_cursor_pos()
    cursor_speed = math.hypot(mx - prev_mx, my - prev_my)
    prev_mx, prev_my = mx, my

    diffX = cat_x - mx
    diffY = cat_y - my
    distance = math.hypot(diffX, diffY)

    cat_hitbox = DISPLAY_SIZE * 0.8
    cursor_over_cat = (abs(mx - cat_x) < cat_hitbox and abs(my - cat_y) < cat_hitbox)

    if cursor_over_cat and cursor_speed > 1:
        pet_timer += dt
    elif not cursor_over_cat:
        pet_timer = max(0, pet_timer - dt * 2)

    if pet_timer >= HEART_PET_TIME and not show_heart:
        show_heart = True
        heart_timer = 0

    if show_heart:
        heart_timer += dt
        if heart_timer >= HEART_SHOW_TIME:
            show_heart = False
            heart_timer = 0
            pet_timer = 0

    def draw_frame():
        root.geometry(f"+{int(cat_x - CAT_ORIGIN_X)}+{int(cat_y - CAT_ORIGIN_Y)}")
        canvas.delete("all")
        if show_heart:
            canvas.create_image(CAT_ORIGIN_X, HEART_H // 2, anchor="center",
                                image=pil_to_photo(heart_img))
        canvas.create_image(CAT_ORIGIN_X, CAT_ORIGIN_Y, anchor="center",
                            image=pil_to_photo(frames[frame_idx % len(frames)]))
        if current_anim not in NO_COLLAR_ANIMS:
            angle = COLLAR_ANGLES.get(current_anim, 0)
            ox, oy = COLLAR_OFFSETS.get(current_anim, (0, 0))
            top_ext = COLLAR_TOP_EXT.get(current_anim, 0)
            collar_img = get_collar_img(angle, top_ext)
            canvas.create_image(CAT_ORIGIN_X + ox, CAT_ORIGIN_Y + 3 + oy, anchor="center",
                                image=pil_to_photo(collar_img))

    if distance < cat_speed or distance < 48:
        idle_time += 1
        if idle_time > 10 and random.random() < 0.005 and idle_animation is None:
            available = ["sleeping", "scratchSelf"]
            if cat_x < 32:
                available.append("scratchWallW")
            if cat_y < 32:
                available.append("scratchWallN")
            if cat_x > screen_w - 32:
                available.append("scratchWallE")
            if cat_y > screen_h - 32:
                available.append("scratchWallS")
            idle_animation = random.choice(available)

        if idle_animation == "sleeping":
            if idleAnimationFrame < 8:
                current_anim = "tired"
            else:
                current_anim = "sleeping"
            if idleAnimationFrame > 192:
                reset_idle_animation()
        elif idle_animation and idle_animation.startswith("scratch"):
            current_anim = idle_animation
            if idleAnimationFrame > 9:
                reset_idle_animation()
        else:
            current_anim = "idle"

        idleAnimationFrame += 1
        frames = animations[current_anim]
        if len(frames) > 1 and anim_timer >= ANIM_FRAME_MS:
            anim_timer = 0
            frame_idx = (frame_idx + 1) % len(frames)
        elif len(frames) == 1:
            frame_idx = 0

        draw_frame()
        root.after(16, update)
        return

    reset_idle_animation()

    if idle_time > 1:
        current_anim = "alert"
        idle_time = min(idle_time, 7)
        idle_time -= 1
        frames = animations[current_anim]
        frame_idx = 0
        draw_frame()
        root.after(16, update)
        return

    idle_time = 0

    direction = ""
    if diffY / distance > 0.5:
        direction += "N"
    if diffY / distance < -0.5:
        direction += "S"
    if diffX / distance > 0.5:
        direction += "W"
    if diffX / distance < -0.5:
        direction += "E"

    current_anim = direction if direction in animations else "idle"

    cat_x -= (diffX / distance) * cat_speed
    cat_y -= (diffY / distance) * cat_speed
    cat_x = max(16, min(cat_x, screen_w - 16))
    cat_y = max(16, min(cat_y, screen_h - 16))

    frames = animations[current_anim]
    if len(frames) > 1 and anim_timer >= ANIM_FRAME_MS:
        anim_timer = 0
        frame_idx = (frame_idx + 1) % len(frames)
    elif len(frames) == 1:
        frame_idx = 0

    draw_frame()
    root.after(16, update)

def toggle_cat(icon, item):
    global cat_visible
    cat_visible = not cat_visible
    if cat_visible:
        root.after(0, lambda: root.deiconify())
    else:
        root.after(0, lambda: root.withdraw())

def quit_cat(icon, item):
    icon.stop()
    root.after(0, root.destroy)

def make_tray_icon():
    icon_img = extract_frame(96, 96).resize((64, 64), Image.NEAREST)
    menu = pystray.Menu(
        pystray.MenuItem("Show/Hide Cat", toggle_cat, default=True),
        pystray.MenuItem("Quit", quit_cat),
    )
    icon = pystray.Icon("desktopcat", icon_img, "Desktop Cat", menu)
    return icon

root.after(0, update)

tray_icon = make_tray_icon()
threading.Thread(target=tray_icon.run, daemon=True).start()

root.deiconify()
root.mainloop()
