"""Generate a tiny self-contained, nested GLB for manual and browser smoke tests."""
import json
import pathlib
import struct

positions, normals, indices = [], [], []
faces = [
    ((0, 0, 1), (1, 0, 0), (0, 1, 0)),
    ((0, 0, -1), (-1, 0, 0), (0, 1, 0)),
    ((1, 0, 0), (0, 0, -1), (0, 1, 0)),
    ((-1, 0, 0), (0, 0, 1), (0, 1, 0)),
    ((0, 1, 0), (1, 0, 0), (0, 0, -1)),
    ((0, -1, 0), (1, 0, 0), (0, 0, 1)),
]
for normal, u, v in faces:
    base = len(positions) // 3
    for x, y in [(-1, -1), (1, -1), (1, 1), (-1, 1)]:
        positions.extend(normal[i] + u[i] * x + v[i] * y for i in range(3))
        normals.extend(normal)
    indices.extend(base + i for i in [0, 1, 2, 0, 2, 3])
position_bytes = struct.pack(f'<{len(positions)}f', *positions)
normal_bytes = struct.pack(f'<{len(normals)}f', *normals)
index_bytes = struct.pack(f'<{len(indices)}H', *indices)
binary = position_bytes + normal_bytes + index_bytes
scene = {
    'asset': {'version': '2.0', 'generator': 'FormCraft test fixture'},
    'scene': 0, 'scenes': [{'name': 'Test assembly', 'nodes': [0]}],
    'nodes': [
        {'name': 'Assembly', 'children': [1], 'translation': [3, -2, 1]},
        {'name': 'Mint cube', 'mesh': 0, 'scale': [1, 1.5, 1]},
    ],
    'meshes': [{'name': 'Cube geometry', 'primitives': [{
        'attributes': {'POSITION': 0, 'NORMAL': 1}, 'indices': 2, 'material': 0,
    }]}],
    'materials': [{'name': 'Mint', 'pbrMetallicRoughness': {
        'baseColorFactor': [0.40, 0.72, 0.54, 1], 'metallicFactor': 0, 'roughnessFactor': 0.7,
    }}],
    'accessors': [
        {'bufferView': 0, 'componentType': 5126, 'count': 24, 'type': 'VEC3', 'min': [-1, -1, -1], 'max': [1, 1, 1]},
        {'bufferView': 1, 'componentType': 5126, 'count': 24, 'type': 'VEC3'},
        {'bufferView': 2, 'componentType': 5123, 'count': 36, 'type': 'SCALAR'},
    ],
    'bufferViews': [
        {'buffer': 0, 'byteOffset': 0, 'byteLength': len(position_bytes)},
        {'buffer': 0, 'byteOffset': len(position_bytes), 'byteLength': len(normal_bytes)},
        {'buffer': 0, 'byteOffset': len(position_bytes) + len(normal_bytes), 'byteLength': len(index_bytes)},
    ],
    'buffers': [{'byteLength': len(binary)}],
}
json_bytes = json.dumps(scene, separators=(',', ':')).encode()
json_bytes += b' ' * (-len(json_bytes) % 4)
binary += b'\0' * (-len(binary) % 4)
body = struct.pack('<I4s', len(json_bytes), b'JSON') + json_bytes
body += struct.pack('<I4s', len(binary), b'BIN\0') + binary
out = pathlib.Path(__file__).resolve().parent.parent / 'tests/fixtures/assembly.glb'
out.parent.mkdir(parents=True, exist_ok=True)
out.write_bytes(b'glTF' + struct.pack('<II', 2, 12 + len(body)) + body)
print(out)
