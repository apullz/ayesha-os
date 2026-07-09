"""
tri-mind sync - bidirectional sync between ayesha-os, github, and huggingface
"""

from .engine import TriMindEngine
from .config import TriMindConfig

__version__ = "1.0.0"
__all__ = ["TriMindEngine", "TriMindConfig"]
