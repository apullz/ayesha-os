"""NEURAL-STRIKE: Local-only feature data client."""
import json
import os
from typing import Optional

class NeuronpediaClient:
    """Local-only client for pre-downloaded feature data.

    All network calls are removed. Load data from local cache/export files.
    Run `download_data.py` or place JSON exports in the data/exports/ directory.
    """

    def __init__(self, cache_dir: str):
        self.cache_dir = cache_dir
        self.model_id = "gemma-2-2b"

    def get_feature(self, layer: int, index: int) -> Optional[dict]:
        """Load a single feature from local cache."""
        cache_key = f"feature_{layer}_{index}"
        return self._load_cache(cache_key)

    def steer(self, prompt: str, features: list[dict], **kwargs) -> Optional[dict]:
        """Steering requires the Neuronpedia API — not available locally."""
        return None

    def get_umap_coordinates(self, layer: int) -> Optional[list]:
        """Load UMAP coordinates from local cache."""
        cache_key = f"umap_layer_{layer}"
        return self._load_cache(cache_key)

    def get_feature_explanations(self, layer: int) -> Optional[dict]:
        """Load auto-interp explanations from local cache."""
        cache_key = f"explanations_layer_{layer}"
        return self._load_cache(cache_key)

    def search_features(self, query: str, limit: int = 20) -> list[dict]:
        """Search features by keyword in cached explanations."""
        results = []
        query_lower = query.lower()

        for layer in range(26):
            explanations = self.get_feature_explanations(layer)
            if not explanations:
                continue

            for idx, expl in explanations.items():
                if isinstance(expl, dict):
                    text = expl.get("explanation", "").lower()
                elif isinstance(expl, str):
                    text = expl.lower()
                else:
                    continue

                if query_lower in text:
                    results.append({
                        "layer": layer,
                        "index": int(idx),
                        "explanation": expl,
                        "feature_id": f"{layer}_{idx}"
                    })
                    if len(results) >= limit:
                        return results
        return results

    def _load_cache(self, key: str) -> Optional[dict]:
        """Load data from local cache."""
        path = os.path.join(self.cache_dir, f"{key}.json")
        if os.path.exists(path):
            try:
                with open(path, "r") as f:
                    return json.load(f)
            except Exception:
                pass
        return None

    def _save_cache(self, key: str, data: dict):
        """Save data to local cache."""
        os.makedirs(self.cache_dir, exist_ok=True)
        path = os.path.join(self.cache_dir, f"{key}.json")
        try:
            with open(path, "w") as f:
                json.dump(data, f)
        except Exception as e:
            print(f"[Cache] Error saving {key}: {e}")
