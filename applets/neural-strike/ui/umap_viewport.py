"""U-MAP viewport for NEURAL-STRIKE - the main neuron visualization"""
import math
import random
from PyQt6.QtWidgets import QWidget, QVBoxLayout, QHBoxLayout, QLabel
from PyQt6.QtCore import Qt, QRectF, QPointF, pyqtSignal
from PyQt6.QtGui import QPainter, QColor, QPen, QBrush, QRadialGradient, QFont


class UMAPViewport(QWidget):
    """Interactive 2D U-MAP visualization of SAE features."""

    feature_clicked = pyqtSignal(dict)
    feature_hovered = pyqtSignal(dict)

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setMinimumSize(600, 400)

        # Feature data
        self.features = []  # List of {x, y, feature_id, layer, cluster, explanation}
        self.selected_feature = None
        self.hovered_feature = None

        # Viewport transform
        self.offset_x = 0
        self.offset_y = 0
        self.zoom = 1.0
        self.min_zoom = 0.1
        self.max_zoom = 10.0

        # Interaction state
        self.dragging = False
        self.drag_start = QPointF()
        self.drag_offset_start = QPointF()

        # Animation
        self.activation_pulse = {}  # feature_id -> alpha
        self.particle_effects = []  # List of particle dicts

        # Cluster colors
        self.cluster_colors = {
            "toxic": QColor("#ff0040"),
            "scotland": QColor("#0066ff"),
            "geography": QColor("#ffaa00"),
            "sentiment": QColor("#aa00ff"),
            "neutral": QColor("#404050"),
        }

        self.setMouseTracking(True)

    def set_features(self, features: list[dict]):
        """Set the feature data to visualize."""
        self.features = features
        if features:
            xs = [f.get("x", 0) for f in features]
            ys = [f.get("y", 0) for f in features]
            self.center_on((min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2)
        self.update()

    def center_on(self, x: float, y: float):
        """Center viewport on coordinates."""
        w, h = self.width(), self.height()
        self.offset_x = w / 2 - x * self.zoom
        self.offset_y = h / 2 - y * self.zoom

    def pulse_feature(self, feature_id: str, intensity: float = 1.0):
        """Start a pulse animation on a feature."""
        self.activation_pulse[feature_id] = 255 * intensity
        self.update()

    def world_to_screen(self, x: float, y: float) -> QPointF:
        """Convert world coordinates to screen coordinates."""
        return QPointF(
            x * self.zoom + self.offset_x,
            y * self.zoom + self.offset_y
        )

    def screen_to_world(self, sx: float, sy: float) -> QPointF:
        """Convert screen coordinates to world coordinates."""
        return QPointF(
            (sx - self.offset_x) / self.zoom,
            (sy - self.offset_y) / self.zoom
        )

    def get_feature_at(self, sx: float, sy: float) -> dict | None:
        """Find the feature nearest to screen coordinates."""
        world = self.screen_to_world(sx, sy)
        min_dist = 15 / self.zoom  # Click radius
        best = None

        for f in self.features:
            fx, fy = f.get("x", 0), f.get("y", 0)
            dist = math.sqrt((fx - world.x()) ** 2 + (fy - world.y()) ** 2)
            if dist < min_dist:
                min_dist = dist
                best = f

        return best

    def paintEvent(self, event):
        painter = QPainter(self)
        painter.setRenderHint(QPainter.RenderHint.Antialiasing)

        w, h = self.width(), self.height()

        # Background
        painter.fillRect(0, 0, w, h, QColor("#0a0a0f"))

        # Grid lines (subtle)
        self._draw_grid(painter, w, h)

        # Draw features
        self._draw_features(painter)

        # Draw selection highlight
        if self.selected_feature:
            self._draw_selection(painter, self.selected_feature)

        # Draw hover highlight
        if self.hovered_feature:
            self._draw_hover(painter, self.hovered_feature)

        # Draw particle effects
        self._draw_particles(painter)

        painter.end()

    def _draw_grid(self, painter: QPainter, w: int, h: int):
        """Draw subtle background grid."""
        pen = QPen(QColor("#1a1a2e"), 1)
        painter.setPen(pen)

        grid_size = 50 * self.zoom
        if grid_size < 20:
            return

        start_x = self.offset_x % grid_size
        start_y = self.offset_y % grid_size

        x = start_x
        while x < w:
            painter.drawLine(int(x), 0, int(x), h)
            x += grid_size

        y = start_y
        while y < h:
            painter.drawLine(0, int(y), w, int(y))
            y += grid_size

    def _draw_features(self, painter: QPainter):
        """Draw all feature points."""
        for f in self.features:
            x, y = f.get("x", 0), f.get("y", 0)
            screen = self.world_to_screen(x, y)

            # Skip if off-screen
            if not (-50 < screen.x() < self.width() + 50 and
                    -50 < screen.y() < self.height() + 50):
                continue

            cluster = f.get("cluster", "neutral")
            color = self.cluster_colors.get(cluster, QColor("#404050"))

            # Check for activation pulse
            fid = f.get("feature_id", "")
            if fid in self.activation_pulse:
                alpha = int(self.activation_pulse[fid])
                color.setAlpha(alpha)
                self.activation_pulse[fid] = max(0, alpha - 10)
                if self.activation_pulse[fid] <= 0:
                    del self.activation_pulse[fid]

            # Point size based on zoom
            size = max(2, min(8, 3 * self.zoom))

            # Draw point with glow
            if cluster != "neutral":
                gradient = QRadialGradient(screen.x(), screen.y(), size * 2)
                gradient.setColorAt(0, QColor(color.red(), color.green(), color.blue(), 180))
                gradient.setColorAt(0.5, QColor(color.red(), color.green(), color.blue(), 60))
                gradient.setColorAt(1, QColor(color.red(), color.green(), color.blue(), 0))
                painter.setBrush(QBrush(gradient))
                painter.setPen(Qt.PenStyle.NoPen)
                painter.drawEllipse(screen, size * 2, size * 2)

            # Core point
            painter.setBrush(QBrush(color))
            painter.setPen(Qt.PenStyle.NoPen)
            painter.drawEllipse(screen, size, size)

    def _draw_selection(self, painter: QPainter, feature: dict):
        """Draw selection highlight around a feature."""
        x, y = feature.get("x", 0), feature.get("y", 0)
        screen = self.world_to_screen(x, y)

        pen = QPen(QColor("#00f0ff"), 2)
        painter.setPen(pen)
        painter.setBrush(Qt.BrushStyle.NoBrush)

        radius = 20
        painter.drawEllipse(screen, radius, radius)

        # Selection ring animation
        painter.setPen(QPen(QColor("#00f0ff"), 1))
        painter.drawEllipse(screen, radius + 5, radius + 5)

    def _draw_hover(self, painter: QPainter, feature: dict):
        """Draw hover highlight."""
        x, y = feature.get("x", 0), feature.get("y", 0)
        screen = self.world_to_screen(x, y)

        pen = QPen(QColor("#ffffff"), 1)
        painter.setPen(pen)
        painter.setBrush(Qt.BrushStyle.NoBrush)
        painter.drawEllipse(screen, 12, 12)

    def _draw_particles(self, painter: QPainter):
        """Draw particle effects."""
        for p in self.particle_effects:
            color = QColor(p.get("color", "#00f0ff"))
            color.setAlpha(int(p.get("alpha", 255)))

            pen = QPen(color, 2)
            painter.setPen(pen)
            painter.drawLine(
                int(p["x"]), int(p["y"]),
                int(p["x"] + p["vx"] * 10),
                int(p["y"] + p["vy"] * 10)
            )

    def wheelEvent(self, event):
        """Zoom with mouse wheel."""
        delta = event.angleDelta().y()
        factor = 1.1 if delta > 0 else 0.9

        old_zoom = self.zoom
        self.zoom = max(self.min_zoom, min(self.max_zoom, self.zoom * factor))

        # Zoom toward mouse position
        mouse_pos = event.position()
        self.offset_x = mouse_pos.x() - (mouse_pos.x() - self.offset_x) * (self.zoom / old_zoom)
        self.offset_y = mouse_pos.y() - (mouse_pos.y() - self.offset_y) * (self.zoom / old_zoom)

        self.update()

    def mousePressEvent(self, event):
        """Handle mouse press for dragging and selection."""
        if event.button() == Qt.MouseButton.LeftButton:
            # Check for feature click
            feature = self.get_feature_at(event.position().x(), event.position().y())
            if feature:
                self.selected_feature = feature
                self.feature_clicked.emit(feature)
                self.update()
                return

            # Start dragging
            self.dragging = True
            self.drag_start = event.position()
            self.drag_offset_start = QPointF(self.offset_x, self.offset_y)

        elif event.button() == Qt.MouseButton.RightButton:
            # Center on click
            world = self.screen_to_world(event.position().x(), event.position().y())
            self.center_on(world.x(), world.y())
            self.update()

    def mouseReleaseEvent(self, event):
        """Handle mouse release."""
        self.dragging = False

    def mouseMoveEvent(self, event):
        """Handle mouse move for dragging and hover."""
        if self.dragging:
            delta = event.position() - self.drag_start
            self.offset_x = self.drag_offset_start.x() + delta.x()
            self.offset_y = self.drag_offset_start.y() + delta.y()
            self.update()
        else:
            # Hover detection
            feature = self.get_feature_at(event.position().x(), event.position().y())
            if feature != self.hovered_feature:
                self.hovered_feature = feature
                if feature:
                    self.feature_hovered.emit(feature)
                self.update()

    def resizeEvent(self, event):
        """Handle resize."""
        super().resizeEvent(event)
        self.update()
