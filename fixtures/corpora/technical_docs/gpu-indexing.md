# GPU indexing service

## Batch construction

The collector groups chunks into batches of 256. Oversized chunks are isolated so one document cannot exhaust worker memory.

## Device execution

Embedding workers use CUDA streams to overlap host-to-device transfer with model execution. A worker falls back to CPU only when its deployment policy explicitly permits fallback.

## Index publication

A completed batch is written to a staging index. The control plane publishes the index only after checksum validation and retrieval smoke tests pass.

## Troubleshooting

If indexing stalls, compare queue age, device memory, and the last published checksum. High queue age with free device memory usually indicates a blocked staging write.
