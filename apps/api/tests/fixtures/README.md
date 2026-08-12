# OTLP ingestion fixture

`otlp-python-sdk-1.41.1.pb.hex` is the protobuf body captured on 2026-08-12
from `examples/trace-ingestion/python/basic_ingest.py` using the pinned
OpenTelemetry Python SDK and OTLP/HTTP exporter version 1.41.1. The hex form is
intentional so the fixture remains reviewable and free of generated binary
artifacts.

The API integration test decodes this exact body and submits it through the
real Axum router, scoped API-key authentication, OTLP mapper, and trace
repository.
