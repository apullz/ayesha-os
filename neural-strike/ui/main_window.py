"""Main Window for NEURAL-STRIKE: LATENT TERRITORY"""
import os
import json
import threading
from PyQt6.QtWidgets import (
    QMainWindow, QWidget, QVBoxLayout, QHBoxLayout,
    QLabel, QStatusBar, QMenuBar, QMenu, QToolBar,
    QSplitter, QMessageBox
)
from PyQt6.QtCore import Qt, pyqtSignal, QObject, QTimer
from PyQt6.QtGui import QAction, QFont, QColor

from ui.umap_viewport import UMAPViewport
from ui.token_scanner import TokenScanner
from ui.feature_inspector import FeatureInspector
from ui.effects import CRTOverlay, GlitchEffect, DataStreamEffect
from data.database import NeuralStrikeDB
from data.neuronpedia_client import NeuronpediaClient
import config


class WorkerSignals(QObject):
    """Signals for worker threads."""
    features_loaded = pyqtSignal(list)
    scan_complete = pyqtSignal(list)
    scan_error = pyqtSignal(str)
    feature_data = pyqtSignal(dict)


class MainWindow(QMainWindow):
    """Main application window for NEURAL-STRIKE."""

    def __init__(self):
        super().__init__()

        # Initialize data layer
        self.db = NeuralStrikeDB(config.DB_PATH)
        self.client = NeuronpediaClient(config.EXPORTS_DIR)
        self.signals = WorkerSignals()

        # Connect signals
        self.signals.features_loaded.connect(self._on_features_loaded)
        self.signals.scan_complete.connect(self._on_scan_complete)
        self.signals.scan_error.connect(self._on_scan_error)
        self.signals.feature_data.connect(self._on_feature_data)

        # Setup UI
        self._setup_window()
        self._setup_menu()
        self._setup_toolbar()
        self._setup_ui()
        self._setup_statusbar()
        self._setup_effects()

        # Load initial data
        self._load_features()

    def _setup_window(self):
        """Configure main window properties."""
        self.setWindowTitle("NEURAL-STRIKE: LATENT TERRITORY")
        self.setMinimumSize(1200, 800)
        self.resize(config.WINDOW_WIDTH, config.WINDOW_HEIGHT)

        # Apply dark theme
        theme_path = os.path.join(config.BASE_DIR, "assets", "theme.qss")
        if os.path.exists(theme_path):
            with open(theme_path, "r") as f:
                self.setStyleSheet(f.read())

    def _setup_menu(self):
        """Setup menu bar."""
        menubar = self.menuBar()
        menubar.setStyleSheet("background-color: #12121a; color: #808090;")

        # File menu
        file_menu = menubar.addMenu("File")
        file_menu.addAction("Export Territory Map", self._export_map)
        file_menu.addAction("Export Feature Data", self._export_data)
        file_menu.addSeparator()
        file_menu.addAction("Exit", self.close)

        # Scan menu
        scan_menu = menubar.addMenu("Scan")
        scan_menu.addAction("Quick Scan", self._quick_scan)
        scan_menu.addAction("Deep Scan", self._deep_scan)

        # View menu
        view_menu = menubar.addMenu("View")
        view_menu.addAction("Reset Zoom", self._reset_zoom)
        view_menu.addAction("Center on Toxic", lambda: self._center_on_cluster("toxic"))
        view_menu.addAction("Center on Scotland", lambda: self._center_on_cluster("scotland"))

        # Help menu
        help_menu = menubar.addMenu("Help")
        help_menu.addAction("About", self._show_about)

    def _setup_toolbar(self):
        """Setup toolbar."""
        toolbar = QToolBar()
        toolbar.setMovable(False)
        toolbar.setStyleSheet("""
            QToolBar {
                background-color: #12121a;
                border-bottom: 1px solid #1a1a2e;
                padding: 4px;
            }
            QToolButton {
                color: #00f0ff;
                background-color: #1a1a2e;
                border: 1px solid #00f0ff;
                border-radius: 4px;
                padding: 6px 12px;
                margin: 2px;
            }
            QToolButton:hover {
                background-color: #00f0ff;
                color: #0a0a0f;
            }
        """)
        self.addToolBar(toolbar)

    def _setup_ui(self):
        """Setup the main UI layout."""
        central_widget = QWidget()
        self.setCentralWidget(central_widget)

        main_layout = QHBoxLayout(central_widget)
        main_layout.setContentsMargins(0, 0, 0, 0)
        main_layout.setSpacing(0)

        # Main splitter
        splitter = QSplitter(Qt.Orientation.Horizontal)

        # Left panel - Scanner
        self.scanner = TokenScanner()
        self.scanner.scan_requested.connect(self._on_scan_requested)
        splitter.addWidget(self.scanner)

        # Center - U-MAP Viewport
        center_widget = QWidget()
        center_layout = QVBoxLayout(center_widget)
        center_layout.setContentsMargins(0, 0, 0, 0)

        # Header
        header = QHBoxLayout()
        title = QLabel("NEURAL-STRIKE")
        title.setStyleSheet("color: #00f0ff; font-size: 20px; font-weight: bold; padding: 8px;")
        header.addWidget(title)

        subtitle = QLabel("Gemma-2-2B Latent Space | l0git_bombers")
        subtitle.setStyleSheet("color: #808090; font-size: 11px; padding: 8px;")
        header.addWidget(subtitle)
        header.addStretch()

        stats = QLabel(f"Features: {self.db.get_feature_count()}")
        stats.setObjectName("subtitle")
        self.stats_label = stats
        header.addWidget(stats)

        center_layout.addLayout(header)

        # U-MAP viewport
        self.viewport = UMAPViewport()
        self.viewport.feature_clicked.connect(self._on_feature_clicked)
        self.viewport.feature_hovered.connect(self._on_feature_hovered)
        center_layout.addWidget(self.viewport)

        splitter.addWidget(center_widget)

        # Right panel - Inspector
        self.inspector = FeatureInspector()
        self.inspector.steer_requested.connect(self._on_steer_requested)
        self.inspector.capture_requested.connect(self._on_capture_requested)
        splitter.addWidget(self.inspector)

        # Set splitter proportions
        splitter.setSizes([300, 800, 300])

        main_layout.addWidget(splitter)

    def _setup_statusbar(self):
        """Setup status bar."""
        self.statusbar = QStatusBar()
        self.setStatusBar(self.statusbar)

        self.status_label = QLabel("Initializing...")
        self.statusbar.addWidget(self.status_label)

        self.gang_label = QLabel("Gang: l0git_bombers")
        self.gang_label.setStyleSheet("color: #ff0040; font-weight: bold;")
        self.statusbar.addPermanentWidget(self.gang_label)

        self.credits_label = QLabel("Credits: 0")
        self.credits_label.setStyleSheet("color: #00ff88;")
        self.statusbar.addPermanentWidget(self.credits_label)

    def _setup_effects(self):
        """Setup visual effects."""
        # CRT overlay
        self.crt = CRTOverlay(self.centralWidget())
        self.crt.setGeometry(self.centralWidget().rect())

        # Glitch effect
        self.glitch = GlitchEffect(self.centralWidget())
        self.glitch.setGeometry(self.centralWidget().rect())

        # Data stream effect
        self.data_stream = DataStreamEffect(self.centralWidget())
        self.data_stream.setGeometry(self.centralWidget().rect())

    def _load_features(self):
        """Load features in background thread."""
        self.status_label.setText("Loading features...")
        thread = threading.Thread(target=self._load_features_worker)
        thread.daemon = True
        thread.start()

    def _load_features_worker(self):
        """Background worker to load features."""
        features = []

        # Generate synthetic feature data for visualization
        # In production, this would load from Neuronpedia exports
        import random

        clusters = {
            "toxic": {"center": (0.3, 0.7), "spread": 0.15, "count": 500},
            "scotland": {"center": (-0.5, -0.3), "spread": 0.12, "count": 400},
            "geography": {"center": (0.0, 0.0), "spread": 0.25, "count": 800},
            "sentiment": {"center": (0.5, -0.5), "spread": 0.18, "count": 600},
            "neutral": {"center": (0.0, 0.0), "spread": 0.4, "count": 2000},
        }

        for cluster_name, info in clusters.items():
            cx, cy = info["center"]
            spread = info["spread"]

            for i in range(info["count"]):
                # Gaussian distribution around cluster center
                x = cx + random.gauss(0, spread)
                y = cy + random.gauss(0, spread)
                layer = random.randint(0, 25)
                index = random.randint(0, 16383)

                features.append({
                    "x": x,
                    "y": y,
                    "feature_id": f"{layer}_{index}",
                    "layer": layer,
                    "index": index,
                    "cluster": cluster_name,
                    "explanation": f"Feature at layer {layer}, index {index} ({cluster_name})",
                    "top_tokens": [],
                })

        self.signals.features_loaded.emit(features)

    def _on_features_loaded(self, features: list):
        """Handle features loaded."""
        self.viewport.set_features(features)
        self.stats_label.setText(f"Features: {len(features)}")
        self.status_label.setText(f"Loaded {len(features)} features")

    def _on_feature_data(self, feature: dict):
        """Handle individual feature data loaded."""
        pass

    def _on_feature_clicked(self, feature: dict):
        """Handle feature click in viewport."""
        self.inspector.set_feature(feature)

        # Trigger glitch effect for toxic features
        if feature.get("cluster") == "toxic":
            self.glitch.trigger(0.5)

    def _on_feature_hovered(self, feature: dict):
        """Handle feature hover."""
        feature_id = feature.get("feature_id", "?")
        layer = feature.get("layer", "?")
        cluster = feature.get("cluster", "?")
        self.status_label.setText(f"Feature: {feature_id} | Layer: {layer} | Cluster: {cluster}")

    def _on_scan_requested(self, prompt: str):
        """Handle scan request from scanner."""
        thread = threading.Thread(target=self._scan_worker, args=(prompt,))
        thread.daemon = True
        thread.start()

    def _scan_worker(self, prompt: str):
        """Background worker for scanning."""
        try:
            # Simulate scanning by finding features that match prompt keywords
            features = self.viewport.features
            triggered = []

            prompt_lower = prompt.lower()
            for f in features:
                explanation = f.get("explanation", "").lower()
                cluster = f.get("cluster", "")

                # Simple keyword matching
                if (prompt_lower in explanation or
                    cluster in prompt_lower or
                    any(kw in prompt_lower for kw in ["toxic", "scotland", "hate", "insult"])):
                    import random
                    activation = random.uniform(1.5, 5.0)
                    if activation > config.ACTIVATION_THRESHOLD:
                        triggered.append({
                            "feature_id": f["feature_id"],
                            "activation": activation,
                            "x": f["x"],
                            "y": f["y"],
                        })

            # Sort by activation
            triggered.sort(key=lambda x: x["activation"], reverse=True)

            # Pulse activated features
            for t in triggered[:20]:
                self.viewport.pulse_feature(t["feature_id"], t["activation"] / 5.0)

            self.signals.scan_complete.emit(triggered)

        except Exception as e:
            self.signals.scan_error.emit(str(e))

    def _on_scan_complete(self, features_triggered: list):
        """Handle scan completion."""
        self.scanner.scan_complete(features_triggered)
        self.glitch.trigger(0.3)

        # Save to database
        if features_triggered:
            self.db.save_scan(
                self.scanner.input_field.toPlainText(),
                [f["feature_id"] for f in features_triggered]
            )

    def _on_scan_error(self, error_msg: str):
        """Handle scan error."""
        self.scanner.scan_error(error_msg)

    def _on_steer_requested(self, steer_data: dict):
        """Handle steering request."""
        self.status_label.setText(f"Steering feature {steer_data['feature_id']}...")
        # In production, this would call the Neuronpedia API
        self.status_label.setText("Steering simulated (API rate limited)")

    def _on_capture_requested(self, capture_data: dict):
        """Handle territory capture."""
        feature_id = capture_data.get("feature_id")
        if feature_id:
            self.db.capture_territory(feature_id, "l0git_bombers")
            self.inspector.territory_label.setText("Captured by l0git_bombers")
            self.inspector.territory_label.setStyleSheet("color: #ff0040;")
            self.credits_label.setText(f"Credits: {int(self.credits_label.text().split(': ')[1]) + 10}")

    def _quick_scan(self):
        """Quick scan of current view."""
        prompt = "What features are active in the current view?"
        self.scanner.input_field.setText(prompt)
        self._on_scan_requested(prompt)

    def _deep_scan(self):
        """Deep scan of entire model."""
        QMessageBox.information(
            self,
            "Deep Scan",
            "Deep scan would analyze all 131K features.\n"
            "This requires significant compute time."
        )

    def _reset_zoom(self):
        """Reset viewport zoom."""
        self.viewport.zoom = 1.0
        if self.viewport.features:
            xs = [f["x"] for f in self.viewport.features]
            ys = [f["y"] for f in self.viewport.features]
            self.viewport.center_on(
                (min(xs) + max(xs)) / 2,
                (min(ys) + max(ys)) / 2
            )
        self.viewport.update()

    def _center_on_cluster(self, cluster: str):
        """Center viewport on a specific cluster."""
        features = [f for f in self.viewport.features if f.get("cluster") == cluster]
        if features:
            xs = [f["x"] for f in features]
            ys = [f["y"] for f in features]
            self.viewport.center_on(
                (min(xs) + max(xs)) / 2,
                (min(ys) + max(ys)) / 2
            )
            self.viewport.update()

    def _export_map(self):
        """Export territory map as image."""
        QMessageBox.information(self, "Export", "Map export would save viewport as PNG.")

    def _export_data(self):
        """Export feature data as JSON."""
        features = self.viewport.features
        path = os.path.join(config.DATA_DIR, "exported_features.json")
        with open(path, "w") as f:
            json.dump(features, f, indent=2)
        QMessageBox.information(self, "Export", f"Exported {len(features)} features to:\n{path}")

    def _show_about(self):
        """Show about dialog."""
        QMessageBox.about(
            self,
            "NEURAL-STRIKE: LATENT TERRITORY",
            "Version 0.1.0\n\n"
            "A mechanistic interpretability game combining\n"
            "Neuronpedia's SAE visualization with WDGWars'\n"
            "territory capture mechanics.\n\n"
            "Gang: l0git_bombers\n"
            "Target: Gemma-2-2B"
        )

    def resizeEvent(self, event):
        """Handle window resize."""
        super().resizeEvent(event)
        # Update effects overlay size
        self.crt.setGeometry(self.centralWidget().rect())
        self.glitch.setGeometry(self.centralWidget().rect())
        self.data_stream.setGeometry(self.centralWidget().rect())
