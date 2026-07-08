"""Visual effects for NEURAL-STRIKE"""
from PyQt6.QtWidgets import QWidget
from PyQt6.QtCore import Qt, QTimer, QRectF
from PyQt6.QtGui import QPainter, QColor, QPen, QLinearGradient


class CRTOverlay(QWidget):
    """CRT scanline overlay effect."""

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)
        self.scanline_offset = 0
        self.flicker_intensity = 0

        self.timer = QTimer()
        self.timer.timeout.connect(self._animate)
        self.timer.start(50)

    def _animate(self):
        self.scanline_offset = (self.scanline_offset + 1) % 4
        self.flicker_intensity = (self.flicker_intensity + 0.1) % 6.28
        self.update()

    def paintEvent(self, event):
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)

        w, h = self.width(), self.height()

        # Scanlines
        painter.setPen(Qt.PenStyle.NoPen)
        for y in range(0, h, 4):
            offset = (y + self.scanline_offset) % 4
            if offset == 0:
                painter.setBrush(QColor(0, 0, 0, 30))
                painter.drawRect(0, y, w, 2)

        # Subtle flicker
        flicker_alpha = int(5 + 3 * (self.flicker_intensity % 1))
        painter.setBrush(QColor(0, 240, 255, flicker_alpha))
        painter.drawRect(QRectF(0, 0, w, h))

        # Chromatic aberration hint at edges
        gradient = QLinearGradient(0, 0, w, 0)
        gradient.setColorAt(0, QColor(255, 0, 170, 15))
        gradient.setColorAt(0.5, QColor(0, 0, 0, 0))
        gradient.setColorAt(1, QColor(0, 240, 255, 15))
        painter.setBrush(gradient)
        painter.drawRect(QRectF(0, 0, w, h))

        painter.end()


class GlitchEffect(QWidget):
    """Glitch/distortion effect for activations."""

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)
        self.active = False
        self.glitch_rects = []
        self.opacity = 0

    def trigger(self, intensity: float = 1.0):
        """Trigger glitch effect."""
        self.active = True
        self.opacity = min(255, int(200 * intensity))
        self.glitch_rects = []
        import random
        for _ in range(int(5 * intensity)):
            x = random.randint(0, self.width())
            y = random.randint(0, self.height())
            w = random.randint(10, 100)
            h = random.randint(2, 8)
            self.glitch_rects.append((x, y, w, h))
        self.update()

        QTimer.singleShot(200, self._clear)

    def _clear(self):
        self.active = False
        self.opacity = 0
        self.glitch_rects = []
        self.update()

    def paintEvent(self, event):
        if not self.active:
            return

        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)

        for x, y, w, h in self.glitch_rects:
            color = QColor(255, 0, 170, self.opacity)
            painter.setBrush(color)
            painter.setPen(Qt.PenStyle.NoPen)
            painter.drawRect(x, y, w, h)

        painter.end()


class DataStreamEffect(QWidget):
    """Matrix-rain data stream effect."""

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setAttribute(Qt.WidgetAttribute.WA_TransparentForMouseEvents)
        self.drops = []
        self.max_drops = 50

        self.timer = QTimer()
        self.timer.timeout.connect(self._animate)
        self.timer.start(100)

    def add_drop(self, x: float, y: float, color: str = "#00f0ff"):
        """Add a new data stream drop."""
        import random
        self.drops.append({
            "x": x,
            "y": y,
            "speed": random.uniform(2, 8),
            "length": random.randint(20, 60),
            "alpha": 255,
            "color": QColor(color),
        })
        if len(self.drops) > self.max_drops:
            self.drops.pop(0)

    def _animate(self):
        for drop in self.drops:
            drop["y"] += drop["speed"]
            drop["alpha"] = max(0, drop["alpha"] - 5)

        self.drops = [d for d in self.drops if d["alpha"] > 0 and d["y"] < self.height()]
        self.update()

    def paintEvent(self, event):
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)

        for drop in self.drops:
            color = drop["color"]
            color.setAlpha(drop["alpha"])

            gradient = QLinearGradient(drop["x"], drop["y"], drop["x"], drop["y"] + drop["length"])
            gradient.setColorAt(0, QColor(color))
            gradient.setColorAt(1, QColor(color.red(), color.green(), color.blue(), 0))

            pen = QPen(gradient, 2)
            painter.setPen(pen)
            painter.drawLine(
                int(drop["x"]), int(drop["y"]),
                int(drop["x"]), int(drop["y"] + drop["length"])
            )

        painter.end()
