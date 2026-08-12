"""Emit one local RAG-shaped OTLP trace without calling a model or hosted service."""

import os

from opentelemetry import trace
from opentelemetry.exporter.otlp.proto.http.trace_exporter import OTLPSpanExporter
from opentelemetry.sdk.resources import Resource
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import SimpleSpanProcessor


def required(name: str) -> str:
    value = os.environ.get(name)
    if not value:
        raise SystemExit(f"Set {name} before running this example.")
    return value


api_url = required("CORPUSLAB_API_URL").rstrip("/")
api_key = required("CORPUSLAB_API_KEY")
project_id = required("CORPUSLAB_PROJECT_ID")
provider = TracerProvider(resource=Resource.create({"service.name": "corpuslab-rag-demo"}))
provider.add_span_processor(
    SimpleSpanProcessor(
        OTLPSpanExporter(
            endpoint=f"{api_url}/api/v1/otel/v1/traces",
            headers={
                "Authorization": f"Bearer {api_key}",
                "x-corpuslab-project-id": project_id,
            },
            compression=None,
        )
    )
)
trace.set_tracer_provider(provider)
tracer = trace.get_tracer("corpuslab.trace-ingestion.example", "1.0")

with tracer.start_as_current_span("rag request"):
    with tracer.start_as_current_span("retrieve") as span:
        span.set_attribute("corpuslab.operation", "retrieval")
        span.set_attribute("corpuslab.evidence.external_chunk_id", "otel-policy-7")
        span.set_attribute("corpuslab.evidence.rank", 1)
        span.set_attribute("corpuslab.evidence.score", 0.31)
        span.set_attribute("corpuslab.evidence.citation", "E1")
        span.set_attribute("gen_ai.provider.name", "local-demo")
    with tracer.start_as_current_span("generate") as span:
        span.set_attribute("gen_ai.operation.name", "chat")
        span.set_attribute("gen_ai.request.model", "local-demo-model")
        span.set_attribute("gen_ai.usage.input_tokens", 12)
        span.set_attribute("gen_ai.usage.output_tokens", 8)

provider.shutdown()
print("Sent one metadata-only RAG trace to CorpusLab.")
