"""Strict catalog loading and complete variable-length semantic references."""
import json
import numpy as np
from model4.structure import structure_digest


class StructureData:
    def __init__(self, path, expected_digest):
        self.data = json.loads(path.read_text())
        self.digest = structure_digest(self.data)
        if self.digest != expected_digest:
            raise ValueError("Card structure checksum mismatch; reconvert/export together")
        if self.data["version"] != 1:
            raise ValueError("Unsupported card structure schema")
        self.roots = self.data["roots"]

    def pack(self, rows, count):
        if len(rows) > count:
            raise ValueError("Semantic references would be truncated")
        result = np.zeros((count, max(1, max(map(len, rows), default=0)), 2), dtype=np.int64)
        for row, refs in enumerate(rows):
            for j, ref in enumerate(refs):
                key, role = ref["key"], ref["role"]
                if key not in self.roots or not 1 <= role <= 7:
                    raise ValueError(f"Unrepresented public rule reference: {ref}")
                result[row, j] = (self.roots[key], role)
        return result
