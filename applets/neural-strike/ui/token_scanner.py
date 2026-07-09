"""Token Scanner panel for NEURAL-STRIKE"""
from PyQt6.QtWidgets import (
    QWidget, QVBoxLayout, QHBoxLayout, QTextEdit,
    QPushButton, QLabel, QProgressBar
)
from PyQt6.QtCore import Qt, pyqtSignal, QTimer
from PyQt6.QtGui import QColor, QTextCharFormat, QFont


class TokenScanner(QWidget):
    """Left panel - Matrix-rain token scanner."""

    scan_requested = pyqtSignal(str)

    def __init__(self, parent=None):
        super().__init__(parent)
        self.setMaximumWidth(350)
        self.setMinimumWidth(250)

        self._setup_ui()
        self._scan_active = False

    def _setup_ui(self):
        layout = QVBoxLayout(self)
        layout.setContentsMargins(8, 8, 8, 8)
        layout.setSpacing(8)

        # Title
        title = QLabel("TOKEN SCANNER")
        title.setObjectName("title")
        title.setAlignment(Qt.AlignmentFlag.AlignCenter)
        layout.addWidget(title)

        # Status
        self.status_label = QLabel("IDLE")
        self.status_label.setObjectName("subtitle")
        self.status_label.setAlignment(Qt.AlignmentFlag.AlignCenter)
        layout.addWidget(self.status_label)

        # Scanner output
        self.scanner_output = QTextEdit()
        self.scanner_output.setObjectName("scanner")
        self.scanner_output.setReadOnly(True)
        self.scanner_output.setPlaceholderText(
            "Enter a prompt below and click SCAN...\n\n"
            "Matrix rain of tokens will appear here\n"
            "with activation lines shooting to the U-MAP."
        )
        layout.addWidget(self.scanner_output)

        # Progress bar
        self.progress = QProgressBar()
        self.progress.setVisible(False)
        self.progress.setTextVisible(False)
        layout.addWidget(self.progress)

        # Input area
        input_layout = QHBoxLayout()

        self.input_field = QTextEdit()
        self.input_field.setMaximumHeight(80)
        self.input_field.setPlaceholderText("Enter prompt to scan...")
        self.input_field.setObjectName("scanner")
        input_layout.addWidget(self.input_field)

        layout.addLayout(input_layout)

        # Buttons
        btn_layout = QHBoxLayout()

        self.scan_btn = QPushButton("SCAN")
        self.scan_btn.setObjectName("success")
        self.scan_btn.clicked.connect(self._on_scan)
        btn_layout.addWidget(self.scan_btn)

        self.clear_btn = QPushButton("CLEAR")
        self.clear_btn.clicked.connect(self._clear_output)
        btn_layout.addWidget(self.clear_btn)

        layout.addLayout(btn_layout)

        # Stats
        self.stats_label = QLabel("Features triggered: 0")
        self.stats_label.setObjectName("subtitle")
        layout.addWidget(self.stats_label)

    def _on_scan(self):
        """Handle scan button click."""
        prompt = self.input_field.toPlainText().strip()
        if not prompt:
            return

        self._scan_active = True
        self.status_label.setText("SCANNING...")
        self.status_label.setStyleSheet("color: #00ff88;")
        self.progress.setVisible(True)
        self.progress.setRange(0, 0)  # Indeterminate

        self.scan_requested.emit(prompt)

    def _clear_output(self):
        """Clear the scanner output."""
        self.scanner_output.clear()
        self.stats_label.setText("Features triggered: 0")

    def append_token(self, token: str, is_activation: bool = False, feature_id: str = None):
        """Append a token to the scanner output with optional highlighting."""
        cursor = self.scanner_output.textCursor()

        format_normal = QTextCharFormat()
        format_normal.setForeground(QColor("#00ff88"))
        format_normal.setFont(QFont("Consolas", 11))

        format_activation = QTextCharFormat()
        format_activation.setForeground(QColor("#ff0040"))
        format_activation.setFont(QFont("Consolas", 11, QFont.Weight.Bold))

        if is_activation:
            cursor.setCharFormat(format_activation)
            cursor.insertText(f"[{token}] ")
        else:
            cursor.setCharFormat(format_normal)
            cursor.insertText(f"{token} ")

        self.scanner_output.setTextCursor(cursor)
        self.scanner_output.ensureCursorVisible()

    def append_activation_line(self, feature_id: str, activation: float):
        """Append an activation line to the output."""
        cursor = self.scanner_output.textCursor()

        format_line = QTextCharFormat()
        format_line.setForeground(QColor("#00f0ff"))
        format_line.setFont(QFont("Consolas", 10))

        cursor.setCharFormat(format_line)
        cursor.insertText(f"\n  -> {feature_id}: {activation:.2f}σ\n")

        self.scanner_output.setTextCursor(cursor)
        self.scanner_output.ensureCursorVisible()

    def scan_complete(self, features_triggered: list[dict]):
        """Handle scan completion."""
        self._scan_active = False
        self.status_label.setText("SCAN COMPLETE")
        self.status_label.setStyleSheet("color: #00f0ff;")
        self.progress.setVisible(False)

        count = len(features_triggered)
        self.stats_label.setText(f"Features triggered: {count}")

        if count > 0:
            self.append_activation_line("SCAN", 0)
            for f in features_triggered[:10]:
                self.append_activation_line(
                    f.get("feature_id", "?"),
                    f.get("activation", 0)
                )

    def scan_error(self, error_msg: str):
        """Handle scan error."""
        self._scan_active = False
        self.status_label.setText("ERROR")
        self.status_label.setStyleSheet("color: #ff0040;")
        self.progress.setVisible(False)

        cursor = self.scanner_output.textCursor()
        format_error = QTextCharFormat()
        format_error.setForeground(QColor("#ff0040"))
        cursor.setCharFormat(format_error)
        cursor.insertText(f"\n[ERROR] {error_msg}\n")
        self.scanner_output.setTextCursor(cursor)
