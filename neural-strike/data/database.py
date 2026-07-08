"""SQLite database for NEURAL-STRIKE feature cache and territories"""
import sqlite3
import json
import os
from typing import Optional

class NeuralStrikeDB:
    """Local database for caching features and tracking territories."""

    def __init__(self, db_path: str):
        self.db_path = db_path
        os.makedirs(os.path.dirname(db_path), exist_ok=True)
        self.conn = sqlite3.connect(db_path)
        self._create_tables()

    def _create_tables(self):
        """Create database tables if they don't exist."""
        self.conn.executescript("""
            CREATE TABLE IF NOT EXISTS features (
                feature_id TEXT PRIMARY KEY,
                layer INTEGER,
                index_in_layer INTEGER,
                explanation TEXT,
                top_tokens TEXT,
                umap_x REAL,
                umap_y REAL,
                cluster TEXT,
                last_updated TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS activations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                feature_id TEXT,
                prompt TEXT,
                activation_value REAL,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (feature_id) REFERENCES features(feature_id)
            );

            CREATE TABLE IF NOT EXISTS territories (
                feature_id TEXT PRIMARY KEY,
                gang_id TEXT,
                capture_time TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                credits INTEGER DEFAULT 0,
                FOREIGN KEY (feature_id) REFERENCES features(feature_id)
            );

            CREATE TABLE IF NOT EXISTS scan_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                prompt TEXT,
                features_triggered TEXT,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE INDEX IF NOT EXISTS idx_features_layer ON features(layer);
            CREATE INDEX IF NOT EXISTS idx_features_cluster ON features(cluster);
            CREATE INDEX IF NOT EXISTS idx_activations_feature ON activations(feature_id);
        """)
        self.conn.commit()

    def upsert_feature(self, feature_id: str, layer: int, index: int,
                        explanation: str = None, top_tokens: list = None,
                        umap_x: float = None, umap_y: float = None,
                        cluster: str = None):
        """Insert or update a feature."""
        self.conn.execute("""
            INSERT OR REPLACE INTO features
            (feature_id, layer, index_in_layer, explanation, top_tokens, umap_x, umap_y, cluster)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        """, (
            feature_id, layer, index,
            explanation,
            json.dumps(top_tokens) if top_tokens else None,
            umap_x, umap_y, cluster
        ))
        self.conn.commit()

    def get_feature(self, feature_id: str) -> Optional[dict]:
        """Get a feature by ID."""
        row = self.conn.execute(
            "SELECT * FROM features WHERE feature_id = ?", (feature_id,)
        ).fetchone()
        if row:
            return {
                "feature_id": row[0],
                "layer": row[1],
                "index": row[2],
                "explanation": row[3],
                "top_tokens": json.loads(row[4]) if row[4] else [],
                "umap_x": row[5],
                "umap_y": row[6],
                "cluster": row[7],
            }
        return None

    def get_features_by_layer(self, layer: int) -> list[dict]:
        """Get all features for a layer."""
        rows = self.conn.execute(
            "SELECT * FROM features WHERE layer = ?", (layer,)
        ).fetchall()
        return [
            {
                "feature_id": r[0], "layer": r[1], "index": r[2],
                "explanation": r[3],
                "top_tokens": json.loads(r[4]) if r[4] else [],
                "umap_x": r[5], "umap_y": r[6], "cluster": r[7],
            }
            for r in rows
        ]

    def get_features_by_cluster(self, cluster: str) -> list[dict]:
        """Get all features in a cluster."""
        rows = self.conn.execute(
            "SELECT * FROM features WHERE cluster = ?", (cluster,)
        ).fetchall()
        return [
            {
                "feature_id": r[0], "layer": r[1], "index": r[2],
                "explanation": r[3],
                "top_tokens": json.loads(r[4]) if r[4] else [],
                "umap_x": r[5], "umap_y": r[6], "cluster": r[7],
            }
            for r in rows
        ]

    def get_all_features(self) -> list[dict]:
        """Get all features."""
        rows = self.conn.execute("SELECT * FROM features").fetchall()
        return [
            {
                "feature_id": r[0], "layer": r[1], "index": r[2],
                "explanation": r[3],
                "top_tokens": json.loads(r[4]) if r[4] else [],
                "umap_x": r[5], "umap_y": r[6], "cluster": r[7],
            }
            for r in rows
        ]

    def record_activation(self, feature_id: str, prompt: str, value: float):
        """Record a feature activation."""
        self.conn.execute(
            "INSERT INTO activations (feature_id, prompt, activation_value) VALUES (?, ?, ?)",
            (feature_id, prompt, value)
        )
        self.conn.commit()

    def get_activations(self, feature_id: str, limit: int = 10) -> list[dict]:
        """Get recent activations for a feature."""
        rows = self.conn.execute(
            "SELECT prompt, activation_value, timestamp FROM activations WHERE feature_id = ? ORDER BY timestamp DESC LIMIT ?",
            (feature_id, limit)
        ).fetchall()
        return [{"prompt": r[0], "value": r[1], "timestamp": r[2]} for r in rows]

    def capture_territory(self, feature_id: str, gang_id: str, credits: int = 10):
        """Capture a territory for a gang."""
        self.conn.execute(
            "INSERT OR REPLACE INTO territories (feature_id, gang_id, credits) VALUES (?, ?, ?)",
            (feature_id, gang_id, credits)
        )
        self.conn.commit()

    def get_territory(self, feature_id: str) -> Optional[dict]:
        """Get territory info for a feature."""
        row = self.conn.execute(
            "SELECT gang_id, capture_time, credits FROM territories WHERE feature_id = ?",
            (feature_id,)
        ).fetchone()
        if row:
            return {"gang_id": row[0], "capture_time": row[1], "credits": row[2]}
        return None

    def get_gang_territories(self, gang_id: str) -> list[dict]:
        """Get all territories for a gang."""
        rows = self.conn.execute(
            "SELECT feature_id, capture_time, credits FROM territories WHERE gang_id = ?",
            (gang_id,)
        ).fetchall()
        return [{"feature_id": r[0], "capture_time": r[1], "credits": r[2]} for r in rows]

    def save_scan(self, prompt: str, features_triggered: list[str]):
        """Save a scan to history."""
        self.conn.execute(
            "INSERT INTO scan_history (prompt, features_triggered) VALUES (?, ?)",
            (prompt, json.dumps(features_triggered))
        )
        self.conn.commit()

    def get_feature_count(self) -> int:
        """Get total number of cached features."""
        return self.conn.execute("SELECT COUNT(*) FROM features").fetchone()[0]

    def close(self):
        """Close database connection."""
        self.conn.close()
