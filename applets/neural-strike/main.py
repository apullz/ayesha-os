"""
NEURAL-STRIKE: LATENT TERRITORY
A mechanistic interpretability game combining Neuronpedia's SAE visualization
with WDGWars' territory capture mechanics.

Target Model: Gemma-2-2B
Gang: l0git_bombers
"""
import sys
import os

# Add project root to path
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from PyQt6.QtWidgets import QApplication
from PyQt6.QtCore import Qt
from PyQt6.QtGui import QFont

from ui.main_window import MainWindow


def main():
    """Main entry point."""
    # High DPI support
    os.environ["QT_AUTO_SCREEN_SCALE_FACTOR"] = "1"

    app = QApplication(sys.argv)
    app.setApplicationName("NEURAL-STRIKE")
    app.setOrganizationName("l0git_bombers")

    # Set default font
    font = QFont("Consolas", 10)
    app.setFont(font)

    # Create and show main window
    window = MainWindow()
    window.show()

    print("=" * 60)
    print("  NEURAL-STRIKE: LATENT TERRITORY")
    print("  Target: Gemma-2-2B | Gang: l0git_bombers")
    print("=" * 60)
    print("  Controls:")
    print("    Left-click: Select feature")
    print("    Right-click: Center view")
    print("    Scroll: Zoom in/out")
    print("    Drag: Pan view")
    print("=" * 60)

    sys.exit(app.exec())


if __name__ == "__main__":
    main()
