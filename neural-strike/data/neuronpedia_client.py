"""Neuronpedia API Client for NEURAL-STRIKE"""
import json
import os
import time
import requests
from typing import Optional

class NeuronpediaClient:
    """Interface to Neuronpedia API and S3 exports."""

    def __init__(self, cache_dir: str):
        self.cache_dir = cache_dir
        self.api_base = "https://www.neuronpedia.org/api"
        self.s3_base = "https://neuronpedia-datasets.s3.us-east-1.amazonaws.com/v1"
        self.model_id = "gemma-2-2b"
        self.session = requests.Session()
        self.session.headers.update({
            "Content-Type": "application/json",
            "User-Agent": "NEURAL-STRIKE/1.0"
        })
        self._rate_limit_remaining = 100
        self._rate_limit_reset = 0

    def get_feature(self, layer: int, index: int) -> Optional[dict]:
        """Fetch a single feature from the API."""
        cache_key = f"feature_{layer}_{index}"
        cached = self._load_cache(cache_key)
        if cached:
            return cached

        try:
            url = f"{self.api_base}/feature/{self.model_id}/gemmascope-transcoder-16k/{index}"
            resp = self.session.get(url, timeout=10)
            if resp.status_code == 200:
                data = resp.json()
                self._save_cache(cache_key, data)
                return data
            elif resp.status_code == 429:
                print(f"[API] Rate limited, waiting...")
                time.sleep(60)
                return self.get_feature(layer, index)
        except Exception as e:
            print(f"[API] Error fetching feature {layer}_{index}: {e}")
        return None

    def steer(self, prompt: str, features: list[dict], **kwargs) -> Optional[dict]:
        """Steer model output using SAE features."""
        if self._rate_limit_remaining <= 0:
            if time.time() < self._rate_limit_reset:
                print("[API] Rate limit exceeded, waiting...")
                time.sleep(self._rate_limit_reset - time.time())
            self._rate_limit_remaining = 100

        data = {
            "prompt": prompt,
            "modelId": self.model_id,
            "features": features,
            "temperature": kwargs.get("temperature", 0.2),
            "n_tokens": kwargs.get("n_tokens", 16),
            "freq_penalty": kwargs.get("freq_penalty", 1.0),
            "seed": kwargs.get("seed", 16),
            "strength_multiplier": kwargs.get("strength_multiplier", 4),
        }

        try:
            resp = self.session.post(
                f"{self.api_base}/steer",
                json=data,
                timeout=30
            )
            self._rate_limit_remaining -= 1
            if resp.status_code == 200:
                return resp.json()
        except Exception as e:
            print(f"[API] Steering error: {e}")
        return None

    def get_umap_coordinates(self, layer: int) -> Optional[list]:
        """Get pre-computed U-MAP coordinates for a layer."""
        cache_key = f"umap_layer_{layer}"
        cached = self._load_cache(cache_key)
        if cached:
            return cached

        try:
            url = f"{self.s3_base}/{self.model_id}/umap/{layer}.json"
            resp = self.session.get(url, timeout=30)
            if resp.status_code == 200:
                data = resp.json()
                self._save_cache(cache_key, data)
                return data
        except Exception as e:
            print(f"[S3] Error fetching U-MAP for layer {layer}: {e}")
        return None

    def get_feature_explanations(self, layer: int) -> Optional[dict]:
        """Get auto-interp explanations for a layer's features."""
        cache_key = f"explanations_layer_{layer}"
        cached = self._load_cache(cache_key)
        if cached:
            return cached

        try:
            url = f"{self.s3_base}/{self.model_id}/explanations/{layer}.json"
            resp = self.session.get(url, timeout=30)
            if resp.status_code == 200:
                data = resp.json()
                self._save_cache(cache_key, data)
                return data
        except Exception as e:
            print(f"[S3] Error fetching explanations for layer {layer}: {e}")
        return None

    def search_features(self, query: str, limit: int = 20) -> list[dict]:
        """Search features by keyword in explanations."""
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
            except:
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
