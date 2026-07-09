"""Feature Inspector panel for NEURAL-STRIKE"""
from PyQt6.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QLabel, QSlider,
    QPushButton, QTextEdit, QListWidget, QListWidgetItem,
    QGroupBox, QSpinBox
)
from PyQt6.QtCore import Qt, pyqtSignal
from PyQt6.QtGui import QColor, QFont


class FeatureInspector(QWidget):
    """Right panel - Feature inspector and steering controls."""

    steer_requested = pyqtSignal(dict)
    capture_requested = pyqtSignal(dict)

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setMaximumWidth(350)
        self.setMinimumWidth(280)

        self.current_feature = None
        self._setup_ui()

    def _setup_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(8, 8, 8, 8)
        layout.setSpacing(8)

        # Title
        title = QLabel("FEATURE INSPECTOR")
        title.setObjectName("title")
        title.setAlignment(Qt.AlignmentFlag.AlignCenter)
        layout.addWidget(title)

        # Feature ID display
        self.feature_id_label = QLabel("No feature selected")
        self.feature_id_label.setObjectName("feature_id")
        self.feature_id_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        layout.addWidget(self.feature_id_label)

        # Layer info
        self.layer_label = QLabel("")
        self.layer_label.setObjectName("subtitle")
        self.layer_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        layout.addWidget(self.layer_label)

        # Explanation
        explanation_group = QGroupBox("Auto-Interp Explanation")
        expl_layout = QVBoxLayout()

        self.explanation_text = QTextEdit()
        self.explanation_text.setObjectName("feature_explanation")
        self.explanation_text.setReadOnly(True)
        self.explanation_text.setMaximumHeight(100)
        self.explanation_text.setPlaceholderText("Click a feature in the U-MAP to see its explanation...")
        expl_layout.addWidget(self.explanation_text)

        explanation_group.setLayout(expl_layout)
        layout.addWidget(explanation_group)

        # Top Tokens
        tokens_group = QGroupBox("Top Activating Tokens")
        tokens_layout = QVBoxLayout()

        self.tokens_list = QListWidget()
        self.tokens_list.setMaximumHeight(120)
        tokens_layout.addWidget(self.tokens_list)

        tokens_group.setLayout(tokens_layout)
        layout.addWidget(tokens_group)

        # Steering Controls
        steer_group = QGroupBox("Feature Steering")
        steer_layout = QVBoxLayout()

        # Strength slider
        slider_layout = QHBoxLayout()
        slider_layout.addWidget(QLabel("Strength:"))

        self.strength_slider = QSlider(Qt.Orientation.Horizontal)
        self.strength_slider.setRange(-100, 100)
        self.strength_slider.setValue(0)
        self.strength_slider.setTickPosition(QSlider.TickPosition.TicksBelow)
        self.strength_slider.setTickInterval(10)
        slider_layout.addWidget(self.strength_slider)

        self.strength_value = QLabel("0.0")
        self.strength_value.setMinimumWidth(40)
        slider_layout.addWidget(self.strength_value)

        steer_layout.addLayout(slider_layout)

        self.strength_slider.valueChanged.connect(
            lambda v: self.strength_value.setText(f"{v/10:.1f}")
        )

        # Steer button
        self.steer_btn = QPushButton("STEER MODEL")
        self.steer_btn.setObjectName("success")
        self.steer_btn.clicked.connect(self._on_steer)
        steer_layout.addWidget(self.steer_btn)

        steer_group.setLayout(steer_layout)
        layout.addWidget(steer_group)

        # Territory
        territory_group = QGroupBox("Territory Control")
        territory_layout = QVBoxLayout()

        self.territory_label = QLabel("Unclaimed")
        self.territory_label.setObjectName("subtitle")
        territory_layout.addWidget(self.territory_label)

        self.capture_btn = QPushButton("CAPTURE TERRITORY")
        self.capture_btn.setObjectName("danger")
        self.capture_btn.clicked.connect(self._on_capture)
        territory_layout.addWidget(self.capture_btn)

        territory_group.setLayout(territory_layout)
        layout.addWidget(territory_group)

        # Activation History
        history_group = QGroupBox("Recent Activations")
        history_layout = QVBoxLayout()

        self.history_list = QListWidget()
        self.history_list.setMaximumHeight(100)
        history_layout.addWidget(self.history_list)

        history_group.setLayout(history_layout)
        layout.addWidget(history_group)

        layout.addStretch()

    def set_feature(self, feature: dict):
        """Display a feature's details."""
        self.current_feature = feature

        feature_id = feature.get("feature_id", "unknown")
        layer = feature.get("layer", "?")
        explanation = feature.get("explanation", "No explanation available")
        top_tokens = feature.get("top_tokens", [])
        cluster = feature.get("cluster", "neutral")

        self.feature_id_label.setText(feature_id)
        self.layer_label.setText(f"Layer {layer} | Cluster: {cluster}")
        self.explanation_text.setText(explanation)

        # Update tokens list
        self.tokens_list.clear()
        if isinstance(top_tokens, list):
            for token in top_tokens[:10]:
                if isinstance(token, dict):
                    text = f"{token.get('token', '?')}: {token.get('score', 0):.3f}"
                else:
                    text = str(token)
                item = QListWidgetItem(text)
                item.setForeground(QColor("#00f0ff"))
                self.tokens_list.addItem(item)

        # Update territory display
        # This would normally come from the database
        self.territory_label.setText(f"Cluster: {cluster}")

    def clear(self):
        """Clear the inspector."""
        self.current_feature = None
        self.feature_id_label.setText("No feature selected")
        self.layer_label.setText("")
        self.explanation_text.clear()
        self.tokens_list.clear()
        self.history_list.clear()
        self.strength_slider.setValue(0)
        self.territory_label.setText("Unclaimed")

    def _on_steer(self):
        """Handle steer button click."""
        if not self.current_feature:
            return

        strength = self.strength_slider.value() / 10.0

        steer_data = {
            "feature_id": self.current_feature.get("feature_id"),
            "layer": self.current_feature.get("layer"),
            "index": self.current_feature.get("index"),
            "strength": strength,
        }
        self.steer_requested.emit(steer_data)

    def _on_capture(self):
        """Handle capture button click."""
        if not self.current_feature:
            return

        capture_data = {
            "feature_id": self.current_feature.get("feature_id"),
            "cluster": self.current_feature.get("cluster"),
        }
        self.capture_requested.emit(capture_data)

    def add_activation(self, prompt: str, value: float):
        """Add an activation to the history."""
        item = QListWidgetItem(f"{value:.2f}σ: {prompt[:40]}...")
        if value > 3.0:
            item.setForeground(QColor("#ff0040"))
        elif value > 2.0:
            item.setForeground(QColor("#ffaa00"))
        else:
            item.setForeground(QColor("#808090"))

        self.history_list.insertItem(0, item)
        if self.history_list.count() > 20:
            self.history_list.takeItem(20)
